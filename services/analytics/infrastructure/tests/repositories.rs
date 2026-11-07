//! Adapter tests against a real Postgres schema. Run with `TEST_DATABASE_URL`
//! set and `--ignored`.

mod common;

use std::collections::BTreeMap;

use analytics_application::ports::{
    AggregateRepository, ConsentRepository, EventRepository, FlagRepository, ServiceTokenRegistry,
};
use analytics_domain::{
    sanitize_event, AnalyticsEventId, ConsentCheck, ConsentCheckId, ConsentPurpose, FeatureFlag,
    FeatureFlagId, Metric, MetricAggregate, MetricAggregateId, NewFeatureFlag, Period,
};
use chrono::{Duration, TimeZone, Utc};
use common::{drop_schema, test_pool};
use rust_decimal::Decimal;

fn at(hour: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 6, hour, 0, 0).unwrap()
}

fn event(name: &str, user_ref: &str) -> analytics_domain::AnalyticsEvent {
    let sanitized = sanitize_event(name, at(10), BTreeMap::new()).expect("fixture is catalogued");
    analytics_domain::AnalyticsEvent::store(
        AnalyticsEventId::generate(),
        sanitized,
        Some(user_ref.to_string()),
        at(10),
    )
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL"]
async fn migrations_apply_and_revert_cleanly() {
    let (pool, schema) = test_pool().await;
    let table: Option<String> = sqlx::query_scalar("SELECT to_regclass('events')::text")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(table.is_some());

    analytics_infrastructure::migrations::revert(&pool)
        .await
        .expect("revert");
    let table: Option<String> = sqlx::query_scalar("SELECT to_regclass('events')::text")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(table.is_none());
    pool.close().await;
    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL"]
async fn an_event_and_its_consent_row_are_inserted_together() {
    let (pool, schema) = test_pool().await;
    let repository = analytics_infrastructure::PgEventRepository::new(pool.clone());
    let check = ConsentCheck::record(
        ConsentCheckId::generate(),
        ConsentPurpose::Analytics,
        true,
        at(10),
        Some("user-ref".to_string()),
    );
    repository
        .insert(&event("valuation_read", "user-ref"), Some(&check))
        .await
        .expect("insert");

    let events = repository
        .events_between(at(0), at(23), 100)
        .await
        .expect("read");
    assert_eq!(events.len(), 1);
    let consents: i64 = sqlx::query_scalar("SELECT count(*) FROM consent_checks")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(consents, 1);
    pool.close().await;
    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL"]
async fn retention_deletes_only_rows_older_than_the_cutoff() {
    let (pool, schema) = test_pool().await;
    let repository = analytics_infrastructure::PgEventRepository::new(pool.clone());
    repository
        .insert(
            &analytics_domain::AnalyticsEvent::store(
                AnalyticsEventId::generate(),
                sanitize_event("valuation_read", at(10), BTreeMap::new()).unwrap(),
                Some("old".to_string()),
                at(10) - Duration::days(200),
            ),
            None,
        )
        .await
        .unwrap();
    repository
        .insert(&event("valuation_read", "new"), None)
        .await
        .unwrap();

    let deleted = repository
        .delete_before(at(10) - Duration::days(180), 100)
        .await
        .unwrap();
    assert_eq!(deleted, 1);
    assert_eq!(
        repository
            .events_between(at(0), at(23), 100)
            .await
            .unwrap()
            .len(),
        1
    );
    pool.close().await;
    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL"]
async fn erasure_deletes_every_supplied_user_ref() {
    let (pool, schema) = test_pool().await;
    let repository = analytics_infrastructure::PgEventRepository::new(pool.clone());
    repository
        .insert(&event("valuation_read", "ref-a"), None)
        .await
        .unwrap();
    repository
        .insert(&event("valuation_read", "ref-b"), None)
        .await
        .unwrap();

    let deleted = repository
        .delete_by_user_refs(&["ref-a".to_string(), "ref-b".to_string()])
        .await
        .unwrap();
    assert_eq!(deleted, 2);
    pool.close().await;
    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL"]
async fn a_flag_round_trips_and_deletes() {
    let (pool, schema) = test_pool().await;
    let repository = analytics_infrastructure::PgFlagRepository::new(pool.clone());
    let flag = FeatureFlag::create(
        FeatureFlagId::generate(),
        NewFeatureFlag {
            key: "new-dashboard".to_string(),
            owner: "product".to_string(),
            expires_at: at(10) + Duration::days(30),
            rollout_percentage: 25,
            status: Some("active".to_string()),
        },
        at(10),
    )
    .unwrap();
    repository.insert(&flag).await.expect("insert flag");
    let found = repository
        .find_by_key("new-dashboard")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.rollout_percentage(), 25);
    assert_eq!(repository.list().await.unwrap().len(), 1);
    assert!(repository.delete("new-dashboard").await.unwrap());
    assert!(repository
        .find_by_key("new-dashboard")
        .await
        .unwrap()
        .is_none());
    pool.close().await;
    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL"]
async fn an_aggregate_upsert_replaces_the_same_key() {
    let (pool, schema) = test_pool().await;
    let repository = analytics_infrastructure::PgAggregateRepository::new(pool.clone());
    let candidate = |value: Decimal| analytics_domain::MetricCandidate {
        metric: Metric::Engagement,
        period: Period::Daily,
        cohort: "all".to_string(),
        period_start: at(0),
        value,
        distinct_users: 20,
    };
    let first = MetricAggregate::from_candidate(
        MetricAggregateId::generate(),
        &candidate(Decimal::new(5, 1)),
        at(12),
    )
    .unwrap();
    repository.upsert(&first).await.unwrap();
    let second = MetricAggregate::from_candidate(
        MetricAggregateId::generate(),
        &candidate(Decimal::new(7, 1)),
        at(13),
    )
    .unwrap();
    repository.upsert(&second).await.unwrap();

    let listed = repository
        .list(Some(Metric::Engagement), None, None, None, 10)
        .await
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].value(), Decimal::new(7, 1));
    pool.close().await;
    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL"]
async fn a_consent_check_is_deleted_after_ninety_days() {
    let (pool, schema) = test_pool().await;
    let repository = analytics_infrastructure::PgConsentRepository::new(pool.clone());
    repository
        .insert(&ConsentCheck::record(
            ConsentCheckId::generate(),
            ConsentPurpose::Analytics,
            true,
            at(10) - Duration::days(100),
            Some("old".to_string()),
        ))
        .await
        .unwrap();
    repository
        .insert(&ConsentCheck::record(
            ConsentCheckId::generate(),
            ConsentPurpose::Analytics,
            true,
            at(10),
            Some("new".to_string()),
        ))
        .await
        .unwrap();
    let deleted = repository
        .delete_before(at(10) - Duration::days(90), 100)
        .await
        .unwrap();
    assert_eq!(deleted, 1);
    pool.close().await;
    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL"]
async fn a_registered_token_resolves_to_its_service_and_operations() {
    let (pool, schema) = test_pool().await;
    sqlx::query(
        "INSERT INTO service_token_registry (service, token_sha256, allowed_operations)
         VALUES ('identity', $1, ARRAY['privacy.erase'])",
    )
    .bind(analytics_application::hashing::sha256_hex("identity-token"))
    .execute(&pool)
    .await
    .unwrap();

    let registry = analytics_infrastructure::PgServiceTokenRegistry::new(pool.clone());
    let caller = registry
        .resolve("identity-token")
        .await
        .unwrap()
        .expect("registered token resolves");
    assert_eq!(caller.service, "identity");
    assert!(caller.allows("privacy.erase"));
    assert!(registry.resolve("unknown").await.unwrap().is_none());
    pool.close().await;
    drop_schema(&schema).await;
}
