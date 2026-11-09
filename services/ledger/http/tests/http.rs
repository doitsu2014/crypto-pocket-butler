//! HTTP boundary tests over in-memory ports. No database, no network.

mod fakes;

use std::sync::Arc;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use chrono::{DateTime, TimeZone, Utc};
use http_body_util::BodyExt;
use ledger_application::ports::{
    AuthError, ReadinessProbe, RepositoryError, ServiceCaller, ServiceTokenRegistry, TokenValidator,
};
use ledger_application::{
    CorrectionsService, IngestService, PnlService, Principal, PrivacyService, ReadService,
    Rebuilder, ReconcileService, Role, TradesService, Trail,
};
use ledger_domain::{IncomingTransaction, Kind, Source, TransactionId};
use ledger_http::{api_router, worker_router, AppState, Metrics};
use serde_json::{json, Value};
use tower::ServiceExt;

use fakes::{
    dec, FakeAccount, FakeAnonymisations, FakeAnonymiser, FakeAssets, FakeAudit, FakeCorrections,
    FakeLease, FakeLots, FakeMarketData, FakePairs, FakeSnapshots, FakeSubjects, FakeTransactions,
    MutableClock, NoPause, SequentialIds,
};

const USER: &str = "11111111-1111-4111-8111-111111111111";
const OTHER: &str = "22222222-2222-4222-8222-222222222222";

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 6, hour, 0, 0).unwrap()
}

#[derive(Default)]
struct FakeValidator;

#[async_trait]
impl TokenValidator for FakeValidator {
    async fn validate(&self, token: &str) -> Result<Principal, AuthError> {
        let subject = match token {
            "user-token" => USER,
            "other-token" => OTHER,
            _ => return Err(AuthError::Missing),
        };
        Ok(Principal {
            subject: subject.to_string(),
            role: Role::User,
        })
    }
}

#[derive(Default)]
struct FakeTokens;

#[async_trait]
impl ServiceTokenRegistry for FakeTokens {
    async fn resolve(&self, token: &str) -> Result<Option<ServiceCaller>, RepositoryError> {
        let caller = match token {
            "portfolio-token" => ServiceCaller {
                service: "portfolio".to_string(),
                operations: vec!["trades.write".to_string()],
            },
            "identity-token" => ServiceCaller {
                service: "identity".to_string(),
                operations: vec![
                    "privacy.export".to_string(),
                    "privacy.anonymise".to_string(),
                ],
            },
            "runner-token" => ServiceCaller {
                service: "job-orchestration".to_string(),
                operations: vec!["runs".to_string()],
            },
            "smoke-token" => ServiceCaller {
                service: "smoke-reader".to_string(),
                operations: vec!["smoke".to_string()],
            },
            _ => return Ok(None),
        };
        Ok(Some(caller))
    }
}

struct Ready;

#[async_trait]
impl ReadinessProbe for Ready {
    async fn check(&self) -> Result<(), String> {
        Ok(())
    }
}

struct Harness {
    transactions: Arc<FakeTransactions>,
    account: Arc<FakeAccount>,
    market: Arc<FakeMarketData>,
    ingest: Arc<IngestService>,
    state: AppState,
}

impl Harness {
    fn api(&self) -> Router {
        api_router(self.state.clone())
    }

    fn worker(&self) -> Router {
        worker_router(self.state.clone())
    }
}

