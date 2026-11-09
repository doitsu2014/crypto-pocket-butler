//! HTTP boundary tests: RFC 7807 problems, correlation IDs, JWT enforcement,
//! admin gating, security headers, and metrics.
//!
//! The router is exercised with `tower::ServiceExt::oneshot`; no port is live.

mod common;

use std::sync::Arc;

use asset_registry_application::admin::RegistryAdmin;
use asset_registry_application::ports::{Clock, ReadinessProbe, TokenValidator};
use asset_registry_application::resolve_asset::ResolveAsset;
use asset_registry_application::resolve_price::ResolvePrice;
use asset_registry_domain::{Asset, ContractAddress, TokenContract};
use asset_registry_http::metrics::Metrics;
use asset_registry_http::routes::router;
use asset_registry_http::state::AppState;
use axum::body::Body;
use axum::http::{header, HeaderMap, Method, Request, StatusCode};
use chrono::{TimeZone, Utc};
use common::fakes::{
    FakeAssets, FakeAudit, FakeChains, FakeMetadata, FakeObservations, FakePriceProvider,
    FakeSources, FakeTokenValidator, FixedClock,
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

struct Ready(bool);

#[async_trait::async_trait]
impl ReadinessProbe for Ready {
    async fn check(&self) -> Result<(), String> {
        if self.0 {
            Ok(())
        } else {
            Err("database down".to_string())
        }
    }
}

fn fixed_clock() -> Arc<dyn Clock> {
    Arc::new(FixedClock::at(
        Utc.with_ymd_and_hms(2026, 11, 6, 12, 0, 0).unwrap(),
    ))
}

fn test_state(
    assets: Arc<FakeAssets>,
    observations: Arc<FakeObservations>,
    audit: Arc<FakeAudit>,
    validator: Arc<dyn TokenValidator>,
    readiness: Arc<dyn ReadinessProbe>,
) -> AppState {
    let chains = Arc::new(FakeChains::default());
    let sources = Arc::new(FakeSources::default());
    let clock = fixed_clock();
    let resolve_asset = Arc::new(ResolveAsset::new(
        assets.clone(),
        chains.clone(),
        observations.clone(),
        clock.clone(),
    ));
    let resolve_price = Arc::new(ResolvePrice::new(
        sources.clone(),
        Arc::new(FakePriceProvider::with(vec![])),
        clock.clone(),
    ));
    let admin = Arc::new(RegistryAdmin::new(
        assets.clone(),
        chains,
        sources,
        observations,
        Arc::new(FakeMetadata::default()),
        audit.clone(),
        clock.clone(),
    ));
    AppState {
        service_name: "asset-registry",
        resolve_asset,
        resolve_price,
        admin,
        assets,
        token_validator: validator,
        audit,
        clock,
        readiness,
        metrics: Arc::new(Metrics::default()),
    }
}

async fn send(
    state: &AppState,
    method: Method,
    uri: &str,
    token: Option<&str>,
    correlation: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, HeaderMap, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    if let Some(correlation) = correlation {
        builder = builder.header("x-correlation-id", correlation);
    }
    let request = match body {
        Some(body) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    };
    let response = router(state.clone()).oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, headers, value)
}

fn seeded_assets() -> Arc<FakeAssets> {
    let assets = Arc::new(FakeAssets::default());
    let asset = Asset::new("asset-usdc", "USDC", "USD Coin", 6, true).unwrap();
    assets.assets.lock().unwrap().push(asset.clone());
    assets.contracts.lock().unwrap().push(
        TokenContract::new(
            "tc-usdc",
            &asset_registry_domain::ChainId::new("ethereum").unwrap(),
            ContractAddress::parse("0xAbC").unwrap(),
            asset.id(),
            6,
        )
        .unwrap(),
    );
    assets
}

fn user_state(assets: Arc<FakeAssets>, audit: Arc<FakeAudit>) -> AppState {
    test_state(
        assets,
        Arc::new(FakeObservations::default()),
        audit,
        Arc::new(FakeTokenValidator::user()),
        Arc::new(Ready(true)),
    )
}

