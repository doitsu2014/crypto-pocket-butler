//! HTTP boundary tests: RFC 7807 problems, correlation ids, JWT validation, the
//! service-token allow-list, C1 onboarding routes, C5 reads, and the C11 run
//! endpoint.
//!
//! The router is exercised with `tower::ServiceExt::oneshot`; no port is live.

#[path = "../../application/tests/fakes.rs"]
mod fakes;

use std::sync::Arc;

use account_application::accounts::AccountService;
use account_application::common::{AssuranceLevel, VerifiedClaims};
use account_application::holdings::HoldingService;
use account_application::imports::ImportService;
use account_application::ports::{Clock, ImportRepository};
use account_application::privacy::PrivacyService;
use account_application::sync::SyncService;
use account_application::trail::Trail;
use account_domain::{
    Account, AccountId, AccountKind, Credential, CredentialId, HoldingSnapshot, HoldingSnapshotId,
    ImportedTransaction, ImportedTransactionId, Operation, ParsedTransaction, SyncRun, SyncRunId,
    TransactionKind, UserId, LEASE_SECONDS,
};
use account_http::metrics::Metrics;
use account_http::state::AppState;
use account_http::{router, worker_router, WorkerState};
use axum::body::Body;
use axum::http::{header, HeaderMap, Method, Request, StatusCode};
use chrono::{DateTime, TimeZone, Utc};
use http_body_util::BodyExt;
use rust_decimal::Decimal;
use serde_json::{json, Value};
use std::str::FromStr;
use tower::ServiceExt;

use fakes::{
    FakeAccounts, FakeAudit, FakeAuthAdmin, FakeChain, FakeCredentials, FakeExchange,
    FakeHoldingStore, FakeHoldings, FakeImports, FakeNotifications, FakePrivacy, FakeRegistry,
    FakeRuns, FakeServiceTokens, FakeTokenValidator, FakeVault, FakeWake, FakeWallets,
    MutableClock, Ready,
};

const USER_A: &str = "11111111-1111-4111-8111-111111111111";
const USER_B: &str = "22222222-2222-4222-8222-222222222222";

fn user_a() -> UserId {
    UserId::parse(USER_A).unwrap()
}

fn user_b() -> UserId {
    UserId::parse(USER_B).unwrap()
}

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, hour, 0, 0).unwrap()
}

fn decimal(raw: &str) -> Decimal {
    Decimal::from_str(raw).unwrap()
}

fn claims(user: UserId, assurance: AssuranceLevel) -> VerifiedClaims {
    VerifiedClaims {
        user_id: user,
        assurance,
    }
}

struct Harness {
    accounts: Arc<FakeAccounts>,
    credentials: Arc<FakeCredentials>,
    runs: Arc<FakeRuns>,
    holding_store: Arc<FakeHoldingStore>,
    imports: Arc<FakeImports>,
    exchange: Arc<FakeExchange>,
    vault: Arc<FakeVault>,
    clock: Arc<MutableClock>,
    state: AppState,
    worker: WorkerState,
}

