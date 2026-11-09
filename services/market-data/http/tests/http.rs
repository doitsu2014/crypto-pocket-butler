//! HTTP boundary tests: RFC 7807 problems, correlation ids, the per-caller
//! service-token allow-list, C6 prices and FX, and the C11 run endpoint.
//!
//! The router is exercised with `tower::ServiceExt::oneshot`; no port is live.

#![allow(dead_code)]

#[path = "../../application/tests/fakes.rs"]
mod fakes;

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use chrono::{DateTime, Duration, TimeZone, Utc};
use http_body_util::BodyExt;
use market_data_application::fx_service::FxService;
use market_data_application::quotes::QuoteService;
use market_data_application::refresh::RefreshService;
use market_data_domain::{
    FxRate, FxRateId, ObservationId, Operation, PriceObservation, RefreshJob, RefreshRun,
    RefreshRunId,
};
use market_data_http::metrics::Metrics;
use market_data_http::{router, worker_router, AppState, WorkerState};
use rust_decimal::Decimal;
use serde_json::Value;
use std::str::FromStr;
use tower::ServiceExt;

use fakes::{
    asset, source, FakeAudit, FakeFx, FakeObservations, FakeProvider, FakeRateLimiter,
    FakeRegistry, FakeRuns, FakeSecrets, FakeServiceTokens, FakeTiers, MutableClock, Ready,
    RecordingSleeper,
};

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, 12, 0, 0).unwrap()
}

fn decimal(raw: &str) -> Decimal {
    Decimal::from_str(raw).unwrap()
}

struct Harness {
    registry: Arc<FakeRegistry>,
    observations: Arc<FakeObservations>,
    fx_repo: Arc<FakeFx>,
    runs: Arc<FakeRuns>,
    provider: Arc<FakeProvider>,
    clock: Arc<MutableClock>,
    state: AppState,
    worker: WorkerState,
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
    let metrics = Arc::new(Metrics::default());

    let quotes = Arc::new(QuoteService::new(
        registry.clone(),
        observations.clone(),
        tiers.clone(),
        metrics.clone(),
        clock.clone(),
    ));
    let fx = Arc::new(FxService::new(
        fx_repo.clone(),
        metrics.clone(),
        clock.clone(),
    ));
    let refresh = Arc::new(RefreshService::new(
        registry.clone(),
        observations.clone(),
        fx_repo.clone(),
        runs.clone(),
        tiers,
        provider.clone(),
        secrets,
        limiter,
        audit,
        clock.clone(),
        sleeper,
        metrics.clone(),
        vec!["EUR".to_string()],
    ));
    let service_tokens = Arc::new(FakeServiceTokens::with(&[
        (
            "portfolio-token",
            "portfolio",
            vec![Operation::PricesRead, Operation::FxRead],
        ),
        (
            "job-token",
            "job-orchestration",
            vec![
                Operation::PricesRead,
                Operation::FxRead,
                Operation::RunRefresh,
            ],
        ),
        (
            "smoke-token",
            "smoke-reader",
            vec![Operation::PricesRead, Operation::FxRead],
        ),
        ("identity-token", "identity", Vec::new()),
    ]));
    let readiness = Arc::new(Ready(true));
    let state = AppState {
        service_name: "market-data-api",
        quotes,
        fx,
        refresh,
        service_tokens,
        readiness,
        metrics: metrics.clone(),
    };
    let worker = WorkerState {
        service_name: "market-data-worker",
        readiness: Arc::new(Ready(true)),
        metrics,
    };
    Harness {
        registry,
        observations,
        fx_repo,
        runs,
        provider,
        clock,
        state,
        worker,
    }
}

async fn send(
    app: axum::Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    extra: &[(&str, &str)],
) -> (StatusCode, axum::http::HeaderMap, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    for (name, value) in extra {
        builder = builder.header(*name, *value);
    }
    let response = app
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, headers, body)
}