fn harness() -> Harness {
    let transactions = Arc::new(FakeTransactions::default());
    let corrections = Arc::new(FakeCorrections::new(transactions.clone()));
    let subjects = Arc::new(FakeSubjects::default());
    let lots = Arc::new(FakeLots::default());
    let snapshots = Arc::new(FakeSnapshots::default());
    let pairs = Arc::new(FakePairs::default());
    let account = Arc::new(FakeAccount::default());
    let market = Arc::new(FakeMarketData::default());
    market.set_price("BTC", "100");
    market.set_price("ETH", "50");
    market.set_price("SOL", "20");
    let asset_resolver = Arc::new(FakeAssets::knowing(&["BTC", "ETH", "SOL"]));
    let audit = Arc::new(FakeAudit::default());
    let clock = Arc::new(MutableClock::at(at(23)));
    let ids = Arc::new(SequentialIds::default());
    let anonymiser = Arc::new(FakeAnonymiser {
        subjects: subjects.clone(),
    });
    let anonymisations = Arc::new(FakeAnonymisations::default());
    let lease = Arc::new(FakeLease::default());
    let trail = Arc::new(Trail::new(audit, clock.clone(), Arc::new(NoPause)));

    let rebuilder = Arc::new(Rebuilder::new(
        transactions.clone(),
        corrections.clone(),
        lots,
        snapshots,
        pairs.clone(),
        clock.clone(),
    ));
    let ingest = Arc::new(IngestService::new(
        transactions.clone(),
        subjects.clone(),
        pairs,
        account.clone(),
        asset_resolver.clone(),
        clock.clone(),
        ids.clone(),
        "USD",
    ));
    let pnl = Arc::new(PnlService::new(
        subjects.clone(),
        rebuilder.clone(),
        account.clone(),
        market.clone(),
        clock.clone(),
        ids.clone(),
        "USD",
    ));
    let trades = Arc::new(TradesService::new(
        transactions.clone(),
        ingest.clone(),
        asset_resolver.clone(),
        rebuilder.clone(),
        pnl.clone(),
        trail.clone(),
        clock.clone(),
        ids.clone(),
        "USD",
    ));
    let corrections_service = Arc::new(CorrectionsService::new(
        transactions.clone(),
        corrections.clone(),
        ingest.clone(),
        asset_resolver,
        pnl.clone(),
        trail.clone(),
        clock.clone(),
        ids.clone(),
    ));
    let reads = Arc::new(ReadService::new(transactions.clone(), subjects.clone()));
    let privacy = Arc::new(PrivacyService::new(
        transactions.clone(),
        corrections,
        subjects.clone(),
        anonymiser,
        anonymisations,
        trail,
        clock.clone(),
        ids.clone(),
    ));
    let reconcile = Arc::new(ReconcileService::new(
        transactions.clone(),
        subjects,
        lease,
        rebuilder,
        pnl.clone(),
        clock,
    ));

    let state = AppState {
        service_name: "ledger-api",
        ingest: ingest.clone(),
        reads,
        pnl,
        corrections: corrections_service,
        trades,
        privacy,
        reconcile,
        service_tokens: Arc::new(FakeTokens),
        token_validator: Arc::new(FakeValidator),
        readiness: Arc::new(Ready),
        metrics: Arc::new(Metrics::default()),
    };
    Harness {
        transactions,
        account,
        market,
        ingest,
        state,
    }
}

async fn send(router: Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, body)
}

fn get(path: &str, token: Option<&str>, subject: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().method("GET").uri(path);
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    if let Some(subject) = subject {
        builder = builder.header("x-auth-subject", subject);
    }
    builder.body(Body::empty()).unwrap()
}

fn post(path: &str, token: Option<&str>, subject: Option<&str>, body: Value) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json");
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    if let Some(subject) = subject {
        builder = builder.header("x-auth-subject", subject);
    }
    builder.body(Body::from(body.to_string())).unwrap()
}

async fn seed_buy(harness: &Harness) -> TransactionId {
    harness
        .account
        .rows
        .lock()
        .unwrap()
        .push(IncomingTransaction {
            account_id: None,
            asset_id: "BTC".to_string(),
            kind: Some(Kind::Buy),
            quantity: Some(dec("1")),
            price: Some(dec("100")),
            currency: None,
            executed_at: Some(at(10)),
            external_ref: Some("v-1".to_string()),
            source: Source::Venue,
            line_error: None,
        });
    harness.ingest.ingest_user(USER).await.unwrap();
    harness.transactions.stored.lock().unwrap()[0].id()
}

