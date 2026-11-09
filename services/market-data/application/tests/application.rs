//! Application tests: C6 price selection and FX, the two leased refresh jobs,
//! retention, audit, and the service-token allow-list — all against port fakes.

mod fakes;

use std::sync::Arc;

use chrono::{DateTime, Duration, TimeZone, Utc};
use market_data_application::common::{AppError, ServiceCaller};
use market_data_application::fx_service::FxService;
use market_data_application::ports::{PriceSource, ProviderError, ProviderQuote, TierRepository};
use market_data_application::quotes::QuoteService;
use market_data_application::refresh::{RefreshService, RunOutcome};
use market_data_application::retention::RetentionService;
use market_data_domain::{
    ErrorCategory, Freshness, FxRate, FxRateId, ObservationId, ObservationStatus, Operation,
    PriceObservation, PriceTier, RefreshJob, RefreshRun, RefreshRunId, RunStatus, Trigger,
    FX_BOUND_SECONDS, HARD_BOUND_SECONDS,
};
use rust_decimal::Decimal;
use std::str::FromStr;

use fakes::{
    asset, source, FakeAudit, FakeFx, FakeMetrics, FakeObservations, FakeProvider, FakeRateLimiter,
    FakeRegistry, FakeRuns, FakeSecrets, FakeTiers, MutableClock, RecordingSleeper,
};

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, 12, 0, 0).unwrap()
}

fn ago(minutes: i64) -> DateTime<Utc> {
    now() - Duration::minutes(minutes)
}

fn decimal(raw: &str) -> Decimal {
    Decimal::from_str(raw).unwrap()
}

