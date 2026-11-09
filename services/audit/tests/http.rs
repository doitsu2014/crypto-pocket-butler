//! HTTP boundary tests: RFC 7807 problems, correlation ids, service-token auth,
//! role gating, deny-by-default, JSONL export streaming, and metrics.
//!
//! The router is exercised with `tower::ServiceExt::oneshot`; no port is live.

mod common;

use audit_http::rate_limit::Limits;
use audit_http::routes::router;
use audit_http::AppState;
use axum::body::Body;
use axum::http::{header, HeaderMap, Method, Request, StatusCode};
use common::*;
use http_body_util::BodyExt;
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

fn json_body(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap_or(Value::Null)
}

fn append_body() -> Value {
    json!({
        "actorId": USER_SUBJECT,
        "actorKind": "user",
        "action": "asset.contract.added",
        "target": "asset-usdc",
        "outcome": "success",
        "sensitivity": "routine"
    })
}

#[tokio::test]
async fn health_and_readiness_are_public() {
    let harness = harness();
    let (status, _, body) = send(&harness.state, Method::GET, "/healthz", &[], None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json_body(&body)["service"], "audit-api");

    let (status, _, _) = send(&harness.state, Method::GET, "/readyz", &[], None).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn metrics_expose_the_audit_counters() {
    let harness = harness();
    let (status, headers, body) = send(&harness.state, Method::GET, "/metrics", &[], None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get(header::CONTENT_TYPE).unwrap(),
        "text/plain; version=0.0.4"
    );
    let text = String::from_utf8(body).unwrap();
    assert!(text.contains("audit_appends_total"));
    assert!(!text.contains("audit_export_errors"));
    assert!(text.contains("audit_support_checks_total"));
    assert!(text.contains("service_up{service=\"audit-api\"} 1"));
}

#[tokio::test]
async fn an_append_with_a_service_token_is_created_and_echoes_the_correlation_id() {
    let harness = harness();
    let headers = vec![
        ("authorization", "Bearer test-service-token-identity"),
        ("idempotency-key", "key-1"),
        ("x-correlation-id", "corr-1"),
    ];
    let (status, response_headers, body) = send(
        &harness.state,
        Method::POST,
        "/v1/events",
        &headers,
        Some(append_body()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(response_headers.get("x-correlation-id").unwrap(), "corr-1");
    assert!(json_body(&body)["id"].is_string());
    assert_eq!(json_body(&body)["created"], true);
    assert_eq!(harness.events.stored().len(), 1);
}

#[tokio::test]
async fn an_append_without_a_service_token_or_with_a_jwt_is_unauthorized() {
    let harness = harness();
    let (status, _, _) = send(
        &harness.state,
        Method::POST,
        "/v1/events",
        &[("idempotency-key", "key-1")],
        Some(append_body()),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _, _) = send(
        &harness.state,
        Method::POST,
        "/v1/events",
        &[
            ("authorization", "Bearer admin-token"),
            ("idempotency-key", "key-1"),
        ],
        Some(append_body()),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "an admin JWT is not a service token"
    );
}

#[tokio::test]
async fn an_append_without_an_idempotency_key_is_refused() {
    let harness = harness();
    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        "/v1/events",
        &[("authorization", "Bearer test-service-token-identity")],
        Some(append_body()),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(json_body(&body)["errors"][0]["field"], "idempotencyKey");
    assert!(harness.events.stored().is_empty());
}

#[tokio::test]
async fn an_invalid_envelope_names_the_field_and_never_the_value() {
    let harness = harness();
    let address = "0x1111111111111111111111111111111111111111";
    let mut body = append_body();
    body["target"] = json!(address);
    let (status, _, response) = send(
        &harness.state,
        Method::POST,
        "/v1/events",
        &[
            ("authorization", "Bearer test-service-token-identity"),
            ("idempotency-key", "key-1"),
        ],
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let parsed = json_body(&response);
    assert_eq!(parsed["errors"][0]["field"], "target");
    assert!(!String::from_utf8_lossy(&response).contains(address));
}

#[tokio::test]
async fn an_emitter_cannot_append_a_system_event_over_http() {
    let harness = harness();
    let mut body = append_body();
    body["actorId"] = Value::Null;
    body["actorKind"] = json!("system");
    body["action"] = json!("system.retention.run");
    let (status, _, response) = send(
        &harness.state,
        Method::POST,
        "/v1/events",
        &[
            ("authorization", "Bearer test-service-token-identity"),
            ("idempotency-key", "key-1"),
        ],
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(json_body(&response)["errors"][0]["field"], "actorKind");
    assert!(harness.events.stored().is_empty());
}

#[tokio::test]
async fn a_retry_with_the_same_key_and_payload_returns_the_same_id() {
    let harness = harness();
    let headers = [
        ("authorization", "Bearer test-service-token-identity"),
        ("idempotency-key", "key-1"),
    ];
    let (status, _, first) = send(
        &harness.state,
        Method::POST,
        "/v1/events",
        &headers,
        Some(append_body()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, _, second) = send(
        &harness.state,
        Method::POST,
        "/v1/events",
        &headers,
        Some(append_body()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(json_body(&first)["id"], json_body(&second)["id"]);
    assert_eq!(json_body(&second)["created"], false);
    assert_eq!(harness.events.stored().len(), 1);
}

#[tokio::test]
async fn a_reused_key_with_a_new_payload_is_a_conflict() {
    let harness = harness();
    let headers = [
        ("authorization", "Bearer test-service-token-identity"),
        ("idempotency-key", "key-1"),
    ];
    let (status, _, _) = send(
        &harness.state,
        Method::POST,
        "/v1/events",
        &headers,
        Some(append_body()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let mut changed = append_body();
    changed["target"] = json!("asset-dai");
    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        "/v1/events",
        &headers,
        Some(changed),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        json_body(&body)["type"],
        "https://errors.cpb/audit/idempotency-conflict"
    );
}

fn recorded_range() -> String {
    "recordedFrom=2026-11-06T00:00:00Z&recordedTo=2026-11-07T00:00:00Z".to_string()
}

#[tokio::test]
async fn the_query_path_requires_admin() {
    let harness = harness();
    let uri = format!("/v1/events?{}", recorded_range());
    let (status, _, _) = send(
        &harness.state,
        Method::GET,
        &uri,
        &[("authorization", "Bearer support-token")],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        &uri,
        &[("authorization", "Bearer admin-token")],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let events = json_body(&body);
    let events = events["events"].as_array().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["action"], "audit.operator.denied");
    assert_eq!(events[0]["sensitivity"], "security");
}

#[tokio::test]
async fn a_query_without_the_recorded_range_is_refused() {
    let harness = harness();
    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        "/v1/events",
        &[("authorization", "Bearer admin-token")],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(json_body(&body)["errors"][0]["field"], "recordedFrom");
}

#[tokio::test]
async fn an_export_streams_json_lines_with_an_export_id() {
    let harness = harness();
    let (status, _, _) = send(
        &harness.state,
        Method::POST,
        "/v1/events",
        &[
            ("authorization", "Bearer test-service-token-identity"),
            ("idempotency-key", "key-1"),
        ],
        Some(append_body()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let request = json!({
        "recordedFrom": "2026-11-06T00:00:00Z",
        "recordedTo": "2026-11-07T00:00:00Z"
    });
    let (status, headers, body) = send(
        &harness.state,
        Method::POST,
        "/v1/exports",
        &[("authorization", "Bearer admin-token")],
        Some(request),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get(header::CONTENT_TYPE).unwrap(),
        "application/x-ndjson"
    );
    assert!(headers.get("x-export-id").is_some());
    let text = String::from_utf8(body).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 1);
    let first: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(first["action"], "asset.contract.added");
    assert_eq!(
        harness.exports.stored()[0].state(),
        audit_domain::ExportState::Success
    );
}

#[tokio::test]
async fn an_export_requires_admin() {
    let harness = harness();
    let request = json!({
        "recordedFrom": "2026-11-06T00:00:00Z",
        "recordedTo": "2026-11-07T00:00:00Z"
    });
    let (status, _, _) = send(
        &harness.state,
        Method::POST,
        "/v1/exports",
        &[("authorization", "Bearer support-token")],
        Some(request),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

fn session_body() -> Value {
    json!({
        "supportAgentId": SUPPORT_SUBJECT,
        "subjectUserId": SUBJECT_USER,
        "reason": "customer reported a billing error",
        "secondApproverId": SECOND_APPROVER,
        "scope": "account_metadata",
        "expiresAt": "2026-11-06T18:00:00Z"
    })
}

#[tokio::test]
async fn starting_a_session_requires_the_support_role() {
    let harness = harness();
    let (status, _, _) = send(
        &harness.state,
        Method::POST,
        "/v1/support-sessions",
        &[("authorization", "Bearer user-token")],
        Some(session_body()),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        "/v1/support-sessions",
        &[("authorization", "Bearer support-token")],
        Some(session_body()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(
        json_body(&body)["state"],
        "requested",
        "a second operator still has to approve"
    );
}

#[tokio::test]
async fn the_named_second_approver_activates_a_requested_session() {
    let harness = harness();
    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        "/v1/support-sessions",
        &[("authorization", "Bearer support-token")],
        Some(session_body()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let session_id = json_body(&body)["id"].as_str().unwrap().to_string();
    let uri = format!("/v1/support-sessions/{session_id}/approve");

    let (status, _, _) = send(
        &harness.state,
        Method::POST,
        &uri,
        &[("authorization", "Bearer support-token")],
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "the requesting agent cannot approve their own session"
    );

    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        &uri,
        &[("authorization", "Bearer approver-token")],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json_body(&body)["state"], "active");
    assert!(!json_body(&body)["approvedAt"].is_null());
}

#[tokio::test]
async fn a_denied_session_is_a_problem_document() {
    let harness = harness();
    let mut body = session_body();
    body["reason"] = json!("look");
    let (status, _, response) = send(
        &harness.state,
        Method::POST,
        "/v1/support-sessions",
        &[("authorization", "Bearer support-token")],
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(
        json_body(&response)["type"],
        "https://errors.cpb/audit/support-session-denied"
    );
}

#[tokio::test]
async fn the_session_check_is_service_authenticated_and_answers_allow() {
    let harness = harness();
    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        "/v1/support-sessions",
        &[("authorization", "Bearer support-token")],
        Some(session_body()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let session_id = json_body(&body)["id"].as_str().unwrap().to_string();
    let (status, _, _) = send(
        &harness.state,
        Method::POST,
        &format!("/v1/support-sessions/{session_id}/approve"),
        &[("authorization", "Bearer approver-token")],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let uri = format!("/v1/support-sessions/{session_id}/check?scope=account_metadata");
    let (status, _, _) = send(&harness.state, Method::GET, &uri, &[], None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        &uri,
        &[("authorization", "Bearer test-service-token-gateway")],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json_body(&body)["decision"], "allow");
}

#[tokio::test]
async fn an_unknown_session_checks_as_a_deny() {
    let harness = harness();
    let uri = format!(
        "/v1/support-sessions/{}/check?scope=account_metadata",
        uuid::Uuid::new_v4()
    );
    let (status, _, body) = send(
        &harness.state,
        Method::GET,
        &uri,
        &[("authorization", "Bearer test-service-token-gateway")],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json_body(&body)["decision"], "deny");
}

#[tokio::test]
async fn revoking_a_session_requires_admin() {
    let harness = harness();
    let (_, _, body) = send(
        &harness.state,
        Method::POST,
        "/v1/support-sessions",
        &[("authorization", "Bearer support-token")],
        Some(session_body()),
    )
    .await;
    let session_id = json_body(&body)["id"].as_str().unwrap().to_string();
    let (status, _, _) = send(
        &harness.state,
        Method::POST,
        &format!("/v1/support-sessions/{session_id}/approve"),
        &[("authorization", "Bearer approver-token")],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let uri = format!("/v1/support-sessions/{session_id}/revoke");

    let (status, _, _) = send(
        &harness.state,
        Method::POST,
        &uri,
        &[("authorization", "Bearer support-token")],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        &uri,
        &[("authorization", "Bearer admin-token")],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json_body(&body)["state"], "revoked");
}

#[tokio::test]
async fn a_privacy_deletion_requires_a_service_token() {
    let harness = harness();
    let body = json!({
        "deletionRequestId": DELETION_REQUEST,
        "subjectUserId": SUBJECT_USER
    });
    let (status, _, _) = send(
        &harness.state,
        Method::POST,
        "/v1/privacy/deletions",
        &[("authorization", "Bearer admin-token")],
        Some(body.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _, response) = send(
        &harness.state,
        Method::POST,
        "/v1/privacy/deletions",
        &[("authorization", "Bearer test-service-token-identity")],
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(json_body(&response)["state"], "requested");
}

#[tokio::test]
async fn only_identity_may_request_a_privacy_deletion() {
    let harness = harness();
    let body = json!({
        "deletionRequestId": DELETION_REQUEST,
        "subjectUserId": SUBJECT_USER
    });
    let (status, _, response) = send(
        &harness.state,
        Method::POST,
        "/v1/privacy/deletions",
        &[("authorization", "Bearer test-service-token-ledger")],
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(json_body(&response)["status"], 403);
    assert!(harness.privacy.stored().is_empty());
}

fn report_body() -> Value {
    json!({
        "category": "reported",
        "owner": "ledger",
        "cutoffAt": "2024-11-06T00:00:00Z",
        "startedAt": "2026-11-06T07:00:00Z",
        "finishedAt": "2026-11-06T08:00:00Z",
        "deletedCount": 120,
        "privilegedRole": "ledger_retention",
        "outcome": "success"
    })
}

#[tokio::test]
async fn a_retention_report_is_recorded_for_the_calling_service_only() {
    let harness = harness();
    let (status, _, body) = send(
        &harness.state,
        Method::POST,
        "/v1/runs",
        &[("authorization", "Bearer test-service-token-ledger")],
        Some(report_body()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(json_body(&body)["owner"], "ledger");

    let (status, _, _) = send(
        &harness.state,
        Method::POST,
        "/v1/runs",
        &[("authorization", "Bearer test-service-token-identity")],
        Some(report_body()),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn an_unlisted_route_is_not_served_and_security_headers_are_present() {
    let harness = harness();
    let (status, headers, body) = send(&harness.state, Method::GET, "/v1/admin", &[], None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(headers.get("x-frame-options").unwrap(), "DENY");
    assert_eq!(json_body(&body)["status"], 404);
}

#[tokio::test]
async fn an_invalid_correlation_id_is_refused_with_the_correlation_header() {
    let harness = harness();
    let (status, headers, _) = send(
        &harness.state,
        Method::GET,
        "/healthz",
        &[("x-correlation-id", "bad id with spaces")],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(headers.get("x-correlation-id").is_some());
}

#[tokio::test]
async fn the_append_rate_limit_is_enforced() {
    let harness = harness_with_limits(Limits {
        appends_per_minute: 1,
        queries_per_minute: 30,
        exports_per_hour: 5,
    });
    let headers = [
        ("authorization", "Bearer test-service-token-identity"),
        ("idempotency-key", "key-1"),
    ];
    let (status, _, _) = send(
        &harness.state,
        Method::POST,
        "/v1/events",
        &headers,
        Some(append_body()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, response_headers, _) = send(
        &harness.state,
        Method::POST,
        "/v1/events",
        &headers,
        Some(append_body()),
    )
    .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(response_headers.get(header::RETRY_AFTER).unwrap(), "60");
}
