//! Support-access HTTP boundary tests (FR16.3–FR16.6, BR3.9–BR3.11).
//!
//! Drives the real Identity router with `tower::ServiceExt::oneshot`. The four
//! support paths are exercised through their success shape and through every
//! refusal in the error catalogue that a caller can reach, and the metadata
//! projection is asserted field by field so a forgotten filter fails here rather
//! than in a support console.

mod common;

use axum::body::Body;
use axum::http::{header, HeaderMap, Method, Request, StatusCode};
use chrono::{DateTime, Duration};
use common::*;
use http_body_util::BodyExt;
use identity_application::ports::{
    AccountSummary, ExternalError, PortfolioSummary, RepositoryError, SupportGrantRepository,
};
use identity_domain::{SupportAccessGrant, SupportGrantId, UserId};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn send(
    state: &identity_http::state::AppState,
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
    let response = identity_http::router(state.clone())
        .oneshot(request)
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, headers, bytes.to_vec())
}

async fn send_auth(
    state: &identity_http::state::AppState,
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

async fn principal(state: &identity_http::state::AppState, token: &str, subject: &str) -> Value {
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

/// Seeds the target, the support agent, and the admin, and returns the target id.
async fn world(harness: &Harness) -> String {
    let target = principal(&harness.state, "user-token", USER_SUBJECT).await;
    principal(&harness.state, "support-token", SUPPORT_SUBJECT).await;
    principal(&harness.state, "admin-token", ADMIN_SUBJECT).await;
    target["userId"].as_str().expect("a user id").to_string()
}

async fn request_grant(harness: &Harness, target_id: &str) -> String {
    let (status, _, body) = send_auth(
        &harness.state,
        Method::POST,
        "/v1/support-grants",
        Some("support-token"),
        Some(json!({ "userId": target_id, "reason": "investigate a sync failure" })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "grant creation: {}",
        String::from_utf8_lossy(&body)
    );
    json_body(&body)["grantId"]
        .as_str()
        .expect("a grant id")
        .to_string()
}

async fn approve_grant(harness: &Harness, grant_id: &str) {
    let (status, _, body) = send_auth(
        &harness.state,
        Method::POST,
        &format!("/v1/support-grants/{grant_id}/approve"),
        Some("admin-token"),
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "grant approval: {}",
        String::from_utf8_lossy(&body)
    );
}

#[tokio::test]
async fn a_support_agent_requests_a_grant_and_it_starts_pending() {
    let harness = harness();
    let target_id = world(&harness).await;

    let (status, _, body) = send_auth(
        &harness.state,
        Method::POST,
        "/v1/support-grants",
        Some("support-token"),
        Some(json!({ "userId": target_id, "reason": "investigate a sync failure" })),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    let body = json_body(&body);
    assert_eq!(body["status"], "pending_approval");
    assert_eq!(body["userId"], target_id);
    assert_eq!(body["reason"], "investigate a sync failure");
    assert!(
        SupportGrantId::parse(body["grantId"].as_str().unwrap()).is_ok(),
        "the response carries the grant id"
    );
    assert!(
        body.get("expiresAt").is_none(),
        "a pending grant carries no window"
    );
}

#[tokio::test]
async fn a_request_by_a_non_support_caller_is_refused_with_its_code() {
    let harness = harness();
    let target_id = world(&harness).await;

    let (status, headers, body) = send_auth(
        &harness.state,
        Method::POST,
        "/v1/support-grants",
        Some("user-token"),
        Some(json!({ "userId": target_id, "reason": "curious" })),
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(
        headers.get(header::CONTENT_TYPE).unwrap(),
        "application/problem+json"
    );
    let body = json_body(&body);
    assert_eq!(body["status"], 403);
    assert_eq!(body["detail"], "support-role-required");
    assert!(body["correlationId"]
        .as_str()
        .is_some_and(|id| !id.is_empty()));
}

#[tokio::test]
async fn a_request_for_an_unknown_user_is_not_found() {
    let harness = harness();
    world(&harness).await;
    let unknown = identity_domain::UserId::generate().to_string();

    let (status, _, body) = send_auth(
        &harness.state,
        Method::POST,
        "/v1/support-grants",
        Some("support-token"),
        Some(json!({ "userId": unknown, "reason": "investigate" })),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(json_body(&body)["status"], 404);
}

#[tokio::test]
async fn a_malformed_support_id_is_a_bad_request_with_its_catalogue_code() {
    let harness = harness();
    let target_id = world(&harness).await;

    let (status, _, body) = send_auth(
        &harness.state,
        Method::POST,
        "/v1/support-grants",
        Some("support-token"),
        Some(json!({ "userId": "not-a-uuid", "reason": "investigate" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body = json_body(&body);
    assert_eq!(body["type"], "https://errors.cpb/identity/bad-request");
    assert_eq!(body["detail"], "invalid-user-id");

    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        "/v1/support-grants/not-a-uuid/metadata",
        Some("support-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body = json_body(&body);
    assert_eq!(body["type"], "https://errors.cpb/identity/bad-request");
    assert_eq!(body["detail"], "invalid-grant-id");

    // Keep the target id used so the world is exercised once.
    assert!(!target_id.is_empty());
}

#[tokio::test]
async fn an_admin_approves_and_the_window_is_sixty_minutes_from_approval() {
    let harness = harness();
    let target_id = world(&harness).await;
    let grant_id = request_grant(&harness, &target_id).await;

    let (status, _, body) = send_auth(
        &harness.state,
        Method::POST,
        &format!("/v1/support-grants/{grant_id}/approve"),
        Some("admin-token"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let body = json_body(&body);
    assert_eq!(body["status"], "active");
    assert!(body["approvedBy"].as_str().is_some());
    let expires_at = DateTime::parse_from_rfc3339(body["expiresAt"].as_str().unwrap())
        .expect("an RFC 3339 instant")
        .with_timezone(&chrono::Utc);
    assert_eq!(expires_at, fixed_now() + Duration::minutes(60));
}

#[tokio::test]
async fn an_approval_by_a_non_admin_is_refused_with_admin_role_required() {
    let harness = harness();
    let target_id = world(&harness).await;
    let grant_id = request_grant(&harness, &target_id).await;

    let (status, _, body) = send_auth(
        &harness.state,
        Method::POST,
        &format!("/v1/support-grants/{grant_id}/approve"),
        Some("support-token"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(json_body(&body)["detail"], "admin-role-required");
}

#[tokio::test]
async fn an_admin_approving_an_unknown_grant_gets_not_found() {
    let harness = harness();
    world(&harness).await;
    let unknown = SupportGrantId::generate().to_string();

    let (status, _, body) = send_auth(
        &harness.state,
        Method::POST,
        &format!("/v1/support-grants/{unknown}/approve"),
        Some("admin-token"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(json_body(&body)["status"], 404);
}

#[tokio::test]
async fn approving_a_stale_request_is_a_conflict_with_request_expired() {
    let harness = harness();
    let target_id = world(&harness).await;
    let grant_id = request_grant(&harness, &target_id).await;
    harness.clock.set(fixed_now() + Duration::minutes(61));

    let (status, _, body) = send_auth(
        &harness.state,
        Method::POST,
        &format!("/v1/support-grants/{grant_id}/approve"),
        Some("admin-token"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(json_body(&body)["detail"], "request-expired");
}

#[tokio::test]
async fn a_second_live_grant_for_the_pair_is_a_conflict() {
    let harness = harness();
    let target_id = world(&harness).await;
    let grant_id = request_grant(&harness, &target_id).await;
    approve_grant(&harness, &grant_id).await;

    let (status, _, body) = send_auth(
        &harness.state,
        Method::POST,
        "/v1/support-grants",
        Some("support-token"),
        Some(json!({ "userId": target_id, "reason": "another look" })),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
    let body = json_body(&body);
    assert_eq!(body["detail"], "grant-already-live");
    assert!(body["errors"].is_null());
}

#[tokio::test]
async fn an_admin_revokes_and_a_second_revoke_is_a_conflict() {
    let harness = harness();
    let target_id = world(&harness).await;
    let grant_id = request_grant(&harness, &target_id).await;
    approve_grant(&harness, &grant_id).await;

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

    let (status, _, body) = send_auth(
        &harness.state,
        Method::DELETE,
        &format!("/v1/support-grants/{grant_id}"),
        Some("admin-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(json_body(&body)["detail"], "grant-not-pending");
}

#[tokio::test]
async fn a_revocation_by_a_non_admin_is_refused_with_admin_role_required() {
    let harness = harness();
    let target_id = world(&harness).await;
    let grant_id = request_grant(&harness, &target_id).await;

    let (status, _, body) = send_auth(
        &harness.state,
        Method::DELETE,
        &format!("/v1/support-grants/{grant_id}"),
        Some("support-token"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(json_body(&body)["detail"], "admin-role-required");
}

#[tokio::test]
async fn the_metadata_projection_carries_exactly_the_allowlisted_fields() {
    let harness = harness();
    let target_id = world(&harness).await;
    let grant_id = request_grant(&harness, &target_id).await;
    approve_grant(&harness, &grant_id).await;

    let account_id = uuid::Uuid::new_v4();
    harness
        .support_accounts
        .accounts
        .lock()
        .unwrap()
        .push(AccountSummary {
            id: account_id,
            label: "Binance main".to_string(),
            source_type: "exchange".to_string(),
            sync_status: "active".to_string(),
            last_synced_at: Some(fixed_now()),
        });
    let portfolio_id = uuid::Uuid::new_v4();
    harness
        .support_portfolios
        .portfolios
        .lock()
        .unwrap()
        .push(PortfolioSummary {
            id: portfolio_id,
            label: "Core".to_string(),
        });

    let (status, headers, body) = send_auth(
        &harness.state,
        Method::GET,
        &format!("/v1/support-grants/{grant_id}/metadata"),
        Some("support-token"),
        None,
    )
    .await;

    assert_eq!(
        status,
        StatusCode::OK,
        "metadata read: {}",
        String::from_utf8_lossy(&body)
    );
    assert_eq!(
        headers.get(header::CONTENT_TYPE).unwrap(),
        "application/json"
    );
    let body = json_body(&body);
    assert_eq!(
        sorted_keys(&body),
        vec![
            "accounts".to_string(),
            "displayName".to_string(),
            "emailVerified".to_string(),
            "portfolios".to_string(),
            "userId".to_string(),
        ]
    );
    assert_eq!(body["userId"], target_id);
    assert_eq!(body["displayName"], "Test User");
    assert_eq!(body["emailVerified"], true);

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
        ]
    );
    assert_eq!(accounts[0]["id"], account_id.to_string());
    assert_eq!(accounts[0]["label"], "Binance main");
    assert_eq!(accounts[0]["sourceType"], "exchange");
    assert_eq!(accounts[0]["syncStatus"], "active");

    let portfolios = body["portfolios"].as_array().expect("an array");
    assert_eq!(portfolios.len(), 1);
    assert_eq!(
        sorted_keys(&portfolios[0]),
        vec!["id".to_string(), "label".to_string()]
    );
    assert_eq!(portfolios[0]["id"], portfolio_id.to_string());

    assert_eq!(
        harness.support_observation.served.lock().unwrap().len(),
        1,
        "one served read is audited"
    );
}

#[tokio::test]
async fn an_unknown_or_foreign_grant_is_refused_uniformly() {
    let harness = harness();
    world(&harness).await;
    let unknown = SupportGrantId::generate().to_string();

    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        &format!("/v1/support-grants/{unknown}/metadata"),
        Some("support-token"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(json_body(&body)["detail"], "support-access-refused");
    let denied = harness.support_observation.denied.lock().unwrap();
    assert_eq!(denied.len(), 1);
    assert_eq!(denied[0].2, "support-access-refused");
}

#[tokio::test]
async fn a_metadata_read_by_a_non_support_caller_is_refused_with_its_code() {
    let harness = harness();
    let target_id = world(&harness).await;
    let grant_id = request_grant(&harness, &target_id).await;

    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        &format!("/v1/support-grants/{grant_id}/metadata"),
        Some("user-token"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(json_body(&body)["detail"], "support-role-required");
}

#[tokio::test]
async fn the_holders_own_inactive_grant_gets_the_precise_code() {
    let harness = harness();
    let target_id = world(&harness).await;
    let grant_id = request_grant(&harness, &target_id).await;

    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        &format!("/v1/support-grants/{grant_id}/metadata"),
        Some("support-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(
        json_body(&body)["detail"],
        "grant-pending",
        "a pending grant is precisely named for its holder"
    );

    approve_grant(&harness, &grant_id).await;
    let (status, _, _) = send_auth(
        &harness.state,
        Method::DELETE,
        &format!("/v1/support-grants/{grant_id}"),
        Some("admin-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        &format!("/v1/support-grants/{grant_id}/metadata"),
        Some("support-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(json_body(&body)["detail"], "grant-revoked");
}

#[tokio::test]
async fn an_expired_grant_is_refused_and_its_expiry_is_persisted() {
    let harness = harness();
    let target_id = world(&harness).await;
    let grant_id = request_grant(&harness, &target_id).await;
    approve_grant(&harness, &grant_id).await;
    harness.clock.set(fixed_now() + Duration::minutes(61));

    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        &format!("/v1/support-grants/{grant_id}/metadata"),
        Some("support-token"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(json_body(&body)["detail"], "grant-expired");
    let stored = harness
        .grants
        .find_by_id(SupportGrantId::parse(&grant_id).unwrap())
        .await
        .unwrap()
        .expect("the grant is stored");
    assert_eq!(stored.status().as_str(), "expired");
}

#[tokio::test]
async fn an_unreachable_metadata_source_is_unavailable_and_audited_as_failed() {
    let harness = harness();
    let target_id = world(&harness).await;
    let grant_id = request_grant(&harness, &target_id).await;
    approve_grant(&harness, &grant_id).await;
    harness
        .support_accounts
        .failure
        .lock()
        .unwrap()
        .replace(ExternalError::Unavailable("account is down".to_string()));

    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        &format!("/v1/support-grants/{grant_id}/metadata"),
        Some("support-token"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    let body = json_body(&body);
    assert_eq!(body["type"], "https://errors.cpb/identity/unavailable");
    assert_eq!(body["detail"], "metadata-source-unavailable");
    let failed = harness.support_observation.failed.lock().unwrap();
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].2, "metadata-source-unavailable");
}

#[tokio::test]
async fn an_unrecordable_served_read_is_unavailable_rather_than_served() {
    let harness = harness();
    let target_id = world(&harness).await;
    let grant_id = request_grant(&harness, &target_id).await;
    approve_grant(&harness, &grant_id).await;
    harness
        .support_observation
        .failure
        .lock()
        .unwrap()
        .replace(RepositoryError::Unavailable("audit is down".to_string()));

    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        &format!("/v1/support-grants/{grant_id}/metadata"),
        Some("support-token"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(json_body(&body)["detail"], "audit-unavailable");
}

#[tokio::test]
async fn an_unrecordable_refusal_is_replaced_by_audit_unavailable() {
    let harness = harness();
    world(&harness).await;
    let unknown = SupportGrantId::generate().to_string();
    harness
        .support_observation
        .failure
        .lock()
        .unwrap()
        .replace(RepositoryError::Unavailable("audit is down".to_string()));

    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        &format!("/v1/support-grants/{unknown}/metadata"),
        Some("support-token"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(json_body(&body)["detail"], "audit-unavailable");
}

#[tokio::test]
async fn a_backend_service_token_is_not_a_user_or_support_token() {
    let harness = harness();
    world(&harness).await;
    let unknown = SupportGrantId::generate().to_string();

    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        &format!("/v1/support-grants/{unknown}/metadata"),
        Some("notification-token"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(json_body(&body)["status"], 401);
}

#[tokio::test]
async fn the_metrics_endpoint_renders_every_support_series_zero_initialised() {
    let harness = harness();

    let (status, _, body) = send_auth(&harness.state, Method::GET, "/metrics", None, None).await;
    assert_eq!(status, StatusCode::OK);
    let text = String::from_utf8_lossy(&body);

    for line in [
        "support_grants_total{operation=\"request\",outcome=\"accepted\"} 0",
        "support_reads_total{outcome=\"served\"} 0",
        "support_read_duration_seconds_bucket{le=\"+Inf\"} 0",
        "support_read_duration_seconds_count 0",
        "support_write_duration_seconds_bucket{le=\"+Inf\"} 0",
        "support_write_duration_seconds_sum 0",
        "support_write_duration_seconds_count 0",
        "support_metadata_source_errors_total{source=\"account\",cause=\"unreachable\"} 0",
        "support_audit_write_failures_total{cause=\"read\"} 0",
        "support_grant_decision_errors_total{cause=\"lookup\"} 0",
        "support_outbox_deliveries_total{outcome=\"delivered\"} 0",
        "support_outbox_pending 0",
        "support_outbox_oldest_age_seconds 0",
        "support_outbox_last_pass_timestamp_seconds 0",
    ] {
        assert!(
            text.contains(line),
            "the metrics body is missing `{line}`:\n{text}"
        );
    }
}

#[tokio::test]
async fn a_refused_support_write_moves_its_counter_and_latency_histogram() {
    let harness = harness();
    let target_id = world(&harness).await;

    let (status, _, _) = send_auth(
        &harness.state,
        Method::POST,
        "/v1/support-grants",
        Some("user-token"),
        Some(json!({ "userId": target_id, "reason": "curious" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _, body) = send_auth(&harness.state, Method::GET, "/metrics", None, None).await;
    assert_eq!(status, StatusCode::OK);
    let text = String::from_utf8_lossy(&body);

    assert!(
        text.contains("support_grants_total{operation=\"request\",outcome=\"refused\"} 1"),
        "the refused write is counted:\n{text}"
    );
    assert!(
        text.contains("support_grants_total{operation=\"request\",outcome=\"accepted\"} 0"),
        "the accepted series stays at zero:\n{text}"
    );
    assert!(
        text.contains("support_write_duration_seconds_count 1"),
        "the refused write is timed:\n{text}"
    );
    assert!(
        text.contains("support_write_duration_seconds_bucket{le=\"+Inf\"} 1"),
        "the observation lands in the histogram:\n{text}"
    );
}

/// Resolves one principal through the real router and returns its user id.
async fn user_id_of(harness: &Harness, token: &str, subject: &str) -> String {
    let view = principal(&harness.state, token, subject).await;
    view["userId"].as_str().expect("a user id").to_string()
}

#[tokio::test]
async fn a_support_agent_reads_their_own_grants_as_a_bare_array() {
    let harness = harness();
    let target_id = world(&harness).await;
    let support_id = user_id_of(&harness, "support-token", SUPPORT_SUBJECT).await;
    let grant_id = request_grant(&harness, &target_id).await;

    let (status, headers, body) = send_auth(
        &harness.state,
        Method::GET,
        "/v1/support-grants/mine",
        Some("support-token"),
        None,
    )
    .await;

    assert_eq!(
        status,
        StatusCode::OK,
        "own-grant read: {}",
        String::from_utf8_lossy(&body)
    );
    assert_eq!(
        headers.get(header::CONTENT_TYPE).unwrap(),
        "application/json"
    );
    let body = json_body(&body);
    let rows = body.as_array().expect("a bare array, not an envelope");
    assert_eq!(rows.len(), 1);
    assert_eq!(
        sorted_keys(&rows[0]),
        vec![
            "id".to_string(),
            "reason".to_string(),
            "requestedAt".to_string(),
            "requestedBy".to_string(),
            "status".to_string(),
            "userId".to_string(),
        ]
    );
    assert_eq!(rows[0]["id"], grant_id);
    assert_eq!(rows[0]["userId"], target_id);
    assert_eq!(
        rows[0]["requestedBy"], support_id,
        "the row names the requesting agent"
    );
    assert_eq!(rows[0]["status"], "pending_approval");
    assert_eq!(
        harness.grants.mine_calls.lock().unwrap().as_slice(),
        [(
            UserId::new(uuid::Uuid::parse_str(&support_id).expect("a uuid")),
            20
        )],
        "one self-scoped read, bounded at twenty"
    );
}

#[tokio::test]
async fn the_mine_read_is_bounded_at_twenty_rows_newest_first() {
    let harness = harness();
    world(&harness).await;
    let support_id = user_id_of(&harness, "support-token", SUPPORT_SUBJECT).await;
    let agent = UserId::new(uuid::Uuid::parse_str(&support_id).expect("a uuid"));

    let seeded: Vec<SupportAccessGrant> = (0..21)
        .map(|minute| {
            SupportAccessGrant::request(
                SupportGrantId::generate(),
                UserId::generate(),
                agent,
                "investigate a sync failure",
                fixed_now() - Duration::minutes(minute),
            )
            .expect("a valid request")
        })
        .collect();
    harness
        .grants
        .grants
        .lock()
        .unwrap()
        .extend(seeded.iter().cloned());

    let (status, _, body) = send_auth(
        &harness.state,
        Method::GET,
        "/v1/support-grants/mine",
        Some("support-token"),
        None,
    )
    .await;

    assert_eq!(
        status,
        StatusCode::OK,
        "own-grant read: {}",
        String::from_utf8_lossy(&body)
    );
    let body = json_body(&body);
    let rows = body.as_array().expect("a bare array");
    assert_eq!(rows.len(), 20, "the twenty-row bound holds");
    assert_eq!(rows[0]["id"], seeded[0].id().to_string(), "newest first");
    assert_eq!(rows[19]["id"], seeded[19].id().to_string());
    assert_ne!(
        rows[19]["id"],
        seeded[20].id().to_string(),
        "the twenty-first row is dropped"
    );
}

#[tokio::test]
async fn a_signed_in_caller_without_the_support_role_is_refused_on_the_mine_read() {
    let harness = harness();
    world(&harness).await;

    let (status, headers, body) = send_auth(
        &harness.state,
        Method::GET,
        "/v1/support-grants/mine",
        Some("user-token"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(
        headers.get(header::CONTENT_TYPE).unwrap(),
        "application/problem+json"
    );
    let body = json_body(&body);
    assert_eq!(body["status"], 403);
    assert_eq!(body["detail"], "support-role-required");
    assert!(
        harness.grants.mine_calls.lock().unwrap().is_empty(),
        "a refused caller reads nothing"
    );
}

#[tokio::test]
async fn a_mine_read_without_a_token_is_unauthorized() {
    let harness = harness();
    world(&harness).await;

    let (status, headers, body) = send_auth(
        &harness.state,
        Method::GET,
        "/v1/support-grants/mine",
        None,
        None,
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        headers.get(header::CONTENT_TYPE).unwrap(),
        "application/problem+json"
    );
    assert_eq!(json_body(&body)["status"], 401);
    assert!(
        harness.grants.mine_calls.lock().unwrap().is_empty(),
        "an unauthenticated caller reads nothing"
    );
}

#[tokio::test]
async fn the_mine_route_is_counted_under_its_own_route_label() {
    let harness = harness();
    world(&harness).await;

    let (status, _, _) = send_auth(
        &harness.state,
        Method::GET,
        "/v1/support-grants/mine",
        Some("support-token"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _, body) = send_auth(&harness.state, Method::GET, "/metrics", None, None).await;
    assert_eq!(status, StatusCode::OK);
    let text = String::from_utf8_lossy(&body);

    assert!(
        text.contains(
            "http_requests_total{service=\"identity-api\",status_class=\"2xx\",route=\"support.grants.mine\"} 1"
        ),
        "the own-grant read has its own route label:\n{text}"
    );
    assert!(
        !text.contains("status_class=\"4xx\",route=\"support.grants.revoke\""),
        "the own-grant read is not counted as a revoke:\n{text}"
    );
}