fn caller(name: &str) -> ServiceCaller {
    let operations = match name {
        "portfolio" | "reporting" | "smoke-reader" => {
            vec![Operation::PricesRead, Operation::FxRead]
        }
        "job-orchestration" => vec![
            Operation::PricesRead,
            Operation::FxRead,
            Operation::RunRefresh,
        ],
        _ => Vec::new(),
    };
    ServiceCaller::new(name, operations, "corr-test")
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

fn reading(price: &str, observed_at: DateTime<Utc>) -> Result<ProviderQuote, ProviderError> {
    Ok(ProviderQuote {
        price: price.to_string(),
        currency: "USD".to_string(),
        observed_at,
    })
}

fn fx_reading(rate: &str, observed_at: DateTime<Utc>) -> Result<ProviderQuote, ProviderError> {
    Ok(ProviderQuote {
        price: rate.to_string(),
        currency: "USD".to_string(),
        observed_at,
    })
}

struct Harness {
    registry: Arc<FakeRegistry>,
    observations: Arc<FakeObservations>,
    fx_repo: Arc<FakeFx>,
    runs: Arc<FakeRuns>,
    tiers: Arc<FakeTiers>,
    provider: Arc<FakeProvider>,
    secrets: Arc<FakeSecrets>,
    limiter: Arc<FakeRateLimiter>,
    audit: Arc<FakeAudit>,
    clock: Arc<MutableClock>,
    sleeper: Arc<RecordingSleeper>,
    metrics: Arc<FakeMetrics>,
    quotes: QuoteService,
    fx: FxService,
    refresh: RefreshService,
    retention: RetentionService,
}

fn harness() -> Harness {
    let registry = Arc::new(FakeRegistry::default());
    registry.set_sources(vec![
        source("binance", 1, false),
        source("coingecko", 2, true),
    ]);
    registry.set_assets(vec![
        asset("asset-btc", 1),
        asset("asset-eth", 102),
        asset("asset-tail", 101),
    ]);
    let observations = Arc::new(FakeObservations::default());
    let fx_repo = Arc::new(FakeFx::default());
    let runs = Arc::new(FakeRuns::default());
    let tiers = Arc::new(FakeTiers::default());
    let provider = Arc::new(FakeProvider::default());
    let secrets = Arc::new(FakeSecrets::default());
    let limiter = Arc::new(FakeRateLimiter::default());
    let audit = Arc::new(FakeAudit::default());
    let clock = Arc::new(MutableClock::at(now()));
    let sleeper = Arc::new(RecordingSleeper::default());
    let metrics = Arc::new(FakeMetrics::default());

    let quotes = QuoteService::new(
        registry.clone(),
        observations.clone(),
        tiers.clone(),
        metrics.clone(),
        clock.clone(),
    );
    let fx = FxService::new(fx_repo.clone(), metrics.clone(), clock.clone());
    let retention = RetentionService::new(observations.clone(), fx_repo.clone(), clock.clone());
    let refresh = RefreshService::new(
        registry.clone(),
        observations.clone(),
        fx_repo.clone(),
        runs.clone(),
        tiers.clone(),
        provider.clone(),
        secrets.clone(),
        limiter.clone(),
        audit.clone(),
        clock.clone(),
        sleeper.clone(),
        metrics.clone(),
        vec!["EUR".to_string(), "VND".to_string()],
    );

    Harness {
        registry,
        observations,
        fx_repo,
        runs,
        tiers,
        provider,
        secrets,
        limiter,
        audit,
        clock,
        sleeper,
        metrics,
        quotes,
        fx,
        refresh,
        retention,
    }
}

async fn top100_tiers(h: &Harness) {
    let tier = market_data_domain::AssetPriceTier::for_rank("asset-btc", 1, now()).unwrap();
    h.tiers.upsert(&tier).await.unwrap();
}

// ---------------------------------------------------------------------------
// WF1: serve prices
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_lowest_priority_source_within_the_bound_wins() {
    let h = harness();
    top100_tiers(&h).await;
    h.observations
        .seed(accepted("asset-btc", "binance", "100", ago(2)));
    h.observations
        .seed(accepted("asset-btc", "coingecko", "99", ago(1)));

    let quotes = h
        .quotes
        .get_prices(&caller("portfolio"), &["asset-btc".to_string()])
        .await
        .unwrap();
    assert_eq!(quotes.len(), 1);
    assert_eq!(quotes[0].source(), Some("binance"));
    assert_eq!(quotes[0].price(), Some(decimal("100")));
    assert_eq!(quotes[0].freshness(), Freshness::Live);
    assert_eq!(quotes[0].observed_at(), Some(ago(2)));
}

#[tokio::test]
async fn a_delayed_feed_is_labelled_delayed() {
    let h = harness();
    top100_tiers(&h).await;
    h.observations
        .seed(accepted("asset-btc", "coingecko", "99", ago(1)));

    let quotes = h
        .quotes
        .get_prices(&caller("portfolio"), &["asset-btc".to_string()])
        .await
        .unwrap();
    assert_eq!(quotes[0].source(), Some("coingecko"));
    assert_eq!(quotes[0].freshness(), Freshness::Delayed);
}

#[tokio::test]
async fn an_expired_primary_falls_back_to_the_next_source() {
    let h = harness();
    top100_tiers(&h).await;
    h.observations
        .seed(accepted("asset-btc", "binance", "100", ago(20)));
    h.observations
        .seed(accepted("asset-btc", "coingecko", "99", ago(1)));

    let quotes = h
        .quotes
        .get_prices(&caller("portfolio"), &["asset-btc".to_string()])
        .await
        .unwrap();
    assert_eq!(quotes[0].source(), Some("coingecko"));
    assert_eq!(quotes[0].freshness(), Freshness::Delayed);
}

#[tokio::test]
async fn an_observation_beyond_the_tier_bound_is_stale_within_the_hard_bound() {
    let h = harness();
    top100_tiers(&h).await;
    h.observations
        .seed(accepted("asset-btc", "binance", "100", ago(20)));

    let quotes = h
        .quotes
        .get_prices(&caller("portfolio"), &["asset-btc".to_string()])
        .await
        .unwrap();
    assert_eq!(quotes[0].freshness(), Freshness::Stale);
    assert_eq!(quotes[0].source(), Some("binance"));
}

#[tokio::test]
async fn a_tail_asset_uses_the_nine_hundred_second_bound() {
    let h = harness();
    top100_tiers(&h).await;
    h.observations
        .seed(accepted("asset-tail", "binance", "5", ago(10)));

    let quotes = h
        .quotes
        .get_prices(&caller("portfolio"), &["asset-tail".to_string()])
        .await
        .unwrap();
    assert_eq!(quotes[0].freshness(), Freshness::Live);
}

#[tokio::test]
async fn a_missing_tier_row_is_treated_as_tail() {
    let h = harness();
    h.observations
        .seed(accepted("asset-eth", "binance", "2000", ago(10)));

    let quotes = h
        .quotes
        .get_prices(&caller("portfolio"), &["asset-eth".to_string()])
        .await
        .unwrap();
    assert_eq!(quotes[0].freshness(), Freshness::Live);
}

#[tokio::test]
async fn an_asset_with_no_observation_is_unpriced() {
    let h = harness();
    let quotes = h
        .quotes
        .get_prices(&caller("portfolio"), &["asset-btc".to_string()])
        .await
        .unwrap();
    assert_eq!(quotes[0].freshness(), Freshness::Unpriced);
    assert_eq!(quotes[0].price(), None);
    assert_eq!(quotes[0].source(), None);
}

#[tokio::test]
async fn an_observation_older_than_the_hard_bound_is_unpriced() {
    let h = harness();
    let old = now() - Duration::seconds(HARD_BOUND_SECONDS + 1);
    h.observations
        .seed(accepted("asset-btc", "binance", "100", old));
    let quotes = h
        .quotes
        .get_prices(&caller("portfolio"), &["asset-btc".to_string()])
        .await
        .unwrap();
    assert_eq!(quotes[0].freshness(), Freshness::Unpriced);
}

#[tokio::test]
async fn the_batch_returns_in_request_order() {
    let h = harness();
    h.observations
        .seed(accepted("asset-btc", "binance", "100", ago(1)));
    h.observations
        .seed(accepted("asset-tail", "binance", "5", ago(1)));
    let quotes = h
        .quotes
        .get_prices(
            &caller("portfolio"),
            &["asset-tail".to_string(), "asset-btc".to_string()],
        )
        .await
        .unwrap();
    assert_eq!(quotes[0].asset_id(), "asset-tail");
    assert_eq!(quotes[1].asset_id(), "asset-btc");
}

#[tokio::test]
async fn one_unpriced_asset_does_not_fail_the_batch() {
    let h = harness();
    h.observations
        .seed(accepted("asset-btc", "binance", "100", ago(1)));
    let quotes = h
        .quotes
        .get_prices(
            &caller("portfolio"),
            &["asset-btc".to_string(), "asset-eth".to_string()],
        )
        .await
        .unwrap();
    assert_eq!(quotes.len(), 2);
    assert_eq!(quotes[0].freshness(), Freshness::Live);
    assert_eq!(quotes[1].freshness(), Freshness::Unpriced);
}

#[tokio::test]
async fn duplicate_ids_are_collapsed_to_one_quote() {
    let h = harness();
    h.observations
        .seed(accepted("asset-btc", "binance", "100", ago(1)));
    let quotes = h
        .quotes
        .get_prices(
            &caller("portfolio"),
            &["asset-btc".to_string(), "asset-btc".to_string()],
        )
        .await
        .unwrap();
    assert_eq!(quotes.len(), 1);
}

#[tokio::test]
async fn an_empty_batch_is_refused() {
    let h = harness();
    let error = h
        .quotes
        .get_prices(&caller("portfolio"), &[])
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Validation(_)));
}

