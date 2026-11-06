//! Edge handlers: local health endpoints and the forwarding proxy.

use axum::body::Body;
use axum::extract::{Extension, Request, State};
use axum::http::{header, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use gateway_application::authorize::RoutedResponse;
use gateway_application::ports::IncomingRequest;
use gateway_domain::{GatewayError, Method};
use serde_json::json;

use crate::middleware::CorrelationId;
use crate::problem::ProblemDetails;
use crate::state::AppState;

/// Liveness probe.
pub async fn health(State(state): State<AppState>) -> Response {
    Json(json!({ "status": "ok", "service": state.service_name })).into_response()
}

/// Readiness probe.
pub async fn ready(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
) -> Response {
    match state.readiness.check().await {
        Ok(()) => Json(json!({ "status": "ready" })).into_response(),
        Err(detail) => ProblemDetails::new(
            503,
            "not-ready",
            "Service not ready",
            Some(detail),
            &correlation.0,
        )
        .into_response(),
    }
}

/// Prometheus text metrics.
pub async fn metrics(State(state): State<AppState>) -> Response {
    (
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        state.metrics.render(state.service_name),
    )
        .into_response()
}

/// Forwards every other request through the gateway request path.
pub async fn proxy(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    request: Request,
) -> Response {
    let (parts, body) = request.into_parts();
    let body = match axum::body::to_bytes(body, 1024 * 1024).await {
        Ok(body) => body.to_vec(),
        Err(_) => {
            return ProblemDetails::new(
                413,
                "payload-too-large",
                "Payload too large",
                Some("request body exceeds 1 MiB".to_string()),
                &correlation.0,
            )
            .into_response();
        }
    };
    let method = match Method::parse(parts.method.as_str()) {
        Ok(method) => method,
        Err(error) => return gateway_problem(error, &correlation.0),
    };
    let headers = parts
        .headers
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.as_str().to_string(), value.to_string()))
        })
        .collect();
    let query = parts.uri.query().map(ToString::to_string);

    let incoming = IncomingRequest {
        method,
        path: parts.uri.path().to_string(),
        query,
        headers,
        body,
    };

    match state.use_case.execute(incoming).await {
        Ok(response) => build_response(response),
        Err(error) => gateway_problem(error, &correlation.0),
    }
}

fn gateway_problem(error: GatewayError, correlation: &str) -> Response {
    ProblemDetails::new(
        error.status(),
        error.problem_type(),
        error.title(),
        Some(error.detail().to_string()),
        correlation,
    )
    .into_response()
}

fn build_response(response: RoutedResponse) -> Response {
    let status = StatusCode::from_u16(response.status).unwrap_or(StatusCode::BAD_GATEWAY);
    let mut builder = Response::builder().status(status);
    for (name, value) in &response.headers {
        let lower = name.to_ascii_lowercase();
        if matches!(
            lower.as_str(),
            "content-length"
                | "content-type"
                | "x-correlation-id"
                | "connection"
                | "transfer-encoding"
        ) {
            continue;
        }
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(name.as_bytes()),
            HeaderValue::from_str(value),
        ) {
            builder = builder.header(name, value);
        }
    }
    if let Some(content_type) = response.content_type {
        if let Ok(value) = HeaderValue::from_str(&content_type) {
            builder = builder.header(header::CONTENT_TYPE, value);
        }
    }
    if let Ok(value) = HeaderValue::from_str(&response.correlation_id) {
        builder = builder.header("x-correlation-id", value);
    }
    builder
        .body(Body::from(response.body))
        .unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response())
}