fn admin_state(assets: Arc<FakeAssets>, audit: Arc<FakeAudit>) -> AppState {
    test_state(
        assets,
        Arc::new(FakeObservations::default()),
        audit,
        Arc::new(FakeTokenValidator::admin()),
        Arc::new(Ready(true)),
    )
}

#[tokio::test]
async fn health_and_readiness_are_public() {
    let state = user_state(
        Arc::new(FakeAssets::default()),
        Arc::new(FakeAudit::default()),
    );

    let (status, _, body) = send(&state, Method::GET, "/healthz", None, None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["service"], "asset-registry");

    let (status, _, _) = send(&state, Method::GET, "/readyz", None, None, None).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn resolving_a_known_token_returns_the_canonical_asset() {
    let state = user_state(seeded_assets(), Arc::new(FakeAudit::default()));

    let (status, headers, body) = send(
        &state,
        Method::GET,
        "/v1/assets/resolve?chain=ethereum&contractAddress=0xabc&symbol=USDC",
        Some("valid-user-token"),
        None,
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], "asset-usdc");
    assert_eq!(body["symbol"], "USDC");
    assert_eq!(body["decimals"], 6);
    assert_eq!(body["canonical"], true);
    assert!(headers.contains_key("x-correlation-id"));
}

#[tokio::test]
async fn a_missing_token_is_rejected_before_any_use_case_runs() {
    let state = user_state(seeded_assets(), Arc::new(FakeAudit::default()));

    let (status, headers, body) = send(
        &state,
        Method::GET,
        "/v1/assets/resolve?chain=ethereum&contractAddress=0xabc&symbol=USDC",
        None,
        None,
        None,
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        headers.get(header::CONTENT_TYPE).unwrap(),
        "application/problem+json"
    );
    assert_eq!(body["status"], 401);
    assert!(body["correlationId"].as_str().unwrap().len() > 8);
}

#[tokio::test]
async fn an_unknown_chain_is_a_validation_problem_naming_the_field() {
    let state = user_state(
        Arc::new(FakeAssets::default()),
        Arc::new(FakeAudit::default()),
    );

    let (status, _, body) = send(
        &state,
        Method::GET,
        "/v1/assets/resolve?chain=cosmos&symbol=ATOM",
        Some("valid-user-token"),
        None,
        None,
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["status"], 422);
    assert!(body["detail"].as_str().unwrap().contains("chain"));
}

#[tokio::test]
async fn an_unresolved_token_is_a_404_problem_with_the_caller_correlation_id() {
    let state = user_state(
        Arc::new(FakeAssets::default()),
        Arc::new(FakeAudit::default()),
    );

    let (status, headers, body) = send(
        &state,
        Method::GET,
        "/v1/assets/resolve?chain=ethereum&contractAddress=0xdeadbeef&symbol=SCAM",
        Some("valid-user-token"),
        Some("corr-123"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        headers.get(header::CONTENT_TYPE).unwrap(),
        "application/problem+json"
    );
    assert_eq!(body["status"], 404);
    assert!(body["type"].as_str().unwrap().ends_with("asset-unresolved"));
    assert_eq!(body["correlationId"], "corr-123");
    assert_eq!(headers.get("x-correlation-id").unwrap(), "corr-123");
}

#[tokio::test]
async fn an_invalid_correlation_id_is_refused_with_400() {
    let state = user_state(
        Arc::new(FakeAssets::default()),
        Arc::new(FakeAudit::default()),
    );

    let (status, _, body) = send(
        &state,
        Method::GET,
        "/healthz",
        None,
        Some("contains spaces"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["status"], 400);
}

#[tokio::test]
async fn a_non_admin_cannot_reach_an_admin_route_and_the_denial_is_audited() {
    let audit = Arc::new(FakeAudit::default());
    let state = user_state(Arc::new(FakeAssets::default()), audit.clone());

    let (status, _, body) = send(
        &state,
        Method::POST,
        "/v1/admin/assets",
        Some("valid-user-token"),
        Some("corr-deny"),
        Some(json!({
            "id": "asset-usdc",
            "symbol": "USDC",
            "name": "USD Coin",
            "decimals": 6
        })),
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["status"], 403);
    let events = audit.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].action, "registry.admin.denied");
    assert_eq!(events[0].outcome, "denied");
    assert_eq!(events[0].correlation_id.as_deref(), Some("corr-deny"));
}