#[tokio::test]
async fn more_than_one_thousand_ids_are_refused() {
    let h = harness();
    let ids: Vec<String> = (0..1_001).map(|index| format!("asset-{index}")).collect();
    let error = h
        .quotes
        .get_prices(&caller("portfolio"), &ids)
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Validation(_)));
}

#[tokio::test]
async fn a_malformed_asset_id_is_refused() {
    let h = harness();
    let error = h
        .quotes
        .get_prices(&caller("portfolio"), &["   ".to_string()])
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Validation(_)));
}

#[tokio::test]
async fn a_caller_without_the_read_operation_is_refused() {
    let h = harness();
    let error = h
        .quotes
        .get_prices(&caller("unknown"), &["asset-btc".to_string()])
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Forbidden(_)));
}

#[tokio::test]
async fn the_source_list_is_read_once_per_request() {
    let h = harness();
    let _ = h
        .quotes
        .get_prices(&caller("portfolio"), &["asset-btc".to_string()])
        .await;
    let _ = h
        .quotes
        .get_prices(&caller("portfolio"), &["asset-btc".to_string()])
        .await;
    assert_eq!(
        h.registry
            .source_calls
            .load(std::sync::atomic::Ordering::Relaxed),
        2
    );
}

#[tokio::test]
async fn no_provider_is_called_on_the_read_path() {
    let h = harness();
    h.observations
        .seed(accepted("asset-btc", "binance", "100", ago(1)));
    let _ = h
        .quotes
        .get_prices(&caller("portfolio"), &["asset-btc".to_string()])
        .await;
    assert_eq!(h.provider.call_count(), 0);
}

#[tokio::test]
async fn a_degraded_c4_read_serves_the_newest_observation_as_stale() {
    let h = harness();
    h.registry.set_fail_sources(true);
    h.observations
        .seed(accepted("asset-btc", "binance", "100", ago(1)));
    let quotes = h
        .quotes
        .get_prices(&caller("portfolio"), &["asset-btc".to_string()])
        .await
        .unwrap();
    assert_eq!(quotes[0].freshness(), Freshness::Stale);
    assert_eq!(quotes[0].source(), Some("binance"));
}

#[tokio::test]
async fn a_degraded_c4_read_marks_assets_without_observations_unpriced() {
    let h = harness();
    h.registry.set_fail_sources(true);
    let quotes = h
        .quotes
        .get_prices(&caller("portfolio"), &["asset-btc".to_string()])
        .await
        .unwrap();
    assert_eq!(quotes[0].freshness(), Freshness::Unpriced);
}

#[tokio::test]
async fn a_stale_quote_names_the_newest_source() {
    let h = harness();
    top100_tiers(&h).await;
    h.observations
        .seed(accepted("asset-btc", "binance", "100", ago(20)));
    h.observations
        .seed(accepted("asset-btc", "coingecko", "99", ago(10)));
    let quotes = h
        .quotes
        .get_prices(&caller("portfolio"), &["asset-btc".to_string()])
        .await
        .unwrap();
    assert_eq!(quotes[0].freshness(), Freshness::Stale);
    assert_eq!(quotes[0].source(), Some("coingecko"));
}

#[tokio::test]
async fn the_read_path_records_freshness_metrics() {
    let h = harness();
    top100_tiers(&h).await;
    h.observations
        .seed(accepted("asset-btc", "binance", "100", ago(1)));
    let _ = h
        .quotes
        .get_prices(&caller("portfolio"), &["asset-btc".to_string()])
        .await;
    let recorded = h.metrics.quotes();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].0, Freshness::Live);
    assert_eq!(recorded[0].1, PriceTier::Top100);
    assert_eq!(recorded[0].2, 60);
}

// ---------------------------------------------------------------------------
// WF2: serve one FX rate
// ---------------------------------------------------------------------------

fn rate(base: &str, quote: &str, value: &str, observed_at: DateTime<Utc>) -> FxRate {
    FxRate::record(
        FxRateId::generate(),
        base,
        quote,
        decimal(value),
        "binance",
        observed_at,
        observed_at,
    )
    .unwrap()
}

#[tokio::test]
async fn a_fresh_direct_pair_is_served_with_its_source() {
    let h = harness();
    h.fx_repo.seed(rate("EUR", "USD", "1.10", ago(5)));
    let served =
        h.fx.get_rate(&caller("portfolio"), "eur", "usd")
            .await
            .unwrap();
    assert_eq!(served.base(), "EUR");
    assert_eq!(served.quote(), "USD");
    assert_eq!(served.rate(), decimal("1.10"));
    assert_eq!(served.source(), "binance");
    assert!(served.derived_from().is_empty());
}

