//! Adapter integration tests against a real Postgres schema.
//!
//! Run with `TEST_DATABASE_URL` pointing at a disposable database:
//! `TEST_DATABASE_URL=postgres://... cargo test -p audit --test repositories -- --ignored`
//!
//! The database user must be able to create roles: the migration creates the four
//! audit roles idempotently so the disposable schema has the same grants as
//! production.

mod common;

use std::collections::BTreeMap;

use audit_application::ports::{
    AuditEventRepository, ExportRepository, PrivacyDeletionRepository, RepositoryError,
    RetentionRepository, SupportSessionRepository,
};
use audit_domain::{
    ActorKind, AuditEvent, AuditEventId, AuditExport, AuditExportId, Context, DeletionRequestId,
    EventClass, ExportFilter, NewAuditEvent, Outcome, PrivacyDeletion, PrivacyDeletionId,
    PrivacyDeletionState, Sensitivity, SupportScope, SupportSession, SupportSessionId,
    SupportSessionOutcome, SupportSessionRequest, SupportSessionState, UserId,
};
use audit_infrastructure::repositories::{
    PgAuditEventRepository, PgExportRepository, PgPrivacyDeletionRepository, PgRetentionRepository,
    PgSupportSessionRepository,
};
use chrono::{TimeZone, Utc};

fn instant(year: i32, month: u32, day: u32, hour: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, day, hour, 0, 0).unwrap()
}

fn event(
    recorded_at: chrono::DateTime<Utc>,
    key: Option<&str>,
    legal_basis: Option<&str>,
    actor: Option<UserId>,
) -> AuditEvent {
    event_with_target(recorded_at, key, legal_basis, actor, "asset-usdc")
}

fn event_with_target(
    recorded_at: chrono::DateTime<Utc>,
    key: Option<&str>,
    legal_basis: Option<&str>,
    actor: Option<UserId>,
    target: &str,
) -> AuditEvent {
    let new = NewAuditEvent {
        actor_id: actor,
        actor_kind: if actor.is_some() {
            ActorKind::User
        } else {
            ActorKind::System
        },
        action: if actor.is_some() {
            "asset.contract.added".to_string()
        } else {
            "system.smoke.check".to_string()
        },
        target: target.to_string(),
        occurred_at: Some(recorded_at),
        context: Context::empty(),
        outcome: Outcome::Success,
        sensitivity: Sensitivity::Routine,
        event_class: Some(EventClass::Domain),
        idempotency_key: key.map(ToString::to_string),
        legal_basis: legal_basis.map(ToString::to_string),
        deletion_request_id: None,
    };
    AuditEvent::record(new, AuditEventId::generate(), recorded_at).unwrap()
}

