//! The Account service's support-metadata endpoint (BR3.9, contract C1/C2).
//!
//! The route is internal: the Identity service's token is the only one allowed
//! to call it, through the new `support_metadata` operation. The projection is
//! the closed allowlist from the functional spec — an account id, label, source
//! type, sync status, and last-sync time, and nothing that names a venue, a
//! provider account, a credential, or a holding.

#[path = "../application/tests/fakes.rs"]
mod fakes;

use std::sync::Arc;

use account_application::accounts::AccountService;
use account_application::common::{AssuranceLevel, VerifiedClaims};
use account_application::holdings::HoldingService;
use account_application::imports::ImportService;
use account_application::privacy::PrivacyService;
use account_application::sync::SyncService;
use account_application::trail::Trail;
use account_domain::{Account, AccountId, AccountKind, AccountStatus, Operation, UserId, Venue};
use account_http::metrics::Metrics;
use account_http::router;
use account_http::state::AppState;
use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use chrono::{DateTime, TimeZone, Utc};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

use fakes::{
    FakeAccounts, FakeAudit, FakeAuthAdmin, FakeChain, FakeCredentials, FakeExchange,
    FakeHoldingStore, FakeHoldings, FakeImports, FakeNotifications, FakePrivacy, FakeRegistry,
    FakeRuns, FakeServiceTokens, FakeTokenValidator, FakeVault, FakeWake, FakeWallets,
    MutableClock, Ready,
};

const USER_A: &str = "11111111-1111-4111-8111-111111111111";
const SUPPORT_OPERATION_PATH: &str = "/v1/internal/support-metadata";

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, hour, 0, 0).unwrap()
}

fn user_a() -> UserId {
    UserId::parse(USER_A).unwrap()
}

fn claims() -> VerifiedClaims {
    VerifiedClaims {
        user_id: user_a(),
        assurance: AssuranceLevel::Aal2,
    }
}

struct Harness {
    accounts: Arc<FakeAccounts>,
    state: AppState,
}

fn harness() -> Harness {
    let accounts = Arc::new(FakeAccounts::default());
    let credentials = Arc::new(FakeCredentials::new(accounts.clone()));
    let wallets = Arc::new(FakeWallets::new(accounts.clone()));
    let holding_store = Arc::new(FakeHoldingStore::default());
    let runs = Arc::new(FakeRuns::new(holding_store.clone()));
    let holdings = Arc::new(FakeHoldings::new(holding_store.clone(), accounts.clone()));
    let imports = Arc::new(FakeImports::new(accounts.clone()));
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
        exchange,
        chain,
        registry,
        vault.clone(),
        notifications,
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
        credentials,
        wallets,
        Arc::new(FakeHoldings::new(holding_store, accounts.clone())),
        imports,
        Arc::new(FakePrivacy::default()),
        vault,
        Arc::new(Trail::new(audit, clock.clone())),
        clock.clone(),
    ));

    let token_validator = Arc::new(FakeTokenValidator::with(vec![("user-token", Ok(claims()))]));
    let service_tokens = Arc::new(FakeServiceTokens::with(&[
        (
            "identity-token",
            "identity",
            vec![
                Operation::PrivacyExport,
                Operation::PrivacyAnonymise,
                Operation::SupportMetadata,
            ],
        ),
        (
            "portfolio-token",
            "portfolio",
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
        service_tokens,
        readiness: Arc::new(Ready(true)),
        metrics: Arc::new(Metrics::default()),
    };

    Harness { accounts, state }
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

fn seed_exchange_account(accounts: &FakeAccounts, user: UserId) {
    let venue = Venue::parse_for(AccountKind::Exchange, "binance").unwrap();
    accounts.seed(Account::from_stored(
        AccountId::generate(),
        user,
        AccountKind::Exchange,
        venue,
        "Binance main".to_string(),
        AccountStatus::Active,
        Some(at(9)),
        at(8),
        at(9),
        None,
    ));
}

#[tokio::test]
async fn the_identity_token_reads_the_allowlisted_projection() {
    let harness = harness();
    let user = user_a();
    seed_exchange_account(&harness.accounts, user);

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
    assert_eq!(sorted_keys(&body), vec!["accounts".to_string()]);
    let accounts = body["accounts"].as_array().expect("an array");
    assert_eq!(accounts.len(), 1);
    assert_eq!(
        sorted_keys(&accounts[0]),
        vec![
            "id".to_string(),
            "label".to_string(),
            "lastSyncedAt".to_string(),
            "sourceType".to_string(),
            "syncStatus".to_string(),
        ],
        "the projection is the closed allowlist"
    );
    assert_eq!(accounts[0]["label"], "Binance main");
    assert_eq!(accounts[0]["sourceType"], "exchange");
    assert_eq!(accounts[0]["syncStatus"], "active");
    assert!(accounts[0]["lastSyncedAt"].as_str().is_some());
}

#[tokio::test]
async fn a_registered_token_without_the_support_operation_is_refused() {
    let harness = harness();
    let user = user_a();
    seed_exchange_account(&harness.accounts, user);

    let (status, body) = get(
        &harness.state,
        &format!("{SUPPORT_OPERATION_PATH}/{user}"),
        Some("portfolio-token"),
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
    assert_eq!(json_body(&body)["accounts"], serde_json::json!([]));
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
