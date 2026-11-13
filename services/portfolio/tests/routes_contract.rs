//! Router contract tests for the Portfolio HTTP boundary (WF1–WF6, C8, C11).
//!
//! Every test drives a real request through `portfolio_http::router` (or the
//! worker's listener) and asserts what a caller observes: the status code, the
//! RFC 7807 problem body, the JSON projection, or a header. The services run
//! against the application-layer port fakes, so the handlers, their DTO
//! mappings, the middleware, and the worker's listener are exercised end to
//! end without a database.

#[path = "../application/tests/fakes.rs"]
mod fakes;

use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{header, HeaderMap, Method, Request, StatusCode};
use chrono::{DateTime, TimeZone, Utc};
use http_body_util::BodyExt;
use portfolio_application::common::{AssuranceLevel, VerifiedClaims};
use portfolio_application::guardrails::GuardrailService;
use portfolio_application::intents::IntentService;
use portfolio_application::portfolios::PortfolioService;
use portfolio_application::ports::{
    AuthError, GuardrailRepository, HoldingObservation, ReadinessProbe, RegisteredService,
    RepositoryError, ServiceTokenRegistry, TargetRepository, TokenValidator,
};
use portfolio_application::privacy::PrivacyService;
use portfolio_application::run::{ValuationRunService, RUN_JOB};
use portfolio_application::targets::TargetService;
use portfolio_application::trades::TradeService;
use portfolio_application::trail::Trail;
use portfolio_application::valuation::ValuationService;
use portfolio_domain::{
    AccountId, Freshness, Guardrail, GuardrailId, Operation, OrderIntentId, Portfolio,
    PortfolioAccount, PortfolioAccountId, PortfolioId, Quote, TargetAllocation, TargetAllocationId,
    UserId, ValuationSnapshotId,
};
use portfolio_http::metrics::Metrics;
use portfolio_http::state::{AppState, WorkerState};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use tower::ServiceExt;

use fakes::{
    FakeAudit, FakeHoldings, FakeIntents, FakeLedger, FakeMarket, FakeMemberships,
    FakeNotifications, FakeOverlays, FakePortfolios, FakePreferences, FakePrivacy, FakeRuns,
    FakeSnapshots, FakeTargets, FakeTrades, MutableClock, Owners, SeqIds,
};

const USER_A: &str = "11111111-1111-4111-8111-111111111111";
const USER_B: &str = "22222222-2222-4222-8222-222222222222";
const ACCOUNT: &str = "33333333-3333-4333-8333-333333333333";

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, hour, 0, 0).unwrap()
}

fn d(raw: &str) -> Decimal {
    Decimal::from_str(raw).unwrap()
}

fn user_a() -> UserId {
    UserId::parse(USER_A).unwrap()
}

fn user_b() -> UserId {
    UserId::parse(USER_B).unwrap()
}

fn account() -> AccountId {
    AccountId::parse(ACCOUNT).unwrap()
}

/// The SPA path accepts one user token; the internal paths accept service
/// tokens. Anything else is refused, which is what the 401/403 cases assert.
struct Validator;

#[async_trait]
impl TokenValidator for Validator {
    async fn validate(&self, token: &str) -> Result<VerifiedClaims, AuthError> {
        match token {
            "valid" => Ok(VerifiedClaims {
                user_id: user_a(),
                assurance: AssuranceLevel::Aal2,
            }),
            "identity-down" => Err(AuthError::Unavailable("jwks unreachable".to_string())),
            other => Err(AuthError::Invalid(format!("unknown token {other}"))),
        }
    }
}

struct Tokens;

#[async_trait]
impl ServiceTokenRegistry for Tokens {
    async fn service_for(&self, token: &str) -> Result<Option<RegisteredService>, RepositoryError> {
        Ok(match token {
            "reporting-token" => Some(RegisteredService {
                name: "reporting".to_string(),
                operations: vec![
                    Operation::SnapshotsRead,
                    Operation::PortfoliosRead,
                    Operation::ValuationRead,
                ],
            }),
            "identity-token" => Some(RegisteredService {
                name: "identity".to_string(),
                operations: vec![
                    Operation::PrivacyExport,
                    Operation::PrivacyAnonymise,
                    Operation::SupportMetadata,
                ],
            }),
            "job-token" => Some(RegisteredService {
                name: "job-orchestration".to_string(),
                operations: vec![Operation::RunValuation],
            }),
            "smoke-token" => Some(RegisteredService {
                name: "smoke-reader".to_string(),
                operations: vec![Operation::SnapshotsRead],
            }),
            "registry-down" => return Err(RepositoryError::Unavailable("registry down".into())),
            _ => None,
        })
    }

    fn token_for(&self, _service: &str) -> Option<String> {
        None
    }
}

struct Ready;

