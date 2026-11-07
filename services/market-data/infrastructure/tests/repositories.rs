//! Adapter integration tests against a real Postgres schema.
//!
//! Run with `TEST_DATABASE_URL` pointing at a disposable database:
//! `TEST_DATABASE_URL=postgres://... cargo test -p market-data-infrastructure --test repositories -- --ignored`

mod common;

use chrono::{DateTime, Duration, TimeZone, Utc};
use market_data_application::ports::{
    ClaimOutcome, FxRepository, ObservationRepository, RateLimiter, RefreshRunRepository,
    ServiceTokenRegistry, TierRepository,
};
use market_data_domain::{
    AssetPriceTier, ErrorCategory, FxRate, FxRateId, ObservationId, ObservationStatus,
    PriceObservation, PriceTier, RefreshJob, RefreshRun, RefreshRunId, RunStatus,
};
use market_data_infrastructure::repositories::{
    PgFxRepository, PgObservationRepository, PgRateLimiter, PgRefreshRunRepository,
    PgTierRepository,
};
use market_data_infrastructure::service_token::{hash_token, PgServiceTokenRegistry};
use rust_decimal::Decimal;
use std::str::FromStr;
use uuid::Uuid;

fn at(minute: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, 10, minute, 0).unwrap()
}

fn decimal(raw: &str) -> Decimal {
    Decimal::from_str(raw).unwrap()
}

fn accepted(
    asset: &str,
    source: &str,
    price: &str,
    observed_at: DateTime<Utc>,
) -> PriceObservation {
    PriceObservation::accept(
        ObservationId::generate(),
        asset,
        source,
        decimal(price),
        "USD",
        observed_at,
        observed_at,
    )
    .unwrap()
}

fn rejected(asset: &str, source: &str, observed_at: DateTime<Utc>) -> PriceObservation {
    PriceObservation::reject(
        ObservationId::generate(),
        asset,
        source,
        Decimal::ZERO,
        "USD",
        observed_at,
        observed_at,
        "malformed price",
    )
    .unwrap()
}

fn fx(base: &str, quote: &str, rate: &str, observed_at: DateTime<Utc>) -> FxRate {
    FxRate::record(
        FxRateId::generate(),
        base,
        quote,
        decimal(rate),
        "binance",
        observed_at,
        observed_at,
    )
    .unwrap()
}

fn run(job: RefreshJob, now: DateTime<Utc>) -> RefreshRun {
    RefreshRun::claim(RefreshRunId::generate(), job, now)
}

