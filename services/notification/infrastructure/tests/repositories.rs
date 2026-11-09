//! Adapter tests against a real Postgres schema. Run with `TEST_DATABASE_URL`
//! set and `--ignored`.

mod common;

use chrono::{DateTime, Duration, TimeZone, Utc};
use common::{drop_schema, test_pool};
use notification_application::ports::{
    AlertRepository, AttemptRepository, DeliveryControl, InsertOutcome, PreferenceRepository,
    ReadinessProbe, ServiceTokenRegistry,
};
use notification_domain::{
    sanitise_payload, Alert, AlertId, AlertStatus, AttemptStatus, Channel, DeliveryAttempt,
    DeliveryAttemptId, ErrorCategory, NotificationPreference, NotificationType, PreferenceId,
    UserId,
};

const USER: &str = "11111111-1111-4111-8111-111111111111";
const OTHER: &str = "22222222-2222-4222-8222-222222222222";

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 8, hour, 0, 0).unwrap()
}

fn user() -> UserId {
    UserId::parse(USER).unwrap()
}

fn alert(key: &str, created_at: DateTime<Utc>) -> Alert {
    Alert::queue(
        AlertId::generate(),
        user(),
        NotificationType::Drift,
        key,
        sanitise_payload(
            NotificationType::Drift,
            [("assetId".to_string(), "bitcoin".to_string())].into(),
        ),
        created_at,
    )
    .expect("a well-formed fixture")
}