#[async_trait]
impl ReadinessProbe for Ready {
    async fn check(&self) -> Result<(), String> {
        Ok(())
    }
}

struct NotReady;

#[async_trait]
impl ReadinessProbe for NotReady {
    async fn check(&self) -> Result<(), String> {
        Err("postgres is down".to_string())
    }
}

struct Harness {
    clock: Arc<MutableClock>,
    portfolios_repo: Arc<FakePortfolios>,
    memberships: Arc<FakeMemberships>,
    targets: Arc<FakeTargets>,
    snapshots: Arc<FakeSnapshots>,
    intents_repo: Arc<FakeIntents>,
    trades_repo: Arc<FakeTrades>,
    runs: Arc<FakeRuns>,
    holdings: Arc<FakeHoldings>,
    market: Arc<FakeMarket>,
    ledger: Arc<FakeLedger>,
    valuation: Arc<ValuationService>,
    run: Arc<ValuationRunService>,
    state: AppState,
}

fn harness() -> Harness {
    let clock = MutableClock::new(at(0));
    let owners: Owners = Arc::default();
    let portfolios_repo = Arc::new(FakePortfolios::new(owners.clone()));
    let memberships = Arc::new(FakeMemberships::new(owners.clone()));
    let targets = Arc::new(FakeTargets::default());
    let snapshots = Arc::new(FakeSnapshots::new(owners.clone()));
    let overlays = Arc::new(FakeOverlays::new(owners.clone()));
    let intents_repo = Arc::new(FakeIntents::new(owners.clone()));
    let trades_repo = Arc::new(FakeTrades::new(owners.clone(), overlays.clone()));
    let runs = Arc::new(FakeRuns::default());
    let privacy_repo = Arc::new(FakePrivacy::default());
    let holdings = Arc::new(FakeHoldings::default());
    let market = Arc::new(FakeMarket::default());
    let ledger = Arc::new(FakeLedger::default());
    let notifications = Arc::new(FakeNotifications::default());
    let audit = Arc::new(FakeAudit::default());
    let preferences = Arc::new(FakePreferences::new(None));
    let ids: Arc<SeqIds> = Arc::new(SeqIds::default());
    let clock: Arc<MutableClock> = Arc::new(clock.clone());

    let trail = Arc::new(Trail::new(audit.clone(), clock.clone()));
    let valuation = Arc::new(ValuationService::new(
        portfolios_repo.clone(),
        memberships.clone(),
        targets.clone(),
        snapshots.clone(),
        overlays.clone(),
        holdings.clone(),
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
        market.clone(),
        valuation.clone(),
        trail.clone(),
        clock.clone(),
        ids.clone(),
    ));
    let trades = Arc::new(TradeService::new(
        portfolios_repo.clone(),
        trades_repo.clone(),
        ledger.clone(),
        trail.clone(),
        clock.clone(),
        ids.clone(),
    ));
    let target_repo: Arc<dyn TargetRepository> = targets.clone();
    let guardrail_repo: Arc<dyn GuardrailRepository> = targets.clone();
    let guardrails = Arc::new(GuardrailService::new(
        target_repo,
        guardrail_repo,
        notifications,
        clock.clone(),
    ));
    let run = Arc::new(ValuationRunService::new(
        portfolios_repo.clone(),
        memberships.clone(),
        overlays.clone(),
        holdings.clone(),
        trades_repo.clone(),
        runs.clone(),
        valuation.clone(),
        guardrails.clone(),
        clock.clone(),
        ids.clone(),
    ));
    let privacy = Arc::new(PrivacyService::new(
        portfolios_repo.clone(),
        memberships.clone(),
        targets.clone(),
        snapshots.clone(),
        overlays.clone(),
        intents_repo.clone(),
        trades_repo.clone(),
        privacy_repo,
        trail,
        clock.clone(),
        ids.clone(),
    ));

    let state = AppState {
        service_name: "portfolio-api",
        portfolios,
        valuation: valuation.clone(),
        targets: targets_service,
        intents,
        trades,
        guardrails,
        privacy,
        token_validator: Arc::new(Validator),
        service_tokens: Arc::new(Tokens),
        readiness: Arc::new(Ready),
        metrics: Arc::new(Metrics::default()),
    };

    Harness {
        clock,
        portfolios_repo,
        memberships,
        targets,
        snapshots,
        intents_repo,
        trades_repo,
        runs,
        holdings,
        market,
        ledger,
        valuation,
        run,
        state,
    }
}

impl Harness {
    fn seed_portfolio(&self, user: UserId) -> Portfolio {
        let name = format!("Portfolio {}", PortfolioId::generate());
        let portfolio = Portfolio::new(PortfolioId::generate(), user, &name, "USD", at(0)).unwrap();
        self.portfolios_repo.seed(portfolio.clone());
        self.memberships.seed(PortfolioAccount::new(
            PortfolioAccountId::generate(),
            portfolio.id(),
            account(),
            at(0),
        ));
        portfolio
    }