fn harness() -> Harness {
    let accounts = Arc::new(FakeAccounts::default());
    let credentials = Arc::new(FakeCredentials::new(accounts.clone()));
    let wallets = Arc::new(FakeWallets::new(accounts.clone()));
    let holding_store = Arc::new(FakeHoldingStore::default());
    let runs = Arc::new(FakeRuns::new(holding_store.clone()));
    let holdings = Arc::new(FakeHoldings::new(holding_store.clone(), accounts.clone()));
    let imports = Arc::new(FakeImports::new(accounts.clone()));
    let privacy = Arc::new(FakePrivacy::default());
    let exchange = Arc::new(FakeExchange::default());
    let chain = Arc::new(FakeChain::default());
    let registry = Arc::new(FakeRegistry::default());
    let vault = Arc::new(FakeVault::default());
    let auth_admin = Arc::new(FakeAuthAdmin::default());
    let audit = Arc::new(FakeAudit::default());
    let notifications = Arc::new(FakeNotifications::default());
    let wake = Arc::new(FakeWake::default());
    let clock = Arc::new(MutableClock::at(at(10)));
    let trail = Arc::new(Trail::new(audit.clone(), clock.clone()));

    let account_service = Arc::new(AccountService::new(
        accounts.clone(),
        credentials.clone(),
        wallets.clone(),
        runs.clone(),
        exchange.clone(),
        registry.clone(),
        vault.clone(),
        auth_admin.clone(),
        wake.clone(),
        trail.clone(),
        clock.clone(),
    ));
    let sync = Arc::new(SyncService::new(
        accounts.clone(),
        credentials.clone(),
        wallets.clone(),
        runs.clone(),
        exchange.clone(),
        chain.clone(),
        registry.clone(),
        vault.clone(),
        notifications.clone(),
        trail.clone(),
        clock.clone(),
    ));
    let import_service = Arc::new(ImportService::new(
        accounts.clone(),
        imports.clone(),
        trail,
        clock.clone(),
    ));
    let holding_service = Arc::new(HoldingService::new(
        accounts.clone(),
        holdings,
        imports.clone(),
    ));
    let privacy_service = Arc::new(PrivacyService::new(
        accounts.clone(),
        credentials.clone(),
        wallets,
        Arc::new(FakeHoldings::new(holding_store.clone(), accounts.clone())),
        imports.clone(),
        privacy.clone(),
        vault.clone(),
        Arc::new(Trail::new(audit.clone(), clock.clone())),
        clock.clone(),
    ));

    let token_validator = Arc::new(FakeTokenValidator::with(vec![
        ("user-token", Ok(claims(user_a(), AssuranceLevel::Aal2))),
        ("aal1-token", Ok(claims(user_a(), AssuranceLevel::Aal1))),
        ("other-token", Ok(claims(user_b(), AssuranceLevel::Aal2))),
        (
            "expired-token",
            Err(account_application::ports::AuthError::Invalid(
                "expired".to_string(),
            )),
        ),
    ]));
    let service_tokens = Arc::new(FakeServiceTokens::with(&[
        (
            "identity-token",
            "identity",
            vec![Operation::PrivacyExport, Operation::PrivacyAnonymise],
        ),
        (
            "portfolio-token",
            "portfolio",
            vec![Operation::HoldingsRead, Operation::TransactionsRead],
        ),
        (
            "ledger-token",
            "ledger",
            vec![Operation::HoldingsRead, Operation::TransactionsRead],
        ),
        (
            "job-token",
            "job-orchestration",
            vec![Operation::RunScheduledSync],
        ),
    ]));

    let state = AppState {
        service_name: "account-api",
        accounts: account_service,
        sync,
        imports: import_service,
        holdings: holding_service,
        privacy: privacy_service,
        token_validator,
        service_tokens: service_tokens.clone(),
        readiness: Arc::new(Ready(true)),
        metrics: Arc::new(Metrics::default()),
    };
    let worker = WorkerState {
        service_name: "account-worker",
        notifier: Arc::new(tokio::sync::Notify::new()),
        service_tokens,
        readiness: Arc::new(Ready(true)),
        metrics: Arc::new(Metrics::default()),
    };

    Harness {
        accounts,
        credentials,
        runs,
        holding_store,
        imports,
        exchange,
        vault,
        clock,
        state,
        worker,
    }
}

fn seed_account(h: &Harness, kind: AccountKind, venue: &str, label: &str) -> Account {
    let account =
        Account::open(AccountId::generate(), user_a(), kind, venue, label, at(9)).unwrap();
    h.accounts.seed(account.clone());
    account
}

fn seed_credential(h: &Harness, account: &Account) -> Credential {
    let mut credential = Credential::pending(
        CredentialId::generate(),
        account.id(),
        "vault://account/seed",
        at(9),
    )
    .unwrap();
    credential.verify(at(9));
    h.credentials.seed(credential.clone());
    h.vault.seed(
        credential.vault_secret_ref(),
        r#"{"apiKey":"api-key-placeholder","apiSecret":"api-secret-placeholder"}"#,
    );
    credential
}

fn seed_run(h: &Harness, account: &Account) -> SyncRun {
    let run = SyncRun::claim(
        SyncRunId::generate(),
        account.id(),
        h.clock.now(),
        LEASE_SECONDS,
    );
    h.runs.seed(run.clone());
    run
}

async fn send(
    state: &AppState,
    method: Method,
    uri: &str,
    headers: &[(&str, &str)],
    body: Option<&[u8]>,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    let mut builder = Request::builder().method(method).uri(uri);
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let request = match body {
        Some(bytes) => {
            let has_content_type = headers
                .iter()
                .any(|(name, _)| name.eq_ignore_ascii_case("content-type"));
            if !has_content_type {
                builder = builder.header(header::CONTENT_TYPE, "application/json");
            }
            builder.body(Body::from(bytes.to_vec())).unwrap()
        }
        None => builder.body(Body::empty()).unwrap(),
    };
    let response = router(state.clone()).oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, headers, bytes.to_vec())
}