#[tokio::test]
async fn an_admin_can_create_an_asset_and_the_change_is_audited() {
    let audit = Arc::new(FakeAudit::default());
    let state = admin_state(Arc::new(FakeAssets::default()), audit.clone());

    let (status, _, body) = send(
        &state,
        Method::POST,
        "/v1/admin/assets",
        Some("valid-admin-token"),
        Some("corr-create"),
        Some(json!({
            "id": "asset-usdc",
            "symbol": "usdc",
            "name": "USD Coin",
            "decimals": 6
        })),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["id"], "asset-usdc");
    assert_eq!(body["symbol"], "USDC");
    let events = audit.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].action, "asset.create");
    assert_eq!(events[0].target, "asset-usdc");
}

#[tokio::test]
async fn a_malformed_admin_body_is_a_422_problem_not_a_plain_text_rejection() {
    let state = admin_state(
        Arc::new(FakeAssets::default()),
        Arc::new(FakeAudit::default()),
    );
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/admin/assets")
        .header(header::AUTHORIZATION, "Bearer valid-admin-token")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{\"id\":"))
        .unwrap();
    let response = router(state).oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        "application/problem+json"
    );
}

#[tokio::test]
async fn security_headers_are_present_on_every_response() {
    let state = user_state(
        Arc::new(FakeAssets::default()),
        Arc::new(FakeAudit::default()),
    );
    let (_, headers, _) = send(&state, Method::GET, "/healthz", None, None, None).await;

    assert_eq!(headers.get("x-content-type-options").unwrap(), "nosniff");
    assert_eq!(headers.get("x-frame-options").unwrap(), "DENY");
    assert_eq!(
        headers.get("content-security-policy").unwrap(),
        "default-src 'none'"
    );
    assert!(headers.contains_key("strict-transport-security"));
}

#[tokio::test]
async fn metrics_expose_request_and_error_counters() {
    let state = test_state(
        Arc::new(FakeAssets::default()),
        Arc::new(FakeObservations::default()),
        Arc::new(FakeAudit::default()),
        Arc::new(FakeTokenValidator::user()),
        Arc::new(Ready(false)),
    );

    let (status, _, _) = send(&state, Method::GET, "/readyz", None, None, None).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);

    let response = router(state)
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(text.contains("http_requests_total"));
    assert!(text.contains("status_class=\"4xx\"") || text.contains("status_class=\"5xx\""));
    assert!(text.contains("http_errors_total"));
    assert!(text.contains("service=\"asset-registry\""));
}

#[tokio::test]
async fn an_unpriced_asset_is_reported_as_unpriced_not_zero() {
    let state = user_state(seeded_assets(), Arc::new(FakeAudit::default()));

    let (status, _, body) = send(
        &state,
        Method::GET,
        "/v1/assets/asset-usdc/price",
        Some("valid-user-token"),
        None,
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "unpriced");
    assert_eq!(body["attempted"], json!([]));
    assert!(
        body.get("price").is_none(),
        "BR2.2: never a zero or fabricated price"
    );
}

#[tokio::test]
async fn pricing_an_unknown_asset_is_a_404_problem() {
    let state = user_state(
        Arc::new(FakeAssets::default()),
        Arc::new(FakeAudit::default()),
    );

    let (status, _, body) = send(
        &state,
        Method::GET,
        "/v1/assets/nope/price",
        Some("valid-user-token"),
        None,
        None,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["status"], 404);
}