#[tokio::test]
async fn a_stale_direct_pair_is_derived_through_two_fresh_legs() {
    let h = harness();
    h.fx_repo.seed(rate(
        "EUR",
        "GBP",
        "0.80",
        now() - Duration::seconds(FX_BOUND_SECONDS + 60),
    ));
    h.fx_repo.seed(rate("EUR", "USD", "1.10", ago(5)));
    h.fx_repo.seed(rate("USD", "GBP", "0.80", ago(3)));
    let served =
        h.fx.get_rate(&caller("portfolio"), "EUR", "GBP")
            .await
            .unwrap();
    assert_eq!(served.rate(), decimal("0.88"));
    assert_eq!(served.source(), "derived");
    assert_eq!(served.derived_from().len(), 2);
}

#[tokio::test]
async fn a_reverse_stored_pair_serves_the_requested_direction() {
    let h = harness();
    h.fx_repo.seed(rate("VND", "USD", "25000", ago(5)));
    let served =
        h.fx.get_rate(&caller("portfolio"), "USD", "VND")
            .await
            .unwrap();
    assert_eq!(served.base(), "USD");
    assert_eq!(served.quote(), "VND");
    assert_eq!(served.rate(), Decimal::ONE / decimal("25000"));
    assert_eq!(served.source(), "binance");
    assert_eq!(served.observed_at(), ago(5));
    assert!(served.derived_from().is_empty());
}

#[tokio::test]
async fn a_reverse_leg_derives_eur_to_vnd_through_usd() {
    let h = harness();
    h.fx_repo.seed(rate("EUR", "USD", "1.10", ago(5)));
    h.fx_repo.seed(rate("VND", "USD", "25000", ago(3)));
    let served =
        h.fx.get_rate(&caller("portfolio"), "EUR", "VND")
            .await
            .unwrap();
    let expected = decimal("1.10") * (Decimal::ONE / decimal("25000"));
    assert_eq!(served.rate(), expected);
    assert_eq!(served.source(), "derived");
    assert_eq!(served.derived_from().len(), 2);
}

#[tokio::test]
async fn a_cross_rate_with_a_stale_leg_is_unavailable() {
    let h = harness();
    h.fx_repo.seed(rate(
        "EUR",
        "USD",
        "1.10",
        now() - Duration::seconds(FX_BOUND_SECONDS + 60),
    ));
    h.fx_repo.seed(rate("USD", "GBP", "0.80", ago(3)));
    let error =
        h.fx.get_rate(&caller("portfolio"), "EUR", "GBP")
            .await
            .unwrap_err();
    assert!(matches!(error, AppError::NotFound("fx rate")));
}

#[tokio::test]
async fn a_missing_pair_is_unavailable() {
    let h = harness();
    let error =
        h.fx.get_rate(&caller("portfolio"), "EUR", "JPY")
            .await
            .unwrap_err();
    assert!(matches!(error, AppError::NotFound("fx rate")));
}

#[tokio::test]
async fn the_same_base_and_quote_is_refused() {
    let h = harness();
    let error =
        h.fx.get_rate(&caller("portfolio"), "EUR", "eur")
            .await
            .unwrap_err();
    assert!(matches!(error, AppError::Validation(_)));
}

#[tokio::test]
async fn a_malformed_currency_is_refused() {
    let h = harness();
    let error =
        h.fx.get_rate(&caller("portfolio"), "EURO", "USD")
            .await
            .unwrap_err();
    assert!(matches!(error, AppError::Validation(_)));
}

#[tokio::test]
async fn a_caller_without_the_fx_operation_is_refused() {
    let h = harness();
    let error =
        h.fx.get_rate(&caller("unknown"), "EUR", "USD")
            .await
            .unwrap_err();
    assert!(matches!(error, AppError::Forbidden(_)));
}

#[tokio::test]
async fn the_fx_path_records_the_age_metric() {
    let h = harness();
    h.fx_repo.seed(rate("EUR", "USD", "1.10", ago(5)));
    let _ = h.fx.get_rate(&caller("portfolio"), "EUR", "USD").await;
    let recorded = h.metrics.fx();
    assert_eq!(recorded[0].0, "EUR/USD");
    assert_eq!(recorded[0].1, 300);
}

// ---------------------------------------------------------------------------
// WF3: refresh runs
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_scheduled_top100_run_prices_its_slice_and_succeeds() {
    let h = harness();
    h.provider
        .script_quote("binance", "asset-btc", vec![reading("100", ago(1))]);
    h.provider
        .script_fx("binance", "EUR", "USD", vec![fx_reading("1.10", ago(1))]);

    let outcome = h
        .refresh
        .run(
            &caller("job-orchestration"),
            RefreshJob::Top100,
            Trigger::Scheduled,
            "corr",
        )
        .await
        .unwrap();
    let run = outcome.run();
    assert_eq!(run.status(), RunStatus::Succeeded);
    assert_eq!(run.requested_count(), 1);
    assert_eq!(run.priced_count(), 1);
    assert_eq!(run.unpriced_count(), 0);

    let observations = h.observations.all();
    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].source_name(), "binance");
    assert_eq!(observations[0].status(), ObservationStatus::Accepted);

    let tiers = h.tiers.all();
    assert_eq!(tiers.len(), 3, "the top100 job refreshes every tier row");
    assert!(tiers
        .iter()
        .any(|tier| tier.asset_id() == "asset-btc" && tier.tier() == PriceTier::Top100));
    assert!(tiers
        .iter()
        .any(|tier| tier.asset_id() == "asset-tail" && tier.tier() == PriceTier::Tail));

    let fx = h.fx_repo.all();
    assert_eq!(fx.len(), 1);
    assert_eq!(fx[0].base(), "EUR");
    assert_eq!(fx[0].quote(), "USD");
}