    fn seed_holding(&self, asset: &str, quantity: &str) {
        self.holdings.seed(HoldingObservation {
            account_id: account(),
            asset_id: asset.to_string(),
            quantity: d(quantity),
            observed_at: at(0),
        });
    }

    fn seed_quote(&self, asset: &str, price: &str, currency: &str) {
        self.market.seed_quote(
            Quote::new(
                asset,
                Some(d(price)),
                Some(currency.to_string()),
                Some("test".to_string()),
                Some(at(0)),
                Freshness::Live,
            )
            .unwrap(),
        );
    }

    fn seed_target(&self, portfolio: &Portfolio, asset: &str, percent: &str) {
        self.targets.seed_target(
            TargetAllocation::new(
                TargetAllocationId::generate(),
                portfolio.id(),
                asset,
                d(percent),
                at(0),
            )
            .unwrap(),
        );
    }

    fn seed_guardrail(&self, portfolio: &Portfolio, asset: &str, threshold: &str) {
        self.targets.seed_guardrail(
            Guardrail::new(
                GuardrailId::generate(),
                portfolio.id(),
                asset,
                d(threshold),
                at(0),
            )
            .unwrap(),
        );
    }

    fn worker_state(&self) -> WorkerState {
        WorkerState {
            service_name: "portfolio-worker",
            runs: self.run.clone(),
            service_tokens: Arc::new(Tokens),
            readiness: Arc::new(Ready),
            metrics: Arc::new(Metrics::default()),
        }
    }
}

fn build_request(
    method: Method,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    match body {
        Some(body) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    }
}

fn json_of(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap_or(Value::Null)
}

async fn send(
    state: &AppState,
    method: Method,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, HeaderMap, Value) {
    let response = portfolio_http::router(state.clone())
        .oneshot(build_request(method, uri, token, body))
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, headers, json_of(&bytes))
}

async fn send_raw(
    state: &AppState,
    method: Method,
    uri: &str,
    token: Option<&str>,
) -> (StatusCode, HeaderMap, String) {
    let response = portfolio_http::router(state.clone())
        .oneshot(build_request(method, uri, token, None))
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, headers, String::from_utf8_lossy(&bytes).to_string())
}

async fn send_worker(
    state: &WorkerState,
    method: Method,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, String) {
    let response = portfolio_http::worker_router(state.clone())
        .oneshot(build_request(method, uri, token, body))
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).to_string())
}

// ---------------------------------------------------------------------------
// WF1: portfolios and membership
// ---------------------------------------------------------------------------

#[tokio::test]
async fn listing_portfolios_creates_the_default_portfolio() {
    let harness = harness();

    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        "/v1/portfolios",
        Some("valid"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let rows = body.as_array().expect("an array");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["name"], "My portfolio");
    assert_eq!(rows[0]["baseCurrency"], "USD");
    assert_eq!(harness.portfolios_repo.all().len(), 1);
}

#[tokio::test]
async fn creating_a_portfolio_answers_201_with_the_stored_row() {
    let harness = harness();

    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        "/v1/portfolios",
        Some("valid"),
        Some(json!({ "name": "Retirement", "baseCurrency": "EUR" })),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["name"], "Retirement");
    assert_eq!(body["baseCurrency"], "EUR");
    assert_eq!(harness.portfolios_repo.all().len(), 1);
}

#[tokio::test]
async fn a_malformed_create_body_is_a_validation_refusal() {
    let harness = harness();
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/portfolios")
        .header(header::AUTHORIZATION, "Bearer valid")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{not json"))
        .unwrap();

    let response = portfolio_http::router(harness.state.clone())
        .oneshot(request)
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = json_of(&response.into_body().collect().await.unwrap().to_bytes());
    assert_eq!(body["errors"][0]["field"], "body");
    assert_eq!(body["status"], 422);
}

#[tokio::test]
async fn a_portfolio_read_returns_the_valuation_projection() {
    let harness = harness();
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "2");
    harness.seed_quote("a", "100", "USD");

    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        &format!("/v1/portfolios/{}", portfolio.id()),
        Some("valid"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "complete");
    assert_eq!(body["totalValue"], "200");
    assert_eq!(body["currency"], "USD");
    assert_eq!(body["lines"][0]["assetId"], "a");
    assert_eq!(body["lines"][0]["quantity"], "2");
    assert_eq!(body["lines"][0]["priceFreshness"], "live");
    assert_eq!(body["lines"][0]["dataStatus"], "priced");
    assert_eq!(body["driftAvailable"], false);
    assert_eq!(body["prompt"], "Set asset targets to see drift.");
}

