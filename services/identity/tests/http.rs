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
use identity_domain::CountryCode;
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
async fn consent_append_and_read_round_trip_with_an_ownership_denial() {
    let harness = harness();
    let view = principal(&harness.state, "user-token", USER_SUBJECT).await;
    let user_id = view["userId"].as_str().unwrap().to_string();

    let (status, _, body) = send_auth(
        &harness.state,
        Method::POST,
        "/v1/consent",
        Some("user-token"),
        Some(json!({ "purpose": "analytics", "granted": true, "source": "settings" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(json_body(&body)["granted"], true);

    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        &format!("/v1/consent/{user_id}"),
        Some("user-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json_body(&body)["analytics"], true);
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
        &[("authorization", "Bearer service-token")],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(*harness.wake.calls.lock().unwrap(), 1);
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