#[tokio::test]
async fn a_manual_run_returns_the_active_run_when_the_lease_is_held() {
    let h = harness();
    let active = RefreshRun::claim(RefreshRunId::generate(), RefreshJob::Top100, now());
    h.runs.seed(active.clone());

    let outcome = h
        .refresh
        .run(
            &caller("job-orchestration"),
            RefreshJob::Top100,
            Trigger::Manual,
            "corr",
        )
        .await
        .unwrap();
    match outcome {
        RunOutcome::AlreadyRunning(run) => assert_eq!(run.id(), active.id()),
        RunOutcome::Completed(_) => panic!("the held lease must not start a run"),
    }
    assert_eq!(h.provider.call_count(), 0);
}

#[tokio::test]
async fn a_scheduled_run_refuses_a_held_lease_with_a_conflict() {
    let h = harness();
    let active = RefreshRun::claim(RefreshRunId::generate(), RefreshJob::Top100, now());
    h.runs.seed(active.clone());

    let error = h
        .refresh
        .run(
            &caller("job-orchestration"),
            RefreshJob::Top100,
            Trigger::Scheduled,
            "corr",
        )
        .await
        .unwrap_err();
    match error {
        AppError::Conflict { run_id, .. } => assert_eq!(run_id, Some(active.id())),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn the_two_jobs_hold_independent_leases() {
    let h = harness();
    let active = RefreshRun::claim(RefreshRunId::generate(), RefreshJob::Top100, now());
    h.runs.seed(active);
    h.provider
        .script_quote("binance", "asset-tail", vec![reading("5", ago(1))]);

    let outcome = h
        .refresh
        .run(
            &caller("job-orchestration"),
            RefreshJob::Tail,
            Trigger::Scheduled,
            "corr",
        )
        .await
        .unwrap();
    assert!(matches!(outcome, RunOutcome::Completed(_)));
}

#[tokio::test]
async fn a_transient_failure_is_retried_before_the_next_source_prices() {
    let h = harness();
    let transient = || ProviderError::transient(ErrorCategory::Network, "timeout");
    h.provider.script_quote(
        "binance",
        "asset-btc",
        vec![Err(transient()), Err(transient()), Err(transient())],
    );
    h.provider
        .script_quote("coingecko", "asset-btc", vec![reading("99", ago(1))]);

    let outcome = h
        .refresh
        .run(
            &caller("job-orchestration"),
            RefreshJob::Top100,
            Trigger::Scheduled,
            "corr",
        )
        .await
        .unwrap();
    assert_eq!(outcome.run().priced_count(), 1);
    let observations = h.observations.accepted();
    assert_eq!(observations[0].source_name(), "coingecko");
    assert_eq!(h.sleeper.pauses().len(), 2, "two retries back off");
    assert_eq!(
        h.provider
            .calls()
            .iter()
            .filter(|(source, asset)| source == "binance" && asset == "asset-btc")
            .count(),
        3
    );
}

#[tokio::test]
async fn a_permanent_failure_is_not_retried() {
    let h = harness();
    h.provider.script_quote(
        "binance",
        "asset-btc",
        vec![Err(ProviderError::permanent(
            ErrorCategory::Provider,
            "400",
        ))],
    );
    h.provider
        .script_quote("coingecko", "asset-btc", vec![reading("99", ago(1))]);

    let outcome = h
        .refresh
        .run(
            &caller("job-orchestration"),
            RefreshJob::Top100,
            Trigger::Scheduled,
            "corr",
        )
        .await
        .unwrap();
    assert_eq!(outcome.run().priced_count(), 1);
    assert!(h.sleeper.pauses().is_empty());
    assert_eq!(
        h.provider
            .calls()
            .iter()
            .filter(|(source, asset)| source == "binance" && asset == "asset-btc")
            .count(),
        1
    );
}

#[tokio::test]
async fn three_consecutive_transient_failures_open_the_circuit() {
    let h = harness();
    let transient = || ProviderError::transient(ErrorCategory::Network, "timeout");
    h.registry.set_assets(vec![asset("asset-btc", 1)]);
    h.provider.script_quote(
        "binance",
        "asset-btc",
        vec![Err(transient()), Err(transient()), Err(transient())],
    );
    h.provider
        .script_quote("coingecko", "asset-btc", vec![reading("99", ago(1))]);
    let first = h
        .refresh
        .run(
            &caller("job-orchestration"),
            RefreshJob::Top100,
            Trigger::Scheduled,
            "corr",
        )
        .await
        .unwrap();
    assert_eq!(first.run().priced_count(), 1);
    assert_eq!(
        h.provider
            .calls()
            .iter()
            .filter(|(source, asset)| source == "binance" && asset == "asset-btc")
            .count(),
        3
    );

    // The circuit is still open on the next run, so the source is skipped for a
    // different asset without another probe.
    h.registry.set_assets(vec![asset("asset-eth", 2)]);
    h.provider
        .script_quote("binance", "asset-eth", vec![reading("1", ago(1))]);
    h.provider
        .script_quote("coingecko", "asset-eth", vec![reading("2", ago(1))]);
    let second = h
        .refresh
        .run(
            &caller("job-orchestration"),
            RefreshJob::Top100,
            Trigger::Scheduled,
            "corr",
        )
        .await
        .unwrap();
    assert_eq!(second.run().priced_count(), 1);
    assert!(!h
        .provider
        .calls()
        .iter()
        .any(|(source, asset)| source == "binance" && asset == "asset-eth"));
}

#[tokio::test]
async fn a_rate_limited_source_is_deferred_and_the_next_source_prices() {
    let h = harness();
    h.limiter.block_source("binance");
    h.provider
        .script_quote("coingecko", "asset-btc", vec![reading("99", ago(1))]);

    let outcome = h
        .refresh
        .run(
            &caller("job-orchestration"),
            RefreshJob::Top100,
            Trigger::Scheduled,
            "corr",
        )
        .await
        .unwrap();
    assert_eq!(outcome.run().priced_count(), 1);
    assert!(h
        .provider
        .calls()
        .iter()
        .all(|(source, _)| source != "binance"));
    assert!(h
        .metrics
        .source_errors()
        .iter()
        .any(|(_, category)| *category == ErrorCategory::RateLimit));
}

#[tokio::test]
async fn a_rate_limited_source_keeps_the_previous_observation_for_selection() {
    let h = harness();
    top100_tiers(&h).await;
    h.observations
        .seed(accepted("asset-btc", "binance", "100", ago(1)));
    h.limiter.block(true);

    let outcome = h
        .refresh
        .run(
            &caller("job-orchestration"),
            RefreshJob::Top100,
            Trigger::Scheduled,
            "corr",
        )
        .await
        .unwrap();
    assert_eq!(outcome.run().unpriced_count(), 1);

    let quotes = h
        .quotes
        .get_prices(&caller("portfolio"), &["asset-btc".to_string()])
        .await
        .unwrap();
    assert_eq!(quotes[0].source(), Some("binance"));
    assert_eq!(quotes[0].freshness(), Freshness::Live);
}

#[tokio::test]
async fn a_zero_reading_is_rejected_and_does_not_replace_the_previous_price() {
    let h = harness();
    top100_tiers(&h).await;
    h.observations
        .seed(accepted("asset-btc", "binance", "100", ago(1)));
    h.provider
        .script_quote("binance", "asset-btc", vec![reading("0", ago(0))]);

    let outcome = h
        .refresh
        .run(
            &caller("job-orchestration"),
            RefreshJob::Top100,
            Trigger::Scheduled,
            "corr",
        )
        .await
        .unwrap();
    assert_eq!(outcome.run().unpriced_count(), 1);
    let stored = h.observations.all();
    assert!(stored
        .iter()
        .any(|row| row.status() == ObservationStatus::Rejected));

    let quotes = h
        .quotes
        .get_prices(&caller("portfolio"), &["asset-btc".to_string()])
        .await
        .unwrap();
    assert_eq!(quotes[0].price(), Some(decimal("100")));
}

#[tokio::test]
async fn a_malformed_reading_is_rejected() {
    let h = harness();
    h.provider
        .script_quote("binance", "asset-btc", vec![reading("NaN", ago(0))]);
    let outcome = h
        .refresh
        .run(
            &caller("job-orchestration"),
            RefreshJob::Top100,
            Trigger::Scheduled,
            "corr",
        )
        .await
        .unwrap();
    assert_eq!(outcome.run().unpriced_count(), 1);
    let rejected = h
        .observations
        .all()
        .into_iter()
        .find(|row| row.status() == ObservationStatus::Rejected)
        .unwrap();
    assert_eq!(rejected.rejection_reason(), Some("malformed price"));
}

#[tokio::test]
async fn a_clock_skewed_reading_is_rejected() {
    let h = harness();
    let ahead = now() + Duration::seconds(600);
    h.provider
        .script_quote("binance", "asset-btc", vec![reading("100", ahead)]);
    let outcome = h
        .refresh
        .run(
            &caller("job-orchestration"),
            RefreshJob::Top100,
            Trigger::Scheduled,
            "corr",
        )
        .await
        .unwrap();
    assert_eq!(outcome.run().unpriced_count(), 1);
    let rejected = h
        .observations
        .all()
        .into_iter()
        .find(|row| row.status() == ObservationStatus::Rejected)
        .unwrap();
    assert_eq!(
        rejected.rejection_reason(),
        Some("clock skew beyond tolerance")
    );
}

#[tokio::test]
async fn one_accepted_observation_per_asset_per_run() {
    let h = harness();
    h.provider
        .script_quote("binance", "asset-btc", vec![reading("100", ago(1))]);
    h.provider
        .script_quote("coingecko", "asset-btc", vec![reading("99", ago(1))]);

    let outcome = h
        .refresh
        .run(
            &caller("job-orchestration"),
            RefreshJob::Top100,
            Trigger::Scheduled,
            "corr",
        )
        .await
        .unwrap();
    assert_eq!(outcome.run().priced_count(), 1);
    assert_eq!(h.observations.accepted().len(), 1);
    assert_eq!(
        h.provider
            .calls()
            .iter()
            .filter(|(source, asset)| source == "binance" && asset == "asset-btc")
            .count(),
        1,
        "the next source is tried only on failure"
    );
    assert!(!h
        .provider
        .calls()
        .iter()
        .any(|(source, asset)| source == "coingecko" && asset == "asset-btc"));
}

#[tokio::test]
async fn the_tail_job_does_not_refresh_tiers_or_fx() {
    let h = harness();
    h.provider
        .script_quote("binance", "asset-tail", vec![reading("5", ago(1))]);

    let outcome = h
        .refresh
        .run(
            &caller("job-orchestration"),
            RefreshJob::Tail,
            Trigger::Scheduled,
            "corr",
        )
        .await
        .unwrap();
    assert_eq!(outcome.run().priced_count(), 1);
    assert!(h.tiers.all().is_empty());
    assert!(h.fx_repo.all().is_empty());
}

#[tokio::test]
async fn the_run_deadline_stops_and_records_what_was_priced() {
    let h = harness();
    let assets: Vec<_> = (0..5)
        .map(|index| asset(&format!("asset-tail-{}", index + 1), 101 + index))
        .collect();
    h.registry.set_assets(assets);
    h.provider
        .advance_clock_on_call(Duration::seconds(601), h.clock.clone());
    for index in 1..=5 {
        h.provider.script_quote(
            "binance",
            &format!("asset-tail-{index}"),
            vec![reading("5", ago(1))],
        );
    }

    let outcome = h
        .refresh
        .run(
            &caller("job-orchestration"),
            RefreshJob::Tail,
            Trigger::Scheduled,
            "corr",
        )
        .await
        .unwrap();
    let run = outcome.run();
    assert_eq!(run.status(), RunStatus::Succeeded);
    assert_eq!(run.requested_count(), 4, "the four in-flight assets finish");
    assert_eq!(run.priced_count(), 4);
}

#[tokio::test]
async fn a_failed_lease_renewal_stops_the_run() {
    let h = harness();
    let assets: Vec<_> = (0..5)
        .map(|index| asset(&format!("asset-tail-{}", index + 1), 101 + index))
        .collect();
    h.registry.set_assets(assets);
    h.provider
        .advance_clock_on_call(Duration::seconds(80), h.clock.clone());
    for index in 1..=5 {
        h.provider.script_quote(
            "binance",
            &format!("asset-tail-{index}"),
            vec![reading("5", ago(1))],
        );
    }

    let outcome = h
        .refresh
        .run(
            &caller("job-orchestration"),
            RefreshJob::Tail,
            Trigger::Scheduled,
            "corr",
        )
        .await
        .unwrap();
    let run = outcome.run();
    assert_eq!(run.status(), RunStatus::Failed);
    assert_eq!(run.requested_count(), 4);
    assert!(run.error_message().unwrap_or_default().contains("lease"));
}

#[tokio::test]
async fn a_run_with_no_usable_source_fails_and_is_audited() {
    let h = harness();
    h.provider.script_quote(
        "binance",
        "asset-btc",
        vec![Err(ProviderError::permanent(
            ErrorCategory::Provider,
            "400",
        ))],
    );

    let outcome = h
        .refresh
        .run(
            &caller("job-orchestration"),
            RefreshJob::Top100,
            Trigger::Scheduled,
            "corr",
        )
        .await
        .unwrap();
    assert_eq!(outcome.run().status(), RunStatus::Failed);
    assert_eq!(outcome.run().requested_count(), 1);
    assert_eq!(outcome.run().priced_count(), 0);
    assert_eq!(outcome.run().unpriced_count(), 1);

    let events = h.audit.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].action(), "price_refresh_failed");
    assert_eq!(events[0].actor_kind(), "system");
    assert_eq!(
        events[0].context().get("job").map(String::as_str),
        Some("top100")
    );
    assert_eq!(
        events[0].context().get("trigger").map(String::as_str),
        Some("scheduled")
    );
}