async fn send_auth(
    state: &AppState,
    method: Method,
    uri: &str,
    token: &str,
    body: Option<Value>,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    let authorization = format!("Bearer {token}");
    match body {
        Some(value) => {
            send(
                state,
                method,
                uri,
                &[("authorization", authorization.as_str())],
                Some(value.to_string().as_bytes()),
            )
            .await
        }
        None => {
            send(
                state,
                method,
                uri,
                &[("authorization", authorization.as_str())],
                None,
            )
            .await
        }
    }
}

async fn send_service(
    state: &AppState,
    method: Method,
    uri: &str,
    token: &str,
    body: Option<Value>,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    send_auth(state, method, uri, token, body).await
}

fn json_body(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap_or(Value::Null)
}

// ---------------------------------------------------------------------------
// Probes and boundary behaviour
// ---------------------------------------------------------------------------

#[tokio::test]
async fn health_readiness_and_metrics_are_public() {
    let h = harness();
    let (status, _, body) = send(&h.state, Method::GET, "/healthz", &[], None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json_body(&body)["service"], "account-api");

    let (status, _, _) = send(&h.state, Method::GET, "/readyz", &[], None).await;
    assert_eq!(status, StatusCode::OK);

    let (status, headers, body) = send(&h.state, Method::GET, "/metrics", &[], None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get(header::CONTENT_TYPE).unwrap(),
        "text/plain; version=0.0.4"
    );
    let text = String::from_utf8(body).unwrap();
    assert!(text.contains("account_request_errors_total"));
    assert!(text.contains("account_health{service=\"account-api\"} 1"));
}

#[tokio::test]
async fn an_unknown_route_is_an_rfc_7807_not_found() {
    let h = harness();
    let (status, headers, body) = send(&h.state, Method::GET, "/v1/nothing", &[], None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        headers.get(header::CONTENT_TYPE).unwrap(),
        "application/problem+json"
    );
    assert_eq!(json_body(&body)["status"], 404);
}

#[tokio::test]
async fn a_correlation_id_is_honoured_and_echoed() {
    let h = harness();
    let (_, headers, _) = send(
        &h.state,
        Method::GET,
        "/healthz",
        &[("x-correlation-id", "abc-123")],
        None,
    )
    .await;
    assert_eq!(headers.get("x-correlation-id").unwrap(), "abc-123");
}