// ---------------------------------------------------------------------------
// Observations
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn observations_round_trip_and_the_newest_accepted_per_source_is_returned() {
    let (pool, schema) = common::test_pool().await;
    let repository = PgObservationRepository::new(pool.clone());
    let ids = vec!["asset-btc".to_string()];
    repository
        .insert(&accepted("asset-btc", "binance", "100", at(0)))
        .await
        .unwrap();
    repository
        .insert(&accepted("asset-btc", "binance", "110", at(5)))
        .await
        .unwrap();
    repository
        .insert(&rejected("asset-btc", "binance", at(6)))
        .await
        .unwrap();
    repository
        .insert(&accepted("asset-btc", "coingecko", "109", at(7)))
        .await
        .unwrap();

    let binance = repository
        .latest_accepted_for_source("binance", &ids)
        .await
        .unwrap();
    assert_eq!(binance.len(), 1);
    assert_eq!(binance[0].price(), decimal("110"));
    assert_eq!(binance[0].observed_at(), at(5));
    assert_eq!(binance[0].status(), ObservationStatus::Accepted);

    let newest = repository.latest_accepted_for_assets(&ids).await.unwrap();
    assert_eq!(newest.len(), 1);
    assert_eq!(newest[0].source_name(), "coingecko");
    assert_eq!(newest[0].price(), decimal("109"));
    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn the_retention_prune_keeps_the_newest_row_per_asset_and_source() {
    let (pool, schema) = common::test_pool().await;
    let repository = PgObservationRepository::new(pool.clone());
    repository
        .insert(&accepted("asset-btc", "binance", "100", at(0)))
        .await
        .unwrap();
    repository
        .insert(&accepted("asset-btc", "binance", "110", at(5)))
        .await
        .unwrap();
    repository
        .insert(&accepted("asset-btc", "coingecko", "109", at(1)))
        .await
        .unwrap();
    repository
        .insert(&accepted("asset-eth", "binance", "20", at(1)))
        .await
        .unwrap();

    let removed = repository.prune_before(at(20)).await.unwrap();
    assert_eq!(removed, 1, "only the superseded binance/btc row is removed");

    let remaining = repository
        .latest_accepted_for_source("binance", &["asset-btc".to_string()])
        .await
        .unwrap();
    assert_eq!(remaining[0].price(), decimal("110"));
    assert_eq!(
        repository
            .latest_accepted_for_source("coingecko", &["asset-btc".to_string()])
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        repository
            .latest_accepted_for_source("binance", &["asset-eth".to_string()])
            .await
            .unwrap()
            .len(),
        1
    );
    common::drop_schema(&schema).await;
}

// ---------------------------------------------------------------------------
// FX readings
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn fx_readings_round_trip_and_the_newest_pair_is_returned() {
    let (pool, schema) = common::test_pool().await;
    let repository = PgFxRepository::new(pool.clone());
    repository
        .insert(&fx("EUR", "USD", "1.10", at(0)))
        .await
        .unwrap();
    repository
        .insert(&fx("EUR", "USD", "1.12", at(3)))
        .await
        .unwrap();
    repository
        .insert(&fx("USD", "GBP", "0.80", at(2)))
        .await
        .unwrap();

    let latest = repository.latest("EUR", "USD").await.unwrap().unwrap();
    assert_eq!(latest.rate(), decimal("1.12"));
    assert_eq!(latest.observed_at(), at(3));
    assert!(repository.latest("EUR", "JPY").await.unwrap().is_none());
    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn the_fx_prune_keeps_the_newest_reading_per_pair() {
    let (pool, schema) = common::test_pool().await;
    let repository = PgFxRepository::new(pool.clone());
    repository
        .insert(&fx("EUR", "USD", "1.10", at(0)))
        .await
        .unwrap();
    repository
        .insert(&fx("EUR", "USD", "1.12", at(3)))
        .await
        .unwrap();
    repository
        .insert(&fx("USD", "GBP", "0.80", at(1)))
        .await
        .unwrap();

    assert_eq!(repository.prune_before(at(20)).await.unwrap(), 1);
    assert_eq!(
        repository
            .latest("EUR", "USD")
            .await
            .unwrap()
            .unwrap()
            .rate(),
        decimal("1.12")
    );
    assert!(repository.latest("USD", "GBP").await.unwrap().is_some());
    common::drop_schema(&schema).await;
}

// ---------------------------------------------------------------------------
// Refresh runs and leases
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn a_run_claim_is_single_flight_per_job_and_the_jobs_are_independent() {
    let (pool, schema) = common::test_pool().await;
    let repository = PgRefreshRunRepository::new(pool.clone());
    let top = run(RefreshJob::Top100, at(0));
    assert_eq!(repository.claim(&top).await.unwrap(), ClaimOutcome::Claimed);

    let second = run(RefreshJob::Top100, at(1));
    match repository.claim(&second).await.unwrap() {
        ClaimOutcome::Held(active) => assert_eq!(active.id(), top.id()),
        ClaimOutcome::Claimed => panic!("a second top100 run must not start"),
    }

    let tail = run(RefreshJob::Tail, at(1));
    assert_eq!(
        repository.claim(&tail).await.unwrap(),
        ClaimOutcome::Claimed
    );
    assert_eq!(
        repository
            .active_for_job(RefreshJob::Top100, at(1))
            .await
            .unwrap()
            .unwrap()
            .id(),
        top.id()
    );
    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn an_expired_lease_is_failed_and_the_next_claim_succeeds() {
    let (pool, schema) = common::test_pool().await;
    let repository = PgRefreshRunRepository::new(pool.clone());
    let first = run(RefreshJob::Top100, at(0));
    repository.claim(&first).await.unwrap();

    let second = run(RefreshJob::Top100, at(6));
    assert_eq!(
        repository.claim(&second).await.unwrap(),
        ClaimOutcome::Claimed
    );
    let stored = repository.find_by_id(first.id()).await.unwrap().unwrap();
    assert_eq!(stored.status(), RunStatus::Failed);
    assert_eq!(stored.error_category(), Some(ErrorCategory::Data));
    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn renewal_only_extends_a_live_lease() {
    let (pool, schema) = common::test_pool().await;
    let repository = PgRefreshRunRepository::new(pool.clone());
    let claimed = run(RefreshJob::Tail, at(0));
    repository.claim(&claimed).await.unwrap();

    assert!(repository.renew(claimed.id(), at(5), at(1)).await.unwrap());
    assert!(!repository
        .renew(claimed.id(), at(20), at(10))
        .await
        .unwrap());
    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn the_sweep_marks_expired_running_runs_failed() {
    let (pool, schema) = common::test_pool().await;
    let repository = PgRefreshRunRepository::new(pool.clone());
    let claimed = run(RefreshJob::Top100, at(0));
    repository.claim(&claimed).await.unwrap();

    let swept = repository.sweep_expired(at(7)).await.unwrap();
    assert_eq!(swept.len(), 1);
    assert_eq!(swept[0].status(), RunStatus::Failed);
    assert!(repository.sweep_expired(at(8)).await.unwrap().is_empty());
    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn finish_persists_counts_and_status() {
    let (pool, schema) = common::test_pool().await;
    let repository = PgRefreshRunRepository::new(pool.clone());
    let mut claimed = run(RefreshJob::Top100, at(0));
    repository.claim(&claimed).await.unwrap();
    claimed.record_counts(4, 3, 1).unwrap();
    claimed.succeed(at(2)).unwrap();
    repository.finish(&claimed).await.unwrap();

    let stored = repository.find_by_id(claimed.id()).await.unwrap().unwrap();
    assert_eq!(stored.status(), RunStatus::Succeeded);
    assert_eq!(stored.requested_count(), 4);
    assert_eq!(stored.priced_count(), 3);
    assert_eq!(stored.unpriced_count(), 1);
    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn finishing_a_run_twice_is_a_conflict() {
    let (pool, schema) = common::test_pool().await;
    let repository = PgRefreshRunRepository::new(pool.clone());
    let mut claimed = run(RefreshJob::Top100, at(0));
    repository.claim(&claimed).await.unwrap();
    claimed.record_counts(1, 1, 0).unwrap();
    claimed.succeed(at(2)).unwrap();
    repository.finish(&claimed).await.unwrap();
    assert!(repository.finish(&claimed).await.is_err());
    common::drop_schema(&schema).await;
}

// ---------------------------------------------------------------------------
// Tiers, rate buckets, and service tokens
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn tier_upsert_refreshes_rank_and_tier() {
    let (pool, schema) = common::test_pool().await;
    let repository = PgTierRepository::new(pool.clone());
    let top = AssetPriceTier::for_rank("asset-btc", 42, at(0)).unwrap();
    repository.upsert(&top).await.unwrap();

    let stored = repository.find("asset-btc").await.unwrap().unwrap();
    assert_eq!(stored.tier(), PriceTier::Top100);
    assert_eq!(stored.bound_seconds(), 300);

    let tail = AssetPriceTier::for_rank("asset-btc", 101, at(3)).unwrap();
    repository.upsert(&tail).await.unwrap();
    let stored = repository.find("asset-btc").await.unwrap().unwrap();
    assert_eq!(stored.tier(), PriceTier::Tail);
    assert_eq!(stored.rank(), 101);

    let bounds = repository
        .bounds_for(&["asset-btc".to_string(), "asset-unknown".to_string()])
        .await
        .unwrap();
    assert_eq!(bounds, vec![("asset-btc".to_string(), 900)]);
    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn the_rate_bucket_is_shared_and_refills() {
    let (pool, schema) = common::test_pool().await;
    let limiter = PgRateLimiter::new(pool.clone());
    assert!(limiter.try_acquire("binance", 2, at(0)).await.unwrap());
    assert!(limiter.try_acquire("binance", 2, at(0)).await.unwrap());
    assert!(!limiter.try_acquire("binance", 2, at(0)).await.unwrap());
    assert!(limiter
        .try_acquire("binance", 2, at(0) + Duration::seconds(60))
        .await
        .unwrap());
    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn the_service_token_registry_resolves_hashed_tokens() {
    let (pool, schema) = common::test_pool().await;
    sqlx::query(
        "INSERT INTO service_token_registry
             (id, token_hash, caller, operations, active, created_at)
         VALUES ($1, $2, $3, $4, true, now())",
    )
    .bind(Uuid::new_v4())
    .bind(hash_token("portfolio-token"))
    .bind("portfolio")
    .bind(vec!["prices_read".to_string(), "fx_read".to_string()])
    .execute(&pool)
    .await
    .unwrap();

    let registry =
        PgServiceTokenRegistry::with_outbound(pool.clone(), &[("market-data", "outbound")]);
    let service = registry
        .service_for("portfolio-token")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(service.name, "portfolio");
    assert!(service.allows(market_data_domain::Operation::PricesRead));
    assert!(!service.allows(market_data_domain::Operation::RunRefresh));
    assert!(registry.service_for("unknown").await.unwrap().is_none());
    assert_eq!(
        registry.token_for("market-data").as_deref(),
        Some("outbound")
    );
    common::drop_schema(&schema).await;
}