#[tokio::test]
async fn a_manual_failed_run_carries_the_service_identity_and_trigger() {
    let h = harness();
    h.provider.script_quote(
        "binance",
        "asset-btc",
        vec![Err(ProviderError::permanent(
            ErrorCategory::Provider,
            "400",
        ))],
    );
    let _ = h
        .refresh
        .run(
            &caller("job-orchestration"),
            RefreshJob::Top100,
            Trigger::Manual,
            "corr",
        )
        .await;
    let events = h.audit.events();
    assert_eq!(
        events[0].context().get("trigger").map(String::as_str),
        Some("manual")
    );
}

#[tokio::test]
async fn a_refused_vault_read_fails_the_run_and_is_audited() {
    let h = harness();
    h.secrets.set_refused(true);
    let outcome = h
        .refresh
        .run(
            &caller("job-orchestration"),
            RefreshJob::Top100,
            Trigger::Scheduled,
            "corr",
        )
        .await
        .unwrap();
    assert_eq!(outcome.run().status(), RunStatus::Failed);
    assert!(outcome
        .run()
        .error_message()
        .unwrap_or_default()
        .contains("refused"));
    assert!(h
        .audit
        .events()
        .iter()
        .any(|event| event.action() == "price_refresh_failed"));
}

#[tokio::test]
async fn an_audit_failure_is_logged_and_counted_but_the_run_still_finishes() {
    let h = harness();
    h.audit.set_fail(true);
    h.provider.script_quote(
        "binance",
        "asset-btc",
        vec![Err(ProviderError::permanent(
            ErrorCategory::Provider,
            "400",
        ))],
    );

    let outcome = h
        .refresh
        .run(
            &caller("job-orchestration"),
            RefreshJob::Top100,
            Trigger::Scheduled,
            "corr",
        )
        .await
        .unwrap();
    assert_eq!(outcome.run().status(), RunStatus::Failed);
    assert_eq!(h.metrics.audit_failures(), 1);
}

