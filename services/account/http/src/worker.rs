//! The worker's listener: health, readiness, metrics, and the internal wake
//! endpoint the API calls. Every path but the probes requires a service token.

use std::sync::Arc;

use account_application::ports::{ReadinessProbe, ServiceTokenRegistry};
use axum::extract::{Extension, Request, State};
use axum::http::{header, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;
use tokio::sync::Notify;

use crate::metrics::Metrics;
use crate::middleware::{correlation_id, security_headers, CorrelationId};
use crate::problem::ProblemDetails;

/// Everything the worker's listener needs.
#[derive(Clone)]
pub struct WorkerState {
    /// Service name used in logs and metrics.
    pub service_name: &'static str,
    /// Wakes the supervisor loop.
    pub notifier: Arc<Notify>,
    /// Internal service-token registry.
    pub service_tokens: Arc<dyn ServiceTokenRegistry>,
    /// Readiness probe.
    pub readiness: Arc<dyn ReadinessProbe>,
    /// Request metrics.
    pub metrics: Arc<Metrics>,
}

/// Builds the worker's router.
pub fn router(state: WorkerState) -> Router {
    let public = Router::new()
        .route("/healthz", get(health))
        .route("/readyz", get(ready))
        .route("/metrics", get(metrics));

    let wake = Router::new()
        .route("/internal/wake", post(wake))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_service,
        ));

    Router::new()
        .merge(public)
        .merge(wake)
        .fallback(fallback)
        .layer(middleware::from_fn(security_headers))
        .layer(middleware::from_fn(correlation_id))
        .with_state(state)
}

async fn health(State(state): State<WorkerState>) -> Response {
    Json(json!({ "status": "ok", "service": state.service_name })).into_response()
}

async fn ready(
    State(state): State<WorkerState>,
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

async fn metrics(State(state): State<WorkerState>) -> Response {
    (
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        state.metrics.render(state.service_name),
    )
        .into_response()
}

async fn wake(State(state): State<WorkerState>) -> Response {
    state.notifier.notify_one();
    (StatusCode::ACCEPTED, Json(json!({ "status": "accepted" }))).into_response()
}

async fn fallback(Extension(correlation): Extension<CorrelationId>) -> Response {
    ProblemDetails::not_found(&correlation.0, "the route does not exist").into_response()
}

async fn require_service(
    State(state): State<WorkerState>,
    request: Request,
    next: Next,
) -> Response {
    let correlation = request
        .extensions()
        .get::<CorrelationId>()
        .map(|correlation| correlation.0.clone())
        .unwrap_or_default();
    let token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| !token.is_empty());
    let Some(token) = token else {
        return ProblemDetails::new(
            401,
            "unauthorized",
            "Unauthorized",
            Some("missing service token".to_string()),
            &correlation,
        )
        .into_response();
    };
    match state.service_tokens.service_for(token).await {
        Ok(Some(_)) => next.run(request).await,
        Ok(None) => ProblemDetails::new(
            401,
            "unauthorized",
            "Unauthorized",
            Some("unknown service token".to_string()),
            &correlation,
        )
        .into_response(),
        Err(error) => ProblemDetails::new(
            503,
            "registry-unavailable",
            "Service-token registry unavailable",
            Some(error.to_string()),
            &correlation,
        )
        .into_response(),
    }
}
