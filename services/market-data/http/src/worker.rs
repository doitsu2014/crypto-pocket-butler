//! The worker's listener: health, readiness, and metrics. The worker owns no
//! inbound data path; Job Orchestration calls the API's C11 endpoint.

use std::sync::Arc;

use axum::extract::{Extension, State};
use axum::http::header;
use axum::middleware;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use market_data_application::ports::ReadinessProbe;

use crate::metrics::Metrics;
use crate::middleware::{correlation_id, security_headers, CorrelationId};
use crate::problem::ProblemDetails;

/// Everything the worker's listener needs.
#[derive(Clone)]
pub struct WorkerState {
    /// Service name used in logs and metrics.
    pub service_name: &'static str,
    /// Readiness probe.
    pub readiness: Arc<dyn ReadinessProbe>,
    /// Workflow metrics.
    pub metrics: Arc<Metrics>,
}

/// Builds the worker's router.
pub fn router(state: WorkerState) -> Router {
    Router::new()
        .route("/healthz", get(health))
        .route("/readyz", get(ready))
        .route("/metrics", get(metrics))
        .fallback(fallback)
        .layer(middleware::from_fn(security_headers))
        .layer(middleware::from_fn(correlation_id))
        .with_state(state)
}

async fn health(State(state): State<WorkerState>) -> Response {
    Json(serde_json::json!({ "status": "ok", "service": state.service_name })).into_response()
}

async fn ready(
    State(state): State<WorkerState>,
    Extension(correlation): Extension<CorrelationId>,
) -> Response {
    match state.readiness.check().await {
        Ok(()) => Json(serde_json::json!({ "status": "ready" })).into_response(),
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

async fn fallback(Extension(correlation): Extension<CorrelationId>) -> Response {
    ProblemDetails::not_found(&correlation.0, "the route does not exist").into_response()
}