#[tokio::test]
async fn a_portfolio_read_refuses_a_malformed_id() {
    let harness = harness();

    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        "/v1/portfolios/not-a-uuid",
        Some("valid"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"][0]["field"], "portfolioId");
}

#[tokio::test]
async fn a_portfolio_read_of_another_users_row_is_forbidden() {
    let harness = harness();
    let portfolio = harness.seed_portfolio(user_b());

    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        &format!("/v1/portfolios/{}", portfolio.id()),
        Some("valid"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["status"], 403);
    assert_eq!(body["type"], "https://errors.cpb/portfolio/forbidden");
}

#[tokio::test]
async fn assigning_an_account_returns_the_membership() {
    let harness = harness();
    let portfolio = harness.seed_portfolio(user_a());
    let second = AccountId::generate();

    let (status, _, body) = send(
        &harness.state,
        Method::PUT,
        &format!("/v1/portfolios/{}/accounts/{second}", portfolio.id()),
        Some("valid"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["portfolioId"], portfolio.id().to_string());
    assert_eq!(body["accountId"], second.to_string());
    assert!(body["addedAt"].is_string());
}

#[tokio::test]
async fn assigning_a_malformed_account_id_is_refused() {
    let harness = harness();
    let portfolio = harness.seed_portfolio(user_a());

    let (status, _, body) = send(
        &harness.state,
        Method::PUT,
        &format!("/v1/portfolios/{}/accounts/nope", portfolio.id()),
        Some("valid"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"][0]["field"], "accountId");
}

// ---------------------------------------------------------------------------
// WF3: targets
// ---------------------------------------------------------------------------

#[tokio::test]
async fn previewing_targets_reports_the_sum_without_saving() {
    let harness = harness();
    let portfolio = harness.seed_portfolio(user_a());

    let (status, _, body) = send(
        &harness.state,
        Method::PUT,
        &format!("/v1/portfolios/{}/targets?preview=true", portfolio.id()),
        Some("valid"),
        Some(json!({
            "targets": [
                { "assetId": "a", "targetPercent": "40" },
                { "assetId": "b", "targetPercent": "50" }
            ]
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["sum"], json!(90));
    assert_eq!(body["difference"], json!(10));
    assert_eq!(body["saved"], false);
    assert!(harness.targets.targets().is_empty());
}

#[tokio::test]
async fn saving_targets_persists_them_and_reports_saved() {
    let harness = harness();
    let portfolio = harness.seed_portfolio(user_a());

    let (status, _, body) = send(
        &harness.state,
        Method::PUT,
        &format!("/v1/portfolios/{}/targets", portfolio.id()),
        Some("valid"),
        Some(json!({
            "targets": [{ "assetId": "a", "targetPercent": "60", "guardrailPoints": "5" }]
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["saved"], true);
    assert_eq!(body["sum"], json!(60));
    assert_eq!(harness.targets.targets().len(), 1);
    assert_eq!(harness.targets.guardrails().len(), 1);
}

#[tokio::test]
async fn an_out_of_range_target_set_is_a_validation_refusal() {
    let harness = harness();
    let portfolio = harness.seed_portfolio(user_a());

    let (status, _, body) = send(
        &harness.state,
        Method::PUT,
        &format!("/v1/portfolios/{}/targets", portfolio.id()),
        Some("valid"),
        Some(json!({
            "targets": [{ "assetId": "a", "targetPercent": "101" }]
        })),
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["status"], 422);
    assert_eq!(body["errors"][0]["field"], "targetPercent");
    assert!(harness.targets.targets().is_empty());
}

// ---------------------------------------------------------------------------
// WF5: order intents
// ---------------------------------------------------------------------------

#[tokio::test]
async fn an_intent_is_previewed_then_recorded_and_cancelled() {
    let harness = harness();
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");
    let uri = format!("/v1/portfolios/{}/order-intents", portfolio.id());

    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        &format!("{uri}?preview=true"),
        Some("valid"),
        Some(json!({ "assetId": "a", "side": "buy", "quantity": "1" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["projectedAllocation"], json!(100));
    assert!(harness.intents_repo.all().is_empty());

    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        &uri,
        Some("valid"),
        Some(json!({ "assetId": "a", "side": "buy", "quantity": "1" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["status"], "pending");
    assert_eq!(body["side"], "buy");
    assert_eq!(harness.intents_repo.all().len(), 1);

    let intent_id = body["id"].as_str().unwrap();
    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        &format!("{uri}/{intent_id}/cancel"),
        Some("valid"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "cancelled");
    assert!(body["cancelledAt"].is_string());
}

#[tokio::test]
async fn an_intent_with_an_unknown_side_is_a_validation_refusal() {
    let harness = harness();
    let portfolio = harness.seed_portfolio(user_a());

    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        &format!("/v1/portfolios/{}/order-intents", portfolio.id()),
        Some("valid"),
        Some(json!({ "assetId": "a", "side": "hold", "quantity": "1" })),
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"][0]["field"], "side");
}

#[tokio::test]
async fn cancelling_an_unknown_intent_is_not_found() {
    let harness = harness();
    let portfolio = harness.seed_portfolio(user_a());

    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        &format!(
            "/v1/portfolios/{}/order-intents/{}/cancel",
            portfolio.id(),
            OrderIntentId::generate()
        ),
        Some("valid"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["type"], "https://errors.cpb/portfolio/not-found");
}

// ---------------------------------------------------------------------------
// WF6: recorded trades
// ---------------------------------------------------------------------------

#[tokio::test]
async fn recording_a_trade_answers_201_and_evaluates_the_guardrails() {
    let harness = harness();
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");
    harness.seed_target(&portfolio, "a", "50");
    harness.seed_guardrail(&portfolio, "a", "5");

    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        &format!("/v1/portfolios/{}/trades", portfolio.id()),
        Some("valid"),
        Some(json!({
            "assetId": "a",
            "side": "buy",
            "quantity": "2",
            "price": "100",
            "priceSource": "user",
            "executedAt": at(1),
            "idempotencyKey": "idem-http-1"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["assetId"], "a");
    assert_eq!(body["quantity"], "2");
    assert_eq!(body["price"], "100");
    assert_eq!(body["priceSource"], "user");
    assert_eq!(body["side"], "buy");
    assert_eq!(harness.trades_repo.all().len(), 1);
    assert_eq!(harness.ledger.calls().len(), 1);
}

#[tokio::test]
async fn recording_a_trade_with_a_bad_price_is_refused() {
    let harness = harness();
    let portfolio = harness.seed_portfolio(user_a());

    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        &format!("/v1/portfolios/{}/trades", portfolio.id()),
        Some("valid"),
        Some(json!({
            "assetId": "a",
            "side": "buy",
            "quantity": "1",
            "price": "not-a-number",
            "priceSource": "user",
            "executedAt": at(1),
            "idempotencyKey": "idem-http-2"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"][0]["field"], "price");
    assert!(harness.trades_repo.all().is_empty());
}

#[tokio::test]
async fn recording_a_trade_for_another_users_portfolio_is_forbidden() {
    let harness = harness();
    let portfolio = harness.seed_portfolio(user_b());

    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        &format!("/v1/portfolios/{}/trades", portfolio.id()),
        Some("valid"),
        Some(json!({
            "assetId": "a",
            "side": "buy",
            "quantity": "1",
            "priceSource": "venue",
            "executedAt": at(1),
            "idempotencyKey": "idem-http-3"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["status"], 403);
    assert!(harness.trades_repo.all().is_empty());
}

// ---------------------------------------------------------------------------
// WF4 / C8: snapshot history and reproduction
// ---------------------------------------------------------------------------

async fn seed_history(harness: &Harness, portfolio: &Portfolio, hours: u32) {
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");
    for hour in 0..hours {
        harness.clock.set(at(hour));
        let fresh = harness.valuation.fresh(portfolio).await.unwrap();
        harness
            .valuation
            .store_snapshot(portfolio, &fresh.valuation, &fresh.inputs, at(hour))
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn history_returns_a_page_with_cursor_and_has_more_headers() {
    let harness = harness();
    let portfolio = harness.seed_portfolio(user_a());
    seed_history(&harness, &portfolio, 3).await;

    let (status, headers, body) = send(
        &harness.state,
        Method::GET,
        &format!("/v1/portfolios/{}/snapshots?limit=2", portfolio.id()),
        Some("valid"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 2);
    assert_eq!(headers.get("x-has-more").unwrap().to_str().unwrap(), "true");
    assert!(headers.get("x-next-cursor").is_some());
    assert_eq!(body[0]["totalValue"], "100");
}

#[tokio::test]
async fn history_refuses_a_malformed_from_instant() {
    let harness = harness();
    let portfolio = harness.seed_portfolio(user_a());

    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        &format!("/v1/portfolios/{}/snapshots?from=yesterday", portfolio.id()),
        Some("valid"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"][0]["field"], "from");
}

#[tokio::test]
async fn history_accepts_the_reporting_service_token() {
    let harness = harness();
    let portfolio = harness.seed_portfolio(user_a());
    seed_history(&harness, &portfolio, 1).await;

    let (status, headers, body) = send(
        &harness.state,
        Method::GET,
        &format!("/v1/portfolios/{}/snapshots", portfolio.id()),
        Some("reporting-token"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(
        headers.get("x-has-more").unwrap().to_str().unwrap(),
        "false"
    );
    assert!(headers.get("x-next-cursor").is_none());
}

#[tokio::test]
async fn a_snapshot_detail_is_reproduced_and_flagged_when_the_digest_breaks() {
    let harness = harness();
    let portfolio = harness.seed_portfolio(user_a());
    seed_history(&harness, &portfolio, 1).await;
    let snapshot_id: ValuationSnapshotId = harness.snapshots.all()[0].id();

    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        &format!("/v1/portfolios/{}/snapshots/{snapshot_id}", portfolio.id()),
        Some("valid"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["verified"], true);
    assert_eq!(body["totalValue"], "100");

    harness.snapshots.tamper(0);
    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        &format!("/v1/portfolios/{}/snapshots/{snapshot_id}", portfolio.id()),
        Some("valid"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["verified"], false);
}

#[tokio::test]
async fn a_snapshot_detail_with_a_bad_id_is_a_validation_refusal() {
    let harness = harness();
    let portfolio = harness.seed_portfolio(user_a());

    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        &format!("/v1/portfolios/{}/snapshots/nope", portfolio.id()),
        Some("valid"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"][0]["field"], "snapshotId");
}

// ---------------------------------------------------------------------------
// C8: the internal routes Reporting and Identity call
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_internal_user_portfolios_route_serves_reporting() {
    let harness = harness();
    harness.seed_portfolio(user_a());

    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        &format!("/v1/users/{USER_A}/portfolios"),
        Some("reporting-token"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn the_internal_user_portfolios_route_refuses_a_token_without_the_operation() {
    let harness = harness();
    harness.seed_portfolio(user_a());

    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        &format!("/v1/users/{USER_A}/portfolios"),
        Some("identity-token"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["status"], 403);
    assert_eq!(body["type"], "https://errors.cpb/portfolio/forbidden");
}

#[tokio::test]
async fn a_malformed_user_id_on_the_internal_route_is_a_validation_refusal() {
    let harness = harness();

    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        "/v1/users/nope/portfolios",
        Some("reporting-token"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"][0]["field"], "userId");
}

#[tokio::test]
async fn the_internal_valuation_route_serves_reporting() {
    let harness = harness();
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");

    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        &format!("/v1/portfolios/{}/valuation", portfolio.id()),
        Some("reporting-token"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["portfolioId"], portfolio.id().to_string());
    assert_eq!(body["totalValue"], "100");
}

#[tokio::test]
async fn the_internal_valuation_route_refuses_a_token_without_the_operation() {
    let harness = harness();
    let portfolio = harness.seed_portfolio(user_a());

    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        &format!("/v1/portfolios/{}/valuation", portfolio.id()),
        Some("identity-token"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["status"], 403);
}

#[tokio::test]
async fn the_privacy_export_returns_every_collection_for_the_user() {
    let harness = harness();
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");
    harness.seed_target(&portfolio, "a", "100");
    harness.seed_guardrail(&portfolio, "a", "5");
    seed_history(&harness, &portfolio, 1).await;

    // The intent and the trade are created through the public routes, so the
    // export is asserted over rows a caller actually produced.
    let (status, _, _) = send(
        &harness.state,
        Method::POST,
        &format!("/v1/portfolios/{}/order-intents", portfolio.id()),
        Some("valid"),
        Some(json!({ "assetId": "a", "side": "buy", "quantity": "1" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, _, _) = send(
        &harness.state,
        Method::POST,
        &format!("/v1/portfolios/{}/trades", portfolio.id()),
        Some("valid"),
        Some(json!({
            "assetId": "a",
            "side": "buy",
            "quantity": "1",
            "price": "100",
            "priceSource": "user",
            "executedAt": at(1),
            "idempotencyKey": "export-seed-1"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        &format!("/v1/internal/privacy/export?userId={USER_A}"),
        Some("identity-token"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["userId"], USER_A);
    assert_eq!(body["portfolios"].as_array().unwrap().len(), 1);
    assert_eq!(body["memberships"].as_array().unwrap().len(), 1);
    assert_eq!(body["targets"][0]["assetId"], "a");
    assert_eq!(body["guardrails"][0]["assetId"], "a");
    assert_eq!(body["snapshots"].as_array().unwrap().len(), 1);
    assert_eq!(body["overlays"].as_array().unwrap().len(), 1);
    assert_eq!(body["intents"][0]["assetId"], "a");
    assert_eq!(body["trades"][0]["assetId"], "a");
}

#[tokio::test]
async fn the_privacy_export_refuses_a_malformed_user() {
    let harness = harness();

    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        "/v1/internal/privacy/export?userId=nope",
        Some("identity-token"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"][0]["field"], "userId");
}

#[tokio::test]
async fn anonymising_the_same_request_twice_reports_it_was_already_applied() {
    let harness = harness();
    harness.seed_portfolio(user_a());
    let body = json!({ "userId": USER_A, "idempotencyKey": "anonymise-1" });
    let uri = "/v1/internal/privacy/anonymise";

    let (status, _, first) = send(
        &harness.state,
        Method::POST,
        uri,
        Some("identity-token"),
        Some(body.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(first["alreadyApplied"], false);
    assert_eq!(first["userId"], USER_A);
    assert!(first["retainedCategories"].as_array().unwrap().len() > 1);

    let (status, _, second) = send(
        &harness.state,
        Method::POST,
        uri,
        Some("identity-token"),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(second["alreadyApplied"], true);
}

#[tokio::test]
async fn anonymising_with_an_empty_key_is_refused() {
    let harness = harness();
    harness.seed_portfolio(user_a());

    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        "/v1/internal/privacy/anonymise",
        Some("identity-token"),
        Some(json!({ "userId": USER_A, "idempotencyKey": "  " })),
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"][0]["field"], "idempotencyKey");
}

#[tokio::test]
async fn the_smoke_route_names_the_service_and_the_caller() {
    let harness = harness();

    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        "/v1/internal/smoke",
        Some("smoke-token"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["service"], "portfolio-api");
    assert_eq!(body["caller"], "smoke-reader");
    assert_eq!(body["status"], "ok");
}

// ---------------------------------------------------------------------------
// Middleware, probes, and metrics
// ---------------------------------------------------------------------------

#[tokio::test]
async fn an_identity_outage_is_refused_with_503() {
    let harness = harness();

    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        "/v1/portfolios",
        Some("identity-down"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        body["type"],
        "https://errors.cpb/portfolio/identity-unavailable"
    );
    assert_eq!(body["detail"], "jwks unreachable");
}

#[tokio::test]
async fn a_service_registry_outage_is_refused_with_503() {
    let harness = harness();

    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        "/v1/internal/smoke",
        Some("registry-down"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        body["type"],
        "https://errors.cpb/portfolio/registry-unavailable"
    );
}

#[tokio::test]
async fn an_unknown_service_token_is_unauthorized() {
    let harness = harness();

    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        "/v1/internal/smoke",
        Some("not-a-token"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["detail"], "unknown service token");
}

#[tokio::test]
async fn a_missing_bearer_token_on_the_spa_route_is_unauthorized() {
    let harness = harness();

    let (status, _, body) = send(&harness.state, Method::GET, "/v1/portfolios", None, None).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["detail"], "missing bearer token");
}

#[tokio::test]
async fn an_invalid_correlation_id_is_refused_with_400() {
    let harness = harness();
    let request = Request::builder()
        .method(Method::GET)
        .uri("/healthz")
        .header("x-correlation-id", "bad id with spaces")
        .body(Body::empty())
        .unwrap();

    let response = portfolio_http::router(harness.state.clone())
        .oneshot(request)
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(response.headers().get("x-correlation-id").is_some());
}

#[tokio::test]
async fn a_supplied_correlation_id_is_echoed_and_security_headers_are_set() {
    let harness = harness();
    let request = Request::builder()
        .method(Method::GET)
        .uri("/healthz")
        .header("x-correlation-id", "corr-abc-123")
        .body(Body::empty())
        .unwrap();

    let response = portfolio_http::router(harness.state.clone())
        .oneshot(request)
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("x-correlation-id")
            .unwrap()
            .to_str()
            .unwrap(),
        "corr-abc-123"
    );
    assert_eq!(
        response
            .headers()
            .get("x-content-type-options")
            .unwrap()
            .to_str()
            .unwrap(),
        "nosniff"
    );
    assert_eq!(
        response
            .headers()
            .get("x-frame-options")
            .unwrap()
            .to_str()
            .unwrap(),
        "DENY"
    );
}

#[tokio::test]
async fn healthz_names_the_service() {
    let harness = harness();

    let (status, _, body) = send(&harness.state, Method::GET, "/healthz", None, None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["service"], "portfolio-api");
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn readiness_reports_a_down_dependency() {
    let mut harness = harness();
    harness.state.readiness = Arc::new(NotReady);

    let (status, _, body) = send(&harness.state, Method::GET, "/readyz", None, None).await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["type"], "https://errors.cpb/portfolio/not-ready");
    assert_eq!(body["detail"], "postgres is down");
}

#[tokio::test]
async fn an_unknown_route_is_a_not_found_problem() {
    let harness = harness();

    let (status, _, body) = send(&harness.state, Method::GET, "/v1/nope", None, None).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["detail"], "the route does not exist");
}

#[tokio::test]
async fn metrics_render_after_requests_have_been_recorded() {
    let harness = harness();
    let (status, _, _) = send(
        &harness.state,
        Method::GET,
        "/v1/portfolios",
        Some("valid"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, headers, text) = send_raw(&harness.state, Method::GET, "/metrics", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get(header::CONTENT_TYPE).unwrap().to_str().unwrap(),
        "text/plain; version=0.0.4"
    );
    assert!(text.contains("http_requests_total{service=\"portfolio-api\",status_class=\"2xx\"}"));
    assert!(text.contains("portfolio_health{service=\"portfolio-api\"} 1"));
}

// ---------------------------------------------------------------------------
// C11: the worker's listener
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_worker_run_endpoint_runs_the_job_and_reports_the_result() {
    let harness = harness();
    harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");
    let state = harness.worker_state();

    let (status, text) = send_worker(
        &state,
        Method::POST,
        &format!("/v1/internal/run/{RUN_JOB}"),
        Some("job-token"),
        Some(json!({ "owner": "job-1" })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let body = json_of(text.as_bytes());
    assert_eq!(body["jobName"], RUN_JOB);
    assert_eq!(body["status"], "succeeded");
    assert_eq!(body["portfolios"], 1);
    assert_eq!(body["snapshots"], 1);
    assert_eq!(body["failed"], 0);
    assert_eq!(harness.snapshots.all().len(), 1);
    assert_eq!(harness.runs.all().len(), 1);
}

#[tokio::test]
async fn the_worker_refuses_an_unknown_job_name() {
    let harness = harness();
    let state = harness.worker_state();

    let (status, text) = send_worker(
        &state,
        Method::POST,
        "/v1/internal/run/not-a-job",
        Some("job-token"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    let body = json_of(text.as_bytes());
    assert_eq!(body["type"], "https://errors.cpb/portfolio/not-found");
    assert_eq!(body["detail"], "the job is not provided by Portfolio");
}

#[tokio::test]
async fn the_worker_refuses_a_caller_without_the_run_operation() {
    let harness = harness();
    let state = harness.worker_state();

    let (status, text) = send_worker(
        &state,
        Method::POST,
        &format!("/v1/internal/run/{RUN_JOB}"),
        Some("identity-token"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(json_of(text.as_bytes())["status"], 403);
}

#[tokio::test]
async fn the_worker_refuses_a_missing_or_unknown_token() {
    let harness = harness();
    let state = harness.worker_state();

    let (status, text) = send_worker(
        &state,
        Method::POST,
        &format!("/v1/internal/run/{RUN_JOB}"),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(json_of(text.as_bytes())["detail"], "missing service token");

    let (status, text) = send_worker(
        &state,
        Method::POST,
        &format!("/v1/internal/run/{RUN_JOB}"),
        Some("not-a-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(json_of(text.as_bytes())["detail"], "unknown service token");
}

#[tokio::test]
async fn the_worker_probes_and_fallback_answer_on_their_own_listener() {
    let harness = harness();
    let state = harness.worker_state();

    let (status, text) = send_worker(&state, Method::GET, "/healthz", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json_of(text.as_bytes())["service"], "portfolio-worker");

    let (status, text) = send_worker(&state, Method::GET, "/readyz", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json_of(text.as_bytes())["status"], "ready");

    let (status, text) = send_worker(&state, Method::GET, "/metrics", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(text.contains("portfolio_health{service=\"portfolio-worker\"} 1"));

    let (status, text) = send_worker(&state, Method::GET, "/nope", None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        json_of(text.as_bytes())["detail"],
        "the route does not exist"
    );
}

#[tokio::test]
async fn the_worker_readiness_reports_a_down_dependency() {
    let harness = harness();
    let mut state = harness.worker_state();
    state.readiness = Arc::new(NotReady);

    let (status, text) = send_worker(&state, Method::GET, "/readyz", None, None).await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        json_of(text.as_bytes())["type"],
        "https://errors.cpb/portfolio/not-ready"
    );
}

// ---------------------------------------------------------------------------
// The listeners themselves
// ---------------------------------------------------------------------------

/// One blocking HTTP/1.1 request against a bound listener, read to EOF.
async fn raw_get(addr: std::net::SocketAddr, path: &str) -> String {
    let path = path.to_string();
    tokio::task::spawn_blocking(move || {
        use std::io::{Read, Write};
        let mut stream = std::net::TcpStream::connect(addr).unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(10)))
            .unwrap();
        write!(
            stream,
            "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        stream.flush().unwrap();
        let mut buffer = String::new();
        let _ = stream.read_to_string(&mut buffer);
        buffer
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn the_api_listener_answers_healthz_on_its_bound_port() {
    let harness = harness();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(portfolio_http::server::serve(
        listener,
        harness.state.clone(),
    ));

    let response = raw_get(addr, "/healthz").await;

    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(
        response.contains("\"service\":\"portfolio-api\""),
        "{response}"
    );
    server.abort();
}

#[tokio::test]
async fn the_worker_listener_answers_readyz_on_its_bound_port() {
    let harness = harness();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(portfolio_http::server::serve_worker(
        listener,
        harness.worker_state(),
    ));

    let response = raw_get(addr, "/readyz").await;

    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(response.contains("\"status\":\"ready\""), "{response}");
    server.abort();
}
