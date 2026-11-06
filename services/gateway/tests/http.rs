//! Gateway edge tests: proxying, RFC 7807 problems, correlation ids, security
//! headers, rate limiting, and metrics.

mod common;

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, HeaderMap, Method as AxumMethod, Request, StatusCode};
use chrono::{TimeZone, Utc};
use common::fakes::{FakeAudit, FakeUpstream, FakeValidator, FixedClock, Ready};
use gateway_application::authorize::AuthorizeAndRoute;
use gateway_application::ports::{Clock, UpstreamError};
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
