//! The Portfolio service's support-metadata endpoint (BR3.9, contract C1/C2).
//!
//! The route is internal and Identity-scoped: only the Identity service token,
//! carrying the new `support_metadata` operation, may read it. The projection is
//! the closed allowlist — a portfolio id and its label — so a support read can
//! never reach a holding, a valuation, or a target allocation.

#[path = "../application/tests/fakes.rs"]
mod fakes;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use chrono::{DateTime, TimeZone, Utc};
use http_body_util::BodyExt;
use portfolio_application::common::ServiceCaller;
use portfolio_application::guardrails::GuardrailService;
use portfolio_application::intents::IntentService;
use portfolio_application::portfolios::PortfolioService;
use portfolio_application::ports::{
    ReadinessProbe, RegisteredService, RepositoryError, ServiceTokenRegistry,
};
use portfolio_application::privacy::PrivacyService;
use portfolio_application::targets::TargetService;
use portfolio_application::trades::TradeService;
use portfolio_application::trail::Trail;
use portfolio_application::valuation::ValuationService;
use portfolio_domain::{Operation, Portfolio, PortfolioId, UserId};
use portfolio_http::metrics::Metrics;
use portfolio_http::router;
use portfolio_http::state::AppState;
use serde_json::Value;
use tower::ServiceExt;

use fakes::{
    FakeAudit, FakeHoldings, FakeIntents, FakeLedger, FakeMarket, FakeMemberships,
    FakeNotifications, FakeOverlays, FakePortfolios, FakePreferences, FakePrivacy, FakeRuns,
    FakeSnapshots, FakeTargets, FakeTrades, MutableClock, Owners, SeqIds,
};

const USER_A: &str = "11111111-1111-4111-8111-111111111111";
const SUPPORT_OPERATION_PATH: &str = "/v1/internal/support-metadata";

fn at() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, 12, 0, 0).unwrap()
}

fn user_a() -> UserId {
    UserId::parse(USER_A).unwrap()
}

/// The internal registry: Identity carries the new operation, Reporting does not.
struct Tokens;

#[async_trait]
impl ServiceTokenRegistry for Tokens {
    async fn service_for(&self, token: &str) -> Result<Option<RegisteredService>, RepositoryError> {
        Ok(match token {
            "identity-token" => Some(RegisteredService {
                name: "identity".to_string(),
                operations: vec![
                    Operation::PrivacyExport,
                    Operation::PrivacyAnonymise,
                    Operation::SupportMetadata,
                ],
            }),
            "reporting-token" => Some(RegisteredService {
                name: "reporting".to_string(),
                operations: vec![
                    Operation::SnapshotsRead,
                    Operation::PortfoliosRead,
                    Operation::ValuationRead,
                ],
            }),
            _ => None,
        })
    }

    fn token_for(&self, _service: &str) -> Option<String> {
        None
    }
}

/// The router never asks for readiness in these tests, but the state needs one.
struct Ready;

#[async_trait]
impl ReadinessProbe for Ready {
    async fn check(&self) -> Result<(), String> {
        Ok(())
    }
}

struct Harness {
    portfolios_repo: Arc<FakePortfolios>,
    state: AppState,
}

fn harness() -> Harness {
    let owners: Owners = Arc::new(Mutex::new(HashMap::new()));
    let portfolios_repo = Arc::new(FakePortfolios::new(owners.clone()));
    let memberships = Arc::new(FakeMemberships::new(owners.clone()));
    let targets = Arc::new(FakeTargets::default());
    let snapshots = Arc::new(FakeSnapshots::new(owners.clone()));
    let overlays = Arc::new(FakeOverlays::new(owners.clone()));
    let intents_repo = Arc::new(FakeIntents::new(owners.clone()));
    let trades_repo = Arc::new(FakeTrades::new(owners.clone(), overlays.clone()));
    let _runs = Arc::new(FakeRuns::default());
    let privacy_repo = Arc::new(FakePrivacy::default());
    let holdings = Arc::new(FakeHoldings::default());
    let market = Arc::new(FakeMarket::default());
    let ledger = Arc::new(FakeLedger::default());
    let notifications = Arc::new(FakeNotifications::default());
    let audit = Arc::new(FakeAudit::default());
    let preferences = Arc::new(FakePreferences::new(None));
    let clock = Arc::new(MutableClock::new(at()));
    let ids = Arc::new(SeqIds::default());

    let trail = Arc::new(Trail::new(audit, clock.clone()));
    let valuation = Arc::new(ValuationService::new(
        portfolios_repo.clone(),
        memberships.clone(),
        targets.clone(),
        snapshots.clone(),
        overlays.clone(),
        holdings,
        market.clone(),
        clock.clone(),
        ids.clone(),
    ));
    let portfolios = Arc::new(PortfolioService::new(
        portfolios_repo.clone(),
        memberships.clone(),
        preferences,
        clock.clone(),
        ids.clone(),
    ));
    let targets_service = Arc::new(TargetService::new(
        portfolios_repo.clone(),
        targets.clone(),
        trail.clone(),
        clock.clone(),
        ids.clone(),
    ));
    let intents = Arc::new(IntentService::new(
        portfolios_repo.clone(),
        intents_repo.clone(),
        market,
        valuation.clone(),
        trail.clone(),
        clock.clone(),
        ids.clone(),
    ));
    let trades = Arc::new(TradeService::new(
        portfolios_repo.clone(),
        trades_repo.clone(),
        ledger,
        trail.clone(),
        clock.clone(),
        ids.clone(),
    ));
    let guardrails = Arc::new(GuardrailService::new(
        targets,
        Arc::new(FakeTargets::default()),
        notifications,
        clock.clone(),
    ));
    let privacy = Arc::new(PrivacyService::new(
        portfolios_repo.clone(),
        memberships,
        Arc::new(FakeTargets::default()),
        snapshots,
        overlays,
        intents_repo,
        trades_repo,
        privacy_repo,
        trail,
        clock.clone(),
        ids,
    ));

    let state = AppState {
        service_name: "portfolio-api",
        portfolios,
        valuation,
        targets: targets_service,
        intents,
        trades,
        guardrails,
        privacy,
        token_validator: Arc::new(fakes::StaticValidator),
        service_tokens: Arc::new(Tokens),
        readiness: Arc::new(Ready),
        metrics: Arc::new(Metrics::default()),
    };

    Harness {
        portfolios_repo,
        state,
    }
}