fn hash(seed: &str) -> String {
    audit_application::hashing::sha256_hex(seed)
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn append_is_idempotent_and_reports_a_changed_payload() {
    let (pool, schema) = common::test_pool().await;
    let repo = PgAuditEventRepository::new(pool.clone());
    let first = event(instant(2026, 11, 6, 12), Some("key-1"), None, None);
    let payload = hash("payload-1");

    assert_eq!(
        repo.append(&first, "identity", &payload).await.unwrap(),
        audit_application::ports::AppendOutcome::Appended
    );
    // A retry builds a fresh event id with the same content and key; the stored
    // id is the first attempt's.
    let same_retry = event(instant(2026, 11, 6, 12), Some("key-1"), None, None);
    match repo
        .append(&same_retry, "identity", &payload)
        .await
        .unwrap()
    {
        audit_application::ports::AppendOutcome::Duplicate {
            event_id,
            same_payload,
        } => {
            assert_eq!(event_id, first.id());
            assert!(same_payload);
        }
        other => panic!("expected a duplicate, got {other:?}"),
    }
    let retried = event_with_target(
        instant(2026, 11, 6, 12),
        Some("key-1"),
        None,
        None,
        "asset-dai",
    );
    match repo
        .append(&retried, "identity", &hash("different"))
        .await
        .unwrap()
    {
        audit_application::ports::AppendOutcome::Duplicate {
            event_id,
            same_payload,
        } => {
            assert_eq!(event_id, first.id());
            assert!(!same_payload);
        }
        other => panic!("expected a duplicate, got {other:?}"),
    }

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn a_stored_event_round_trips_with_its_context() {
    let (pool, schema) = common::test_pool().await;
    let repo = PgAuditEventRepository::new(pool.clone());
    let mut context = BTreeMap::new();
    context.insert("correlationId".to_string(), "corr-1".to_string());
    let new = NewAuditEvent {
        actor_id: Some(UserId::parse(common::USER_SUBJECT).unwrap()),
        actor_kind: ActorKind::User,
        action: "asset.contract.added".to_string(),
        target: "asset-usdc".to_string(),
        occurred_at: Some(instant(2026, 11, 6, 11)),
        context: Context::parse(context).unwrap(),
        outcome: Outcome::Success,
        sensitivity: Sensitivity::Routine,
        event_class: None,
        idempotency_key: None,
        legal_basis: None,
        deletion_request_id: None,
    };
    let event =
        AuditEvent::record(new, AuditEventId::generate(), instant(2026, 11, 6, 12)).unwrap();
    repo.append(&event, "identity", &hash("x")).await.unwrap();

    let found = repo.find_by_id(event.id()).await.unwrap().unwrap();
    assert_eq!(found.id(), event.id());
    assert_eq!(found.context().get("correlationId"), Some("corr-1"));
    assert_eq!(found.actor_id(), event.actor_id());

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn query_and_export_honour_the_recorded_range_and_the_cursor() {
    let (pool, schema) = common::test_pool().await;
    let repo = PgAuditEventRepository::new(pool.clone());
    for hour in [9, 10, 11] {
        let event = event(instant(2026, 11, 6, hour), None, None, None);
        repo.append(&event, "identity", &hash(&hour.to_string()))
            .await
            .unwrap();
    }

    let query = audit_application::ports::EventQuery {
        action_prefix: None,
        target: None,
        sensitivity: None,
        occurred_from: None,
        occurred_to: None,
        recorded_from: instant(2026, 11, 6, 0),
        recorded_to: instant(2026, 11, 7, 0),
        cursor: None,
        limit: 2,
    };
    let first = repo.query(&query).await.unwrap();
    assert_eq!(first.len(), 2);
    assert_eq!(first[0].occurred_at(), instant(2026, 11, 6, 11));

    let second = audit_application::ports::EventQuery {
        cursor: Some(audit_application::ports::Cursor {
            occurred_at: first[1].occurred_at(),
            id: first[1].id(),
        }),
        limit: 2,
        ..query.clone()
    };
    let second = repo.query(&second).await.unwrap();
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].occurred_at(), instant(2026, 11, 6, 9));

    let outside = audit_application::ports::EventQuery {
        recorded_from: instant(2026, 12, 1, 0),
        recorded_to: instant(2026, 12, 2, 0),
        ..query.clone()
    };
    assert!(repo.query(&outside).await.unwrap().is_empty());

    let filter = ExportFilter::new(
        Vec::new(),
        None,
        None,
        instant(2026, 11, 6, 0),
        instant(2026, 11, 7, 0),
        None,
        None,
    )
    .unwrap();
    assert_eq!(repo.count_for_export(&filter, 100).await.unwrap(), 3);
    let batch = repo.export_batch(&filter, None, 2).await.unwrap();
    assert_eq!(batch.len(), 2);
    let next = repo
        .export_batch(
            &filter,
            Some(audit_application::ports::Cursor {
                occurred_at: batch[1].occurred_at(),
                id: batch[1].id(),
            }),
            2,
        )
        .await
        .unwrap();
    assert_eq!(next.len(), 1);

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn a_trail_insert_is_visible_to_a_read_and_never_updatable_by_audit_app() {
    let (pool, schema) = common::test_pool().await;
    let grants = sqlx::query_as::<_, (String, String)>(
        "SELECT privilege_type, table_name
         FROM information_schema.role_table_grants
         WHERE grantee = 'audit_app' AND table_name = 'audit_events'
         ORDER BY privilege_type",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let privileges: Vec<String> = grants.into_iter().map(|(privilege, _)| privilege).collect();
    assert!(privileges.contains(&"SELECT".to_string()));
    assert!(privileges.contains(&"INSERT".to_string()));
    assert!(!privileges.contains(&"UPDATE".to_string()));
    assert!(!privileges.contains(&"DELETE".to_string()));

    let retention = sqlx::query_as::<_, (String,)>(
        "SELECT privilege_type
         FROM information_schema.role_table_grants
         WHERE grantee = 'audit_retention' AND table_name = 'audit_events'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let privileges: Vec<String> = retention
        .into_iter()
        .map(|(privilege,)| privilege)
        .collect();
    assert!(privileges.contains(&"DELETE".to_string()));
    assert!(privileges.contains(&"INSERT".to_string()));
    assert!(!privileges.contains(&"UPDATE".to_string()));

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn a_terminal_support_session_cannot_be_updated() {
    let (pool, schema) = common::test_pool().await;
    let repo = PgSupportSessionRepository::new(pool.clone());
    let request = SupportSessionRequest::new(
        UserId::parse(common::SUPPORT_SUBJECT).unwrap(),
        UserId::parse(common::SUBJECT_USER).unwrap(),
        "customer reported a billing error",
        UserId::parse(common::SECOND_APPROVER).unwrap(),
        SupportScope::AccountMetadata,
        instant(2026, 11, 6, 12),
        instant(2026, 11, 6, 18),
    )
    .unwrap();
    let session = SupportSession::approve(
        request,
        SupportSessionId::generate(),
        instant(2026, 11, 6, 12),
    )
    .unwrap();
    repo.insert(&session).await.unwrap();
    assert_eq!(
        repo.find_by_id(session.id()).await.unwrap().unwrap().id(),
        session.id()
    );

    let revoked = session.revoke(instant(2026, 11, 6, 13)).unwrap();
    repo.update(&revoked).await.unwrap();
    let found = repo.find_by_id(revoked.id()).await.unwrap().unwrap();
    assert_eq!(found.stored_state(), SupportSessionState::Revoked);
    assert_eq!(found.outcome(), Some(SupportSessionOutcome::Revoked));

    let error = repo.update(&revoked).await.unwrap_err();
    assert!(
        matches!(error, RepositoryError::Conflict(_)),
        "the terminal-state trigger must refuse a second update, got {error:?}"
    );

    common::drop_schema(&schema).await;
}

fn export_filter() -> ExportFilter {
    ExportFilter::new(
        Vec::new(),
        None,
        None,
        instant(2026, 11, 1, 0),
        instant(2026, 11, 30, 0),
        None,
        None,
    )
    .unwrap()
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn a_terminal_export_cannot_be_updated_and_stale_requests_are_recovered() {
    let (pool, schema) = common::test_pool().await;
    let repo = PgExportRepository::new(pool.clone());
    let requested = AuditExport::requested(
        AuditExportId::generate(),
        UserId::parse(common::ADMIN_SUBJECT).unwrap(),
        export_filter(),
        instant(2026, 8, 6, 12),
    );
    repo.insert(&requested).await.unwrap();

    let recovered = repo
        .mark_stale_failed(instant(2026, 9, 6, 0), instant(2026, 11, 6, 12))
        .await
        .unwrap();
    assert_eq!(recovered, 1);

    let terminal = AuditExport::from_stored(
        requested.id(),
        requested.requested_by(),
        requested.filter().clone(),
        requested.requested_at(),
        Some(instant(2026, 11, 6, 13)),
        4,
        audit_domain::ExportState::Success,
        Some(audit_domain::ExportOutcome::Success),
        Some("c".repeat(64)),
    );
    let error = repo.update(&terminal).await.unwrap_err();
    assert!(matches!(error, RepositoryError::Conflict(_)));

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn retention_lists_partitions_and_never_deletes_a_held_row() {
    let (pool, schema) = common::test_pool().await;
    let repo = PgRetentionRepository::new(pool.clone());

    // Create an old month explicitly so the sweep has a partition to examine.
    sqlx::query("SELECT ensure_audit_partition($1)")
        .bind(chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap())
        .execute(&pool)
        .await
        .unwrap();

    let events = PgAuditEventRepository::new(pool.clone());
    let plain = event(
        instant(2025, 1, 15, 9),
        None,
        None,
        Some(UserId::parse(common::SUBJECT_USER).unwrap()),
    );
    let held = event(
        instant(2025, 1, 15, 10),
        None,
        Some("legal-obligation"),
        Some(UserId::parse(common::SUBJECT_USER).unwrap()),
    );
    events
        .append(&plain, "identity", &hash("plain"))
        .await
        .unwrap();
    events
        .append(&held, "identity", &hash("held"))
        .await
        .unwrap();

    let summaries = repo.partition_summaries().await.unwrap();
    let january = summaries
        .iter()
        .find(|summary| summary.name == "audit_events_202501")
        .expect("the January partition is listed");
    assert!(january.holds_legal_hold);
    assert_eq!(january.row_count, 2);

    let deleted = repo.delete_batch(instant(2025, 6, 1, 0), 10).await.unwrap();
    assert_eq!(deleted, 1, "only the unheld row is deleted");
    let remaining = events
        .query(&audit_application::ports::EventQuery {
            action_prefix: None,
            target: None,
            sensitivity: None,
            occurred_from: None,
            occurred_to: None,
            recorded_from: instant(2025, 1, 1, 0),
            recorded_to: instant(2025, 2, 1, 0),
            cursor: None,
            limit: 10,
        })
        .await
        .unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].legal_basis(), Some("legal-obligation"));

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn privacy_labels_keep_a_legally_held_record_and_delete_the_rest() {
    let (pool, schema) = common::test_pool().await;
    let repo = PgPrivacyDeletionRepository::new(pool.clone());
    let events = PgAuditEventRepository::new(pool.clone());
    let subject = UserId::parse(common::SUBJECT_USER).unwrap();

    let plain = event(instant(2026, 11, 6, 9), None, None, Some(subject));
    let held = event(
        instant(2026, 11, 6, 10),
        None,
        Some("legal-obligation"),
        Some(subject),
    );
    events
        .append(&plain, "identity", &hash("plain"))
        .await
        .unwrap();
    events
        .append(&held, "identity", &hash("held"))
        .await
        .unwrap();

    let request = DeletionRequestId::parse(common::DELETION_REQUEST).unwrap();
    let deletion = PrivacyDeletion::requested(
        PrivacyDeletionId::generate(),
        request,
        subject,
        instant(2026, 11, 6, 12),
    );
    repo.insert(&deletion).await.unwrap();
    assert_eq!(
        repo.find_by_request(request).await.unwrap().unwrap().id(),
        deletion.id()
    );

    let labels = repo.labels_for_subject(subject, 10).await.unwrap();
    assert_eq!(labels.len(), 2);
    assert_eq!(
        labels
            .iter()
            .filter(|label| label.legal_basis.is_none())
            .count(),
        1
    );

    let deleted = repo.delete_unlabelled_for_subject(subject).await.unwrap();
    assert_eq!(deleted, 1);

    let completed = deletion
        .complete(
            audit_domain::DeletionReport {
                deleted: 1,
                kept_with_legal_basis: 1,
                defects: 0,
            },
            instant(2026, 11, 6, 13),
        )
        .unwrap();
    repo.update(&completed).await.unwrap();
    assert_eq!(
        repo.find_by_request(request)
            .await
            .unwrap()
            .unwrap()
            .state(),
        PrivacyDeletionState::Completed
    );

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn the_down_migration_refuses_while_a_recent_record_exists() {
    let (pool, schema) = common::test_pool().await;
    let repo = PgAuditEventRepository::new(pool.clone());
    let recent = event(instant(2026, 11, 6, 12), None, None, None);
    repo.append(&recent, "identity", &hash("recent"))
        .await
        .unwrap();

    let error = audit_infrastructure::migrations::revert(&pool).await;
    assert!(
        error.is_err(),
        "a rollback must never delete the six-month window as a side effect"
    );

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn a_dropped_partition_takes_its_rows_with_it() {
    let (pool, schema) = common::test_pool().await;
    let repo = PgRetentionRepository::new(pool.clone());
    sqlx::query("SELECT ensure_audit_partition($1)")
        .bind(chrono::NaiveDate::from_ymd_opt(2025, 3, 1).unwrap())
        .execute(&pool)
        .await
        .unwrap();

    let events = PgAuditEventRepository::new(pool.clone());
    let old = event(instant(2025, 3, 15, 9), None, None, None);
    events.append(&old, "identity", &hash("old")).await.unwrap();
    assert_eq!(
        repo.partition_summaries()
            .await
            .unwrap()
            .iter()
            .filter(|summary| summary.name == "audit_events_202503")
            .count(),
        1
    );

    repo.drop_partition("audit_events_202503").await.unwrap();
    assert_eq!(
        repo.partition_summaries()
            .await
            .unwrap()
            .iter()
            .filter(|summary| summary.name == "audit_events_202503")
            .count(),
        0
    );
    assert!(repo.drop_partition("not_a_partition").await.is_err());

    common::drop_schema(&schema).await;
}
