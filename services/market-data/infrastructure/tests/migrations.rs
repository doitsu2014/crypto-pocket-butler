//! Migration tests against a real Postgres schema.
//!
//! Run with `TEST_DATABASE_URL` pointing at a disposable database:
//! `TEST_DATABASE_URL=postgres://... cargo test -p market-data-infrastructure --test migrations -- --ignored`

mod common;

use sqlx::Row;

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn the_migration_applies_reverts_and_reapplies_on_a_fresh_schema() {
    let (pool, schema) = common::test_pool().await;
    market_data_infrastructure::migrations::revert(&pool)
        .await
        .expect("down migration on a fresh schema");
    market_data_infrastructure::migrations::run(&pool)
        .await
        .expect("re-run migration");
    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn the_six_service_tables_exist_in_the_service_schema() {
    let (pool, schema) = common::test_pool().await;
    let rows = sqlx::query(
        "SELECT table_name FROM information_schema.tables
         WHERE table_schema = $1 ORDER BY table_name",
    )
    .bind(&schema)
    .fetch_all(&pool)
    .await
    .unwrap();
    let tables: Vec<String> = rows
        .iter()
        .map(|row| row.get::<String, _>("table_name"))
        .collect();
    for expected in [
        "asset_price_tiers",
        "fx_rates",
        "price_observations",
        "refresh_runs",
        "service_token_registry",
        "source_rate_buckets",
    ] {
        assert!(
            tables.contains(&expected.to_string()),
            "missing table {expected}; found {tables:?}"
        );
    }
    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn the_read_indexes_and_the_per_job_lease_index_exist() {
    let (pool, schema) = common::test_pool().await;
    let rows = sqlx::query("SELECT indexname FROM pg_indexes WHERE schemaname = $1")
        .bind(&schema)
        .fetch_all(&pool)
        .await
        .unwrap();
    let indexes: Vec<String> = rows
        .iter()
        .map(|row| row.get::<String, _>("indexname"))
        .collect();
    for expected in [
        "price_observations_asset_source_idx",
        "fx_rates_pair_idx",
        "refresh_runs_job_status_idx",
        "refresh_runs_one_live",
    ] {
        assert!(
            indexes.contains(&expected.to_string()),
            "missing index {expected}; found {indexes:?}"
        );
    }
    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn the_schema_declares_no_large_object_or_bytea_storage() {
    let (pool, schema) = common::test_pool().await;
    let row = sqlx::query(
        "SELECT count(*) AS count FROM information_schema.columns
         WHERE table_schema = $1 AND data_type = 'bytea'",
    )
    .bind(&schema)
    .fetch_one(&pool)
    .await
    .unwrap();
    let count: i64 = row.get("count");
    assert_eq!(count, 0, "market data stores no binary payloads");
    common::drop_schema(&schema).await;
}