async fn get(state: &AppState, uri: &str, token: Option<&str>) -> (StatusCode, Vec<u8>) {
    let mut builder = Request::builder().method(Method::GET).uri(uri);
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let request = builder.body(Body::empty()).unwrap();
    let response = router(state.clone()).oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, bytes.to_vec())
}

fn json_body(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap_or(Value::Null)
}

fn sorted_keys(value: &Value) -> Vec<String> {
    let mut keys: Vec<String> = value
        .as_object()
        .expect("an object")
        .keys()
        .cloned()
        .collect();
    keys.sort();
    keys
}

fn seed_portfolio(repo: &FakePortfolios, user: UserId, name: &str) -> PortfolioId {
    let portfolio = Portfolio::new(PortfolioId::generate(), user, name, "USD", at())
        .expect("a valid portfolio");
    let id = portfolio.id();
    repo.seed(portfolio);
    id
}

#[tokio::test]
async fn the_identity_token_reads_the_allowlisted_projection() {
    let harness = harness();
    let user = user_a();
    let core = seed_portfolio(&harness.portfolios_repo, user, "Core");
    seed_portfolio(&harness.portfolios_repo, user, "Retirement");

    let (status, body) = get(
        &harness.state,
        &format!("{SUPPORT_OPERATION_PATH}/{user}"),
        Some("identity-token"),
    )
    .await;

    assert_eq!(
        status,
        StatusCode::OK,
        "support metadata: {}",
        String::from_utf8_lossy(&body)
    );
    let body = json_body(&body);
    assert_eq!(sorted_keys(&body), vec!["portfolios".to_string()]);
    let portfolios = body["portfolios"].as_array().expect("an array");
    assert_eq!(portfolios.len(), 2);
    for portfolio in portfolios {
        assert_eq!(
            sorted_keys(portfolio),
            vec!["id".to_string(), "label".to_string()],
            "the projection is the closed allowlist"
        );
    }
    assert_eq!(portfolios[0]["id"], core.to_string());
    assert_eq!(portfolios[0]["label"], "Core");
}

#[tokio::test]
async fn a_registered_token_without_the_support_operation_is_refused() {
    let harness = harness();
    let user = user_a();
    seed_portfolio(&harness.portfolios_repo, user, "Core");

    let (status, body) = get(
        &harness.state,
        &format!("{SUPPORT_OPERATION_PATH}/{user}"),
        Some("reporting-token"),
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(json_body(&body)["status"], 403);
}

#[tokio::test]
async fn an_unauthenticated_request_is_refused() {
    let harness = harness();
    let user = user_a();

    let (status, body) = get(
        &harness.state,
        &format!("{SUPPORT_OPERATION_PATH}/{user}"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(json_body(&body)["status"], 401);
}

#[tokio::test]
async fn a_missing_target_answers_an_empty_projection_rather_than_existence() {
    let harness = harness();

    let (status, body) = get(
        &harness.state,
        &format!("{SUPPORT_OPERATION_PATH}/{}", UserId::generate()),
        Some("identity-token"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json_body(&body)["portfolios"], serde_json::json!([]));
}

#[tokio::test]
async fn a_malformed_user_id_is_a_validation_refusal() {
    let harness = harness();

    let (status, body) = get(
        &harness.state,
        &format!("{SUPPORT_OPERATION_PATH}/not-a-uuid"),
        Some("identity-token"),
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let body = json_body(&body);
    assert_eq!(body["errors"][0]["field"], "userId");
}

#[tokio::test]
async fn a_service_caller_cannot_reach_the_projection_without_the_operation() {
    // The handler and the domain helper agree: an identity token carrying only
    // the privacy operations is refused (defence in depth on `service_allowed`).
    let caller = ServiceCaller::new("reporting".to_string(), "corr-test");
    let operation = Operation::SupportMetadata;
    assert!(!portfolio_domain::service_allowed(
        &caller.service,
        operation
    ));
    assert!(portfolio_domain::service_allowed("identity", operation));
}
