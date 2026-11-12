//! Gateway edge tests: proxying, RFC 7807 problems, correlation ids, security
//! headers, rate limiting, and metrics.

mod common;

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, HeaderMap, Method as AxumMethod, Request, StatusCode};
use chrono::{TimeZone, Utc};
use common::fakes::{FakeAudit, FakeUpstream, FakeValidator, FixedClock, Ready};
use gateway_application::authorize::AuthorizeAndRoute;
use gateway_application::ports::{Clock, Principal, UpstreamError};
use gateway_domain::{Method, RouteTable};
use gateway_http::metrics::Metrics;
use gateway_http::rate_limit::RateLimiter;
use gateway_http::routes::router;
use gateway_http::state::AppState;
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

fn state(
    validator: FakeValidator,
    upstream: Arc<FakeUpstream>,
    audit: Arc<FakeAudit>,
    limit: u32,
) -> AppState {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock::at(
        Utc.with_ymd_and_hms(2026, 11, 6, 12, 0, 0).unwrap(),
    ));
    let use_case = Arc::new(AuthorizeAndRoute::new(
        Arc::new(validator),
        upstream,
        audit,
        clock,
        RouteTable::default_registry_routes(),
    ));
    AppState {
        service_name: "gateway",
        use_case,
        readiness: Arc::new(Ready(true)),
        metrics: Arc::new(Metrics::default()),
        rate_limiter: Arc::new(RateLimiter::new(limit, limit)),
    }
}

async fn send(
    state: &AppState,
    method: AxumMethod,
    uri: &str,
    token: Option<&str>,
    correlation: Option<&str>,
    body: Option<&str>,
) -> (StatusCode, HeaderMap, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    if let Some(correlation) = correlation {
        builder = builder.header("x-correlation-id", correlation);
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
    let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, headers, value)
}