#[tokio::test]
async fn health_is_public() {
    let harness = harness();
    let (status, body) = send(harness.api(), get("/healthz", None, None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn metrics_render_the_health_gauge() {
    let harness = harness();
    let response = harness
        .api()
        .oneshot(get("/metrics", None, None))
        .await
        .unwrap();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(text.contains("ledger_health"));
}

#[tokio::test]
async fn the_transaction_list_requires_a_jwt() {
    let harness = harness();
    let (status, _) = send(
        harness.api(),
        get(&format!("/v1/users/{USER}/transactions"), None, None),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_user_reads_only_their_own_transactions() {
    let harness = harness();
    let (status, body) = send(
        harness.api(),
        get(
            &format!("/v1/users/{USER}/transactions"),
            Some("user-token"),
            Some(USER),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["transactions"].as_array().unwrap().is_empty());

    let (status, _) = send(
        harness.api(),
        get(
            &format!("/v1/users/{OTHER}/transactions"),
            Some("user-token"),
            Some(USER),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn a_stored_transaction_is_returned_with_its_source() {
    let harness = harness();
    seed_buy(&harness).await;
    let (status, body) = send(
        harness.api(),
        get(
            &format!("/v1/users/{USER}/transactions"),
            Some("user-token"),
            Some(USER),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["transactions"][0]["assetId"], "BTC");
    assert_eq!(body["transactions"][0]["source"], "venue");
    assert_eq!(body["transactions"][0]["priceMissing"], false);
}

#[tokio::test]
async fn pnl_is_returned_with_its_coverage_label() {
    let harness = harness();
    seed_buy(&harness).await;
    let (status, body) = send(
        harness.api(),
        get(
            &format!("/v1/users/{USER}/pnl"),
            Some("user-token"),
            Some(USER),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["coverageLabel"], "complete");
    assert_eq!(body["netInvested"], "100");
}

#[tokio::test]
async fn a_market_outage_returns_200_with_marks_unavailable() {
    let harness = harness();
    seed_buy(&harness).await;
    *harness.market.failing.lock().unwrap() = true;
    let (status, body) = send(
        harness.api(),
        get(
            &format!("/v1/users/{USER}/pnl"),
            Some("user-token"),
            Some(USER),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["marksUnavailable"], true);
    assert_eq!(body["coverageLabel"], "partial");
    assert!(body["netInvested"].is_null());
}

#[tokio::test]
async fn a_correction_is_created_for_the_owner() {
    let harness = harness();
    let id = seed_buy(&harness).await;
    let (status, body) = send(
        harness.api(),
        post(
            &format!("/v1/transactions/{id}/corrections"),
            Some("user-token"),
            Some(USER),
            json!({ "field": "quantity", "newValue": "2", "reason": "a typo" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["field"], "quantity");
    assert_eq!(body["oldValue"], "1");
}

#[tokio::test]
async fn an_invalid_correction_is_a_field_problem() {
    let harness = harness();
    let id = seed_buy(&harness).await;
    let (status, body) = send(
        harness.api(),
        post(
            &format!("/v1/transactions/{id}/corrections"),
            Some("user-token"),
            Some(USER),
            json!({ "field": "quantity", "newValue": "0", "reason": "a typo" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"][0]["field"], "newValue");
}

#[tokio::test]
async fn a_recorded_trade_requires_its_service_token() {
    let harness = harness();
    let (status, _) = send(
        harness.api(),
        post(
            &format!("/v1/users/{USER}/trades"),
            None,
            None,
            json!({ "assetId": "BTC", "kind": "buy", "quantity": "1", "price": "100", "executedAt": at(10) }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn portfolio_records_a_trade_and_the_price_source_is_user() {
    let harness = harness();
    let (status, body) = send(
        harness.api(),
        post(
            &format!("/v1/users/{USER}/trades"),
            Some("portfolio-token"),
            None,
            json!({ "assetId": "BTC", "kind": "buy", "quantity": "1", "price": "100", "executedAt": at(10) }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["transaction"]["priceSource"], "user");
    assert_eq!(body["transaction"]["source"], "recorded_trade");
}

#[tokio::test]
async fn a_caller_without_the_trades_operation_is_forbidden() {
    let harness = harness();
    let (status, _) = send(
        harness.api(),
        post(
            &format!("/v1/users/{USER}/trades"),
            Some("identity-token"),
            None,
            json!({ "assetId": "BTC", "kind": "buy", "quantity": "1", "price": "100", "executedAt": at(10) }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn identity_exports_the_ledger_rows() {
    let harness = harness();
    seed_buy(&harness).await;
    let (status, body) = send(
        harness.api(),
        get(
            &format!("/v1/internal/privacy/export?userId={USER}"),
            Some("identity-token"),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["userId"], USER);
    assert_eq!(body["transactions"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn identity_anonymises_and_a_retry_is_idempotent() {
    let harness = harness();
    seed_buy(&harness).await;
    let request = json!({ "userId": USER, "idempotencyKey": "idem-1" });
    let (status, first) = send(
        harness.api(),
        post(
            "/v1/internal/privacy/anonymise",
            Some("identity-token"),
            None,
            request.clone(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(first["alreadyApplied"], false);
    let (status, second) = send(
        harness.api(),
        post(
            "/v1/internal/privacy/anonymise",
            Some("identity-token"),
            None,
            request,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(second["alreadyApplied"], true);
}

#[tokio::test]
async fn a_non_identity_caller_cannot_anonymise() {
    let harness = harness();
    let (status, _) = send(
        harness.api(),
        post(
            "/v1/internal/privacy/anonymise",
            Some("portfolio-token"),
            None,
            json!({ "userId": USER, "idempotencyKey": "idem-1" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn the_worker_runs_a_reconcile() {
    let harness = harness();
    seed_buy(&harness).await;
    let (status, body) = send(
        harness.worker(),
        post(
            "/v1/internal/run/ledger-reconcile",
            Some("runner-token"),
            None,
            json!({}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "completed");
    assert_eq!(body["subjects"], 1);
}

#[tokio::test]
async fn the_worker_ingests_a_user() {
    let harness = harness();
    harness
        .account
        .rows
        .lock()
        .unwrap()
        .push(IncomingTransaction {
            account_id: None,
            asset_id: "BTC".to_string(),
            kind: Some(Kind::Buy),
            quantity: Some(dec("1")),
            price: Some(dec("100")),
            currency: None,
            executed_at: Some(at(10)),
            external_ref: Some("v-1".to_string()),
            source: Source::Venue,
            line_error: None,
        });
    let (status, body) = send(
        harness.worker(),
        post(
            "/v1/internal/run/ledger-ingest",
            Some("runner-token"),
            None,
            json!({ "userId": USER }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["stored"], 1);
}

#[tokio::test]
async fn smoke_requires_the_smoke_operation() {
    let harness = harness();
    let (status, _) = send(
        harness.api(),
        get("/v1/internal/smoke", Some("smoke-token"), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = send(
        harness.api(),
        get("/v1/internal/smoke", Some("portfolio-token"), None),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn an_unknown_path_is_a_problem_document() {
    let harness = harness();
    let (status, body) = send(harness.api(), get("/v1/nothing", None, None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["status"], 404);
    assert!(body["correlationId"].is_string());
}

#[tokio::test]
async fn ready_reports_the_service_ready() {
    let harness = harness();
    let (status, body) = send(harness.api(), get("/readyz", None, None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ready");
}

#[tokio::test]
async fn a_correlation_id_is_echoed_on_the_response() {
    let harness = harness();
    let request = Request::builder()
        .method("GET")
        .uri("/healthz")
        .header("x-correlation-id", "corr-123")
        .body(Body::empty())
        .unwrap();
    let response = harness.api().oneshot(request).await.unwrap();
    assert_eq!(
        response
            .headers()
            .get("x-correlation-id")
            .unwrap()
            .to_str()
            .unwrap(),
        "corr-123"
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
}

#[tokio::test]
async fn an_invalid_correlation_id_is_refused() {
    let harness = harness();
    let request = Request::builder()
        .method("GET")
        .uri("/healthz")
        .header("x-correlation-id", "bad id with spaces")
        .body(Body::empty())
        .unwrap();
    let response = harness.api().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_malformed_body_is_a_field_problem_without_the_value() {
    let harness = harness();
    let request = Request::builder()
        .method("POST")
        .uri(format!("/v1/users/{USER}/trades"))
        .header("authorization", "Bearer portfolio-token")
        .header("content-type", "application/json")
        .body(Body::from("{not json"))
        .unwrap();
    let (status, body) = send(harness.api(), request).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"][0]["field"], "body");
}

#[tokio::test]
async fn a_malformed_cursor_is_refused() {
    let harness = harness();
    let (status, body) = send(
        harness.api(),
        get(
            &format!("/v1/users/{USER}/transactions?cursor=nope"),
            Some("user-token"),
            Some(USER),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"][0]["field"], "cursor");
}

#[tokio::test]
async fn a_correction_for_another_users_transaction_is_forbidden() {
    let harness = harness();
    let id = seed_buy(&harness).await;
    let (status, _) = send(
        harness.api(),
        post(
            &format!("/v1/transactions/{id}/corrections"),
            Some("other-token"),
            Some(OTHER),
            json!({ "field": "quantity", "newValue": "2", "reason": "a typo" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn an_invalid_portfolio_id_is_refused() {
    let harness = harness();
    let (status, body) = send(
        harness.api(),
        get(
            &format!("/v1/users/{USER}/pnl?portfolioId=nope"),
            Some("user-token"),
            Some(USER),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"][0]["field"], "portfolioId");
}

#[tokio::test]
async fn metrics_count_the_requests_they_serve() {
    let harness = harness();
    let _ = send(harness.api(), get("/healthz", None, None)).await;
    let response = harness
        .api()
        .oneshot(get("/metrics", None, None))
        .await
        .unwrap();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(text.contains("ledger_requests_total"));
}
