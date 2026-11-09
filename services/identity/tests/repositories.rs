//! Adapter integration tests against a real Postgres schema.
//!
//! Run with `TEST_DATABASE_URL` pointing at a disposable database:
//! `TEST_DATABASE_URL=postgres://... cargo test --manifest-path services/identity/Cargo.toml --test repositories -- --ignored`
//!
//! The database user must be able to create roles and the pgsodium extension:
//! the migration creates the three identity roles idempotently and the
//! protect/unprotect functions call pgsodium.

mod common;

use chrono::{DateTime, Duration, TimeZone, Utc};
use identity_application::ports::{
    ConsentRepository, DeletionRepository, DeviceSessionRepository, ExportRepository,
    JurisdictionRepository, ShreddingKeyRepository, SupportGrantRepository, UserRepository,
};
use identity_domain::claims::{AssuranceLevel, Role, VerifiedClaims};
use identity_domain::consent::{ConsentPurpose, ConsentRecord, ConsentSource};
use identity_domain::deletion::{DeletionRequest, DeletionStatus};
use identity_domain::export::{Contributor, ExportRequest, ExportStatus};
use identity_domain::ids::{
    ConsentRecordId, DeletionRequestId, DeviceSessionId, ExportRequestId, SupportGrantId, UserId,
};
use identity_domain::session::DeviceSession;
use identity_domain::support::{SupportAccessGrant, SupportGrantStatus};
use identity_domain::user::{User, UserStatus};
use identity_infrastructure::repositories::{
    PgConsentRepository, PgDeletionRepository, PgDeviceSessionRepository, PgExportRepository,
    PgJurisdictionRepository, PgShreddingKeyRepository, PgSupportGrantRepository, PgUserRepository,
};

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, hour, 0, 0).unwrap()
}

fn claims() -> VerifiedClaims {
    VerifiedClaims {
        subject: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa".to_string(),
        role: Role::User,
        email_confirmed: true,
        email: Some("user@example.com".to_string()),
        assurance: AssuranceLevel::Aal1,
        has_exchange_credentials: false,
        session_id: None,
        user_agent: None,
        location: None,
    }
}