fn brief(key: &str, created_at: DateTime<Utc>) -> Alert {
    Alert::queue(
        AlertId::generate(),
        user(),
        NotificationType::Brief,
        key,
        sanitise_payload(
            NotificationType::Brief,
            [(
                "briefId".to_string(),
                "55555555-5555-4555-8555-555555555555".to_string(),
            )]
            .into(),
        ),
        created_at,
    )
    .expect("a well-formed fixture")
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL"]
async fn migrations_apply_and_revert_cleanly() {
    let (pool, schema) = test_pool().await;
    let table: Option<String> = sqlx::query_scalar("SELECT to_regclass('alerts')::text")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(table.is_some());

    notification_infrastructure::migrations::revert(&pool)
        .await
        .expect("revert");
    let table: Option<String> = sqlx::query_scalar("SELECT to_regclass('alerts')::text")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(table.is_none());
    pool.close().await;
    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL"]
async fn a_preference_row_upserts() {
    let (pool, schema) = test_pool().await;
    let repository = notification_infrastructure::PgPreferenceRepository::new(pool.clone());
    let first = NotificationPreference::stored(
        PreferenceId::generate(),
        user(),
        NotificationType::Drift,
        Channel::Email,
        true,
        at(9),
    );
    repository.upsert(&first).await.unwrap();
    let second = NotificationPreference::stored(
        PreferenceId::generate(),
        user(),
        NotificationType::Drift,
        Channel::Email,
        false,
        at(10),
    );
    repository.upsert(&second).await.unwrap();

    let rows = repository.list_for_user(user()).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert!(!rows[0].enabled());
    assert_eq!(rows[0].updated_at(), at(10));
    pool.close().await;
    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL"]
async fn an_alert_insert_dedupes_by_key() {
    let (pool, schema) = test_pool().await;
    let repository = notification_infrastructure::PgAlertRepository::new(pool.clone());
    let first = alert("drift:1", at(9));
    let inserted = repository.insert(&first).await.unwrap();
    assert!(matches!(inserted, InsertOutcome::Inserted(_)));

    let duplicate = alert("drift:1", at(10));
    let outcome = repository.insert(&duplicate).await.unwrap();
    match outcome {
        InsertOutcome::Duplicate(existing) => assert_eq!(existing.id(), first.id()),
        InsertOutcome::Inserted(_) => panic!("a duplicate key must not insert"),
    }
    pool.close().await;
    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL"]
async fn the_feed_pages_newest_first() {
    let (pool, schema) = test_pool().await;
    let repository = notification_infrastructure::PgAlertRepository::new(pool.clone());
    let older = alert("drift:1", at(9));
    let newer = alert("drift:2", at(10));
    repository.insert(&older).await.unwrap();
    repository.insert(&newer).await.unwrap();

    let page = repository.list_for_user(user(), None, 1).await.unwrap();
    assert_eq!(page.len(), 1);
    assert_eq!(page[0].id(), newer.id());

    let next = repository
        .list_for_user(user(), Some((page[0].created_at(), page[0].id())), 10)
        .await
        .unwrap();
    assert_eq!(next.len(), 1);
    assert_eq!(next[0].id(), older.id());

    assert!(repository
        .list_for_user(UserId::parse(OTHER).unwrap(), None, 10)
        .await
        .unwrap()
        .is_empty());
    pool.close().await;
    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL"]
async fn the_feed_hides_a_type_disabled_for_in_app() {
    let (pool, schema) = test_pool().await;
    let alerts = notification_infrastructure::PgAlertRepository::new(pool.clone());
    let preferences = notification_infrastructure::PgPreferenceRepository::new(pool.clone());
    alerts.insert(&alert("drift:1", at(9))).await.unwrap();
    alerts.insert(&brief("brief:1", at(10))).await.unwrap();
    preferences
        .upsert(&NotificationPreference::stored(
            PreferenceId::generate(),
            user(),
            NotificationType::Drift,
            Channel::InApp,
            false,
            at(9),
        ))
        .await
        .unwrap();

    let page = alerts.list_for_user(user(), None, 10).await.unwrap();
    assert_eq!(page.len(), 1);
    assert_eq!(page[0].kind(), NotificationType::Brief);

    preferences
        .upsert(&NotificationPreference::stored(
            PreferenceId::generate(),
            user(),
            NotificationType::Drift,
            Channel::InApp,
            true,
            at(10),
        ))
        .await
        .unwrap();
    assert_eq!(
        alerts.list_for_user(user(), None, 10).await.unwrap().len(),
        2
    );
    pool.close().await;
    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL"]
async fn due_alerts_are_filtered_and_ordered() {
    let (pool, schema) = test_pool().await;
    let repository = notification_infrastructure::PgAlertRepository::new(pool.clone());
    let due_now = alert("drift:1", at(9));
    let later = alert("drift:2", at(10));
    repository.insert(&due_now).await.unwrap();
    repository.insert(&later).await.unwrap();
    repository
        .set_status(later.id(), AlertStatus::Queued, Some(at(12)))
        .await
        .unwrap();

    let due = repository.due(at(11), 10).await.unwrap();
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].id(), due_now.id());
    assert!(repository.has_due(at(11)).await.unwrap());
    assert!(!repository.has_due(at(8)).await.unwrap());
    pool.close().await;
    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL"]
async fn attempts_insert_and_list() {
    let (pool, schema) = test_pool().await;
    let alerts = notification_infrastructure::PgAlertRepository::new(pool.clone());
    let attempts = notification_infrastructure::PgAttemptRepository::new(pool.clone());
    let stored = alert("drift:1", at(9));
    alerts.insert(&stored).await.unwrap();

    let sent = DeliveryAttempt::sent(
        DeliveryAttemptId::generate(),
        stored.id(),
        user(),
        Channel::InApp,
        1,
        at(9),
    )
    .unwrap();
    attempts.insert(&sent).await.unwrap();
    let failed = DeliveryAttempt::failed(
        DeliveryAttemptId::generate(),
        stored.id(),
        user(),
        Channel::Email,
        1,
        Some(ErrorCategory::Network),
        at(9),
    )
    .unwrap();
    attempts.insert(&failed).await.unwrap();

    let rows = attempts.list_for_alert(stored.id()).await.unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|row| row.status() == AttemptStatus::Sent));
    assert!(rows
        .iter()
        .any(|row| row.error_category() == Some(ErrorCategory::Network)));
    pool.close().await;
    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL"]
async fn the_wake_flag_sets_and_clears() {
    let (pool, schema) = test_pool().await;
    let control = notification_infrastructure::PgDeliveryControl::new(pool.clone());
    assert!(!control.wake_requested().await.unwrap());
    control.request_wake(at(9)).await.unwrap();
    assert!(control.wake_requested().await.unwrap());
    control.clear_wake().await.unwrap();
    assert!(!control.wake_requested().await.unwrap());
    pool.close().await;
    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL"]
async fn the_lease_is_single_flight() {
    let (pool, schema) = test_pool().await;
    let control = notification_infrastructure::PgDeliveryControl::new(pool.clone());
    let first = uuid::Uuid::new_v4();
    let second = uuid::Uuid::new_v4();
    assert!(control.acquire_lease(first, at(9), 600).await.unwrap());
    assert!(!control.acquire_lease(second, at(9), 600).await.unwrap());
    control.release_lease(second).await.unwrap();
    assert!(!control.acquire_lease(second, at(9), 600).await.unwrap());
    control.release_lease(first).await.unwrap();
    assert!(control.acquire_lease(second, at(9), 600).await.unwrap());
    pool.close().await;
    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL"]
async fn an_expired_lease_can_be_taken() {
    let (pool, schema) = test_pool().await;
    let control = notification_infrastructure::PgDeliveryControl::new(pool.clone());
    let first = uuid::Uuid::new_v4();
    let second = uuid::Uuid::new_v4();
    assert!(control.acquire_lease(first, at(9), 60).await.unwrap());
    assert!(control
        .acquire_lease(second, at(9) + Duration::minutes(2), 60)
        .await
        .unwrap());
    pool.close().await;
    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL"]
async fn erasure_rewrites_alerts_and_attempts() {
    let (pool, schema) = test_pool().await;
    let alerts = notification_infrastructure::PgAlertRepository::new(pool.clone());
    let attempts = notification_infrastructure::PgAttemptRepository::new(pool.clone());
    let stored = alert("drift:1", at(9));
    alerts.insert(&stored).await.unwrap();
    attempts
        .insert(
            &DeliveryAttempt::sent(
                DeliveryAttemptId::generate(),
                stored.id(),
                user(),
                Channel::InApp,
                1,
                at(9),
            )
            .unwrap(),
        )
        .await
        .unwrap();

    let replacement = UserId::generate();
    let rows = alerts.anonymise_user(user(), replacement).await.unwrap();
    assert_eq!(rows.alerts, 1);
    assert_eq!(rows.attempts, 1);

    let page = alerts.list_for_user(replacement, None, 10).await.unwrap();
    assert_eq!(page.len(), 1);
    assert_eq!(page[0].user_id(), replacement);
    let rows = attempts.list_for_alert(stored.id()).await.unwrap();
    assert_eq!(rows[0].user_id(), replacement);
    pool.close().await;
    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL"]
async fn retention_deletes_rows_before_the_cutoff() {
    let (pool, schema) = test_pool().await;
    let alerts = notification_infrastructure::PgAlertRepository::new(pool.clone());
    let attempts = notification_infrastructure::PgAttemptRepository::new(pool.clone());
    let old = alert("drift:old", at(9) - Duration::days(400));
    alerts.insert(&old).await.unwrap();
    attempts
        .insert(
            &DeliveryAttempt::sent(
                DeliveryAttemptId::generate(),
                old.id(),
                user(),
                Channel::InApp,
                1,
                at(9) - Duration::days(400),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let recent = alert("drift:recent", at(9));
    alerts.insert(&recent).await.unwrap();

    let cutoff = at(9) - Duration::days(365);
    assert_eq!(attempts.delete_before(cutoff, 100).await.unwrap(), 1);
    assert_eq!(alerts.delete_before(cutoff, 100).await.unwrap(), 1);
    assert_eq!(
        alerts.list_for_user(user(), None, 10).await.unwrap().len(),
        1
    );
    pool.close().await;
    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL"]
async fn a_service_token_resolves_to_its_operations() {
    let (pool, schema) = test_pool().await;
    let hash = notification_application::sha256_hex("account-token");
    sqlx::query(
        "INSERT INTO service_token_registry (service, token_sha256, allowed_operations)
         VALUES ($1, $2, $3)",
    )
    .bind("account")
    .bind(hash)
    .bind(vec!["alerts.submit".to_string()])
    .execute(&pool)
    .await
    .unwrap();

    let registry = notification_infrastructure::PgServiceTokenRegistry::new(pool.clone());
    let caller = registry
        .resolve("account-token")
        .await
        .unwrap()
        .expect("the token resolves");
    assert_eq!(caller.service, "account");
    assert!(caller.allows("alerts.submit"));
    assert!(registry.resolve("unknown").await.unwrap().is_none());
    pool.close().await;
    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL"]
async fn readiness_requires_the_migrations() {
    let (pool, schema) = test_pool().await;
    let probe = notification_infrastructure::NotificationReadinessProbe::new(pool.clone());
    probe.check().await.expect("a migrated schema is ready");
    pool.close().await;
    drop_schema(&schema).await;
}
