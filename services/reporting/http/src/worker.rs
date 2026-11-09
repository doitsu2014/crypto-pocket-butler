//! The worker's listener: health, readiness, metrics, and the C11 brief-generation
//! run endpoint Job Orchestration calls. Every run path requires a service token.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, Path, State};
use axum::http::header;
use axum::middleware;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use reporting_application::common::ServiceCaller;
use reporting_application::generation::RUN_JOB;
use serde_json::json;
use uuid::Uuid;

use crate::dto::{RunRequest, RunResultDto};
use crate::middleware::{
    correlation_id, record_worker_metrics, require_worker_service, security_headers, CorrelationId,
    ServicePrincipal,
};
use crate::problem::{from_app_error, ProblemDetails};
use crate::state::WorkerState;

/// Builds the worker's router.
pub fn router(state: WorkerState) -> Router {
    let public = Router::new()
        .route("/healthz", get(health))
        .route("/readyz", get(ready))
        .route("/metrics", get(metrics));

    let runs = Router::new()
        .route("/v1/internal/run/{jobName}", post(run_generation))
        .route("/v1/internal/smoke", get(smoke))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_worker_service,
        ));

    Router::new()
        .merge(public)
        .merge(runs)
        .fallback(fallback)
        .layer(middleware::from_fn(security_headers))
        .layer(middleware::from_fn(correlation_id))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            record_worker_metrics,
        ))
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

async fn run_generation(
    State(state): State<WorkerState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ServicePrincipal(principal)): Extension<ServicePrincipal>,
    Path(job_name): Path<String>,
    payload: Result<Json<RunRequest>, JsonRejection>,
) -> Response {
    if job_name != RUN_JOB {
        return ProblemDetails::not_found(&correlation.0, "the job is not provided by Reporting")
            .into_response();
    }
    let owner = match payload {
        Ok(Json(request)) => request
            .owner
            .filter(|owner| !owner.trim().is_empty())
            .unwrap_or_else(|| format!("reporting-worker-{}", std::process::id())),
        Err(_) => format!("reporting-worker-{}", std::process::id()),
    };
    let caller = ServiceCaller::from_registered(&principal, &correlation.0);
    match state.generation.run(&caller, &owner).await {
        Ok(report) => {
            state
                .metrics
                .record_run(report.failed > 0, report.duration_seconds);
            state
                .metrics
                .record_briefs(report.generated, report.failed, report.skipped);
            state
                .metrics
                .record_email(report.email_submitted, report.email_failed);
            Json(RunResultDto::from(report)).into_response()
        }
        Err(error) => {
            state.metrics.record_run(true, 0.0);
            from_app_error(&error, &correlation.0).into_response()
        }
    }
}

async fn smoke(
    State(state): State<WorkerState>,
    Extension(ServicePrincipal(principal)): Extension<ServicePrincipal>,
) -> Response {
    let caller = ServiceCaller::from_registered(&principal, "smoke");
    if let Err(error) = caller.require_operation(reporting_domain::Operation::RunBriefGeneration) {
        return from_app_error(&error, "smoke").into_response();
    }
    Json(json!({
        "status": "ok",
        "service": state.service_name,
        "correlationId": Uuid::new_v4().to_string(),
    }))
    .into_response()
}

async fn fallback(Extension(correlation): Extension<CorrelationId>) -> Response {
    ProblemDetails::not_found(&correlation.0, "the route does not exist").into_response()
}