#[tokio::test]
async fn an_invalid_correlation_id_is_refused() {
    let h = harness();
    let (status, _, body) = send(
        &h.state,
        Method::GET,
        "/healthz",
        &[("x-correlation-id", "bad id!")],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(json_body(&body).get("correlationId").is_some());
}

#[tokio::test]
async fn a_missing_or_expired_jwt_is_unauthorized() {
    let h = harness();
    let body = json!({"venue":"binance","label":"Main","apiKey":"a","apiSecret":"b"});
    let (status, _, _) = send(
        &h.state,
        Method::POST,
        "/v1/accounts/exchange",
        &[],
        Some(body.to_string().as_bytes()),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _, _) = send_auth(
        &h.state,
        Method::POST,
        "/v1/accounts/exchange",
        "expired-token",
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// C1 onboarding
// ---------------------------------------------------------------------------

fn exchange_body(label: &str) -> Value {
    json!({
        "venue": "binance",
        "label": label,
        "apiKey": "api-key-placeholder",
        "apiSecret": "api-secret-placeholder",
    })
}

#[tokio::test]
async fn adding_an_exchange_account_returns_201_and_the_account_shape() {
    let h = harness();
    let (status, _, body) = send_auth(
        &h.state,
        Method::POST,
        "/v1/accounts/exchange",
        "user-token",
        Some(exchange_body("Main")),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let value = json_body(&body);
    assert_eq!(value["kind"], "exchange");
    assert_eq!(value["status"], "active");
    assert_eq!(value["label"], "Main");
    assert!(value["id"].is_string());
}

#[tokio::test]
async fn an_exchange_key_below_aal2_is_forbidden() {
    let h = harness();
    let (status, _, _) = send_auth(
        &h.state,
        Method::POST,
        "/v1/accounts/exchange",
        "aal1-token",
        Some(exchange_body("Main")),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn a_duplicate_label_is_a_422_field_problem() {
    let h = harness();
    seed_account(&h, AccountKind::Exchange, "binance", "Main");
    let (status, headers, body) = send_auth(
        &h.state,
        Method::POST,
        "/v1/accounts/exchange",
        "user-token",
        Some(exchange_body("MAIN")),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        headers.get(header::CONTENT_TYPE).unwrap(),
        "application/problem+json"
    );
    let value = json_body(&body);
    assert_eq!(value["errors"][0]["field"], "label");
}

#[tokio::test]
async fn adding_a_wallet_returns_201() {
    let h = harness();
    let (status, _, body) = send_auth(
        &h.state,
        Method::POST,
        "/v1/accounts/wallet",
        "user-token",
        Some(json!({
            "chain": "evm:1",
            "address": "0x52908400098527886E0F7030069857D2E4169EE7",
            "label": "Wallet",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(json_body(&body)["kind"], "wallet");
}

#[tokio::test]
async fn adding_a_walletconnect_connection_returns_201() {
    let h = harness();
    let (status, _, body) = send_auth(
        &h.state,
        Method::POST,
        "/v1/accounts/walletconnect",
        "user-token",
        Some(json!({
            "chain": "solana",
            "address": "So11111111111111111111111111111111111111112",
            "label": "WC",
            "sessionRef": "wc-1",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(json_body(&body)["kind"], "walletconnect");
}

#[tokio::test]
async fn resync_returns_202_with_the_run_id() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Exchange, "binance", "Main");
    let uri = format!("/v1/accounts/{}/resync", account.id());
    let (status, _, body) = send_auth(&h.state, Method::POST, &uri, "user-token", None).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let value = json_body(&body);
    assert_eq!(value["status"], "syncing");
    assert!(value["runId"].is_string());
}

#[tokio::test]
async fn listing_accounts_returns_the_users_accounts_and_refuses_another_user() {
    let h = harness();
    seed_account(&h, AccountKind::Exchange, "binance", "Main");
    let uri = format!("/v1/users/{USER_A}/accounts");
    let (status, _, body) = send_auth(&h.state, Method::GET, &uri, "user-token", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json_body(&body).as_array().unwrap().len(), 1);

    let (status, _, _) = send_auth(&h.state, Method::GET, &uri, "other-token", None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn a_csv_upload_is_accepted_and_missing_the_file_name_is_a_422() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Csv, "csv:summary", "History");
    let uri = format!("/v1/accounts/{}/imports", account.id());
    let authorization = "Bearer user-token";
    let (status, _, body) = send(
        &h.state,
        Method::POST,
        &uri,
        &[
            ("authorization", authorization),
            ("x-file-name", "trades.csv"),
            ("content-type", "text/csv"),
        ],
        Some(
            b"external_id,kind,asset,quantity,price,executed_at\n\
t-1,buy,asset-btc,1,30000,2026-11-01T10:00:00Z\n",
        ),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert!(json_body(&body)["batchId"].is_string());

    let (status, _, _) = send(
        &h.state,
        Method::POST,
        &uri,
        &[("authorization", authorization)],
        Some(b"x"),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn a_batch_can_be_read_back_over_http() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Csv, "csv:summary", "History");
    let batch = account_domain::ImportBatch::new(
        account_domain::ImportBatchId::generate(),
        account.id(),
        account_domain::ImportSource::Generic,
        "trades.csv",
        at(9),
    )
    .unwrap();
    h.imports.create_batch(&batch, b"header\n").await.unwrap();
    let uri = format!("/v1/imports/{}", batch.id());
    let (status, _, body) = send_auth(&h.state, Method::GET, &uri, "user-token", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json_body(&body)["batchId"], batch.id().to_string());
}

// ---------------------------------------------------------------------------
// C5 reads
// ---------------------------------------------------------------------------

#[tokio::test]
async fn holdings_are_served_to_portfolio_with_their_last_sync_time() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Exchange, "binance", "Main");
    let snapshot = HoldingSnapshot::record(
        HoldingSnapshotId::generate(),
        SyncRunId::generate(),
        account.id(),
        "BTC",
        Some("asset-btc"),
        decimal("1.5"),
        at(10),
    )
    .unwrap();
    h.holding_store.snapshots.lock().unwrap().push(snapshot);
    let uri = format!("/v1/users/{USER_A}/holdings");
    let (status, _, body) =
        send_service(&h.state, Method::GET, &uri, "portfolio-token", None).await;
    assert_eq!(status, StatusCode::OK);
    let value = json_body(&body);
    assert_eq!(value.as_array().unwrap().len(), 1);
    assert_eq!(value[0]["quantity"], "1.5");
    assert_eq!(value[0]["assetId"], "asset-btc");
}

#[tokio::test]
async fn holdings_are_refused_for_a_service_without_the_operation() {
    let h = harness();
    let uri = format!("/v1/users/{USER_A}/holdings");
    let (status, _, _) = send_service(&h.state, Method::GET, &uri, "identity-token", None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn transactions_are_served_to_ledger() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Csv, "csv:summary", "History");
    let parsed = ParsedTransaction {
        external_id: Some("t-1".to_string()),
        asset_id: "asset-btc".to_string(),
        kind: TransactionKind::Buy,
        quantity: decimal("1"),
        price: None,
        executed_at: at(9),
    };
    let row = ImportedTransaction::record(
        ImportedTransactionId::generate(),
        account_domain::ImportBatchId::generate(),
        account.id(),
        &parsed,
    )
    .unwrap();
    h.imports.insert_transactions(&[row]).await.unwrap();
    let uri = format!("/v1/accounts/{}/transactions", account.id());
    let (status, _, body) = send_service(&h.state, Method::GET, &uri, "ledger-token", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json_body(&body).as_array().unwrap().len(), 1);
    assert_eq!(json_body(&body)[0]["kind"], "buy");
}

#[tokio::test]
async fn an_unknown_service_token_is_unauthorized() {
    let h = harness();
    let uri = format!("/v1/users/{USER_A}/holdings");
    let (status, _, _) = send_service(&h.state, Method::GET, &uri, "made-up-token", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// Privacy
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_privacy_export_is_served_to_identity_only() {
    let h = harness();
    seed_account(&h, AccountKind::Exchange, "binance", "Main");
    let uri = format!("/v1/internal/privacy/export?userId={USER_A}");
    let (status, _, body) = send_service(&h.state, Method::GET, &uri, "identity-token", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json_body(&body)["userId"], USER_A);

    let (status, _, _) = send_service(&h.state, Method::GET, &uri, "portfolio-token", None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn the_privacy_anonymise_is_served_to_identity_only() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Exchange, "binance", "Main");
    seed_credential(&h, &account);
    let (status, _, body) = send_service(
        &h.state,
        Method::POST,
        "/v1/internal/privacy/anonymise",
        "identity-token",
        Some(json!({"userId": USER_A, "idempotencyKey": "idem-1"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json_body(&body)["alreadyApplied"], false);

    let (status, _, _) = send_service(
        &h.state,
        Method::POST,
        "/v1/internal/privacy/anonymise",
        "identity-token",
        Some(json!({"userId": USER_A, "idempotencyKey": ""})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ---------------------------------------------------------------------------
// C11 run endpoint
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_scheduled_run_is_served_to_job_orchestration_only() {
    let h = harness();
    let (status, _, body) = send_service(
        &h.state,
        Method::POST,
        "/v1/internal/run/account-sync",
        "job-token",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json_body(&body)["jobName"], "account-sync");

    let (status, _, _) = send_service(
        &h.state,
        Method::POST,
        "/v1/internal/run/account-sync",
        "identity-token",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn a_named_account_run_returns_succeeded_or_conflicts_on_a_live_lease() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Exchange, "binance", "Main");
    seed_credential(&h, &account);
    h.exchange.set_balances(Ok(Vec::new()));
    let (status, _, body) = send_service(
        &h.state,
        Method::POST,
        "/v1/internal/run/account-sync",
        "job-token",
        Some(json!({"accountIds": [account.id().to_string()]})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json_body(&body)["status"], "succeeded");

    let second = seed_account(&h, AccountKind::Exchange, "binance", "Second");
    seed_run(&h, &second);
    let (status, _, _) = send_service(
        &h.state,
        Method::POST,
        "/v1/internal/run/account-sync",
        "job-token",
        Some(json!({"accountIds": [second.id().to_string()]})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn an_unknown_job_name_is_not_found() {
    let h = harness();
    let (status, _, _) = send_service(
        &h.state,
        Method::POST,
        "/v1/internal/run/other-job",
        "job-token",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// Worker listener
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_worker_health_and_wake_endpoint_behave() {
    let h = harness();
    let request = Request::builder()
        .method(Method::GET)
        .uri("/healthz")
        .body(Body::empty())
        .unwrap();
    let response = worker_router(h.worker.clone())
        .oneshot(request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let request = Request::builder()
        .method(Method::POST)
        .uri("/internal/wake")
        .header(header::AUTHORIZATION, "Bearer job-token")
        .body(Body::empty())
        .unwrap();
    let response = worker_router(h.worker.clone())
        .oneshot(request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);

    let request = Request::builder()
        .method(Method::POST)
        .uri("/internal/wake")
        .body(Body::empty())
        .unwrap();
    let response = worker_router(h.worker).oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
