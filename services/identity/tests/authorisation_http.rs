//! HTTP tests for the C4 `/internal/authorize` adapter and the admin
//! role-change endpoint (BR2.1–BR2.14).
//!
//! Both endpoints take the caller's service token from `x-service-token` and the
//! acting user's Supabase JWT from `Authorization`. The JWT is supplied through
//! the harness's validated-claims seam; the real JWKS validator is covered by
//! the adapter suite.

mod common;

use axum::body::Body;
use axum::http::{header, HeaderMap, Method, Request, StatusCode};
use common::*;
use http_body_util::BodyExt;
use identity_http::router;
use identity_http::state::AppState;
use serde_json::{json, Value};
use tower::ServiceExt;

async fn send(
    state: &AppState,
    method: Method,
    uri: &str,
    service_token: Option<&str>,
    user_token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = service_token {
        builder = builder.header("x-service-token", token);
    }
    if let Some(token) = user_token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
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

fn json_body(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap_or(Value::Null)
}

fn body_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

#[tokio::test]
async fn a_body_carrying_caller_identity_is_rejected_with_400() {
    let harness = authorisation_harness();
    for field in ["callerSubject", "roleClaim"] {
        let mut body = json!({
            "operation": "principal:read",
            "resourceKind": "account",
            "scope": "resource",
            "resourceOwnerId": harness.user_id.to_string(),
        });
        body[field] = json!("22222222-2222-4222-8222-222222222222");
        let (status, headers, bytes) = send(
            &harness.harness.state,
            Method::POST,
            "/internal/authorize",
            Some("account-token"),
            Some("user-token"),
            Some(body),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{field}");
        assert_eq!(
            headers.get(header::CONTENT_TYPE).unwrap(),
            "application/problem+json"
        );
        assert!(
            body_text(&bytes).contains("caller-identity-not-accepted"),
            "{field}: {}",
            body_text(&bytes)
        );
    }
    assert!(
        harness.harness.authorisation_audit.events().is_empty(),
        "a malformed request is not a decision and is not audited"
    );
}

#[tokio::test]
async fn a_missing_or_unknown_service_token_is_unauthorized() {
    let harness = authorisation_harness();
    let body = json!({ "operation": "principal:read" });

    let (status, _, _) = send(
        &harness.harness.state,
        Method::POST,
        "/internal/authorize",
        None,
        Some("user-token"),
        Some(body.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _, _) = send(
        &harness.harness.state,
        Method::POST,
        "/internal/authorize",
        Some("not-a-service-token"),
        Some("user-token"),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_missing_user_jwt_is_unauthorized_with_a_valid_service_token() {
    let harness = authorisation_harness();
    let (status, _, _) = send(
        &harness.harness.state,
        Method::POST,
        "/internal/authorize",
        Some("account-token"),
        None,
        Some(json!({ "operation": "principal:read" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_service_may_ask_only_for_the_operations_on_its_allowlist() {
    let harness = authorisation_harness();
    // The smoke token may ask for `principal:read`, not for a resource write.
    let (status, _, bytes) = send(
        &harness.harness.state,
        Method::POST,
        "/internal/authorize",
        Some("smoke-token"),
        Some("user-token"),
        Some(json!({ "operation": "resource:write" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(body_text(&bytes).contains("operation-not-allowed"));

    // A service that is not on the endpoint at all is refused too.
    let (status, _, _) = send(
        &harness.harness.state,
        Method::POST,
        "/internal/authorize",
        Some("notification-token"),
        Some("user-token"),
        Some(json!({ "operation": "principal:read" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Account may ask for owner-scoped and collection names, never an
    // administrative or confirmation operation.
    for operation in ["admin:users:list", "guard:confirm", "support:metadata:read"] {
        let (status, _, bytes) = send(
            &harness.harness.state,
            Method::POST,
            "/internal/authorize",
            Some("account-token"),
            Some("user-token"),
            Some(json!({ "operation": operation })),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{operation}");
        assert!(body_text(&bytes).contains("operation-not-allowed"));
    }
    let (status, _, _) = send(
        &harness.harness.state,
        Method::POST,
        "/internal/authorize",
        Some("account-token"),
        Some("user-token"),
        Some(json!({ "operation": "collection:list" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn an_owned_row_is_allowed_and_returns_the_decision() {
    let harness = authorisation_harness();
    let (status, _, bytes) = send(
        &harness.harness.state,
        Method::POST,
        "/internal/authorize",
        Some("account-token"),
        Some("user-token"),
        Some(json!({
            "operation": "principal:read",
            "resourceKind": "account",
            "scope": "resource",
            "resourceOwnerId": harness.user_id.to_string(),
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body_text(&bytes));
    let body = json_body(&bytes);
    assert_eq!(body["allowed"], true);
    assert_eq!(body["reason"], "allowed");
    assert_eq!(body["httpStatus"], 200);
}

#[tokio::test]
async fn a_role_denial_and_a_status_denial_are_403() {
    let harness = authorisation_harness();
    // Support is denied every owner-scoped operation.
    let (status, _, bytes) = send(
        &harness.harness.state,
        Method::POST,
        "/internal/authorize",
        Some("account-token"),
        Some("support-token"),
        Some(json!({
            "operation": "principal:read",
            "resourceKind": "account",
            "scope": "resource",
            "resourceOwnerId": harness.support_id.to_string(),
        })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(body_text(&bytes).contains("no_role"));

    // A disabled account is denied every authenticated operation.
    harness
        .harness
        .status_reader
        .set(harness.user_id, identity_domain::UserStatus::Disabled);
    let (status, _, bytes) = send(
        &harness.harness.state,
        Method::POST,
        "/internal/authorize",
        Some("account-token"),
        Some("user-token"),
        Some(json!({
            "operation": "principal:read",
            "resourceKind": "account",
            "scope": "resource",
            "resourceOwnerId": harness.user_id.to_string(),
        })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(body_text(&bytes).contains("status_denied"));
}

#[tokio::test]
async fn a_foreign_row_is_404_not_owner_and_discloses_nothing() {
    let harness = authorisation_harness();
    let (status, headers, bytes) = send(
        &harness.harness.state,
        Method::POST,
        "/internal/authorize",
        Some("portfolio-token"),
        Some("user-token"),
        Some(json!({
            "operation": "resource:read",
            "resourceKind": "portfolio",
            "scope": "resource",
            "resourceOwnerId": harness.other_id.to_string(),
        })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        headers.get(header::CONTENT_TYPE).unwrap(),
        "application/problem+json"
    );
    let text = body_text(&bytes);
    assert!(text.contains("not_owner"));
    assert!(!text.contains(&harness.other_id.to_string()), "{text}");
}

#[tokio::test]
async fn a_denial_that_cannot_be_audited_is_503_audit_unavailable() {
    let harness = authorisation_harness();
    harness.harness.authorisation_audit.fail_next(
        identity_application::ports::RepositoryError::Unavailable("audit down".to_string()),
    );
    let (status, _, bytes) = send(
        &harness.harness.state,
        Method::POST,
        "/internal/authorize",
        Some("account-token"),
        Some("user-token"),
        Some(json!({
            "operation": "resource:read",
            "resourceKind": "account",
            "scope": "resource",
            "resourceOwnerId": harness.other_id.to_string(),
        })),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(body_text(&bytes).contains("audit_unavailable"));
}

#[tokio::test]
async fn a_role_change_is_refused_for_a_non_admin() {
    let harness = authorisation_harness();
    let (status, _, bytes) = send(
        &harness.harness.state,
        Method::PUT,
        "/internal/admin/role",
        Some("gateway-token"),
        Some("user-token"),
        Some(json!({
            "userId": harness.other_id.to_string(),
            "newRole": "admin",
            "reason": "promotion",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(harness.harness.role_writer.writes().is_empty());
    assert!(body_text(&bytes).contains("forbidden"));
}

#[tokio::test]
async fn a_role_change_without_a_reason_is_422_and_names_the_field() {
    let harness = authorisation_harness();
    let (status, _, bytes) = send(
        &harness.harness.state,
        Method::PUT,
        "/internal/admin/role",
        Some("gateway-token"),
        Some("admin-token"),
        Some(json!({
            "userId": harness.other_id.to_string(),
            "newRole": "admin",
            "reason": "   ",
        })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        body_text(&bytes)
    );
    let body = json_body(&bytes);
    assert_eq!(body["errors"][0]["field"], "reason");
    assert!(harness.harness.role_writer.writes().is_empty());
}

#[tokio::test]
async fn an_admin_role_change_succeeds_and_writes_the_provider_claim() {
    let harness = authorisation_harness();
    let (status, _, bytes) = send(
        &harness.harness.state,
        Method::PUT,
        "/internal/admin/role",
        Some("gateway-token"),
        Some("admin-token"),
        Some(json!({
            "userId": harness.other_id.to_string(),
            "newRole": "support",
            "reason": "support rotation",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body_text(&bytes));
    let body = json_body(&bytes);
    assert_eq!(body["userId"], harness.other_id.to_string());
    assert_eq!(body["newRole"], "support");
    assert_eq!(
        harness.harness.role_writer.writes(),
        vec![(OTHER_SUBJECT.to_string(), identity_domain::Role::Support)]
    );
}

#[tokio::test]
async fn the_admin_role_change_is_reachable_through_the_v1_surface() {
    let harness = authorisation_harness();
    let uri = format!("/v1/admin/users/{}/role", harness.other_id);

    // A regular user cannot reach it, even through the user surface.
    let (status, _, _) = send(
        &harness.harness.state,
        Method::PUT,
        &uri,
        None,
        Some("user-token"),
        Some(json!({ "newRole": "support", "reason": "rotation" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(harness.harness.role_writer.writes().is_empty());

    // A blank reason is a 422 that names the field and writes nothing.
    let (status, _, bytes) = send(
        &harness.harness.state,
        Method::PUT,
        &uri,
        None,
        Some("admin-token"),
        Some(json!({ "newRole": "support", "reason": "   " })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        body_text(&bytes)
    );
    assert_eq!(json_body(&bytes)["errors"][0]["field"], "reason");

    // An admin succeeds and the provider write happens.
    let (status, _, bytes) = send(
        &harness.harness.state,
        Method::PUT,
        &uri,
        None,
        Some("admin-token"),
        Some(json!({ "newRole": "support", "reason": "support rotation" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body_text(&bytes));
    assert_eq!(json_body(&bytes)["newRole"], "support");
    assert_eq!(
        harness.harness.role_writer.writes(),
        vec![(OTHER_SUBJECT.to_string(), identity_domain::Role::Support)]
    );
}

#[tokio::test]
async fn the_authorisation_metrics_and_the_c4_route_label_are_emitted() {
    let harness = authorisation_harness();
    let _ = send(
        &harness.harness.state,
        Method::POST,
        "/internal/authorize",
        Some("account-token"),
        Some("user-token"),
        Some(json!({
            "operation": "principal:read",
            "resourceKind": "account",
            "scope": "resource",
            "resourceOwnerId": harness.user_id.to_string(),
        })),
    )
    .await;

    let (status, _, bytes) = send(
        &harness.harness.state,
        Method::GET,
        "/metrics",
        None,
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let text = body_text(&bytes);
    assert!(text.contains("authorisation_decisions_total"), "{text}");
    assert!(
        text.contains("http_requests_total{service=\"identity-api\",status_class=\"2xx\",route=\"internal.authorize\"}"),
        "{text}"
    );
}
