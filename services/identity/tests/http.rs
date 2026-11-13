//! HTTP boundary tests: RFC 7807 problems, correlation ids, JWT validation,
//! role and ownership denials, the pre-auth gate, and the worker wake endpoint.
//!
//! The router is exercised with `tower::ServiceExt::oneshot`; no port is live.

mod common;

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, HeaderMap, Method, Request, StatusCode};
use common::*;
use http_body_util::BodyExt;
use identity_application::ports::{DeletionRepository, ExternalError};
use identity_domain::user::UserStatus;
use identity_domain::{CountryCode, Role};
use identity_http::metrics::Metrics;
use identity_http::router;
use identity_http::state::AppState;
use identity_http::worker::{router as worker_router, WorkerState};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn send(
    state: &AppState,
    method: Method,
    uri: &str,
    headers: &[(&str, &str)],
    body: Option<Value>,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    let mut builder = Request::builder().method(method).uri(uri);
    for (name, value) in headers {
        builder = builder.header(*name, *value);
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
    (status, headers, bytes.to_vec())
}

async fn send_auth(
    state: &AppState,
    method: Method,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    match token {
        Some(token) => {
            let authorization = format!("Bearer {token}");
            send(
                state,
                method,
                uri,
                &[("authorization", authorization.as_str())],
                body,
            )
            .await
        }
        None => send(state, method, uri, &[], body).await,
    }
}

fn json_body(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap_or(Value::Null)
}

async fn principal(state: &AppState, token: &str, subject: &str) -> Value {
    let (status, _, body) = send_auth(
        state,
        Method::GET,
        &format!("/v1/principals/{subject}"),
        Some(token),
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "principal resolution: {}",
        String::from_utf8_lossy(&body)
    );
    json_body(&body)
}

#[tokio::test]
async fn health_readiness_and_metrics_are_public() {
    let harness = harness();
    let (status, _, body) = send(&harness.state, Method::GET, "/healthz", &[], None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json_body(&body)["service"], "identity-api");

    let (status, _, _) = send(&harness.state, Method::GET, "/readyz", &[], None).await;
    assert_eq!(status, StatusCode::OK);

    let (status, headers, body) = send(&harness.state, Method::GET, "/metrics", &[], None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get(header::CONTENT_TYPE).unwrap(),
        "text/plain; version=0.0.4"
    );
    let text = String::from_utf8(body).unwrap();
    assert!(text.contains("identity_export_status_total"));
    assert!(text.contains("service_up{service=\"identity-api\"} 1"));
}

#[tokio::test]
async fn a_request_without_a_jwt_is_unauthorized_and_echoes_the_correlation_id() {
    let harness = harness();
    let user_id = identity_domain::UserId::generate().to_string();
    let (status, headers, body) = send(
        &harness.state,
        Method::GET,
        &format!("/v1/consent/{user_id}"),
        &[("x-correlation-id", "corr-401")],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        headers.get(header::CONTENT_TYPE).unwrap(),
        "application/problem+json"
    );
    assert_eq!(headers.get("x-correlation-id").unwrap(), "corr-401");
    assert_eq!(json_body(&body)["correlationId"], "corr-401");
}

#[tokio::test]
async fn an_invalid_jwt_is_refused_before_any_write() {
    let harness = harness();
    let (status, _, _) = send_auth(
        &harness.state,
        Method::POST,
        "/v1/consent",
        Some("not-a-token"),
        Some(json!({ "purpose": "analytics", "granted": true })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(harness.consent.records.lock().unwrap().is_empty());
}

#[tokio::test]
async fn principal_resolution_returns_the_documented_principal_and_next_step() {
    let harness = harness();
    let view = principal(&harness.state, "unconfirmed-token", USER_SUBJECT).await;
    assert!(view["userId"].is_string());
    assert_eq!(view["role"], "user");
    assert_eq!(view["status"], "pending");
    assert_eq!(view["nextStep"], "verify_email");

    let (status, _, _) = send_auth(
        &harness.state,
        Method::GET,
        &format!("/v1/principals/{OTHER_SUBJECT}"),
        Some("user-token"),
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "a caller cannot resolve another subject"
    );
}

#[tokio::test]
async fn a_deleted_user_is_refused_with_a_problem_document() {
    let harness = harness();
    let mut user = identity_domain::User::provision(
        identity_domain::UserId::generate(),
        &claims(),
        fixed_now(),
    );
    user.anonymise(fixed_now());
    harness.users.seed(user, USER_SUBJECT);

    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        &format!("/v1/principals/{USER_SUBJECT}"),
        Some("user-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(json_body(&body)["status"], 403);
}

#[tokio::test]
async fn the_legacy_consent_append_is_retired_and_the_read_still_denies_another_user() {
    let harness = harness();
    let view = principal(&harness.state, "user-token", USER_SUBJECT).await;
    let user_id = view["userId"].as_str().unwrap().to_string();

    // U4 retires the purpose-shaped append: it wrote rows the event gate could
    // not see. The caller is pointed at the event route.
    let (status, _, body) = send_auth(
        &harness.state,
        Method::POST,
        "/v1/consent",
        Some("user-token"),
        Some(json!({ "purpose": "analytics", "granted": true, "source": "settings" })),
    )
    .await;
    assert_eq!(status, StatusCode::GONE);
    assert_eq!(
        json_body(&body)["type"],
        "https://errors.cpb/identity/consent-route-retired"
    );
    assert!(
        harness.consent.records.lock().unwrap().is_empty(),
        "no purpose-shaped row is written"
    );

    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        &format!("/v1/consent/{user_id}"),
        Some("user-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json_body(&body)["analytics"], false);
    assert_eq!(json_body(&body)["marketing"], false);

    let (status, _, _) = send_auth(
        &harness.state,
        Method::GET,
        &format!("/v1/consent/{user_id}"),
        Some("other-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn a_preference_change_with_a_bad_currency_is_a_field_level_422() {
    let harness = harness();
    let view = principal(&harness.state, "user-token", USER_SUBJECT).await;
    let user_id = view["userId"].as_str().unwrap().to_string();

    let (status, _, body) = send_auth(
        &harness.state,
        Method::PATCH,
        &format!("/v1/users/{user_id}/preferences"),
        Some("user-token"),
        Some(json!({
            "locale": "vi",
            "baseCurrency": "usd",
            "timezone": "Asia/Ho_Chi_Minh"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json_body(&body)["errors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|error| error["field"] == "baseCurrency"));

    let (status, _, body) = send_auth(
        &harness.state,
        Method::PATCH,
        &format!("/v1/users/{user_id}/preferences"),
        Some("user-token"),
        Some(json!({
            "locale": "vi",
            "baseCurrency": "EUR",
            "timezone": "Asia/Ho_Chi_Minh"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json_body(&body)["locale"], "vi");
    assert_eq!(json_body(&body)["baseCurrency"], "EUR");
}

#[tokio::test]
async fn the_session_list_and_current_session_revoke_require_confirmation() {
    let harness = harness();
    let view = principal(&harness.state, "user-token", USER_SUBJECT).await;
    let user_id = view["userId"].as_str().unwrap().to_string();

    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        &format!("/v1/users/{user_id}/sessions"),
        Some("user-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let sessions = json_body(&body)["sessions"].as_array().unwrap().clone();
    assert_eq!(sessions.len(), 1);
    let session_id = sessions[0]["id"].as_str().unwrap().to_string();
    assert_eq!(sessions[0]["current"], true);

    let (status, _, _) = send_auth(
        &harness.state,
        Method::DELETE,
        &format!("/v1/users/{user_id}/sessions/{session_id}"),
        Some("user-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    let (status, _, _) = send_auth(
        &harness.state,
        Method::DELETE,
        &format!("/v1/users/{user_id}/sessions/{session_id}?confirmCurrent=true"),
        Some("user-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(harness.auth_admin.revoked.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn an_export_request_is_owner_only_and_a_ready_export_serves_a_signed_link() {
    let harness = harness();
    let view = principal(&harness.state, "user-token", USER_SUBJECT).await;
    let _user_id = view["userId"].as_str().unwrap().to_string();

    let (status, _, body) = send_auth(
        &harness.state,
        Method::POST,
        "/v1/exports",
        Some("user-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let export_id = json_body(&body)["exportId"].as_str().unwrap().to_string();

    harness
        .export_worker
        .run_once("worker-1", fixed_now())
        .await
        .unwrap();

    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        &format!("/v1/exports/{export_id}"),
        Some("user-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json_body(&body)["status"], "ready");
    assert!(json_body(&body)["signedDownload"].is_string());

    let (status, _, _) = send_auth(
        &harness.state,
        Method::GET,
        &format!("/v1/exports/{export_id}"),
        Some("other-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn the_signup_gate_is_unauthenticated_and_issues_or_refuses() {
    let harness = harness();
    harness.rules.seed(identity_domain::JurisdictionRule {
        code: CountryCode::parse("VN").unwrap(),
        allowed: true,
        minimum_age: 18,
        disclosures_required: true,
        updated_at: fixed_now(),
    });

    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        "/v1/signup-gate",
        &[],
        Some(json!({
            "jurisdiction": "VN",
            "declaredAge": 21,
            "disclosuresAcknowledged": true
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(json_body(&body)["gateToken"].is_string());

    let (status, _, _) = send(
        &harness.state,
        Method::POST,
        "/v1/signup-gate",
        &[],
        Some(json!({
            "jurisdiction": "US",
            "declaredAge": 21,
            "disclosuresAcknowledged": true
        })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn support_grant_routes_enforce_the_support_and_admin_roles() {
    let harness = harness();
    let target = principal(&harness.state, "user-token", USER_SUBJECT).await;
    let target_id = target["userId"].as_str().unwrap().to_string();
    principal(&harness.state, "support-token", SUPPORT_SUBJECT).await;
    principal(&harness.state, "admin-token", ADMIN_SUBJECT).await;

    let (status, _, _) = send_auth(
        &harness.state,
        Method::POST,
        "/v1/support-grants",
        Some("user-token"),
        Some(json!({ "userId": target_id, "reason": "investigate a sync failure" })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "a regular user cannot request"
    );

    let (status, _, body) = send_auth(
        &harness.state,
        Method::POST,
        "/v1/support-grants",
        Some("support-token"),
        Some(json!({ "userId": target_id, "reason": "investigate a sync failure" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let grant_id = json_body(&body)["grantId"].as_str().unwrap().to_string();
    assert_eq!(json_body(&body)["status"], "pending_approval");

    let (status, _, _) = send_auth(
        &harness.state,
        Method::POST,
        &format!("/v1/support-grants/{grant_id}/approve"),
        Some("support-token"),
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "a support agent cannot approve"
    );

    let (status, _, body) = send_auth(
        &harness.state,
        Method::POST,
        &format!("/v1/support-grants/{grant_id}/approve"),
        Some("admin-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json_body(&body)["status"], "active");

    let (status, _, _) = send_auth(
        &harness.state,
        Method::DELETE,
        &format!("/v1/support-grants/{grant_id}"),
        Some("support-token"),
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "a support agent cannot revoke"
    );

    let (status, _, body) = send_auth(
        &harness.state,
        Method::DELETE,
        &format!("/v1/support-grants/{grant_id}"),
        Some("admin-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json_body(&body)["status"], "revoked");
}

#[tokio::test]
async fn the_c11_sweep_endpoint_requires_a_service_token_and_wakes_the_worker() {
    let harness = harness();
    let (status, _, _) = send(
        &harness.state,
        Method::POST,
        "/v1/internal/run/audit-privacy-sweep",
        &[],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _, _) = send(
        &harness.state,
        Method::POST,
        "/v1/internal/run/audit-privacy-sweep",
        &[("authorization", "Bearer run-token")],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(*harness.wake.calls.lock().unwrap(), 1);
}

#[tokio::test]
async fn the_c11_sweep_endpoint_refuses_another_services_token() {
    let harness = harness();
    let (status, _, _) = send(
        &harness.state,
        Method::POST,
        "/v1/internal/run/audit-privacy-sweep",
        &[("authorization", "Bearer notification-token")],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(*harness.wake.calls.lock().unwrap(), 0);
}

#[tokio::test]
async fn the_retired_support_grant_check_route_is_gone() {
    let harness = harness();
    let uri = format!(
        "/v1/internal/support-grants/aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa/check?userId={USER_SUBJECT}"
    );

    let (status, _, _) = send_auth(
        &harness.state,
        Method::GET,
        &uri,
        Some("notification-token"),
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "the old check endpoint no longer exists, for any token"
    );

    let (status, _, _) = send_auth(
        &harness.state,
        Method::GET,
        &uri,
        Some("gateway-token"),
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "the gateway cannot see a retired route either"
    );
}

#[tokio::test]
async fn the_contact_endpoint_returns_the_verified_address_to_a_service_caller() {
    let harness = harness();
    let view = principal(&harness.state, "user-token", USER_SUBJECT).await;
    let user_id = view["userId"].as_str().unwrap().to_string();

    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        &format!("/v1/users/{user_id}/contact"),
        Some("notification-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let body = json_body(&body);
    assert_eq!(body["email"], "user@example.com");
    assert_eq!(body["verified"], true);
    assert_eq!(body["userId"], user_id);
}

#[tokio::test]
async fn the_contact_endpoint_requires_a_service_token() {
    let harness = harness();
    let view = principal(&harness.state, "user-token", USER_SUBJECT).await;
    let user_id = view["userId"].as_str().unwrap().to_string();
    let (status, _, _) = send(
        &harness.state,
        Method::GET,
        &format!("/v1/users/{user_id}/contact"),
        &[],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn an_unconfirmed_contact_answers_without_an_address() {
    let harness = harness();
    let view = principal(&harness.state, "unconfirmed-token", USER_SUBJECT).await;
    let user_id = view["userId"].as_str().unwrap().to_string();
    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        &format!("/v1/users/{user_id}/contact"),
        Some("notification-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let body = json_body(&body);
    assert_eq!(body["verified"], false);
    assert!(body["email"].is_null());
}

#[tokio::test]
async fn the_contact_endpoint_refuses_another_services_token() {
    let harness = harness();
    let view = principal(&harness.state, "user-token", USER_SUBJECT).await;
    let user_id = view["userId"].as_str().unwrap().to_string();
    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        &format!("/v1/users/{user_id}/contact"),
        Some("service-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(json_body(&body)["status"], 403);

    let (status, _, _) = send_auth(
        &harness.state,
        Method::GET,
        &format!("/v1/users/{user_id}/contact"),
        Some("api-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn a_contact_for_an_unknown_user_is_404() {
    let harness = harness();
    let (status, _, _) = send_auth(
        &harness.state,
        Method::GET,
        "/v1/users/99999999-9999-4999-8999-999999999999/contact",
        Some("notification-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn the_worker_wake_endpoint_requires_a_service_token() {
    let state = WorkerState {
        service_name: "identity-worker",
        notifier: Arc::new(tokio::sync::Notify::new()),
        service_tokens: Arc::new(common::fakes::FakeServiceTokens::with(&[(
            "service-token",
            "identity-api",
        )])),
        readiness: Arc::new(common::fakes::Ready(true)),
        metrics: Arc::new(Metrics::default()),
    };

    let request = Request::builder()
        .method(Method::POST)
        .uri("/internal/wake")
        .body(Body::empty())
        .unwrap();
    let response = worker_router(state.clone()).oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let request = Request::builder()
        .method(Method::POST)
        .uri("/internal/wake")
        .header(header::AUTHORIZATION, "Bearer service-token")
        .body(Body::empty())
        .unwrap();
    let response = worker_router(state).oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
}

#[tokio::test]
async fn an_unknown_route_is_a_404_problem_with_a_correlation_id() {
    let harness = harness();
    let (status, headers, body) = send(
        &harness.state,
        Method::GET,
        "/v1/not-a-route",
        &[("x-correlation-id", "corr-404")],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        headers.get(header::CONTENT_TYPE).unwrap(),
        "application/problem+json"
    );
    assert_eq!(json_body(&body)["correlationId"], "corr-404");
}

#[tokio::test]
async fn the_due_user_endpoint_returns_active_users_to_reporting() {
    let harness = harness();
    let view = principal(&harness.state, "user-token", USER_SUBJECT).await;
    let user_id = view["userId"].as_str().unwrap().to_string();

    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        "/v1/internal/users/due?period=daily&at=2026-11-08T08:00:00Z",
        Some("reporting-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let body = json_body(&body);
    let rows = body.as_array().expect("an array");
    assert!(rows.iter().any(|row| row["userId"] == user_id.as_str()));
    assert!(rows.iter().all(|row| row["timezone"].is_string()));
}

#[tokio::test]
async fn the_due_user_endpoint_refuses_another_services_token() {
    let harness = harness();
    let (status, _, _) = send_auth(
        &harness.state,
        Method::GET,
        "/v1/internal/users/due?period=daily",
        Some("notification-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn the_due_user_endpoint_refuses_an_unknown_period() {
    let harness = harness();
    let (status, _, _) = send_auth(
        &harness.state,
        Method::GET,
        "/v1/internal/users/due?period=yearly",
        Some("reporting-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn the_due_user_endpoint_requires_a_service_token() {
    let harness = harness();
    let (status, _, _) = send(
        &harness.state,
        Method::GET,
        "/v1/internal/users/due?period=daily",
        &[],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// U6 — the jurisdiction gate's internal endpoints (C8)
// ---------------------------------------------------------------------------

const JURISDICTION_CHECK: &str = "/v1/internal/jurisdiction/check";
const JURISDICTION_PUBLISH: &str = "/v1/internal/jurisdiction/publish";

async fn send_service(
    state: &AppState,
    uri: &str,
    token: &str,
    body: Option<Value>,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    let authorization = format!("Bearer {token}");
    send(
        state,
        Method::POST,
        uri,
        &[("authorization", authorization.as_str())],
        body,
    )
    .await
}

#[tokio::test]
async fn jurisdiction_check_returns_an_allowed_decision_with_the_list_version() {
    let harness = harness();
    let (status, _, body) = send_service(
        &harness.state,
        JURISDICTION_CHECK,
        "service-token",
        Some(json!({ "countryCode": "VN", "ip": "192.0.2.10" })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let body = json_body(&body);
    assert_eq!(body["allowed"], true);
    assert_eq!(body["listVersion"], "2026-11-10.1");
    assert!(
        body["evidenceId"].as_str().is_some_and(|id| !id.is_empty()),
        "the decision cites its evidence record"
    );
    assert_eq!(
        harness.decision_audit.actions(),
        vec!["identity.jurisdiction.check"]
    );
}

#[tokio::test]
async fn jurisdiction_check_returns_a_denied_decision_naming_each_restricting_signal() {
    let harness = harness();

    let (status, _, body) = send_service(
        &harness.state,
        JURISDICTION_CHECK,
        "service-token",
        Some(json!({ "countryCode": "KP", "ip": "192.0.2.10" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let body = json_body(&body);
    assert_eq!(body["allowed"], false);
    assert_eq!(body["restrictedBy"], "self_declared");

    let (status, _, body) = send_service(
        &harness.state,
        JURISDICTION_CHECK,
        "service-token",
        Some(json!({ "countryCode": "VN", "ip": "192.0.2.20" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let body = json_body(&body);
    assert_eq!(body["allowed"], false);
    assert_eq!(body["restrictedBy"], "ip");
}

#[tokio::test]
async fn jurisdiction_check_refuses_an_invalid_country_and_a_client_forwarded_ip() {
    let harness = harness();

    let (status, _, body) = send_service(
        &harness.state,
        JURISDICTION_CHECK,
        "service-token",
        Some(json!({ "countryCode": "vn", "ip": "192.0.2.10" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(json_body(&body)["status"], 422);

    // A client-supplied forwarding header is not a trusted address source.
    let authorization = "Bearer service-token";
    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        JURISDICTION_CHECK,
        &[
            ("authorization", authorization),
            ("x-forwarded-for", "203.0.113.9"),
        ],
        Some(json!({ "countryCode": "VN", "ip": "192.0.2.10" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(json_body(&body)["status"], 422);
}

#[tokio::test]
async fn jurisdiction_check_fails_closed_when_the_provider_is_unavailable() {
    let harness = harness();
    harness
        .geo
        .fail_next(ExternalError::Unavailable("provider timeout".to_string()));
    harness
        .geo
        .fail_next(ExternalError::Unavailable("provider timeout".to_string()));

    let (status, headers, body) = send_service(
        &harness.state,
        JURISDICTION_CHECK,
        "service-token",
        Some(json!({ "countryCode": "VN", "ip": "192.0.2.10" })),
    )
    .await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        headers.get(header::CONTENT_TYPE).unwrap(),
        "application/problem+json"
    );
    assert_eq!(json_body(&body)["status"], 503);
    assert_eq!(
        harness.decision_audit.actions().len(),
        0,
        "a provider failure records no decision"
    );
}

#[tokio::test]
async fn jurisdiction_publish_requires_the_admin_role_and_stores_a_new_version() {
    let harness = harness();

    let publish_body = json!({
        "version": "2026-11-10.2",
        "entries": [
            { "countryCode": "VN", "status": "allowed", "source": "compliance" },
            { "countryCode": "US", "status": "allowed", "source": "compliance" }
        ]
    });

    let (status, _, _) = send_service(
        &harness.state,
        JURISDICTION_PUBLISH,
        "user-token",
        Some(publish_body.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "a non-admin cannot publish");

    let (status, _, body) = send_service(
        &harness.state,
        JURISDICTION_PUBLISH,
        "admin-token",
        Some(publish_body),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let body = json_body(&body);
    assert_eq!(body["version"], "2026-11-10.2");
    assert_eq!(body["entryCount"], 2);
    assert_eq!(
        harness.jurisdiction_lists.published_versions(),
        vec!["2026-11-10.2"]
    );
}

#[tokio::test]
async fn jurisdiction_publish_refuses_an_empty_list_and_a_checksum_mismatch() {
    let harness = harness();

    let (status, _, body) = send_service(
        &harness.state,
        JURISDICTION_PUBLISH,
        "admin-token",
        Some(json!({ "version": "2026-11-10.2", "entries": [] })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(json_body(&body)["status"], 422);

    let (status, _, body) = send_service(
        &harness.state,
        JURISDICTION_PUBLISH,
        "admin-token",
        Some(json!({
            "version": "2026-11-10.3",
            "entries": [
                { "countryCode": "VN", "status": "allowed", "source": "compliance" }
            ],
            "checksum": "0".repeat(64)
        })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(json_body(&body)["status"], 422);
    assert!(
        harness.jurisdiction_lists.published_versions().is_empty(),
        "a refused publish never stores a version"
    );
}

#[tokio::test]
async fn jurisdiction_endpoints_refuse_an_unauthenticated_caller() {
    let harness = harness();

    let (status, _, _) = send(
        &harness.state,
        Method::POST,
        JURISDICTION_CHECK,
        &[],
        Some(json!({ "countryCode": "VN", "ip": "192.0.2.10" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _, _) = send(
        &harness.state,
        Method::POST,
        JURISDICTION_PUBLISH,
        &[],
        Some(json!({ "version": "2026-11-10.2", "entries": [] })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

#[tokio::test]
async fn user_registry_principal_accepts_the_declaration_from_the_trusted_headers() {
    let harness = harness();

    let (status, _, _) = send_auth(
        &harness.state,
        Method::GET,
        &format!("/v1/principals/{USER_SUBJECT}"),
        Some("user-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "the token's own declaration clears");

    // The middleware fills the declaration from the trusted caller headers when
    // the token carries none.
    let (status, _, _) = send(
        &harness.state,
        Method::GET,
        &format!("/v1/principals/{USER_SUBJECT}"),
        &[
            ("authorization", "Bearer undeclared-token"),
            ("x-country-code", "VN"),
            ("x-client-ip", "192.0.2.10"),
        ],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn user_registry_principal_missing_declaration_is_a_422() {
    let harness = harness();
    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        &format!("/v1/principals/{USER_SUBJECT}"),
        &[("authorization", "Bearer undeclared-token")],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(json_body(&body)["errors"][0]["field"], "jurisdiction");
    assert_eq!(harness.users.len(), 0);
}

#[tokio::test]
async fn user_registry_gate_denial_is_a_422_problem() {
    let harness = harness();
    harness
        .gate_clearance
        .deny_next("the declared country is restricted");
    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        &format!("/v1/principals/{USER_SUBJECT}"),
        Some("user-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(json_body(&body)["status"], 422);
    assert_eq!(harness.users.len(), 0);
}

#[tokio::test]
async fn user_registry_a_record_is_owner_or_admin_only() {
    let harness = harness();
    let target = active_user();
    harness.users.seed(target.clone(), USER_SUBJECT);
    let other = active_user();
    harness.users.seed(other.clone(), OTHER_SUBJECT);

    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        &format!("/v1/users/{}", target.id()),
        Some("user-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_eq!(json_body(&body)["id"], target.id().to_string());

    let (status, _, _) = send_auth(
        &harness.state,
        Method::GET,
        &format!("/v1/users/{}", other.id()),
        Some("user-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _, _) = send_auth(
        &harness.state,
        Method::GET,
        &format!("/v1/users/{}", target.id()),
        Some("admin-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn the_admin_user_lookup_is_a_body_carrying_post_that_returns_the_role() {
    let harness = harness();
    let target = active_user();
    harness.users.seed(target.clone(), USER_SUBJECT);
    principal(&harness.state, "admin-token", ADMIN_SUBJECT).await;
    harness.role_reader.set(USER_SUBJECT, Role::Support);

    let (status, _, body) = send_auth(
        &harness.state,
        Method::POST,
        "/v1/admin/users/lookup",
        Some("admin-token"),
        Some(json!({ "email": "user@example.com" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let body = json_body(&body);
    assert_eq!(body["id"], target.id().to_string());
    assert_eq!(body["role"], "support");
    assert_eq!(body["email"], "user@example.com");
    assert!(
        body.get("authSubject").is_none(),
        "the provider subject must not leave Identity: {body}"
    );
}

#[tokio::test]
async fn the_admin_user_lookup_is_admin_only_validates_the_body_and_a_miss_is_404() {
    let harness = harness();
    let target = active_user();
    harness.users.seed(target.clone(), USER_SUBJECT);
    principal(&harness.state, "admin-token", ADMIN_SUBJECT).await;
    harness.role_reader.set(USER_SUBJECT, Role::User);

    let (status, _, _) = send_auth(
        &harness.state,
        Method::POST,
        "/v1/admin/users/lookup",
        Some("user-token"),
        Some(json!({ "email": "user@example.com" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _, body) = send_auth(
        &harness.state,
        Method::POST,
        "/v1/admin/users/lookup",
        Some("admin-token"),
        Some(json!({ "email": "not-an-address" })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        String::from_utf8_lossy(&body)
    );
    assert_eq!(json_body(&body)["errors"][0]["field"], "email");

    let (status, _, body) = send_auth(
        &harness.state,
        Method::POST,
        "/v1/admin/users/lookup",
        Some("admin-token"),
        Some(json!({})),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        String::from_utf8_lossy(&body)
    );

    let (status, _, _) = send_auth(
        &harness.state,
        Method::POST,
        "/v1/admin/users/lookup",
        Some("admin-token"),
        Some(json!({ "email": "missing@example.com" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_provider_failure_on_the_admin_lookup_is_a_503_and_no_role_is_returned() {
    let harness = harness();
    let target = active_user();
    harness.users.seed(target.clone(), USER_SUBJECT);
    principal(&harness.state, "admin-token", ADMIN_SUBJECT).await;
    harness
        .role_reader
        .fail_next(ExternalError::Unavailable("supabase is down".to_string()));

    let (status, _, body) = send_auth(
        &harness.state,
        Method::POST,
        "/v1/admin/users/lookup",
        Some("admin-token"),
        Some(json!({ "email": "user@example.com" })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::SERVICE_UNAVAILABLE,
        "{}",
        String::from_utf8_lossy(&body)
    );
    assert!(json_body(&body).get("role").is_none());
}

#[tokio::test]
async fn the_u1_admin_search_path_is_no_longer_served() {
    let harness = harness();
    let target = active_user();
    harness.users.seed(target.clone(), USER_SUBJECT);
    principal(&harness.state, "admin-token", ADMIN_SUBJECT).await;

    let (status, _, _) = send_auth(
        &harness.state,
        Method::GET,
        "/v1/admin/users?email=user@example.com",
        Some("admin-token"),
        None,
    )
    .await;
    assert!(
        matches!(
            status,
            StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED
        ),
        "the removed U1 search path must not answer: {status}"
    );
}

#[tokio::test]
async fn the_pending_grant_list_is_admin_only_and_accepts_only_the_pending_status() {
    let harness = harness();
    let target = principal(&harness.state, "user-token", USER_SUBJECT).await;
    let target_id = target["userId"].as_str().unwrap().to_string();
    let agent = principal(&harness.state, "support-token", SUPPORT_SUBJECT).await;
    let agent_id = agent["userId"].as_str().unwrap().to_string();
    principal(&harness.state, "admin-token", ADMIN_SUBJECT).await;

    let (status, _, body) = send_auth(
        &harness.state,
        Method::POST,
        "/v1/support-grants",
        Some("support-token"),
        Some(json!({ "userId": target_id, "reason": "investigate a sync failure" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let grant_id = json_body(&body)["grantId"].as_str().unwrap().to_string();

    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        "/v1/support-grants?status=pending_approval",
        Some("admin-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let body = json_body(&body);
    let grants = body.as_array().expect("the list is an array");
    assert_eq!(grants.len(), 1, "{body}");
    assert_eq!(grants[0]["id"], grant_id);
    assert_eq!(grants[0]["userId"], target_id);
    assert_eq!(grants[0]["requestedBy"], agent_id);
    assert_eq!(grants[0]["status"], "pending_approval");
    assert_eq!(grants[0]["reason"], "investigate a sync failure");

    let (status, _, _) = send_auth(
        &harness.state,
        Method::GET,
        "/v1/support-grants?status=pending_approval",
        Some("support-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    for uri in ["/v1/support-grants?status=active", "/v1/support-grants"] {
        let (status, _, body) =
            send_auth(&harness.state, Method::GET, uri, Some("admin-token"), None).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{uri}: {}",
            String::from_utf8_lossy(&body)
        );
    }
}

#[tokio::test]
async fn the_u10_admin_routes_are_labelled_routes_not_other() {
    let harness = harness();
    let target = active_user();
    harness.users.seed(target.clone(), USER_SUBJECT);
    principal(&harness.state, "admin-token", ADMIN_SUBJECT).await;
    principal(&harness.state, "user-token", USER_SUBJECT).await;
    harness.role_reader.set(USER_SUBJECT, Role::User);

    let (_, _, _) = send_auth(
        &harness.state,
        Method::POST,
        "/v1/admin/users/lookup",
        Some("admin-token"),
        Some(json!({ "email": "user@example.com" })),
    )
    .await;
    let (_, _, _) = send_auth(
        &harness.state,
        Method::PUT,
        &format!("/v1/admin/users/{}/role", target.id()),
        Some("admin-token"),
        Some(json!({ "newRole": "support", "reason": "moved to support" })),
    )
    .await;
    let (_, _, _) = send_auth(
        &harness.state,
        Method::POST,
        &format!("/v1/admin/users/{}/disable", target.id()),
        Some("admin-token"),
        Some(json!({ "reason": "account abuse" })),
    )
    .await;
    let (_, _, _) = send_auth(
        &harness.state,
        Method::POST,
        &format!("/v1/admin/users/{}/restore", target.id()),
        Some("admin-token"),
        Some(json!({ "reason": "appeal upheld" })),
    )
    .await;
    let (_, _, _) = send_auth(
        &harness.state,
        Method::GET,
        "/v1/support-grants?status=pending_approval",
        Some("admin-token"),
        None,
    )
    .await;

    let rendered = harness.state.metrics.render("identity-api");
    for label in [
        "admin.users.lookup",
        "admin.users.role",
        "admin.users.disable",
        "admin.users.restore",
        "support.grants",
    ] {
        assert!(
            rendered.contains(&format!("route=\"{label}\"")),
            "the {label} route is labelled rather than folded into other:\n{rendered}"
        );
    }
}

#[tokio::test]
async fn user_registry_internal_provision_requires_the_gateway_and_the_matching_hash() {
    let harness = harness();
    let hash = identity_application::user_registry::subject_hash(USER_SUBJECT);
    let uri = format!("/v1/internal/users/by-subject/{hash}");
    let body = json!({
        "subject": USER_SUBJECT,
        "role": "user",
        "email": "user@example.com",
        "emailConfirmed": true,
        "displayName": "Test User",
        "assurance": "aal1",
        "hasExchangeCredentials": false,
        "countryCode": "VN"
    });
    let authorization = bearer("gateway-token");
    let (status, _, response) = send(
        &harness.state,
        Method::PUT,
        &uri,
        &[
            ("authorization", authorization.as_str()),
            ("x-client-ip", "192.0.2.10"),
        ],
        Some(body.clone()),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&response)
    );

    // A second PUT refreshes and answers 200.
    let (status, _, _) = send(
        &harness.state,
        Method::PUT,
        &uri,
        &[
            ("authorization", authorization.as_str()),
            ("x-client-ip", "192.0.2.10"),
        ],
        Some(body.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // The documented service-token checks: no token, a wrong service, and a
    // mismatched subject hash.
    let (status, _, _) = send(&harness.state, Method::PUT, &uri, &[], Some(body.clone())).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let notification = bearer("notification-token");
    let (status, _, _) = send(
        &harness.state,
        Method::PUT,
        &uri,
        &[("authorization", notification.as_str())],
        Some(body.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let wrong_hash = identity_application::user_registry::subject_hash(OTHER_SUBJECT);
    let (status, _, _) = send(
        &harness.state,
        Method::PUT,
        &format!("/v1/internal/users/by-subject/{wrong_hash}"),
        &[
            ("authorization", authorization.as_str()),
            ("x-client-ip", "192.0.2.10"),
        ],
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn user_registry_internal_lookup_returns_the_metadata_boundary() {
    let harness = harness();
    let target = active_user();
    harness.users.seed(target.clone(), USER_SUBJECT);
    let hash = identity_application::user_registry::subject_hash(USER_SUBJECT);
    let authorization = bearer("gateway-token");

    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        &format!("/v1/internal/users/by-subject/{hash}"),
        &[("authorization", authorization.as_str())],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let body = json_body(&body);
    assert_eq!(body["id"], target.id().to_string());
    assert_eq!(body["email"], "user@example.com");
    assert!(body.get("role").is_none(), "the boundary omits the role");

    let (status, _, _) = send(
        &harness.state,
        Method::GET,
        &format!(
            "/v1/internal/users/by-subject/{}",
            identity_application::user_registry::subject_hash(OTHER_SUBJECT)
        ),
        &[("authorization", authorization.as_str())],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        "/v1/internal/users?email=user@example.com",
        &[("authorization", authorization.as_str())],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_eq!(json_body(&body)["id"], target.id().to_string());
}

// ── U7: the settings routes (D0, P1, P2, D1, D2) ───────────────────────────

/// A stored record in an explicit lifecycle state. `deleted` carries the
/// anonymised shape the database constraint requires.
fn user_in(status: UserStatus) -> identity_domain::User {
    let deleted = status == UserStatus::Deleted;
    identity_domain::User::from_stored(
        identity_domain::UserId::generate(),
        identity_domain::claims::Role::User,
        status,
        if deleted {
            None
        } else {
            Some("Test User".to_string())
        },
        false,
        if deleted {
            None
        } else {
            Some("user@example.com".to_string())
        },
        false,
        !deleted,
        identity_domain::Locale::En,
        identity_domain::BaseCurrency::parse("USD").unwrap(),
        identity_domain::TimeZone::parse("UTC").unwrap(),
        fixed_now(),
        fixed_now(),
        deleted.then(fixed_now),
    )
}

fn open_request(
    user_id: identity_domain::UserId,
    requested_at: chrono::DateTime<chrono::Utc>,
) -> identity_domain::deletion::DeletionRequest {
    identity_domain::deletion::DeletionRequest::request(
        identity_domain::DeletionRequestId::generate(),
        user_id,
        requested_at,
    )
}

#[tokio::test]
async fn d0_every_status_is_read_from_the_callers_own_record() {
    for status in [
        UserStatus::Pending,
        UserStatus::Active,
        UserStatus::Disabled,
        UserStatus::PendingDeletion,
        UserStatus::Deleted,
    ] {
        let harness = harness();
        harness.users.seed(user_in(status), USER_SUBJECT);

        let (status_code, _, body) = send_auth(
            &harness.state,
            Method::GET,
            "/v1/me/account-status",
            Some("user-token"),
            None,
        )
        .await;
        assert_eq!(
            status_code,
            StatusCode::OK,
            "{}",
            String::from_utf8_lossy(&body)
        );
        assert_eq!(json_body(&body)["status"], status.as_str());
        assert_eq!(
            json_body(&body).as_object().unwrap().len(),
            1,
            "D0 answers the status and nothing else"
        );
    }
}

#[tokio::test]
async fn d0_ignores_any_user_id_in_the_request() {
    let harness = harness();
    let other = user_in(UserStatus::Active);
    harness.users.seed(other.clone(), OTHER_SUBJECT);
    harness
        .users
        .seed(user_in(UserStatus::Disabled), USER_SUBJECT);

    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        &format!("/v1/me/account-status?userId={}", other.id()),
        Some("user-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        json_body(&body)["status"],
        "disabled",
        "the answer comes from the caller's own record, never from the request"
    );
}

#[tokio::test]
async fn the_settings_reads_without_a_token_are_unauthorized() {
    let harness = harness();
    for (method, uri) in [
        (Method::GET, "/v1/me/account-status"),
        (Method::GET, "/v1/me/profile"),
        (Method::PATCH, "/v1/me/profile"),
        (Method::GET, "/v1/me/deletion-request"),
        (Method::DELETE, "/v1/me/deletion-request"),
    ] {
        let (status, _, body) = send_auth(&harness.state, method.clone(), uri, None, None).await;
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "{method} {uri}: {}",
            String::from_utf8_lossy(&body)
        );
    }
}

#[tokio::test]
async fn p1_the_profile_read_returns_the_exact_field_set() {
    let harness = harness();
    let user = user_in(UserStatus::Active);
    harness.users.seed(user.clone(), USER_SUBJECT);

    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        "/v1/me/profile",
        Some("user-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let body = json_body(&body);
    let mut keys: Vec<&str> = body
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "displayName",
            "displayNameOverridden",
            "email",
            "emailVerified",
            "providerDisplayName",
            "status",
            "userId",
        ]
    );
    assert_eq!(body["userId"], user.id().to_string());
    assert_eq!(body["displayName"], "Test User");
    assert_eq!(body["displayNameOverridden"], false);
    assert_eq!(body["providerDisplayName"], "Test User");
    assert_eq!(body["email"], "user@example.com");
    assert_eq!(body["emailVerified"], true);
    assert_eq!(body["status"], "active");
}

#[tokio::test]
async fn p1_an_unavailable_store_is_a_problem_503() {
    let harness = harness();
    harness
        .users
        .fail_next(identity_application::ports::RepositoryError::Unavailable(
            "database timed out".to_string(),
        ));

    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        "/v1/me/profile",
        Some("user-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        json_body(&body)["type"],
        "https://errors.cpb/identity/unavailable"
    );
}

#[tokio::test]
async fn p2_a_display_name_save_returns_the_profile_shape() {
    let harness = harness();
    let user = user_in(UserStatus::Active);
    harness.users.seed(user.clone(), USER_SUBJECT);

    let (status, _, body) = send_auth(
        &harness.state,
        Method::PATCH,
        "/v1/me/profile",
        Some("user-token"),
        Some(json!({ "displayName": "  Mai Anh  " })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let body = json_body(&body);
    assert_eq!(body["displayName"], "Mai Anh");
    assert_eq!(body["displayNameOverridden"], true);
    assert_eq!(body["providerDisplayName"], "Test User");
    assert_eq!(body["userId"], user.id().to_string());
    assert_eq!(
        harness.users.get(user.id()).unwrap().display_name(),
        Some("Mai Anh")
    );
}

#[tokio::test]
async fn p2_both_or_neither_selector_is_422_and_an_invalid_name_names_display_name() {
    let harness = harness();
    harness
        .users
        .seed(user_in(UserStatus::Active), USER_SUBJECT);

    for body in [
        json!({ "displayName": "Mai Anh", "useProviderName": true }),
        json!({}),
        json!({ "useProviderName": false }),
    ] {
        let (status, _, response) = send_auth(
            &harness.state,
            Method::PATCH,
            "/v1/me/profile",
            Some("user-token"),
            Some(body.clone()),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{body}: {}",
            String::from_utf8_lossy(&response)
        );
        assert_eq!(json_body(&response)["status"], 422);
    }

    let (status, _, response) = send_auth(
        &harness.state,
        Method::PATCH,
        "/v1/me/profile",
        Some("user-token"),
        Some(json!({ "displayName": "   " })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let response = json_body(&response);
    assert_eq!(response["errors"][0]["field"], "displayName");
    assert!(
        response
            .get("detail")
            .is_some_and(|detail| detail.is_string()),
        "the RFC 7807 body still carries its summary"
    );
}

#[tokio::test]
async fn p2_adopting_a_missing_provider_name_is_409() {
    let harness = harness();
    harness
        .users
        .seed(user_in(UserStatus::Active), USER_SUBJECT);

    let (status, _, body) = send_auth(
        &harness.state,
        Method::PATCH,
        "/v1/me/profile",
        Some("nameless-token"),
        Some(json!({ "useProviderName": true })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "{}",
        String::from_utf8_lossy(&body)
    );
    assert_eq!(
        json_body(&body)["type"],
        "https://errors.cpb/identity/conflict"
    );

    let (status, _, body) = send_auth(
        &harness.state,
        Method::PATCH,
        "/v1/me/profile",
        Some("user-token"),
        Some(json!({ "useProviderName": true })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_eq!(json_body(&body)["displayNameOverridden"], false);
}

#[tokio::test]
async fn d1_the_open_request_is_returned_and_an_empty_answer_is_the_pinned_no_record() {
    let open_harness = harness();
    let user = user_in(UserStatus::PendingDeletion);
    open_harness.users.seed(user.clone(), USER_SUBJECT);
    let request = open_request(user.id(), fixed_now());
    open_harness.deletions.insert(&request).await.unwrap();

    let (status, _, body) = send_auth(
        &open_harness.state,
        Method::GET,
        "/v1/me/deletion-request",
        Some("user-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let body = json_body(&body);
    assert_eq!(body["deletionId"], request.id().to_string());
    assert_eq!(body["status"], "requested");

    let restored = harness();
    restored
        .users
        .seed(user_in(UserStatus::Active), USER_SUBJECT);
    let (status, headers, body) = send_auth(
        &restored.state,
        Method::GET,
        "/v1/me/deletion-request",
        Some("user-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        headers.get(header::CONTENT_TYPE).unwrap(),
        "application/problem+json"
    );
    assert_eq!(
        json_body(&body)["type"],
        "https://crypto-pocket-butler.example/problems/no-record",
        "the pinned type is what tells the SPA 'no record' from any other 404"
    );
}

#[tokio::test]
async fn d2_the_cancel_answers_200_then_409_and_410() {
    // 200: the account is pending deletion and the request is open.
    let open_harness = harness();
    let user = user_in(UserStatus::PendingDeletion);
    open_harness.users.seed(user.clone(), USER_SUBJECT);
    let request = open_request(user.id(), fixed_now());
    open_harness.deletions.insert(&request).await.unwrap();

    let (status, _, body) = send_auth(
        &open_harness.state,
        Method::DELETE,
        "/v1/me/deletion-request",
        Some("user-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert!(json_body(&body)["cancelledAt"].is_string());
    assert_eq!(
        open_harness.users.get(user.id()).unwrap().status(),
        UserStatus::Active
    );

    // 409: nothing is open.
    let empty = harness();
    empty.users.seed(user_in(UserStatus::Active), USER_SUBJECT);
    let (status, _, _) = send_auth(
        &empty.state,
        Method::DELETE,
        "/v1/me/deletion-request",
        Some("user-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // 410: the recovery window has closed.
    let closed = harness();
    let stale = user_in(UserStatus::PendingDeletion);
    closed.users.seed(stale.clone(), USER_SUBJECT);
    let old = open_request(stale.id(), fixed_now() - chrono::Duration::days(31));
    closed.deletions.insert(&old).await.unwrap();
    let (status, _, body) = send_auth(
        &closed.state,
        Method::DELETE,
        "/v1/me/deletion-request",
        Some("user-token"),
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::GONE,
        "{}",
        String::from_utf8_lossy(&body)
    );
    assert_eq!(json_body(&body)["status"], 410);
}

#[tokio::test]
async fn the_settings_paths_are_labelled_routes_not_other() {
    let harness = harness();
    let user = user_in(UserStatus::Active);
    harness.users.seed(user.clone(), USER_SUBJECT);

    let calls = [
        (Method::GET, "/v1/me/account-status"),
        (Method::GET, "/v1/me/profile"),
        (Method::PATCH, "/v1/me/profile"),
        (Method::GET, "/v1/me/deletion-request"),
        (Method::DELETE, "/v1/me/deletion-request"),
        (Method::POST, "/v1/privacy/deletions"),
    ];
    for (method, uri) in calls {
        let body = match (method.clone(), uri) {
            (Method::PATCH, _) => Some(json!({ "displayName": "Mai Anh" })),
            _ => None,
        };
        send_auth(
            &harness.state,
            method.clone(),
            uri,
            Some("user-token"),
            body,
        )
        .await;
    }

    let rendered = harness.state.metrics.render("identity-api");
    for label in [
        "me.account-status",
        "me.profile",
        "me.deletion.request",
        "privacy.deletions",
    ] {
        assert!(
            rendered.contains(&format!("route=\"{label}\"")),
            "the {label} route is labelled rather than folded into other:\n{rendered}"
        );
    }
}