async fn stored_user(
    pool: &sqlx::PgPool,
) -> (
    PgUserRepository,
    User,
    identity_domain::ShreddingKey,
    UserId,
) {
    let users = PgUserRepository::new(pool.clone());
    let keys = PgShreddingKeyRepository::new(pool.clone());
    let user = User::provision(UserId::generate(), &claims(), at(9));
    let key = keys.create_for_user(user.id(), at(9)).await.unwrap();
    users.insert(&user, &claims().subject, &key).await.unwrap();
    let id = user.id();
    (users, user, key, id)
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn a_user_is_found_by_the_hashed_subject_and_the_subject_is_protected_at_rest() {
    let (pool, schema) = common::test_pool().await;
    let (users, user, _key, id) = stored_user(&pool).await;

    let found = users
        .find_by_auth_subject(&claims().subject)
        .await
        .unwrap()
        .expect("the stored user is found by its subject");
    assert_eq!(found.id(), id);
    assert_eq!(found.status(), UserStatus::Pending);

    let stored: Vec<u8> =
        sqlx::query_scalar("SELECT auth_subject_protected FROM user_personal WHERE user_id = $1")
            .bind(id.as_uuid())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_ne!(
        stored,
        claims().subject.as_bytes(),
        "the subject is never stored in clear text"
    );

    assert_eq!(
        users.reveal_auth_subject(id).await.unwrap().as_deref(),
        Some(claims().subject.as_str()),
        "unprotect returns the subject for the Auth admin delete"
    );

    users.anonymise(&user).await.unwrap();
    assert!(
        users
            .find_by_auth_subject(&claims().subject)
            .await
            .unwrap()
            .is_none(),
        "an anonymised user is no longer resolvable by its old subject"
    );
    assert_eq!(users.reveal_auth_subject(id).await.unwrap(), None);

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn consent_latest_returns_the_newest_decision_and_delete_removes_the_history() {
    let (pool, schema) = common::test_pool().await;
    let (_users, user, _key, id) = stored_user(&pool).await;
    let consent = PgConsentRepository::new(pool.clone());

    let older = ConsentRecord::record(
        ConsentRecordId::generate(),
        id,
        ConsentPurpose::Analytics,
        true,
        ConsentSource::Onboarding,
        at(9),
    );
    let newer = ConsentRecord::record(
        ConsentRecordId::generate(),
        id,
        ConsentPurpose::Analytics,
        false,
        ConsentSource::Settings,
        at(10),
    );
    consent.append(&older).await.unwrap();
    consent.append(&newer).await.unwrap();

    let latest = consent
        .latest(id, ConsentPurpose::Analytics)
        .await
        .unwrap()
        .expect("a decision exists");
    assert_eq!(latest.id(), newer.id());
    assert!(!latest.granted());
    assert_eq!(
        consent.latest(id, ConsentPurpose::Marketing).await.unwrap(),
        None,
        "a missing purpose is not granted"
    );

    assert_eq!(consent.delete_for_user(id).await.unwrap(), 2);
    assert_eq!(
        consent.latest(id, ConsentPurpose::Analytics).await.unwrap(),
        None
    );
    let _ = user;

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn a_device_session_upserts_one_row_per_auth_session() {
    let (pool, schema) = common::test_pool().await;
    let (_users, _user, _key, id) = stored_user(&pool).await;
    let sessions = PgDeviceSessionRepository::new(pool.clone());

    let first = DeviceSession::new(
        DeviceSessionId::generate(),
        id,
        "auth-session-1",
        Some("Firefox".to_string()),
        Some("Hanoi, VN".to_string()),
        at(9),
    );
    sessions.upsert(&first).await.unwrap();

    let mut second = DeviceSession::new(
        DeviceSessionId::generate(),
        id,
        "auth-session-1",
        Some("Firefox 130".to_string()),
        None,
        at(10),
    );
    second.touch(Some("Firefox 130".to_string()), None, at(10));
    sessions.upsert(&second).await.unwrap();

    let listed = sessions.list_for_user(id).await.unwrap();
    assert_eq!(listed.len(), 1, "one row per (user, auth session)");
    assert_eq!(listed[0].last_activity_at(), at(10));
    assert_eq!(listed[0].location(), None);

    let mut stored = listed[0].clone();
    stored.revoke(at(11));
    sessions.update(&stored).await.unwrap();
    let found = sessions.find(stored.id()).await.unwrap().unwrap();
    assert!(found.is_revoked());

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn a_support_grant_round_trips_and_reads_as_expired_after_its_window() {
    let (pool, schema) = common::test_pool().await;
    let (_users, _user, _key, target) = stored_user(&pool).await;
    let grants = PgSupportGrantRepository::new(pool.clone());

    let mut grant = SupportAccessGrant::request(
        SupportGrantId::generate(),
        target,
        UserId::generate(),
        "investigate a reported sync failure",
        at(9),
    )
    .unwrap();
    grants.insert(&grant).await.unwrap();
    grant
        .approve(
            UserId::generate(),
            Role::Admin,
            at(10),
            Duration::minutes(60),
        )
        .unwrap();
    grants.update(&grant).await.unwrap();

    let found = grants.find_by_id(grant.id()).await.unwrap().unwrap();
    assert_eq!(found.status(), SupportGrantStatus::Active);
    assert_eq!(found.expires_at(), Some(at(10) + Duration::minutes(60)));
    assert_eq!(found.effective_status(at(12)), SupportGrantStatus::Expired);

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn export_claims_are_atomic_and_an_expired_lease_is_reclaimed() {
    let (pool, schema) = common::test_pool().await;
    let (_users, _user, _key, id) = stored_user(&pool).await;
    let exports = PgExportRepository::new(pool.clone());

    let first = ExportRequest::request(
        ExportRequestId::generate(),
        id,
        at(9),
        at(9) + chrono::Duration::minutes(15),
    );
    exports.insert(&first).await.unwrap();

    let claimed = exports
        .claim_next("worker-1", at(9), Duration::seconds(60))
        .await
        .unwrap()
        .expect("a request is claimable");
    assert_eq!(claimed.status(), ExportStatus::Building);
    assert_eq!(claimed.lease_owner(), Some("worker-1"));

    let reclaimed = exports
        .claim_next(
            "worker-2",
            at(9) + chrono::Duration::seconds(61),
            Duration::seconds(60),
        )
        .await
        .unwrap()
        .expect("an expired lease is reclaimed");
    assert_eq!(reclaimed.id(), claimed.id());
    assert_eq!(reclaimed.lease_owner(), Some("worker-2"));

    let active = exports.find_active_for_user(id).await.unwrap();
    assert_eq!(active.map(|export| export.id()), Some(first.id()));

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn deletion_claims_are_atomic_and_a_failed_request_is_retryable() {
    let (pool, schema) = common::test_pool().await;
    let (_users, _user, _key, id) = stored_user(&pool).await;
    let deletions = PgDeletionRepository::new(pool.clone());

    let request = DeletionRequest::request(DeletionRequestId::generate(), id, at(9));
    deletions.insert(&request).await.unwrap();

    let mut claimed = deletions
        .claim_next("worker-1", at(9), Duration::seconds(60))
        .await
        .unwrap()
        .expect("a request is claimable");
    assert_eq!(claimed.status(), DeletionStatus::InProgress);

    claimed.fail("ledger timed out", at(10)).unwrap();
    deletions.update(&claimed).await.unwrap();

    let retried = deletions
        .claim_next("worker-2", at(11), Duration::seconds(60))
        .await
        .unwrap()
        .expect("a failed request is retryable");
    assert_eq!(retried.status(), DeletionStatus::InProgress);

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn the_jurisdiction_seed_allows_the_launch_markets_and_blocks_everything_else() {
    let (pool, schema) = common::test_pool().await;
    let rules = PgJurisdictionRepository::new(pool.clone());

    let vietnam = rules.find_by_code("VN").await.unwrap().unwrap();
    assert!(vietnam.allowed);
    assert_eq!(vietnam.minimum_age, 18);
    assert!(vietnam.disclosures_required);
    assert!(rules.find_by_code("GB").await.unwrap().is_some());
    assert!(rules.find_by_code("DE").await.unwrap().is_some());
    assert!(
        rules.find_by_code("US").await.unwrap().is_none(),
        "a country outside the launch scope is blocked by an absent rule"
    );

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn protect_and_unprotect_are_security_definer_functions_the_app_may_execute() {
    let (pool, schema) = common::test_pool().await;
    let keys = PgShreddingKeyRepository::new(pool.clone());
    let key = keys
        .create_for_user(UserId::generate(), at(9))
        .await
        .unwrap();

    let app = common::role_pool(&schema, "identity_app").await;
    let protected: Vec<u8> = sqlx::query_scalar("SELECT protect($1, $2)")
        .bind("the-subject")
        .bind(key.key_id())
        .fetch_one(&app)
        .await
        .unwrap();
    let revealed: String = sqlx::query_scalar("SELECT unprotect($1, $2)")
        .bind(&protected)
        .bind(key.key_id())
        .fetch_one(&app)
        .await
        .unwrap();
    assert_eq!(revealed, "the-subject");

    keys.destroy(key.id(), at(10)).await.unwrap();
    assert!(
        sqlx::query_scalar::<_, String>("SELECT unprotect($1, $2)")
            .bind(&protected)
            .bind(key.key_id())
            .fetch_one(&app)
            .await
            .is_err(),
        "a destroyed key no longer decrypts the subject"
    );

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn progress_and_contributions_are_round_tripped_for_a_ready_export() {
    let (pool, schema) = common::test_pool().await;
    let (_users, _user, _key, id) = stored_user(&pool).await;
    let exports = PgExportRepository::new(pool.clone());

    let mut export = ExportRequest::request(
        ExportRequestId::generate(),
        id,
        at(9),
        at(9) + chrono::Duration::minutes(15),
    );
    exports.insert(&export).await.unwrap();
    export
        .claim("worker-1", at(9), Duration::seconds(60))
        .unwrap();
    for contributor in Contributor::ALL {
        export.record_success(contributor, at(9));
    }
    export
        .mark_ready(
            "identity-exports/export.json",
            at(9) + chrono::Duration::hours(24),
            at(9),
        )
        .unwrap();
    exports.update(&export).await.unwrap();

    let found = exports.find_by_id(export.id()).await.unwrap().unwrap();
    assert_eq!(found.progress(), 100);
    assert_eq!(found.status(), ExportStatus::Ready);
    assert_eq!(found.file_reference(), Some("identity-exports/export.json"));
    assert_eq!(
        found
            .contributions()
            .values()
            .filter(|state| **state == identity_domain::export::ContributionState::Succeeded)
            .count(),
        3
    );

    let expired = exports
        .list_expired_ready(at(9) + chrono::Duration::hours(24))
        .await
        .unwrap();
    assert_eq!(expired.len(), 1);

    common::drop_schema(&schema).await;
}