#[tokio::test]
async fn the_run_records_its_metrics() {
    let h = harness();
    h.provider
        .script_quote("binance", "asset-btc", vec![reading("100", ago(1))]);
    h.provider
        .script_fx("binance", "EUR", "USD", vec![fx_reading("1.10", ago(1))]);
    let _ = h
        .refresh
        .run(
            &caller("job-orchestration"),
            RefreshJob::Top100,
            Trigger::Scheduled,
            "corr",
        )
        .await;
    assert!(h
        .metrics
        .runs()
        .iter()
        .any(|(job, status)| *job == RefreshJob::Top100 && status.as_str() == "succeeded"));
    assert!(h
        .metrics
        .unpriced()
        .iter()
        .any(|(job, count)| *job == RefreshJob::Top100 && *count == 0));
}

#[tokio::test]
async fn source_quotes_and_launch_bases_are_both_refreshed() {
    let h = harness();
    h.registry.set_sources(vec![
        source("binance", 1, false),
        PriceSource::new("coingecko", 2, "USDT", 120, true, true).unwrap(),
    ]);
    h.provider
        .script_quote("binance", "asset-btc", vec![reading("100", ago(1))]);
    h.provider
        .script_fx("binance", "EUR", "USD", vec![fx_reading("1.10", ago(1))]);
    h.provider
        .script_fx("binance", "VND", "USD", vec![fx_reading("0.00004", ago(1))]);
    h.provider
        .script_fx("coingecko", "USDT", "USD", vec![fx_reading("1.00", ago(1))]);

    let outcome = h
        .refresh
        .run(
            &caller("job-orchestration"),
            RefreshJob::Top100,
            Trigger::Scheduled,
            "corr",
        )
        .await
        .unwrap();
    assert_eq!(outcome.run().status(), RunStatus::Succeeded);

    let stored = h.fx_repo.all();
    for pair in [("EUR", "USD"), ("VND", "USD"), ("USDT", "USD")] {
        assert!(
            stored
                .iter()
                .any(|rate| rate.base() == pair.0 && rate.quote() == pair.1),
            "missing {} / {}",
            pair.0,
            pair.1
        );
    }

    // USD → VND is the reverse of the VND → USD reading stored for the run.
    let usd_vnd =
        h.fx.get_rate(&caller("portfolio"), "USD", "VND")
            .await
            .unwrap();
    assert_eq!(usd_vnd.rate(), Decimal::ONE / decimal("0.00004"));

    // USDT → EUR crosses USDT → USD with the inversion of EUR → USD.
    let usdt_eur =
        h.fx.get_rate(&caller("portfolio"), "USDT", "EUR")
            .await
            .unwrap();
    assert_eq!(
        usdt_eur.rate(),
        decimal("1.00") * (Decimal::ONE / decimal("1.10"))
    );
    assert_eq!(usdt_eur.source(), "derived");
}