#[tokio::test]
async fn a_declared_route_is_proxied_and_the_response_is_returned() {
    let upstream = Arc::new(FakeUpstream::ok());
    let state = state(
        FakeValidator::user(),
        upstream.clone(),
        Arc::new(FakeAudit::default()),
        100,
    );

    let (status, headers, body) = send(
        &state,
        AxumMethod::GET,
        "/v1/assets/resolve?chain=ethereum",
        Some("valid-user-token"),
        Some("corr-proxy"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);
    assert_eq!(headers.get("x-correlation-id").unwrap(), "corr-proxy");
    assert_eq!(upstream.calls(), 1);
}

#[tokio::test]
async fn a_u4_me_route_is_proxied_to_identity() {
    let upstream = Arc::new(FakeUpstream::ok());
    let state = state(
        FakeValidator::user(),
        upstream.clone(),
        Arc::new(FakeAudit::default()),
        100,
    );

    let (status, _, body) = send(
        &state,
        AxumMethod::PUT,
        "/v1/me/preferences",
        Some("valid-user-token"),
        Some("corr-u4"),
        Some("{\"locale\":\"vi\"}"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);
    assert_eq!(upstream.calls(), 1);
}

#[tokio::test]
async fn every_settings_route_is_proxied_to_identity_instead_of_404() {
    let upstream = Arc::new(FakeUpstream::ok());
    let state = state(
        FakeValidator::user(),
        upstream.clone(),
        Arc::new(FakeAudit::default()),
        1_000,
    );
    let export_id = "11111111-1111-4111-8111-111111111111";
    let cases = [
        (AxumMethod::GET, "/v1/me/account-status".to_string(), None),
        (AxumMethod::GET, "/v1/me/profile".to_string(), None),
        (
            AxumMethod::PATCH,
            "/v1/me/profile".to_string(),
            Some("{\"displayName\":\"Mai Anh\"}"),
        ),
        (AxumMethod::GET, "/v1/me/deletion-request".to_string(), None),
        (
            AxumMethod::DELETE,
            "/v1/me/deletion-request".to_string(),
            None,
        ),
        (
            AxumMethod::POST,
            "/v1/privacy/deletions".to_string(),
            Some("{\"reason\":\"leaving\"}"),
        ),
        (AxumMethod::POST, "/v1/exports".to_string(), Some("{}")),
        (AxumMethod::GET, format!("/v1/exports/{export_id}"), None),
    ];

    for (method, uri, body) in cases {
        let (status, _, _) = send(
            &state,
            method.clone(),
            &uri,
            Some("valid-user-token"),
            None,
            body,
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "{method:?} {uri} resolves to Identity instead of 404"
        );
    }
    assert_eq!(
        upstream.calls(),
        8,
        "every settings path reaches the Identity upstream"
    );
}

#[tokio::test]
async fn the_sixth_deletion_request_in_a_minute_is_rate_limited() {
    let upstream = Arc::new(FakeUpstream::ok());
    let state = state(
        FakeValidator::user(),
        upstream.clone(),
        Arc::new(FakeAudit::default()),
        1_000,
    );

    for attempt in 1..=5 {
        let (status, _, _) = send(
            &state,
            AxumMethod::POST,
            "/v1/privacy/deletions",
            Some("valid-user-token"),
            None,
            Some("{\"reason\":\"leaving\"}"),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "deletion request {attempt} is inside the 5/min budget"
        );
    }

    let (status, headers, body) = send(
        &state,
        AxumMethod::POST,
        "/v1/privacy/deletions",
        Some("valid-user-token"),
        None,
        Some("{\"reason\":\"leaving\"}"),
    )
    .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(body["status"], 429);
    assert!(
        headers.contains_key("retry-after"),
        "a 429 carries Retry-After"
    );
    assert_eq!(
        upstream.calls(),
        5,
        "the sixth request never reaches Identity"
    );
}

#[tokio::test]
async fn a_missing_token_is_a_401_problem() {
    let state = state(
        FakeValidator::user(),
        Arc::new(FakeUpstream::ok()),
        Arc::new(FakeAudit::default()),
        100,
    );

    let (status, headers, body) = send(
        &state,
        AxumMethod::GET,
        "/v1/assets/resolve",
        None,
        None,
        None,
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        headers.get(header::CONTENT_TYPE).unwrap(),
        "application/problem+json"
    );
    assert_eq!(body["status"], 401);
    assert!(body["correlationId"].as_str().unwrap().len() > 8);
}

#[tokio::test]
async fn an_undeclared_route_is_a_404_problem() {
    let upstream = Arc::new(FakeUpstream::ok());
    let state = state(
        FakeValidator::user(),
        upstream.clone(),
        Arc::new(FakeAudit::default()),
        100,
    );

    let (status, _, body) = send(
        &state,
        AxumMethod::GET,
        "/v1/secret/export",
        Some("valid-user-token"),
        Some("corr-404"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["status"], 404);
    assert_eq!(body["correlationId"], "corr-404");
    assert_eq!(upstream.calls(), 0);
}

#[tokio::test]
async fn a_user_cannot_reach_an_admin_route_but_an_admin_can() {
    let user_state = state(
        FakeValidator::user(),
        Arc::new(FakeUpstream::ok()),
        Arc::new(FakeAudit::default()),
        100,
    );
    let (status, _, body) = send(
        &user_state,
        AxumMethod::POST,
        "/v1/admin/assets",
        Some("valid-user-token"),
        None,
        Some("{}"),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["status"], 403);

    let admin_upstream = Arc::new(FakeUpstream::ok());
    let admin_state = state(
        FakeValidator::admin(),
        admin_upstream.clone(),
        Arc::new(FakeAudit::default()),
        100,
    );
    let (status, _, _) = send(
        &admin_state,
        AxumMethod::POST,
        "/v1/admin/assets",
        Some("valid-admin-token"),
        None,
        Some("{}"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(admin_upstream.calls(), 1);
}

#[tokio::test]
async fn the_admin_role_change_is_reachable_through_the_gateway() {
    let user_state = state(
        FakeValidator::user(),
        Arc::new(FakeUpstream::ok()),
        Arc::new(FakeAudit::default()),
        100,
    );
    let (status, _, _) = send(
        &user_state,
        AxumMethod::PUT,
        "/v1/admin/users/11111111-1111-4111-8111-111111111111/role",
        Some("valid-user-token"),
        None,
        Some(r#"{"newRole":"admin","reason":"promotion"}"#),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let admin_upstream = Arc::new(FakeUpstream::ok());
    let admin_state = state(
        FakeValidator::admin(),
        admin_upstream.clone(),
        Arc::new(FakeAudit::default()),
        100,
    );
    let (status, _, _) = send(
        &admin_state,
        AxumMethod::PUT,
        "/v1/admin/users/11111111-1111-4111-8111-111111111111/role",
        Some("valid-admin-token"),
        None,
        Some(r#"{"newRole":"admin","reason":"promotion"}"#),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(admin_upstream.calls(), 1);
}

#[tokio::test]
async fn the_admin_user_lookup_is_a_body_carrying_post_through_the_gateway() {
    let user_state = state(
        FakeValidator::user(),
        Arc::new(FakeUpstream::ok()),
        Arc::new(FakeAudit::default()),
        100,
    );
    let (status, _, _) = send(
        &user_state,
        AxumMethod::POST,
        "/v1/admin/users/lookup",
        Some("valid-user-token"),
        None,
        Some(r#"{"email":"ada@example.com"}"#),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let admin_upstream = Arc::new(FakeUpstream::ok());
    let admin_state = state(
        FakeValidator::admin(),
        admin_upstream.clone(),
        Arc::new(FakeAudit::default()),
        100,
    );
    let (status, _, _) = send(
        &admin_state,
        AxumMethod::POST,
        "/v1/admin/users/lookup",
        Some("valid-admin-token"),
        None,
        Some(r#"{"email":"ada@example.com"}"#),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(admin_upstream.calls(), 1);
}

#[tokio::test]
async fn the_admin_disable_and_restore_are_reachable_through_the_gateway() {
    let user_state = state(
        FakeValidator::user(),
        Arc::new(FakeUpstream::ok()),
        Arc::new(FakeAudit::default()),
        100,
    );
    for action in ["disable", "restore"] {
        let (status, _, _) = send(
            &user_state,
            AxumMethod::POST,
            &format!("/v1/admin/users/11111111-1111-4111-8111-111111111111/{action}"),
            Some("valid-user-token"),
            None,
            Some(r#"{"reason":"policy"}"#),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{action} is admin-only");
    }

    let admin_upstream = Arc::new(FakeUpstream::ok());
    let admin_state = state(
        FakeValidator::admin(),
        admin_upstream.clone(),
        Arc::new(FakeAudit::default()),
        100,
    );
    for action in ["disable", "restore"] {
        let (status, _, _) = send(
            &admin_state,
            AxumMethod::POST,
            &format!("/v1/admin/users/11111111-1111-4111-8111-111111111111/{action}"),
            Some("valid-admin-token"),
            None,
            Some(r#"{"reason":"policy"}"#),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{action} is proxied");
    }
    assert_eq!(admin_upstream.calls(), 2);
}

#[tokio::test]
async fn the_pending_grant_list_is_reachable_through_the_gateway() {
    let user_state = state(
        FakeValidator::user(),
        Arc::new(FakeUpstream::ok()),
        Arc::new(FakeAudit::default()),
        100,
    );
    let (status, _, _) = send(
        &user_state,
        AxumMethod::GET,
        "/v1/support-grants?status=pending_approval",
        Some("valid-user-token"),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let admin_upstream = Arc::new(FakeUpstream::ok());
    let admin_state = state(
        FakeValidator::admin(),
        admin_upstream.clone(),
        Arc::new(FakeAudit::default()),
        100,
    );
    let (status, _, _) = send(
        &admin_state,
        AxumMethod::GET,
        "/v1/support-grants?status=pending_approval",
        Some("valid-admin-token"),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(admin_upstream.calls(), 1);
}

#[tokio::test]
async fn the_mine_route_is_proxied_to_identity_with_the_jwt_unchanged() {
    let upstream = Arc::new(FakeUpstream::ok());
    let state = state(
        FakeValidator::with_token(
            "valid-support-token",
            Principal {
                subject: "support-1".to_string(),
                role: gateway_domain::Role::Support,
            },
        ),
        upstream.clone(),
        Arc::new(FakeAudit::default()),
        100,
    );

    let (status, _, body) = send(
        &state,
        AxumMethod::GET,
        "/v1/support-grants/mine",
        Some("valid-support-token"),
        Some("corr-mine"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);
    assert_eq!(upstream.calls(), 1, "the mine path reaches Identity");
    let forwarded = upstream.last_request();
    assert_eq!(forwarded.service.as_str(), "identity");
    assert_eq!(forwarded.path_and_query, "/v1/support-grants/mine");
    assert_eq!(
        forwarded.header("authorization"),
        Some("Bearer valid-support-token"),
        "the user JWT is forwarded unchanged"
    );

    // A sibling GET the table does not declare stays denied.
    let (status, _, _) = send(
        &state,
        AxumMethod::GET,
        "/v1/support-grants/aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
        Some("valid-support-token"),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        upstream.calls(),
        1,
        "the undeclared sibling never reaches Identity"
    );
}

#[tokio::test]
async fn the_21st_role_change_in_a_minute_is_rate_limited() {
    // The global per-IP budget is high, so only the route's own 20/min budget
    // can produce the 429.
    let upstream = Arc::new(FakeUpstream::ok());
    let state = state(
        FakeValidator::admin(),
        upstream.clone(),
        Arc::new(FakeAudit::default()),
        1_000,
    );

    for attempt in 1..=20 {
        let (status, _, _) = send(
            &state,
            AxumMethod::PUT,
            "/v1/admin/users/11111111-1111-4111-8111-111111111111/role",
            Some("valid-admin-token"),
            None,
            Some(r#"{"newRole":"support","reason":"rotation"}"#),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "role change {attempt} is inside the 20/min budget"
        );
    }

    let (status, headers, body) = send(
        &state,
        AxumMethod::PUT,
        "/v1/admin/users/11111111-1111-4111-8111-111111111111/role",
        Some("valid-admin-token"),
        None,
        Some(r#"{"newRole":"support","reason":"rotation"}"#),
    )
    .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(body["status"], 429);
    assert!(
        headers.contains_key("retry-after"),
        "a 429 carries Retry-After"
    );
    assert_eq!(
        upstream.calls(),
        20,
        "the 21st change never reaches Identity"
    );
}

#[tokio::test]
async fn an_upstream_timeout_is_a_504_problem_with_the_correlation_id() {
    let upstream = Arc::new(FakeUpstream::with_responses(vec![
        Err(UpstreamError::Timeout),
        Err(UpstreamError::Timeout),
        Err(UpstreamError::Timeout),
    ]));
    let state = state(
        FakeValidator::user(),
        upstream,
        Arc::new(FakeAudit::default()),
        100,
    );

    let (status, _, body) = send(
        &state,
        AxumMethod::GET,
        "/v1/assets/resolve",
        Some("valid-user-token"),
        Some("corr-timeout"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::GATEWAY_TIMEOUT);
    assert_eq!(body["status"], 504);
    assert_eq!(body["correlationId"], "corr-timeout");
}

#[tokio::test]
async fn an_invalid_correlation_id_is_refused_before_proxying() {
    let upstream = Arc::new(FakeUpstream::ok());
    let state = state(
        FakeValidator::user(),
        upstream.clone(),
        Arc::new(FakeAudit::default()),
        100,
    );

    let (status, _, body) = send(
        &state,
        AxumMethod::GET,
        "/v1/assets/resolve",
        Some("valid-user-token"),
        Some("bad id"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["status"], 400);
    assert_eq!(upstream.calls(), 0);
}

#[tokio::test]
async fn security_headers_are_present_on_every_response() {
    let state = state(
        FakeValidator::user(),
        Arc::new(FakeUpstream::ok()),
        Arc::new(FakeAudit::default()),
        100,
    );
    let (_, headers, _) = send(&state, AxumMethod::GET, "/healthz", None, None, None).await;

    assert_eq!(headers.get("x-content-type-options").unwrap(), "nosniff");
    assert_eq!(headers.get("x-frame-options").unwrap(), "DENY");
    assert_eq!(
        headers.get("content-security-policy").unwrap(),
        "default-src 'none'"
    );
    assert!(headers.contains_key("strict-transport-security"));
}

#[tokio::test]
async fn the_rate_limiter_returns_429_with_retry_after() {
    let state = state(
        FakeValidator::user(),
        Arc::new(FakeUpstream::ok()),
        Arc::new(FakeAudit::default()),
        1,
    );

    let (first, _, _) = send(&state, AxumMethod::GET, "/healthz", None, None, None).await;
    assert_eq!(first, StatusCode::OK);

    let (second, headers, body) = send(&state, AxumMethod::GET, "/healthz", None, None, None).await;
    assert_eq!(second, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(body["status"], 429);
    assert!(headers.contains_key("retry-after"));
}

#[tokio::test]
async fn metrics_expose_request_and_error_counters() {
    let state = state(
        FakeValidator::user(),
        Arc::new(FakeUpstream::with_responses(vec![
            Err(UpstreamError::Timeout),
            Err(UpstreamError::Timeout),
            Err(UpstreamError::Timeout),
        ])),
        Arc::new(FakeAudit::default()),
        100,
    );
    let _ = send(
        &state,
        AxumMethod::GET,
        "/v1/assets/resolve",
        Some("valid-user-token"),
        None,
        None,
    )
    .await;

    let response = router(state)
        .oneshot(
            Request::builder()
                .method(AxumMethod::GET)
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(text.contains("http_requests_total"));
    assert!(text.contains("status_class=\"5xx\""));
    assert!(text.contains("http_errors_total"));
    assert!(text.contains("service=\"gateway\""));
}

#[tokio::test]
async fn health_and_readiness_are_public() {
    let state = state(
        FakeValidator::user(),
        Arc::new(FakeUpstream::ok()),
        Arc::new(FakeAudit::default()),
        100,
    );
    let (status, _, body) = send(&state, AxumMethod::GET, "/healthz", None, None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["service"], "gateway");

    let (status, _, _) = send(&state, AxumMethod::GET, "/readyz", None, None, None).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn a_timeout_is_recorded_as_a_5xx_for_the_error_rate() {
    // The metric path is covered above; this test pins the mapping used by the
    // operation alert (5xx / total).
    let upstream = Arc::new(FakeUpstream::with_responses(vec![
        Err(UpstreamError::Timeout),
        Err(UpstreamError::Timeout),
        Err(UpstreamError::Timeout),
    ]));
    let state = state(
        FakeValidator::user(),
        upstream,
        Arc::new(FakeAudit::default()),
        100,
    );
    let (status, _, _) = send(
        &state,
        AxumMethod::GET,
        "/v1/assets/resolve",
        Some("valid-user-token"),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::GATEWAY_TIMEOUT);
    assert_eq!(state.metrics.errors_total(), 1);
    assert_eq!(state.metrics.requests_total(), 1);
}

#[tokio::test]
async fn the_method_parser_rejects_nothing_the_gateway_uses() {
    assert!(Method::parse("GET").is_ok());
    assert!(Method::parse("BREW").is_err());
}

// ---------------------------------------------------------------------------
// U6 — the trusted client address (NFR23.3)
// ---------------------------------------------------------------------------

async fn client_ip_probe(request: Request<Body>) -> String {
    request
        .headers()
        .get("x-client-ip")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("none")
        .to_string()
}

fn client_ip_router() -> axum::Router {
    axum::Router::new()
        .route("/probe", axum::routing::get(client_ip_probe))
        .layer(axum::middleware::from_fn(
            gateway_http::middleware::client_ip,
        ))
}

async fn probe_client_ip(request: Request<Body>) -> String {
    let response = client_ip_router().oneshot(request).await.unwrap();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn client_ip_strips_forwarded_for_and_uses_the_observed_peer() {
    let mut request = Request::builder()
        .uri("/probe")
        .header("x-forwarded-for", "203.0.113.9")
        .header("x-client-ip", "203.0.113.10")
        .body(Body::empty())
        .unwrap();
    request
        .extensions_mut()
        .insert(axum::extract::connect_info::ConnectInfo(
            std::net::SocketAddr::from(([192, 0, 2, 44], 40000)),
        ));

    assert_eq!(probe_client_ip(request).await, "192.0.2.44");
}

#[tokio::test]
async fn client_ip_uses_the_trusted_edge_header_when_no_peer_is_attached() {
    let request = Request::builder()
        .uri("/probe")
        .header("x-forwarded-for", "203.0.113.9")
        .header("x-real-ip", "198.51.100.7")
        .body(Body::empty())
        .unwrap();

    assert_eq!(probe_client_ip(request).await, "198.51.100.7");
}

#[tokio::test]
async fn client_ip_never_forwards_a_caller_supplied_value() {
    let spoofed = Request::builder()
        .uri("/probe")
        .header("x-client-ip", "203.0.113.10")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        probe_client_ip(spoofed).await,
        "none",
        "a client-written x-client-ip is stripped"
    );

    let malformed = Request::builder()
        .uri("/probe")
        .header("x-real-ip", "not-an-address")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        probe_client_ip(malformed).await,
        "none",
        "a malformed edge header never reaches downstream"
    );
}

// ---------------------------------------------------------------------------
// U3 — per-route budgets for the support triggers (NFR11.1)
// ---------------------------------------------------------------------------

fn principals(tokens: &[&str], subject: &str) -> FakeValidator {
    let mut map = std::collections::HashMap::new();
    for token in tokens {
        map.insert(
            (*token).to_string(),
            Ok(gateway_application::ports::Principal {
                subject: subject.to_string(),
                role: gateway_domain::Role::User,
            }),
        );
    }
    FakeValidator {
        principals: std::sync::Mutex::new(map),
    }
}

fn upstream_with(capacity: usize) -> Arc<FakeUpstream> {
    let response = gateway_application::ports::UpstreamResponse {
        status: 200,
        headers: Vec::new(),
        body: br#"{"ok":true}"#.to_vec(),
        content_type: Some("application/json".to_string()),
    };
    Arc::new(FakeUpstream::with_responses(vec![Ok(response); capacity]))
}

#[tokio::test]
async fn the_21st_creation_for_one_token_is_rate_limited_and_another_token_passes() {
    let upstream = upstream_with(40);
    let state = state(
        principals(&["support-token-a", "support-token-b"], "support-1"),
        upstream.clone(),
        Arc::new(FakeAudit::default()),
        1_000,
    );

    for attempt in 1..=20 {
        let (status, _, _) = send(
            &state,
            AxumMethod::POST,
            "/v1/support-grants",
            Some("support-token-a"),
            None,
            Some("{\"userId\":\"x\",\"reason\":\"y\"}"),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "creation {attempt} is inside budget"
        );
    }

    let (status, headers, body) = send(
        &state,
        AxumMethod::POST,
        "/v1/support-grants",
        Some("support-token-a"),
        None,
        Some("{\"userId\":\"x\",\"reason\":\"y\"}"),
    )
    .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(body["status"], 429);
    assert!(
        headers.contains_key("retry-after"),
        "a 429 carries Retry-After"
    );
    assert_eq!(
        upstream.calls(),
        20,
        "the 21st request never reaches Identity"
    );

    let (status, _, _) = send(
        &state,
        AxumMethod::POST,
        "/v1/support-grants",
        Some("support-token-b"),
        None,
        Some("{\"userId\":\"x\",\"reason\":\"y\"}"),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "a second token with the same subject has its own bucket"
    );
}

#[tokio::test]
async fn a_flood_of_distinct_tokens_from_one_address_still_hits_the_ip_budget() {
    let tokens = [
        "flood-token-1",
        "flood-token-2",
        "flood-token-3",
        "flood-token-4",
        "flood-token-5",
        "flood-token-6",
        "flood-token-7",
    ];
    let upstream = upstream_with(10);
    let state = state(
        principals(&tokens, "flood-subject"),
        upstream.clone(),
        Arc::new(FakeAudit::default()),
        5,
    );

    for token in tokens.iter().take(5) {
        let (status, _, _) = send(
            &state,
            AxumMethod::GET,
            "/v1/assets/resolve",
            Some(token),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{token} is inside the IP budget");
    }

    let (status, headers, _) = send(
        &state,
        AxumMethod::GET,
        "/v1/assets/resolve",
        Some(tokens[5]),
        None,
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::TOO_MANY_REQUESTS,
        "a fresh token does not reset the per-IP window"
    );
    assert!(headers.contains_key("retry-after"));
    assert_eq!(upstream.calls(), 5);
}

#[test]
fn the_route_bucket_is_keyed_on_the_token_not_the_subject() {
    let limiter = RateLimiter::new(1_000, 1_000);

    assert!(limiter
        .check_route("/v1/support-grants", "token-a", 1)
        .is_ok());
    match limiter.check_route("/v1/support-grants", "token-a", 1) {
        Err(retry_after) => assert!(
            (1..=60).contains(&retry_after),
            "Retry-After names the rest of the window: {retry_after}"
        ),
        Ok(()) => panic!("the route bucket is exhausted"),
    }
    assert!(
        limiter
            .check_route("/v1/support-grants", "token-b", 1)
            .is_ok(),
        "another token gets its own bucket even with the same subject"
    );
    assert!(
        limiter
            .check_route("/v1/support-grants/{id}/metadata", "token-a", 1)
            .is_ok(),
        "a different route has its own bucket"
    );
    assert!(
        limiter.check("192.0.2.10", true).is_ok(),
        "the route bucket never consumes the IP window"
    );
}