async fn authenticated(
    app: axum::Router,
    method: &str,
    uri: &str,
    token: &str,
) -> (StatusCode, axum::http::HeaderMap, Value) {
    send(app, method, uri, Some(token), &[]).await
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

// ---------------------------------------------------------------------------
// Platform routes
// ---------------------------------------------------------------------------

#[tokio::test]
async fn healthz_reports_ok() {
    let h = harness();
    let (status, _, body) = send(router(h.state), "GET", "/healthz", None, &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn readyz_reports_ready() {
    let h = harness();
    let (status, _, body) = send(router(h.state), "GET", "/readyz", None, &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ready");
}

#[tokio::test]
async fn readyz_reports_503_when_a_dependency_is_down() {
    let h = harness();
    let state = AppState {
        readiness: Arc::new(Ready(false)),
        ..h.state
    };
    let (status, _, _) = send(router(state), "GET", "/readyz", None, &[]).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn metrics_are_exposed_in_prometheus_text() {
    let h = harness();
    let response = router(h.state)
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(text.contains("market_data_health"));
    assert!(text.contains("market_data_request_duration_seconds"));
}

#[tokio::test]
async fn the_worker_listener_exposes_its_probes() {
    let h = harness();
    let (status, _, body) = send(worker_router(h.worker), "GET", "/healthz", None, &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["service"], "market-data-worker");
}

// ---------------------------------------------------------------------------
// C6 prices
// ---------------------------------------------------------------------------

#[tokio::test]
async fn prices_return_provenance_and_freshness() {
    let h = harness();
    h.observations.seed(accepted(
        "asset-btc",
        "binance",
        "100",
        now() - Duration::seconds(60),
    ));
    let (status, _, body) = authenticated(
        router(h.state),
        "GET",
        "/v1/prices?assetIds=asset-btc",
        "portfolio-token",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["assetId"], "asset-btc");
    assert_eq!(body[0]["price"], "100");
    assert_eq!(body[0]["source"], "binance");
    assert_eq!(body[0]["freshness"], "live");
    assert!(body[0]["observedAt"].is_string());
}

#[tokio::test]
async fn prices_accept_repeated_and_comma_separated_ids() {
    let h = harness();
    h.observations.seed(accepted(
        "asset-btc",
        "binance",
        "100",
        now() - Duration::seconds(60),
    ));
    h.observations.seed(accepted(
        "asset-eth",
        "binance",
        "2000",
        now() - Duration::seconds(60),
    ));
    let (status, _, body) = authenticated(
        router(h.state),
        "GET",
        "/v1/prices?assetIds=asset-btc,asset-eth&assetIds=asset-tail",
        "portfolio-token",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn prices_mark_an_unobserved_asset_unpriced() {
    let h = harness();
    let (status, _, body) = authenticated(
        router(h.state),
        "GET",
        "/v1/prices?assetIds=asset-btc",
        "portfolio-token",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["freshness"], "unpriced");
    assert!(body[0]["price"].is_null());
    assert!(body[0]["source"].is_null());
}

#[tokio::test]
async fn prices_require_a_bearer_token() {
    let h = harness();
    let (status, _, body) = send(
        router(h.state),
        "GET",
        "/v1/prices?assetIds=asset-btc",
        None,
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["status"], 401);
}

#[tokio::test]
async fn prices_refuse_an_unknown_service_token() {
    let h = harness();
    let (status, _, _) = authenticated(
        router(h.state),
        "GET",
        "/v1/prices?assetIds=asset-btc",
        "unknown-token",
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn prices_refuse_a_caller_without_the_operation() {
    let h = harness();
    let (status, _, _) = authenticated(
        router(h.state),
        "GET",
        "/v1/prices?assetIds=asset-btc",
        "identity-token",
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn prices_refuse_a_missing_or_empty_list() {
    let h = harness();
    let (status, _, _) = authenticated(
        router(h.state.clone()),
        "GET",
        "/v1/prices",
        "portfolio-token",
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let (status, _, _) = authenticated(
        router(h.state),
        "GET",
        "/v1/prices?assetIds=%20%20",
        "portfolio-token",
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn prices_refuse_more_than_one_thousand_ids() {
    let h = harness();
    let ids: Vec<String> = (0..1_001).map(|index| format!("asset-{index}")).collect();
    let uri = format!("/v1/prices?assetIds={}", ids.join(","));
    let (status, _, _) = authenticated(router(h.state), "GET", &uri, "portfolio-token").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn prices_return_503_when_postgres_is_down() {
    let h = harness();
    h.observations.set_fail(true);
    let (status, _, _) = authenticated(
        router(h.state),
        "GET",
        "/v1/prices?assetIds=asset-btc",
        "portfolio-token",
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn the_smoke_reader_token_may_read_prices() {
    let h = harness();
    h.observations.seed(accepted(
        "asset-btc",
        "binance",
        "100",
        now() - Duration::seconds(60),
    ));
    let (status, _, _) = authenticated(
        router(h.state),
        "GET",
        "/v1/prices?assetIds=asset-btc",
        "smoke-token",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

// ---------------------------------------------------------------------------
// C6 FX
// ---------------------------------------------------------------------------

#[tokio::test]
async fn fx_returns_a_fresh_direct_pair() {
    let h = harness();
    h.fx_repo
        .seed(rate("EUR", "USD", "1.10", now() - Duration::seconds(60)));
    let (status, _, body) = authenticated(
        router(h.state.clone()),
        "GET",
        "/v1/fx?base=EUR&quote=USD",
        "portfolio-token",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["base"], "EUR");
    assert_eq!(body["quote"], "USD");
    assert_eq!(body["rate"], "1.10");
    assert_eq!(body["source"], "binance");
}

#[tokio::test]
async fn fx_returns_404_when_no_fresh_rate_exists() {
    let h = harness();
    let (status, _, body) = authenticated(
        router(h.state),
        "GET",
        "/v1/fx?base=EUR&quote=JPY",
        "portfolio-token",
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["status"], 404);
}

#[tokio::test]
async fn fx_refuses_an_identical_pair() {
    let h = harness();
    let (status, _, _) = authenticated(
        router(h.state),
        "GET",
        "/v1/fx?base=EUR&quote=eur",
        "portfolio-token",
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn fx_refuses_a_malformed_currency() {
    let h = harness();
    let (status, _, _) = authenticated(
        router(h.state),
        "GET",
        "/v1/fx?base=EURO&quote=USD",
        "portfolio-token",
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn fx_requires_a_bearer_token() {
    let h = harness();
    let (status, _, _) = send(
        router(h.state),
        "GET",
        "/v1/fx?base=EUR&quote=USD",
        None,
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn fx_returns_503_when_postgres_is_down() {
    let h = harness();
    h.fx_repo.set_fail(true);
    let (status, _, _) = authenticated(
        router(h.state),
        "GET",
        "/v1/fx?base=EUR&quote=USD",
        "portfolio-token",
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn the_smoke_reader_token_may_read_fx() {
    let h = harness();
    h.fx_repo
        .seed(rate("EUR", "USD", "1.10", now() - Duration::seconds(60)));
    let (status, _, _) = authenticated(
        router(h.state),
        "GET",
        "/v1/fx?base=EUR&quote=USD",
        "smoke-token",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

// ---------------------------------------------------------------------------
// C11 runs
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_top100_run_returns_200_with_its_result() {
    let h = harness();
    h.provider
        .script_quote("binance", "asset-btc", vec![Ok(quote("100"))]);
    h.provider
        .script_fx("binance", "EUR", "USD", vec![Ok(quote("1.10"))]);
    let (status, _, body) = send(
        router(h.state),
        "POST",
        "/v1/internal/run/price-refresh-top100",
        Some("job-token"),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["jobName"], "price-refresh-top100");
    assert_eq!(body["status"], "succeeded");
    assert!(body["runId"].is_string());
    assert!(body["startedAt"].is_string());
}

#[tokio::test]
async fn the_tail_run_returns_200_with_its_result() {
    let h = harness();
    h.registry.set_assets(vec![asset("asset-tail", 101)]);
    h.provider
        .script_quote("binance", "asset-tail", vec![Ok(quote("5"))]);
    let (status, _, body) = send(
        router(h.state),
        "POST",
        "/v1/internal/run/price-refresh-tail",
        Some("job-token"),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["jobName"], "price-refresh-tail");
    assert_eq!(body["status"], "succeeded");
}

#[tokio::test]
async fn an_unknown_job_is_a_404() {
    let h = harness();
    let (status, _, _) = send(
        router(h.state),
        "POST",
        "/v1/internal/run/price-refresh-all",
        Some("job-token"),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_scheduled_held_lease_is_a_409_with_the_active_run_id() {
    let h = harness();
    let active = RefreshRun::claim(RefreshRunId::generate(), RefreshJob::Top100, now());
    h.runs.seed(active.clone());
    let (status, _, body) = send(
        router(h.state),
        "POST",
        "/v1/internal/run/price-refresh-top100",
        Some("job-token"),
        &[("x-run-trigger", "scheduled")],
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["runId"], active.id().to_string());
}

#[tokio::test]
async fn a_manual_held_lease_is_a_202_with_the_active_run_id() {
    let h = harness();
    let active = RefreshRun::claim(RefreshRunId::generate(), RefreshJob::Top100, now());
    h.runs.seed(active.clone());
    let (status, _, body) = send(
        router(h.state),
        "POST",
        "/v1/internal/run/price-refresh-top100",
        Some("job-token"),
        &[("x-run-trigger", "manual")],
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(body["status"], "skipped");
    assert_eq!(body["runId"], active.id().to_string());
}

#[tokio::test]
async fn an_invalid_trigger_header_is_a_422() {
    let h = harness();
    let (status, _, _) = send(
        router(h.state),
        "POST",
        "/v1/internal/run/price-refresh-top100",
        Some("job-token"),
        &[("x-run-trigger", "cron")],
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn a_run_refuses_a_missing_token() {
    let h = harness();
    let (status, _, _) = send(
        router(h.state),
        "POST",
        "/v1/internal/run/price-refresh-top100",
        None,
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_run_refuses_a_caller_without_the_operation() {
    let h = harness();
    let (status, _, _) = send(
        router(h.state),
        "POST",
        "/v1/internal/run/price-refresh-top100",
        Some("portfolio-token"),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ---------------------------------------------------------------------------
// Boundary behaviour
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_valid_correlation_id_is_echoed() {
    let h = harness();
    let (_, headers, _) = send(
        router(h.state),
        "GET",
        "/healthz",
        None,
        &[("x-correlation-id", "test-correlation-1")],
    )
    .await;
    assert_eq!(
        headers
            .get("x-correlation-id")
            .and_then(|value| value.to_str().ok()),
        Some("test-correlation-1")
    );
}

#[tokio::test]
async fn an_unknown_route_is_a_404_problem() {
    let h = harness();
    let (status, _, body) = send(router(h.state), "GET", "/v1/nope", None, &[]).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["status"], 404);
    assert!(body["correlationId"].is_string());
}

#[tokio::test]
async fn problem_responses_use_the_problem_content_type() {
    let h = harness();
    let response = router(h.state)
        .oneshot(
            Request::builder()
                .uri("/v1/prices?assetIds=asset-btc")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("application/problem+json")
    );
}

fn quote(price: &str) -> market_data_application::ports::ProviderQuote {
    market_data_application::ports::ProviderQuote {
        price: price.to_string(),
        currency: "USD".to_string(),
        observed_at: now() - Duration::seconds(60),
    }
}