#[tokio::test]
async fn a_caller_without_the_run_operation_is_refused() {
    let h = harness();
    let error = h
        .refresh
        .run(
            &caller("portfolio"),
            RefreshJob::Top100,
            Trigger::Scheduled,
            "corr",
        )
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Forbidden(_)));
}

#[tokio::test]
async fn a_run_whose_c4_source_list_fails_is_recorded_as_failed() {
    let h = harness();
    h.registry.set_fail_sources(true);
    let outcome = h
        .refresh
        .run(
            &caller("job-orchestration"),
            RefreshJob::Top100,
            Trigger::Scheduled,
            "corr",
        )
        .await
        .unwrap();
    assert_eq!(outcome.run().status(), RunStatus::Failed);
}

#[tokio::test]
async fn a_run_whose_canonical_asset_list_fails_is_recorded_as_failed() {
    let h = harness();
    h.registry.set_fail_assets(true);
    let outcome = h
        .refresh
        .run(
            &caller("job-orchestration"),
            RefreshJob::Top100,
            Trigger::Scheduled,
            "corr",
        )
        .await
        .unwrap();
    assert_eq!(outcome.run().status(), RunStatus::Failed);
}

#[tokio::test]
async fn a_sweep_marks_expired_runs_failed() {
    let h = harness();
    h.runs.seed(RefreshRun::claim(
        RefreshRunId::generate(),
        RefreshJob::Top100,
        now(),
    ));
    h.clock.advance(Duration::seconds(301));
    assert_eq!(h.refresh.sweep().await.unwrap(), 1);
}

// ---------------------------------------------------------------------------
// WF retention
// ---------------------------------------------------------------------------

#[tokio::test]
async fn retention_prunes_rows_older_than_ninety_days_keeping_the_newest() {
    let h = harness();
    let ancient = now() - Duration::days(100);
    let recent = now() - Duration::days(10);
    h.observations
        .seed(accepted("asset-btc", "binance", "100", ancient));
    h.observations
        .seed(accepted("asset-btc", "binance", "110", recent));
    h.observations
        .seed(accepted("asset-eth", "binance", "20", ancient));
    h.fx_repo.seed(rate("EUR", "USD", "1.10", ancient));
    h.fx_repo.seed(rate("EUR", "USD", "1.12", recent));

    let report = h.retention.sweep().await.unwrap();
    assert_eq!(report.observations, 1);
    assert_eq!(report.fx_rates, 1);
    assert!(h
        .observations
        .all()
        .iter()
        .any(|row| row.asset_id() == "asset-eth"));
}

#[tokio::test]
async fn retention_days_matches_the_policy() {
    assert_eq!(market_data_domain::RETENTION_DAYS, 90);
    assert_eq!(market_data_domain::MAX_ASSET_IDS, 1_000);
}

#[tokio::test]
async fn a_provider_key_is_read_from_vault_for_the_call() {
    let h = harness();
    h.secrets.set_key("binance", "key-1");
    h.provider
        .script_quote("binance", "asset-btc", vec![reading("100", ago(1))]);
    let _ = h
        .refresh
        .run(
            &caller("job-orchestration"),
            RefreshJob::Top100,
            Trigger::Scheduled,
            "corr",
        )
        .await;
    assert_eq!(h.observations.accepted().len(), 1);
}
