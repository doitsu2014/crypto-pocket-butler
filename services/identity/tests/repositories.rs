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
use identity_application::authorisation::ports::StatusReader;
use identity_application::jurisdiction::ListRepository;
use identity_application::ports::{
    ConsentRepository, DeletionRepository, DeviceSessionRepository, DueUserRepository,
    ExportRepository, JurisdictionRepository, RepositoryError, ShreddingKeyRepository,
    SupportGrantRepository, UserRepository,
};
use identity_domain::claims::{AssuranceLevel, Role, VerifiedClaims};
use identity_domain::consent::{ConsentPurpose, ConsentRecord, ConsentSource};
use identity_domain::deletion::{DeletionRequest, DeletionStatus};
use identity_domain::export::{Contributor, ExportRequest, ExportStatus};
use identity_domain::ids::{
    ConsentRecordId, DeletionRequestId, DeviceSessionId, ExportRequestId, SupportGrantId, UserId,
};
use identity_domain::jurisdiction::{
    CountryCode, JurisdictionListEntry, JurisdictionListVersion, JurisdictionStatus,
};
use identity_domain::session::DeviceSession;
use identity_domain::support::{SupportAccessGrant, SupportGrantStatus};
use identity_domain::user::{User, UserStatus};
use identity_domain::{ShreddingKey, ShreddingKeyId, ShreddingKeyStatus};
use identity_infrastructure::authorisation::status::StatusReaderLocal;
use identity_infrastructure::authorisation::status_cache::StatusCache;
use identity_infrastructure::repositories::{
    PgConsentRepository, PgDeletionRepository, PgDeviceSessionRepository, PgExportRepository,
    PgJurisdictionListRepository, PgJurisdictionRepository, PgShreddingKeyRepository,
    PgSupportGrantRepository, PgUserRepository,
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
        display_name: Some("Test User".to_string()),
        assurance: AssuranceLevel::Aal1,
        has_exchange_credentials: false,
        session_id: None,
        user_agent: None,
        location: None,
        country_code: Some("VN".to_string()),
        client_ip: Some("192.0.2.10".to_string()),
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
    let (users, mut user, _key, id) = stored_user(&pool).await;

    let found = users
        .find_by_auth_subject(&claims().subject)
        .await
        .unwrap()
        .expect("the stored user is found by its subject");
    assert_eq!(found.id(), id);
    assert_eq!(
        found.status(),
        UserStatus::Active,
        "a verified provider email starts the record active"
    );

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

    // Erasure accepts only an account the lifecycle moved to `pending_deletion`
    // (U5). Drive the stored user through the guarded edge, then assert the
    // at-rest protection survives the erasure itself.
    user.transition(UserStatus::PendingDeletion, at(10))
        .unwrap();
    users.update(&user).await.unwrap();

    user.anonymise(at(11));
    users.anonymise(&user).await.unwrap();
    // The erased record keeps its pseudonymous hash (BR5.8): a still-valid
    // token resolves to the `deleted` tombstone and is denied, rather than
    // being treated as an unknown subject and provisioned anew. What erasure
    // removes is the personal data and the encrypted subject, not the fact
    // that this subject once had an account.
    let erased = users
        .find_by_auth_subject(&claims().subject)
        .await
        .unwrap()
        .expect("the erased tombstone still answers its old subject");
    assert_eq!(erased.id(), id);
    assert_eq!(erased.status(), UserStatus::Deleted);
    assert_eq!(erased.email(), None, "the personal fields are erased");
    assert_eq!(users.reveal_auth_subject(id).await.unwrap(), None);

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn the_erasure_guard_refuses_an_account_that_is_not_pending_deletion() {
    let (pool, schema) = common::test_pool().await;
    let (users, user, _key, id) = stored_user(&pool).await;
    assert_eq!(user.status(), UserStatus::Active);

    let error = users
        .anonymise(&user)
        .await
        .expect_err("an active account is not pending deletion, so the erasure is refused");
    assert!(
        matches!(error, RepositoryError::Conflict(_)),
        "the guard refuses with a conflict, got {error:?}"
    );

    assert!(
        users
            .find_by_auth_subject(&claims().subject)
            .await
            .unwrap()
            .is_some(),
        "the refused erasure leaves the record addressable"
    );
    assert_eq!(
        users.reveal_auth_subject(id).await.unwrap().as_deref(),
        Some(claims().subject.as_str()),
        "the refused erasure leaves the protected subject intact"
    );

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
        at(9) + Duration::minutes(1),
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

    let after_window = at(9) + chrono::Duration::days(31);
    let mut claimed = deletions
        .claim_next("worker-1", after_window, Duration::seconds(60))
        .await
        .unwrap()
        .expect("a due request is claimable");
    assert_eq!(claimed.status(), DeletionStatus::InProgress);

    claimed.fail("ledger timed out", after_window).unwrap();
    claimed
        .record_attempt_failure(after_window)
        .expect("the failed attempt backs off");
    deletions.update(&claimed).await.unwrap();

    let not_yet = deletions
        .claim_next(
            "worker-2",
            after_window + Duration::seconds(1),
            Duration::seconds(60),
        )
        .await
        .unwrap();
    assert!(
        not_yet.is_none(),
        "a backed-off request is not claimed before its next attempt"
    );

    let retried = deletions
        .claim_next(
            "worker-2",
            after_window + Duration::seconds(61),
            Duration::seconds(60),
        )
        .await
        .unwrap()
        .expect("a failed request is retryable once the backoff elapses");
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

// ---------------------------------------------------------------------------
// U6 — the jurisdiction list adapter (BR1.6, BR1.10, W3)
// ---------------------------------------------------------------------------

fn list_entry(code: &str, status: JurisdictionStatus) -> JurisdictionListEntry {
    JurisdictionListEntry {
        code: CountryCode::parse(code).expect("a fixture country code is valid"),
        status,
        source: "compliance".to_string(),
        updated_at: at(9),
    }
}

fn list_fixture(
    version: &str,
    countries: &[(&str, JurisdictionStatus)],
) -> JurisdictionListVersion {
    JurisdictionListVersion::new(
        version,
        at(9),
        countries
            .iter()
            .map(|(code, status)| list_entry(code, *status))
            .collect(),
    )
    .expect("a complete list is valid")
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn jurisdiction_a_published_list_version_round_trips_with_its_rules_and_checksum() {
    let (pool, schema) = common::test_pool().await;
    let lists = PgJurisdictionListRepository::new(pool.clone());

    assert!(
        lists.active().await.unwrap().is_none(),
        "an empty store has no active version"
    );

    let list = list_fixture(
        "2026-11-10.1",
        &[
            ("VN", JurisdictionStatus::Allowed),
            ("KP", JurisdictionStatus::Restricted),
        ],
    );
    lists.publish(&list).await.unwrap();

    let active = lists
        .active()
        .await
        .unwrap()
        .expect("the published version is active");
    assert_eq!(active.version(), "2026-11-10.1");
    assert_eq!(active.entry_count(), 2);
    assert_eq!(active.checksum(), list.checksum());
    assert_eq!(
        active.status_for(&CountryCode::parse("KP").unwrap()),
        JurisdictionStatus::Restricted
    );

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn jurisdiction_a_new_publish_supersedes_the_previous_version_which_stays_readable() {
    let (pool, schema) = common::test_pool().await;
    let lists = PgJurisdictionListRepository::new(pool.clone());

    let first = list_fixture("2026-11-10.1", &[("VN", JurisdictionStatus::Allowed)]);
    let second = list_fixture(
        "2026-11-10.2",
        &[
            ("VN", JurisdictionStatus::Allowed),
            ("US", JurisdictionStatus::Restricted),
        ],
    );
    lists.publish(&first).await.unwrap();
    lists.publish(&second).await.unwrap();

    let active = lists.active().await.unwrap().expect("a version is active");
    assert_eq!(active.version(), "2026-11-10.2");
    assert_eq!(active.entry_count(), 2);

    let first_state: String =
        sqlx::query_scalar("SELECT status FROM jurisdiction_list_versions WHERE version = $1")
            .bind("2026-11-10.1")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(first_state, "superseded");

    let first_rules: i64 =
        sqlx::query_scalar("SELECT count(*) FROM jurisdiction_list_rules WHERE version = $1")
            .bind("2026-11-10.1")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        first_rules, 1,
        "the superseded version keeps its rules so old decisions keep a citation"
    );

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn jurisdiction_a_stored_list_with_a_mismatched_checksum_is_refused() {
    let (pool, schema) = common::test_pool().await;
    let lists = PgJurisdictionListRepository::new(pool.clone());

    // Simulate a partial load: the stored checksum does not cover the rules.
    sqlx::query(
        "INSERT INTO jurisdiction_list_versions (version, updated_at, entry_count, checksum, status)
         VALUES ('2026-11-10.9', $1, 2, $2, 'active')",
    )
    .bind(at(9))
    .bind("0".repeat(64))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO jurisdiction_list_rules (version, country_code, status, source, updated_at)
         VALUES ('2026-11-10.9', 'VN', 'allowed', 'compliance', $1)",
    )
    .bind(at(9))
    .execute(&pool)
    .await
    .unwrap();

    let error = lists
        .active()
        .await
        .expect_err("a mismatched checksum is never served");
    assert!(
        matches!(error, RepositoryError::InvalidData(_)),
        "the refusal names invalid stored data, got {error:?}"
    );

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn user_registry_provision_writes_the_user_the_key_and_the_subject_atomically() {
    let (pool, schema) = common::test_pool().await;
    let users = PgUserRepository::new(pool.clone());

    let user = User::provision(UserId::generate(), &claims(), at(9));
    let key = users
        .provision(&user, &claims().subject, at(9))
        .await
        .expect("the first provisioning writes the record and its key");
    assert_eq!(key.user_id(), user.id());

    let found = users
        .find_by_auth_subject(&claims().subject)
        .await
        .unwrap()
        .expect("the provisioned user resolves by subject");
    assert_eq!(found.id(), user.id());
    assert_eq!(found.display_name(), Some("Test User"));
    assert_eq!(found.status(), UserStatus::Active);

    let keys: i64 = sqlx::query_scalar("SELECT count(*) FROM shredding_keys WHERE user_id = $1")
        .bind(user.id().as_uuid())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        keys, 1,
        "the key reference is created in the same transaction"
    );

    // A second provision for the same subject conflicts; the transaction rolls
    // back, leaving exactly one user, one key, and one personal row.
    let duplicate = User::provision(UserId::generate(), &claims(), at(10));
    let error = users
        .provision(&duplicate, &claims().subject, at(10))
        .await
        .expect_err("one record per provider subject");
    assert!(matches!(error, RepositoryError::Conflict(_)), "{error:?}");

    let counts: (i64, i64, i64) = sqlx::query_as(
        "SELECT
            (SELECT count(*) FROM users),
            (SELECT count(*) FROM shredding_keys),
            (SELECT count(*) FROM user_personal)",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        counts,
        (1, 1, 1),
        "a failed provision leaves no partial row"
    );

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn user_registry_profile_overrides_round_trip_and_the_email_lookup_is_exact() {
    let (pool, schema) = common::test_pool().await;
    let users = PgUserRepository::new(pool.clone());

    let mut user = User::provision(UserId::generate(), &claims(), at(9));
    users
        .provision(&user, &claims().subject, at(9))
        .await
        .unwrap();
    user.update_profile(
        Some("Dana".to_string()),
        Some("dana@example.com".to_string()),
        at(10),
    )
    .unwrap();
    users.update(&user).await.unwrap();

    let found = users.find_by_id(user.id()).await.unwrap().unwrap();
    assert_eq!(found.display_name(), Some("Dana"));
    assert_eq!(found.email(), Some("dana@example.com"));
    assert!(found.display_name_overridden());
    assert!(found.email_overridden());
    assert!(
        !found.email_verified(),
        "an overridden address is unverified"
    );

    let by_email = users
        .find_by_email("dana@example.com")
        .await
        .unwrap()
        .expect("the admin lookup finds the exact address");
    assert_eq!(by_email.id(), user.id());
    assert!(
        users
            .find_by_email("DANA@example.com")
            .await
            .unwrap()
            .is_none(),
        "the lookup is exact, not case-folded"
    );

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn user_registry_a_hash_collision_is_refused_without_returning_the_record() {
    let (pool, schema) = common::test_pool().await;
    let users = PgUserRepository::new(pool.clone());

    let user = User::provision(UserId::generate(), &claims(), at(9));
    users
        .provision(&user, &claims().subject, at(9))
        .await
        .unwrap();

    // Force the stored lookup hash to name a different subject while the
    // ciphertext still decrypts to the original one: a collision.
    let other_subject = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
    sqlx::query("UPDATE user_personal SET auth_subject_hash = $1 WHERE user_id = $2")
        .bind(identity_application::user_registry::subject_hash(
            other_subject,
        ))
        .bind(user.id().as_uuid())
        .execute(&pool)
        .await
        .unwrap();

    let error = users
        .find_by_auth_subject(other_subject)
        .await
        .expect_err("a collision fails closed");
    assert!(
        matches!(error, RepositoryError::Validation(_)),
        "the refusal is a validation error, got {error:?}"
    );

    // The refusal is stable and leaves the record untouched: a retry does not
    // start returning the record the presented subject does not own, and no
    // second user was created for it.
    assert!(
        users.find_by_auth_subject(other_subject).await.is_err(),
        "a retry is refused the same way"
    );
    let users_count: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(users_count, 1, "the collision creates no record");
    let stored_status: String = sqlx::query_scalar("SELECT status FROM users WHERE id = $1")
        .bind(user.id().as_uuid())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stored_status, "active", "the original record is untouched");

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn user_registry_migration_closes_the_status_set_and_the_down_maps_new_states() {
    let (pool, schema) = common::test_pool().await;

    sqlx::query(
        "INSERT INTO users (id, role, status, locale, base_currency, timezone, created_at, updated_at)
         VALUES ($1, 'user', 'disabled', 'en', 'USD', 'UTC', $2, $2)",
    )
    .bind(UserId::generate().as_uuid())
    .bind(at(9))
    .execute(&pool)
    .await
    .expect("the widened status check accepts disabled");
    sqlx::query(
        "INSERT INTO users (id, role, status, locale, base_currency, timezone, created_at, updated_at)
         VALUES ($1, 'user', 'pending_deletion', 'en', 'USD', 'UTC', $2, $2)",
    )
    .bind(UserId::generate().as_uuid())
    .bind(at(9))
    .execute(&pool)
    .await
    .expect("the widened status check accepts pending_deletion");

    let unknown = sqlx::query(
        "INSERT INTO users (id, role, status, locale, base_currency, timezone, created_at, updated_at)
         VALUES ($1, 'user', 'archived', 'en', 'USD', 'UTC', $2, $2)",
    )
    .bind(UserId::generate().as_uuid())
    .bind(at(9))
    .execute(&pool)
    .await;
    assert!(unknown.is_err(), "a status outside the set is refused");

    // 0008, 0007, 0006 and 0005 are newer than 0004, so the first four
    // rollbacks revert them; the fifth is the 0004 down this test exercises.
    identity_infrastructure::migrations::revert(&pool)
        .await
        .expect("the 0008 down migration applies");
    identity_infrastructure::migrations::revert(&pool)
        .await
        .expect("the 0007 down migration applies");
    identity_infrastructure::migrations::revert(&pool)
        .await
        .expect("the 0006 down migration applies");
    identity_infrastructure::migrations::revert(&pool)
        .await
        .expect("the 0005 down migration applies");
    identity_infrastructure::migrations::revert(&pool)
        .await
        .expect("the 0004 down migration applies to a populated table");
    let statuses: Vec<String> = sqlx::query_scalar("SELECT status FROM users ORDER BY status")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(statuses, vec!["active".to_string(), "active".to_string()]);

    let columns: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM information_schema.columns
         WHERE table_schema = current_schema()
           AND table_name = 'users'
           AND column_name IN ('display_name', 'display_name_overridden', 'email_overridden')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(columns, 0, "the down migration removes the new columns");

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn user_registry_reveal_auth_subject_is_none_for_an_unknown_user() {
    let (pool, schema) = common::test_pool().await;
    let users = PgUserRepository::new(pool.clone());
    let unknown = UserId::generate();

    assert_eq!(
        users.reveal_auth_subject(unknown).await.unwrap(),
        None,
        "an unknown user has no personal row to decrypt"
    );
    assert!(
        users.find_by_id(unknown).await.unwrap().is_none(),
        "an unknown id is a `None`, never an error"
    );

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn shredding_keys_active_for_user_returns_the_live_reference_and_destroy_refuses_an_unknown_key(
) {
    let (pool, schema) = common::test_pool().await;
    let keys = PgShreddingKeyRepository::new(pool.clone());
    let user_id = UserId::generate();

    assert!(
        keys.active_for_user(user_id).await.unwrap().is_none(),
        "a user without a key is a `None`, never an error"
    );
    let key = keys.create_for_user(user_id, at(9)).await.unwrap();
    let active = keys
        .active_for_user(user_id)
        .await
        .unwrap()
        .expect("the created key is the active one");
    assert_eq!(active.id(), key.id());
    assert_eq!(active.user_id(), user_id);

    let error = keys
        .destroy(ShreddingKeyId::generate(), at(10))
        .await
        .expect_err("destroying an unknown key is refused");
    assert!(
        matches!(error, RepositoryError::Conflict(_)),
        "the refusal is a conflict, got {error:?}"
    );
    assert!(
        keys.active_for_user(user_id).await.unwrap().is_some(),
        "the refused destroy leaves the live key in place"
    );

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn user_registry_lists_only_active_users_for_the_due_sweep() {
    let (pool, schema) = common::test_pool().await;
    let (users, _active, _key, active_id) = stored_user(&pool).await;

    let other_claims = VerifiedClaims {
        subject: "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb".to_string(),
        email: Some("other@example.com".to_string()),
        ..claims()
    };
    let other = User::provision(UserId::generate(), &other_claims, at(9));
    users
        .provision(&other, &other_claims.subject, at(9))
        .await
        .unwrap();

    // An unconfirmed provider email starts `pending`; the due sweep must skip it.
    let pending_claims = VerifiedClaims {
        subject: "cccccccc-cccc-4ccc-8ccc-cccccccccccc".to_string(),
        email_confirmed: false,
        ..claims()
    };
    let pending = User::provision(UserId::generate(), &pending_claims, at(9));
    users
        .provision(&pending, &pending_claims.subject, at(9))
        .await
        .unwrap();

    let listed = users.list_active(10, 0).await.unwrap();
    let ids: Vec<UserId> = listed.iter().map(|candidate| candidate.user_id).collect();
    assert_eq!(ids.len(), 2, "only the two active users are due");
    assert!(ids.contains(&active_id));
    assert!(ids.contains(&other.id()));

    let first = users.list_active(1, 0).await.unwrap();
    let second = users.list_active(1, 1).await.unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(second.len(), 1);
    assert_ne!(
        first[0].user_id, second[0].user_id,
        "the pages are ordered by id and do not repeat a row"
    );
    assert!(
        users.list_active(10, 2).await.unwrap().is_empty(),
        "an offset past the end is empty"
    );

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn user_registry_a_second_insert_of_the_same_user_conflicts_and_leaves_one_record() {
    let (pool, schema) = common::test_pool().await;
    let (users, user, key, _id) = stored_user(&pool).await;

    let error = users
        .insert(&user, "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb", &key)
        .await
        .expect_err("a user id is inserted once");
    assert!(
        matches!(error, RepositoryError::Conflict(_)),
        "the duplicate id is a conflict, got {error:?}"
    );

    let counts: (i64, i64) =
        sqlx::query_as("SELECT (SELECT count(*) FROM users), (SELECT count(*) FROM user_personal)")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(counts, (1, 1), "the failed insert leaves no partial row");

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn user_registry_queries_report_an_unavailable_store() {
    let (pool, schema) = common::test_pool().await;
    let users = PgUserRepository::new(pool.clone());
    let keys = PgShreddingKeyRepository::new(pool.clone());
    let user = User::provision(UserId::generate(), &claims(), at(9));
    let key = ShreddingKey::from_stored(
        ShreddingKeyId::generate(),
        user.id(),
        format!("identity/user/{}", user.id()),
        1,
        ShreddingKeyStatus::Active,
        at(9),
        None,
    );

    // Drop the schema out from under the pool: every query now fails at the
    // database boundary and must surface as `Unavailable`, never as a panic or
    // a silently empty answer.
    common::drop_schema(&schema).await;

    let errors = vec![
        users.find_by_id(user.id()).await.unwrap_err(),
        users
            .find_by_auth_subject(&claims().subject)
            .await
            .unwrap_err(),
        users
            .find_by_subject_hash(&"00".repeat(32))
            .await
            .unwrap_err(),
        users.find_by_email("user@example.com").await.unwrap_err(),
        users.reveal_auth_subject(user.id()).await.unwrap_err(),
        users.update(&user).await.unwrap_err(),
        users.anonymise(&user).await.unwrap_err(),
        users.list_active(10, 0).await.unwrap_err(),
        users
            .provision(&user, &claims().subject, at(9))
            .await
            .unwrap_err(),
        users
            .insert(&user, &claims().subject, &key)
            .await
            .unwrap_err(),
        keys.create_for_user(user.id(), at(9)).await.unwrap_err(),
        keys.active_for_user(user.id()).await.unwrap_err(),
        keys.destroy(ShreddingKeyId::generate(), at(9))
            .await
            .unwrap_err(),
    ];
    for error in errors {
        assert!(
            matches!(error, RepositoryError::Unavailable(_)),
            "a store failure surfaces as unavailable, got {error:?}"
        );
    }
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn authorisation_status_read_by_user_id_sees_the_stored_status() {
    let (pool, schema) = common::test_pool().await;
    let (users, _active, _key, active_id) = stored_user(&pool).await;

    let keys = PgShreddingKeyRepository::new(pool.clone());
    let disabled = User::from_stored(
        UserId::generate(),
        Role::User,
        UserStatus::Disabled,
        None,
        false,
        None,
        false,
        false,
        identity_domain::Locale::En,
        identity_domain::BaseCurrency::parse("USD").unwrap(),
        identity_domain::TimeZone::parse("UTC").unwrap(),
        at(9),
        at(9),
        None,
    );
    let key = keys.create_for_user(disabled.id(), at(9)).await.unwrap();
    users
        .insert(&disabled, "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb", &key)
        .await
        .unwrap();

    let reader = StatusReaderLocal::new(pool.clone());
    assert_eq!(
        reader.status_of(active_id).await.unwrap(),
        Some(UserStatus::Active)
    );
    assert_eq!(
        reader.status_of(disabled.id()).await.unwrap(),
        Some(UserStatus::Disabled)
    );
    assert_eq!(reader.status_of(UserId::generate()).await.unwrap(), None);

    common::drop_schema(&schema).await;
}

#[tokio::test]
async fn authorisation_cache_is_consulted_only_after_a_failed_read() {
    use std::sync::Arc;

    let id = UserId::generate();
    let reader = Arc::new(common::fakes::FakeStatusReader::default());
    reader.set(id, UserStatus::Active);
    let clock = Arc::new(common::fakes::MutableClock::at(common::fixed_now()));
    let cache = StatusCache::with_limits(
        reader.clone(),
        clock.clone(),
        Arc::new(common::fakes::FakeObservation::default()),
        Duration::seconds(30),
        10,
    );

    // A normal read goes to the source and warms the cache.
    assert_eq!(cache.status_of(id).await.unwrap(), Some(UserStatus::Active));
    assert_eq!(reader.calls(), 1);

    // A failed read is answered from the warm cache.
    reader.fail_next(identity_application::authorisation::ports::StatusReadError::Timeout);
    assert_eq!(cache.status_of(id).await.unwrap(), Some(UserStatus::Active));
    assert_eq!(reader.calls(), 2);

    // A cold cache over a failing source fails closed.
    let cold_reader = Arc::new(common::fakes::FakeStatusReader::default());
    cold_reader.fail_next(identity_application::authorisation::ports::StatusReadError::Timeout);
    let cold = StatusCache::with_limits(
        cold_reader,
        clock,
        Arc::new(common::fakes::FakeObservation::default()),
        Duration::seconds(30),
        10,
    );
    assert!(
        cold.status_of(id).await.is_err(),
        "a cold cache fails closed"
    );
}

#[tokio::test]
async fn authorisation_cache_never_serves_past_thirty_seconds() {
    use std::sync::Arc;

    let id = UserId::generate();
    let reader = Arc::new(common::fakes::FakeStatusReader::default());
    reader.set(id, UserStatus::Active);
    let clock = Arc::new(common::fakes::MutableClock::at(common::fixed_now()));
    let cache = StatusCache::with_limits(
        reader.clone(),
        clock.clone(),
        Arc::new(common::fakes::FakeObservation::default()),
        Duration::seconds(30),
        10,
    );
    assert_eq!(cache.status_of(id).await.unwrap(), Some(UserStatus::Active));

    clock.set(common::fixed_now() + Duration::seconds(29));
    reader.fail_next(identity_application::authorisation::ports::StatusReadError::Timeout);
    assert_eq!(
        cache.status_of(id).await.unwrap(),
        Some(UserStatus::Active),
        "a 29-second-old entry is still inside the window"
    );

    clock.set(common::fixed_now() + Duration::seconds(31));
    reader.fail_next(identity_application::authorisation::ports::StatusReadError::Timeout);
    assert!(
        cache.status_of(id).await.is_err(),
        "a 31-second-old entry is never served"
    );
}

#[tokio::test]
async fn authorisation_cache_evicts_the_least_recently_used_entry_at_its_cap() {
    use std::sync::Arc;

    let a = UserId::generate();
    let b = UserId::generate();
    let c = UserId::generate();
    let reader = Arc::new(common::fakes::FakeStatusReader::default());
    reader.set(a, UserStatus::Active);
    reader.set(b, UserStatus::Disabled);
    reader.set(c, UserStatus::Active);
    let clock = Arc::new(common::fakes::MutableClock::at(common::fixed_now()));
    let cache = StatusCache::with_limits(
        reader.clone(),
        clock,
        Arc::new(common::fakes::FakeObservation::default()),
        Duration::seconds(30),
        2,
    );

    assert_eq!(cache.status_of(a).await.unwrap(), Some(UserStatus::Active));
    assert_eq!(
        cache.status_of(b).await.unwrap(),
        Some(UserStatus::Disabled)
    );

    // Touch A, so B is the least recently used entry.
    reader.fail_next(identity_application::authorisation::ports::StatusReadError::Timeout);
    assert_eq!(cache.status_of(a).await.unwrap(), Some(UserStatus::Active));

    // Warming C over the cap evicts B.
    assert_eq!(cache.status_of(c).await.unwrap(), Some(UserStatus::Active));

    reader.fail_next(identity_application::authorisation::ports::StatusReadError::Timeout);
    assert_eq!(cache.status_of(a).await.unwrap(), Some(UserStatus::Active));
    reader.fail_next(identity_application::authorisation::ports::StatusReadError::Timeout);
    assert!(
        cache.status_of(b).await.is_err(),
        "the least recently used entry was evicted"
    );
}

// ---------------------------------------------------------------------------
// U4 — preferences, consent events, the audit outbox, and migration 0005
// ---------------------------------------------------------------------------

use axum::http::StatusCode;
use std::sync::Arc;

const CONSENT_UP: &str = include_str!("../migrations/0005_preferences_consent.up.sql");
const CONSENT_DOWN: &str = include_str!("../migrations/0005_preferences_consent.down.sql");

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn consent_preferences_upsert_touches_only_the_supplied_columns() {
    use identity_application::preferences::ports::PreferencesRepository;
    use identity_domain::consent::{BaseCurrencyCode, PreferenceUpdate};
    use identity_domain::preferences::Locale;
    use identity_infrastructure::preferences::repository::PgPreferencesRepository;

    let (pool, schema) = common::test_pool().await;
    let (_users, _user, _key, id) = stored_user(&pool).await;
    let preferences = PgPreferencesRepository::new(pool.clone());

    assert_eq!(
        preferences.load(id).await.unwrap(),
        None,
        "a read before the first write creates no row"
    );

    let stored = preferences
        .upsert(
            id,
            &PreferenceUpdate {
                locale: Some(Locale::Vi),
                ..PreferenceUpdate::default()
            },
            at(9),
        )
        .await
        .unwrap();
    assert_eq!(stored.locale, Some(Locale::Vi));
    assert_eq!(stored.base_currency, None);

    let stored = preferences
        .upsert(
            id,
            &PreferenceUpdate {
                base_currency: Some(BaseCurrencyCode::Eur),
                ..PreferenceUpdate::default()
            },
            at(10),
        )
        .await
        .unwrap();
    assert_eq!(
        stored.locale,
        Some(Locale::Vi),
        "a partial write keeps the unsupplied field"
    );
    assert_eq!(stored.base_currency, Some(BaseCurrencyCode::Eur));

    let provider_locale: String = sqlx::query_scalar("SELECT locale FROM users WHERE id = $1")
        .bind(id.as_uuid())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        provider_locale, "en",
        "the write never touches u1's provider-derived column"
    );

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn consent_state_read_returns_the_newest_event_per_kind() {
    use identity_application::consent::ports::ConsentRepository as ConsentEventRepository;
    use identity_domain::consent::{ConsentAction, ConsentEventDraft, ConsentKind, ConsentPolicy};
    use identity_infrastructure::consent::repository::PgConsentEventRepository;

    let (pool, schema) = common::test_pool().await;
    let (_users, _user, _key, id) = stored_user(&pool).await;
    let consent = PgConsentEventRepository::new(pool.clone());
    let version = ConsentPolicy::parse("2026-11-01.1").unwrap();

    for (kind, action) in [
        (ConsentKind::Terms, ConsentAction::Granted),
        (ConsentKind::Terms, ConsentAction::Withdrawn),
        (ConsentKind::Risk, ConsentAction::Granted),
        (ConsentKind::Analytics, ConsentAction::Withdrawn),
    ] {
        consent
            .insert(&ConsentEventDraft::new(id, kind, action, version.clone()))
            .await
            .unwrap();
    }

    let newest = consent.newest_per_kind(id).await.unwrap();
    let state = identity_domain::consent::ConsentStateView::from_newest(id, newest);
    assert_eq!(
        state.state(ConsentKind::Terms),
        identity_domain::consent::ConsentState::Withdrawn
    );
    assert_eq!(
        state.state(ConsentKind::Risk),
        identity_domain::consent::ConsentState::Granted
    );
    assert_eq!(
        state.state(ConsentKind::Age),
        identity_domain::consent::ConsentState::None
    );
    assert_eq!(state.missing(), vec![ConsentKind::Terms, ConsentKind::Age]);

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn consent_history_pages_newest_first_without_repeating_an_event() {
    use identity_application::consent::ports::{
        ConsentRepository as ConsentEventRepository, HistoryCursor,
    };
    use identity_domain::consent::{ConsentAction, ConsentEventDraft, ConsentKind, ConsentPolicy};
    use identity_infrastructure::consent::repository::PgConsentEventRepository;

    let (pool, schema) = common::test_pool().await;
    let (_users, _user, _key, id) = stored_user(&pool).await;
    let consent = PgConsentEventRepository::new(pool.clone());
    let version = ConsentPolicy::parse("2026-11-01.1").unwrap();

    let first = consent
        .insert(&ConsentEventDraft::new(
            id,
            ConsentKind::Terms,
            ConsentAction::Granted,
            version.clone(),
        ))
        .await
        .unwrap();
    let second = consent
        .insert(&ConsentEventDraft::new(
            id,
            ConsentKind::Risk,
            ConsentAction::Granted,
            version.clone(),
        ))
        .await
        .unwrap();
    let third = consent
        .insert(&ConsentEventDraft::new(
            id,
            ConsentKind::Age,
            ConsentAction::Granted,
            version,
        ))
        .await
        .unwrap();

    let page = consent.history(id, 2, None).await.unwrap();
    assert_eq!(page.len(), 2);
    assert_eq!(page[0].id(), third.id());
    assert_eq!(page[1].id(), second.id());

    let next = consent
        .history(
            id,
            2,
            Some(HistoryCursor {
                occurred_at: page[1].occurred_at(),
                seq: page[1].seq(),
            }),
        )
        .await
        .unwrap();
    assert_eq!(next.len(), 1);
    assert_eq!(next[0].id(), first.id());
    assert!(
        next.iter().all(|event| event.id() != second.id()),
        "the page does not repeat the cursor row"
    );

    common::drop_schema(&schema).await;
}

/// The withdrawal decision the use case makes, used by the serialisation test.
async fn withdraw_terms(
    pool: sqlx::PgPool,
    id: UserId,
) -> Result<bool, identity_application::ports::RepositoryError> {
    use identity_application::consent::ports::{
        AuditIntent, ConsentRepository as ConsentEventRepository,
    };
    use identity_domain::consent::{ConsentAction, ConsentEventDraft, ConsentKind, ConsentPolicy};
    use identity_infrastructure::consent::repository::PgConsentEventRepository;

    let consent = PgConsentEventRepository::new(pool);
    let mut transaction = consent.begin_withdraw(id, ConsentKind::Terms).await?;
    let newest = transaction.newest(ConsentKind::Terms).await?;
    match newest {
        Some(event) if event.action() == ConsentAction::Granted => {
            let version =
                ConsentPolicy::parse(event.policy_version()).unwrap_or_else(|_| unreachable!());
            let draft = ConsentEventDraft::withdraw(id, ConsentKind::Terms, version);
            let stored = transaction.insert_event(&draft).await?;
            let intent =
                AuditIntent::consent_withdrawn(&stored, event.policy_version(), "corr-serialise");
            transaction.enqueue_audit(&intent).await?;
            transaction.commit().await?;
            Ok(true)
        }
        _ => {
            transaction.rollback().await?;
            Ok(false)
        }
    }
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn consent_withdrawals_serialise_on_the_advisory_lock_and_write_the_outbox() {
    use identity_application::consent::ports::ConsentRepository as ConsentEventRepository;
    use identity_domain::consent::{ConsentAction, ConsentEventDraft, ConsentKind, ConsentPolicy};
    use identity_infrastructure::consent::repository::PgConsentEventRepository;

    let (pool, schema) = common::test_pool().await;
    let (_users, _user, _key, id) = stored_user(&pool).await;
    let consent = PgConsentEventRepository::new(pool.clone());
    consent
        .insert(&ConsentEventDraft::new(
            id,
            ConsentKind::Terms,
            ConsentAction::Granted,
            ConsentPolicy::parse("2026-11-01.1").unwrap(),
        ))
        .await
        .unwrap();

    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(2));
    let mut handles = Vec::new();
    for _ in 0..2 {
        let pool = pool.clone();
        let barrier = barrier.clone();
        handles.push(tokio::spawn(async move {
            barrier.wait().await;
            withdraw_terms(pool, id).await.unwrap()
        }));
    }
    let wrote: Vec<bool> = futures_join(handles).await;

    assert_eq!(
        wrote.iter().filter(|wrote| **wrote).count(),
        1,
        "two racing withdrawals produce exactly one event"
    );
    let withdrawn: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM consent_records WHERE user_id = $1 AND kind = 'terms' AND action = 'withdrawn'",
    )
    .bind(id.as_uuid())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(withdrawn, 1);

    let (state, attempts, idempotency): (String, i32, String) =
        sqlx::query_as("SELECT state, attempts, idempotency_key FROM consent_audit_outbox")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(state, "pending");
    assert_eq!(attempts, 0);
    assert!(idempotency.starts_with("consent-withdrawn:"));

    let payload: serde_json::Value = sqlx::query_scalar("SELECT payload FROM consent_audit_outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(payload["action"], "identity.consent.withdrawn");
    assert_eq!(payload["target"], "consent:terms");
    assert_eq!(payload["context"]["previousState"], "granted");
    assert_eq!(payload["context"]["policyVersion"], "2026-11-01.1");

    common::drop_schema(&schema).await;
}

async fn futures_join<T>(handles: Vec<tokio::task::JoinHandle<T>>) -> Vec<T> {
    let mut values = Vec::new();
    for handle in handles {
        values.push(handle.await.unwrap());
    }
    values
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn consent_records_are_append_only_for_the_application_and_erasable_by_the_erasure_role() {
    use identity_application::consent::ports::ConsentRepository as ConsentEventRepository;
    use identity_domain::consent::{ConsentAction, ConsentEventDraft, ConsentKind, ConsentPolicy};
    use identity_infrastructure::consent::repository::PgConsentEventRepository;

    let (pool, schema) = common::test_pool().await;
    let (_users, _user, _key, id) = stored_user(&pool).await;
    let consent = PgConsentEventRepository::new(pool.clone());
    consent
        .insert(&ConsentEventDraft::new(
            id,
            ConsentKind::Terms,
            ConsentAction::Granted,
            ConsentPolicy::parse("2026-11-01.1").unwrap(),
        ))
        .await
        .unwrap();

    let app = common::role_pool(&schema, "identity_app").await;
    let update = sqlx::query("UPDATE consent_records SET action = 'withdrawn'")
        .execute(&app)
        .await;
    assert!(update.is_err(), "the application may not rewrite history");
    let delete = sqlx::query("DELETE FROM consent_records")
        .execute(&app)
        .await;
    assert!(delete.is_err(), "the application may not delete history");
    sqlx::query(
        "INSERT INTO consent_records (user_id, kind, action, occurred_at, policy_version)
         VALUES ($1, 'risk', 'granted', now(), '2026-11-01.1')",
    )
    .bind(id.as_uuid())
    .execute(&app)
    .await
    .expect("the application still appends events");
    app.close().await;

    let erasure = common::role_pool(&schema, "identity_retention").await;
    let removed = sqlx::query("DELETE FROM consent_records WHERE user_id = $1")
        .bind(id.as_uuid())
        .execute(&erasure)
        .await
        .expect("the erasure role may delete the user's consent rows");
    assert_eq!(removed.rows_affected(), 2);
    erasure.close().await;

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn consent_erasure_store_deletes_as_the_erasure_role_and_is_refused_as_identity_app() {
    use identity_application::consent::ports::ConsentRepository as ConsentEventRepository;
    use identity_application::ports::{ErasureStore, RepositoryError};
    use identity_domain::consent::{ConsentAction, ConsentEventDraft, ConsentKind, ConsentPolicy};
    use identity_infrastructure::consent::erasure::PgRetentionStore;
    use identity_infrastructure::consent::repository::PgConsentEventRepository;

    let (pool, schema) = common::test_pool().await;
    let (_users, _user, _key, id) = stored_user(&pool).await;
    let consent = PgConsentEventRepository::new(pool.clone());
    let version = ConsentPolicy::parse("2026-11-01.1").unwrap();

    let seed = |kind, action| ConsentEventDraft::new(id, kind, action, version.clone());
    consent
        .insert(&seed(ConsentKind::Terms, ConsentAction::Granted))
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO user_preferences (user_id, locale, updated_at) VALUES ($1, 'vi', now())",
    )
    .bind(id.as_uuid())
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO consent_records_legacy_archive
             (id, user_id, kind, action, occurred_at, policy_version)
         VALUES (gen_random_uuid(), $1, 'risk', 'granted', now(), 'v0')",
    )
    .bind(id.as_uuid())
    .execute(&pool)
    .await
    .unwrap();

    // The erasure role deletes the consent, preference, and archived rows.
    let erasure_pool = common::role_pool(&schema, "identity_retention").await;
    let removed = PgRetentionStore::new(erasure_pool.clone())
        .erase_consent(id)
        .await
        .expect("the erasure role may delete an erased user's rows");
    assert_eq!(removed, 1);
    let left: i64 = sqlx::query_scalar(
        "SELECT (SELECT count(*) FROM consent_records WHERE user_id = $1)
              + (SELECT count(*) FROM user_preferences WHERE user_id = $1)
              + (SELECT count(*) FROM consent_records_legacy_archive WHERE user_id = $1)",
    )
    .bind(id.as_uuid())
    .fetch_one(&erasure_pool)
    .await
    .unwrap();
    assert_eq!(left, 0, "every erasure-scoped row is gone");
    erasure_pool.close().await;

    // Re-seed and prove the application role is refused the same delete.
    consent
        .insert(&seed(ConsentKind::Risk, ConsentAction::Granted))
        .await
        .unwrap();
    let app_pool = common::role_pool(&schema, "identity_app").await;
    let refused = PgRetentionStore::new(app_pool.clone())
        .erase_consent(id)
        .await;
    assert!(
        matches!(refused, Err(RepositoryError::Forbidden(_))),
        "the append-only revoke still refuses the application, got {refused:?}"
    );
    app_pool.close().await;

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn consent_migration_gives_a_late_legacy_row_a_seq_before_the_constraint() {
    use identity_application::consent::ports::ConsentRepository as ConsentEventRepository;
    use identity_domain::consent::{ConsentKind, ConsentState, ConsentStateView};
    use identity_infrastructure::consent::repository::PgConsentEventRepository;

    let (pool, schema) = common::test_pool_before(5).await;
    let (_users, _user, _key, id) = stored_user(&pool).await;

    // Drive 0005 statement by statement so the test can insert a legacy-shaped
    // row in the window between the first `seq` backfill and the constraint.
    let mut connection = pool.acquire().await.unwrap();
    sqlx::query("SET statement_timeout = 0")
        .execute(&mut *connection)
        .await
        .unwrap();
    let mut injected: Option<ConsentRecordId> = None;
    for statement in identity_infrastructure::migrations::split_statements(CONSENT_UP) {
        sqlx::query(&statement)
            .execute(&mut *connection)
            .await
            .unwrap_or_else(|error| panic!("statement failed: {statement}: {error}"));
        if injected.is_none() && statement.contains("CALL consent_backfill_seq") {
            let late = ConsentRecordId::generate();
            sqlx::query(
                "INSERT INTO consent_records (id, user_id, purpose, granted, recorded_at, source)
                 VALUES ($1, $2, 'analytics', true, $3, 'settings')",
            )
            .bind(late.as_uuid())
            .bind(id.as_uuid())
            .bind(at(12))
            .execute(&pool)
            .await
            .expect("a legacy-shaped insert during the seq window succeeds");
            injected = Some(late);
        }
    }
    drop(connection);

    let late = injected.expect("the seq window was injected");
    let (seq, kind): (Option<i64>, Option<String>) =
        sqlx::query_as("SELECT seq, kind FROM consent_records WHERE id = $1")
            .bind(late.as_uuid())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        seq.is_some(),
        "the default assigned the late row a sequence, so VALIDATE did not abort"
    );
    assert_eq!(kind, None, "the migration does not invent an event shape");

    // The gate still sees it: the read maps the legacy columns.
    let repository = PgConsentEventRepository::new(pool.clone());
    let newest = repository.newest_per_kind(id).await.unwrap();
    let state = ConsentStateView::from_newest(id, newest);
    assert_eq!(state.state(ConsentKind::Analytics), ConsentState::Granted);

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn consent_migration_maps_legacy_rows_and_the_down_collapses_and_archives() {
    let (pool, schema) = common::test_pool_before(5).await;
    let (_users, _user, _key, id) = stored_user(&pool).await;

    for (purpose, granted, source, hour) in [
        ("analytics", true, "onboarding", 9),
        ("marketing", false, "settings", 10),
        ("disclosures", true, "support", 11),
    ] {
        sqlx::query(
            "INSERT INTO consent_records (id, user_id, purpose, granted, recorded_at, source)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(ConsentRecordId::generate().as_uuid())
        .bind(id.as_uuid())
        .bind(purpose)
        .bind(granted)
        .bind(at(hour))
        .bind(source)
        .execute(&pool)
        .await
        .unwrap();
    }

    // The live-safe path applies 0005 statement by statement; the runner is the
    // one the release job uses.
    identity_infrastructure::migrations::run(&pool)
        .await
        .expect("0005 applies over the legacy rows");

    let mapped: Vec<(String, String, DateTime<Utc>, String)> = sqlx::query_as(
        "SELECT kind, action, occurred_at, policy_version
         FROM consent_records ORDER BY occurred_at",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(mapped.len(), 3);
    assert_eq!(mapped[0].0, "analytics");
    assert_eq!(mapped[0].1, "granted");
    assert_eq!(mapped[0].2, at(9));
    assert_eq!(mapped[0].3, "legacy:onboarding");
    assert_eq!(mapped[1].0, "marketing");
    assert_eq!(mapped[1].1, "withdrawn");
    assert_eq!(mapped[2].0, "terms", "disclosures maps one-to-one to terms");
    assert_eq!(mapped[2].3, "legacy:support");

    let missing_seq: i64 =
        sqlx::query_scalar("SELECT count(*) FROM consent_records WHERE seq IS NULL")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(missing_seq, 0, "every row has a monotonic sequence");

    let can_update: bool = sqlx::query_scalar(
        "SELECT has_table_privilege('identity_app', 'consent_records', 'UPDATE')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let can_delete: bool = sqlx::query_scalar(
        "SELECT has_table_privilege('identity_app', 'consent_records', 'DELETE')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!can_update && !can_delete, "the application is append-only");

    // A newer terms withdrawal, and two events the old shape cannot hold.
    sqlx::query(
        "INSERT INTO consent_records (user_id, kind, action, occurred_at, policy_version)
         VALUES ($1, 'terms', 'withdrawn', $2, '2026-11-01.2')",
    )
    .bind(id.as_uuid())
    .bind(at(12))
    .execute(&pool)
    .await
    .unwrap();
    for kind in ["risk", "age"] {
        sqlx::query(
            "INSERT INTO consent_records (user_id, kind, action, occurred_at, policy_version)
             VALUES ($1, $2, 'granted', $3, '2026-11-01.2')",
        )
        .bind(id.as_uuid())
        .bind(kind)
        .bind(at(13))
        .execute(&pool)
        .await
        .unwrap();
    }

    // A legacy-shaped row written by a previous replica during the overlap. It
    // has no event columns and no explicit `seq`; the default must give it one.
    let late = ConsentRecordId::generate();
    sqlx::query(
        "INSERT INTO consent_records (id, user_id, purpose, granted, recorded_at, source)
         VALUES ($1, $2, 'disclosures', true, $3, 'onboarding')",
    )
    .bind(late.as_uuid())
    .bind(id.as_uuid())
    .bind(at(14))
    .execute(&pool)
    .await
    .unwrap();

    let total_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM consent_records WHERE user_id = $1")
            .bind(id.as_uuid())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(total_count, 7, "six event rows and one legacy-shaped row");

    // The down archives every row before collapsing, so a rollback never loses a
    // grant or withdrawal — event-shaped or legacy-shaped. It must be runnable a
    // second time.
    sqlx::raw_sql(CONSENT_DOWN).execute(&pool).await.unwrap();
    let archived_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM consent_records_legacy_archive WHERE user_id = $1",
    )
    .bind(id.as_uuid())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        archived_count, total_count,
        "every row is archived before the collapse"
    );
    let archived_ids: Vec<uuid::Uuid> = sqlx::query_scalar(
        "SELECT id FROM consent_records_legacy_archive WHERE user_id = $1 ORDER BY id",
    )
    .bind(id.as_uuid())
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(
        archived_ids.contains(&late.as_uuid()),
        "the legacy-shaped row is archived too"
    );
    let archived_kinds: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT kind FROM consent_records_legacy_archive
         WHERE user_id = $1 ORDER BY kind",
    )
    .bind(id.as_uuid())
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        archived_kinds,
        vec!["age", "analytics", "marketing", "risk", "terms"]
    );

    let collapsed: Vec<(String, bool)> =
        sqlx::query_as("SELECT purpose, granted FROM consent_records ORDER BY purpose")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        collapsed,
        vec![
            ("analytics".to_string(), true),
            ("disclosures".to_string(), true),
            ("marketing".to_string(), false)
        ],
        "the newest terms-mapped row is the late disclosures grant"
    );

    sqlx::raw_sql(CONSENT_DOWN)
        .execute(&pool)
        .await
        .expect("a second rollback re-runs the IF NOT EXISTS archive path cleanly");

    common::drop_schema(&schema).await;
}

async fn stub_audit(
    status: StatusCode,
) -> (
    String,
    Arc<std::sync::Mutex<Vec<Option<String>>>>,
    tokio::task::JoinHandle<()>,
) {
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorded = seen.clone();
    let app = axum::Router::new().fallback(move |headers: axum::http::HeaderMap| {
        let recorded = recorded.clone();
        async move {
            recorded.lock().unwrap().push(
                headers
                    .get("idempotency-key")
                    .and_then(|value| value.to_str().ok())
                    .map(ToString::to_string),
            );
            status
        }
    });
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let address = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (format!("http://{address}"), seen, handle)
}

async fn seed_outbox(pool: &sqlx::PgPool, user_id: UserId, key: &str, payload: serde_json::Value) {
    sqlx::query(
        "WITH event AS (
             INSERT INTO consent_records (user_id, kind, action, occurred_at, policy_version)
             VALUES ($1, 'terms', 'granted', now(), '2026-11-01.1')
             RETURNING id
         )
         INSERT INTO consent_audit_outbox
             (id, consent_record_id, payload, idempotency_key, state, attempts,
              next_attempt_at, created_at)
         SELECT gen_random_uuid(), id, $2, $3, 'pending', 0, $4, $4 FROM event",
    )
    .bind(user_id.as_uuid())
    .bind(payload)
    .bind(key)
    .bind(common::fixed_now())
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn consent_outbox_drainer_marks_delivery_retry_and_dead_with_the_idempotency_key() {
    use identity_application::consent::ports::DeliveryOutcome as Outcome;
    use identity_infrastructure::consent::outbox::AuditOutboxDrainer;
    use std::time::Duration;

    let (pool, schema) = common::test_pool().await;
    let (_users, _user, _key, id) = stored_user(&pool).await;
    let payload = serde_json::json!({
        "actorId": id.to_string(),
        "action": "identity.consent.withdrawn",
        "target": "consent:terms",
        "occurredAt": common::fixed_now(),
        "outcome": "success",
        "context": { "previousState": "granted", "policyVersion": "2026-11-01.1" },
    });

    // 202: accepted, marked delivered, and the key is sent.
    let (url, seen, handle) = stub_audit(StatusCode::ACCEPTED).await;
    seed_outbox(&pool, id, "consent-withdrawn:accepted", payload.clone()).await;
    let observation = Arc::new(common::fakes::FakeObservationSink::default());
    let drainer = AuditOutboxDrainer::new(
        pool.clone(),
        url,
        "service-token",
        observation.clone(),
        Duration::from_secs(60),
    );
    drainer.run_once(common::fixed_now()).await;
    let (state, attempts): (String, i32) = sqlx::query_as(
        "SELECT state, attempts FROM consent_audit_outbox WHERE idempotency_key = $1",
    )
    .bind("consent-withdrawn:accepted")
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(state, "delivered");
    assert_eq!(attempts, 1);
    assert_eq!(
        seen.lock().unwrap().as_slice(),
        [Some("consent-withdrawn:accepted".to_string())]
    );
    assert_eq!(
        observation.deliveries.lock().unwrap().as_slice(),
        [Outcome::Delivered]
    );
    handle.abort();

    // 500: retryable, attempts advanced, next attempt in the future.
    let (url, _seen, handle) = stub_audit(StatusCode::INTERNAL_SERVER_ERROR).await;
    seed_outbox(&pool, id, "consent-withdrawn:retry", payload.clone()).await;
    let drainer = AuditOutboxDrainer::new(
        pool.clone(),
        url,
        "service-token",
        observation.clone(),
        Duration::from_secs(60),
    );
    drainer.run_once(common::fixed_now()).await;
    let (state, attempts): (String, i32) = sqlx::query_as(
        "SELECT state, attempts FROM consent_audit_outbox WHERE idempotency_key = $1",
    )
    .bind("consent-withdrawn:retry")
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(state, "pending");
    assert_eq!(attempts, 1);
    let next_attempt: DateTime<Utc> = sqlx::query_scalar(
        "SELECT next_attempt_at FROM consent_audit_outbox WHERE idempotency_key = $1",
    )
    .bind("consent-withdrawn:retry")
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(next_attempt > common::fixed_now());
    assert_eq!(observation.deliveries.lock().unwrap().len(), 2);
    handle.abort();

    // 400: permanent, marked dead and never retried.
    let (url, _seen, handle) = stub_audit(StatusCode::BAD_REQUEST).await;
    seed_outbox(&pool, id, "consent-withdrawn:dead", payload).await;
    let drainer = AuditOutboxDrainer::new(
        pool.clone(),
        url,
        "service-token",
        observation.clone(),
        Duration::from_secs(60),
    );
    drainer.run_once(common::fixed_now()).await;
    let (state, attempts): (String, i32) = sqlx::query_as(
        "SELECT state, attempts FROM consent_audit_outbox WHERE idempotency_key = $1",
    )
    .bind("consent-withdrawn:dead")
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(state, "dead");
    assert_eq!(attempts, 1);
    handle.abort();

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn consent_outbox_drainer_refuses_a_token_shaped_payload_without_sending_it() {
    use identity_infrastructure::consent::outbox::AuditOutboxDrainer;
    use std::time::Duration;

    let (pool, schema) = common::test_pool().await;
    let (_users, _user, _key, id) = stored_user(&pool).await;

    let (url, seen, handle) = stub_audit(StatusCode::ACCEPTED).await;
    seed_outbox(
        &pool,
        id,
        "consent-withdrawn:token",
        serde_json::json!({ "actorId": "eyJhbGciOiJIUzI1NiJ9", "action": "identity.consent.withdrawn" }),
    )
    .await;
    let drainer = AuditOutboxDrainer::new(
        pool.clone(),
        url,
        "service-token",
        Arc::new(common::fakes::FakeObservationSink::default()),
        Duration::from_secs(60),
    );
    drainer.run_once(common::fixed_now()).await;

    let state: String =
        sqlx::query_scalar("SELECT state FROM consent_audit_outbox WHERE idempotency_key = $1")
            .bind("consent-withdrawn:token")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(state, "dead", "a token-shaped value is never sent");
    assert!(
        seen.lock().unwrap().is_empty(),
        "the drainer refused the delivery before posting"
    );
    handle.abort();

    common::drop_schema(&schema).await;
}

// ── U7: the settings projections (P1, D0, D1) ──────────────────────────────
//
// These three reads are answered by the existing adapter statements. The tests
// pin the projection each route relies on so a later statement change cannot
// silently drop a field the settings surface renders.

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn the_self_profile_projection_resolves_by_auth_subject_with_both_flags() {
    let (pool, schema) = common::test_pool().await;
    let (users, mut user, _key, id) = stored_user(&pool).await;

    let found = users
        .find_by_auth_subject(&claims().subject)
        .await
        .unwrap()
        .expect("the stored user resolves by its auth subject");
    assert_eq!(found.id(), id);
    assert_eq!(found.display_name(), Some("Test User"));
    assert!(
        !found.display_name_overridden(),
        "a provider-supplied name is not overridden"
    );
    assert_eq!(found.email(), Some("user@example.com"));
    assert!(found.email_verified());
    assert_eq!(found.status(), UserStatus::Active);

    user.update_profile(Some("  Mai Anh  ".to_string()), None, at(10))
        .unwrap();
    users.update(&user).await.unwrap();

    let overridden = users
        .find_by_auth_subject(&claims().subject)
        .await
        .unwrap()
        .expect("the overridden profile still resolves");
    assert_eq!(overridden.display_name(), Some("Mai Anh"));
    assert!(overridden.display_name_overridden());
    assert_eq!(
        overridden.email(),
        Some("user@example.com"),
        "a display-name write never touches the provider email"
    );

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn the_open_deletion_read_returns_only_an_uncancelled_unerased_request() {
    let (pool, schema) = common::test_pool().await;
    let (_users, _user, _key, id) = stored_user(&pool).await;
    let deletions = PgDeletionRepository::new(pool.clone());

    assert!(
        deletions.find_active_for_user(id).await.unwrap().is_none(),
        "no request is a `None`, never an error"
    );

    let mut request = DeletionRequest::request(DeletionRequestId::generate(), id, at(9));
    deletions.insert(&request).await.unwrap();
    let open = deletions
        .find_active_for_user(id)
        .await
        .unwrap()
        .expect("the open request is returned");
    assert_eq!(open.id(), request.id());
    assert_eq!(open.status(), DeletionStatus::Requested);

    request.cancel(at(10)).unwrap();
    deletions.update(&request).await.unwrap();
    assert!(
        deletions.find_active_for_user(id).await.unwrap().is_none(),
        "a cancelled request is history, not the open request"
    );

    let other = UserId::generate();
    assert!(
        deletions
            .find_active_for_user(other)
            .await
            .unwrap()
            .is_none(),
        "the read is scoped to one user"
    );

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with pgsodium (disposable Postgres)"]
async fn the_account_status_read_answers_every_status_by_auth_subject() {
    let (pool, schema) = common::test_pool().await;
    let (users, user, _key, id) = stored_user(&pool).await;

    // `pending` is the transient provisioning state: an unconfirmed provider
    // email starts the record there.
    let pending_claims = VerifiedClaims {
        subject: "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb".to_string(),
        email_confirmed: false,
        ..claims()
    };
    let pending = User::provision(UserId::generate(), &pending_claims, at(9));
    let keys = PgShreddingKeyRepository::new(pool.clone());
    let pending_key = keys.create_for_user(pending.id(), at(9)).await.unwrap();
    users
        .insert(&pending, &pending_claims.subject, &pending_key)
        .await
        .unwrap();
    assert_eq!(
        users
            .find_by_auth_subject(&pending_claims.subject)
            .await
            .unwrap()
            .expect("the pending record resolves")
            .status(),
        UserStatus::Pending,
        "a status read answers `pending` too"
    );

    // Every other status, through the guarded edges from an active record.
    let mut current = user;
    let timeline = [
        (
            UserStatus::Disabled,
            "the disabled status is readable by its subject",
        ),
        (
            UserStatus::Active,
            "the restored status is readable by its subject",
        ),
        (
            UserStatus::PendingDeletion,
            "the pending-deletion status is readable by its subject",
        ),
    ];
    for (next, message) in timeline {
        current.transition(next, at(11)).unwrap();
        users.update(&current).await.unwrap();
        assert_eq!(
            users
                .find_by_auth_subject(&claims().subject)
                .await
                .unwrap()
                .expect(message)
                .status(),
            next,
            "{message}"
        );
    }

    // `deleted` is only reachable through the erasure shape: the row keeps its
    // anonymised marker and drops the personal fields. A status read still
    // answers it by subject, because the subject hash survives until the Auth
    // admin delete completes.
    current.anonymise(at(11));
    users.update(&current).await.unwrap();
    assert_eq!(
        users
            .find_by_auth_subject(&claims().subject)
            .await
            .unwrap()
            .expect("the deleted status is readable by its subject")
            .status(),
        UserStatus::Deleted
    );
    assert_eq!(
        users.find_by_id(id).await.unwrap().unwrap().status(),
        UserStatus::Deleted
    );

    common::drop_schema(&schema).await;
}
