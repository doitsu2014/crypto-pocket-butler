//! Job Orchestration HTTP boundary: operator routes, service enforcement, DTOs,
//! RFC 7807 problems and Prometheus text metrics.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use chrono::{DateTime, Utc};
use job_orchestration_application::common::{AppError, Caller};
use job_orchestration_application::operator::{JobView, OperatorService};
use job_orchestration_application::ports::{
    AuthError, HeartbeatRepository, ReadinessProbe, ServiceTokenRegistry, TokenValidator,
    VerifiedClaims,
};
use job_orchestration_application::scheduler::Scheduler;
use job_orchestration_domain::{JobRun, Operation};
use serde::{Deserialize, Serialize};
use serde_json::json;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// The router state.
#[derive(Clone)]
pub struct AppState {
    /// The service name.
    pub service_name: &'static str,
    /// Operator reads and triggers.
    pub operator: Arc<OperatorService>,
    /// The tick runner (used by the supervisor binary).
    pub scheduler: Arc<Scheduler>,
    /// The job -> schedule text map for manual runs and backfills.
    pub schedules: Arc<BTreeMap<String, String>>,
    /// JWT validation.
    pub token_validator: Arc<dyn TokenValidator>,
    /// Service-token registry.
    pub service_tokens: Arc<dyn ServiceTokenRegistry>,
    /// Heartbeat age.
    pub heartbeat: Arc<dyn HeartbeatRepository>,
    /// Readiness.
    pub readiness: Arc<dyn ReadinessProbe>,
    /// Metrics.
    pub metrics: Arc<Metrics>,
}

// ---------------------------------------------------------------------------
// Metrics
// ---------------------------------------------------------------------------

/// Metrics exposed at `/metrics`.
#[derive(Debug, Default)]
pub struct Metrics {
    total_2xx: AtomicU64,
    total_3xx: AtomicU64,
    total_4xx: AtomicU64,
    total_5xx: AtomicU64,
    pub(crate) runs_succeeded: AtomicU64,
    pub(crate) runs_failed: AtomicU64,
    pub(crate) runs_skipped: AtomicU64,
    pub(crate) runs_abandoned: AtomicU64,
    pub(crate) job_failures: AtomicU64,
    pub(crate) tick_millis: AtomicU64,
    pub(crate) heartbeat_age: AtomicI64,
}

impl Metrics {
    /// Records one response status.
    pub fn record(&self, status: u16) {
        match status {
            200..=299 => self.total_2xx.fetch_add(1, Ordering::Relaxed),
            300..=399 => self.total_3xx.fetch_add(1, Ordering::Relaxed),
            400..=499 => self.total_4xx.fetch_add(1, Ordering::Relaxed),
            _ => self.total_5xx.fetch_add(1, Ordering::Relaxed),
        };
    }

    /// Records one tick.
    pub fn record_tick(
        &self,
        succeeded: usize,
        failed: usize,
        skipped: usize,
        abandoned: usize,
        millis: u64,
    ) {
        self.runs_succeeded
            .fetch_add(succeeded as u64, Ordering::Relaxed);
        self.runs_failed.fetch_add(failed as u64, Ordering::Relaxed);
        self.runs_skipped
            .fetch_add(skipped as u64, Ordering::Relaxed);
        self.runs_abandoned
            .fetch_add(abandoned as u64, Ordering::Relaxed);
        self.job_failures
            .fetch_add((failed + abandoned) as u64, Ordering::Relaxed);
        self.tick_millis.store(millis, Ordering::Relaxed);
    }

    /// Records the heartbeat age.
    pub fn record_heartbeat_age(&self, age: Option<i64>) {
        self.heartbeat_age
            .store(age.unwrap_or(-1), Ordering::Relaxed);
    }

    /// Renders the exposition.
    #[must_use]
    pub fn render(&self, service: &str) -> String {
        let mut output = String::from(
            "# HELP http_requests_total Requests handled, by status class.\n\
             # TYPE http_requests_total counter\n",
        );
        for (class, value) in [
            ("2xx", self.total_2xx.load(Ordering::Relaxed)),
            ("3xx", self.total_3xx.load(Ordering::Relaxed)),
            ("4xx", self.total_4xx.load(Ordering::Relaxed)),
            ("5xx", self.total_5xx.load(Ordering::Relaxed)),
        ] {
            output.push_str(&format!(
                "http_requests_total{{service=\"{service}\",status_class=\"{class}\"}} {value}\n"
            ));
        }
        for (outcome, value) in [
            ("succeeded", self.runs_succeeded.load(Ordering::Relaxed)),
            ("failed", self.runs_failed.load(Ordering::Relaxed)),
            ("skipped", self.runs_skipped.load(Ordering::Relaxed)),
            ("abandoned", self.runs_abandoned.load(Ordering::Relaxed)),
        ] {
            output.push_str(&format!(
                "job_orchestration_runs_total{{service=\"{service}\",job=\"*\",outcome=\"{outcome}\"}} {value}\n"
            ));
        }
        output.push_str(&format!(
            "job_orchestration_job_failures_total{{service=\"{service}\",job=\"*\"}} {}\n",
            self.job_failures.load(Ordering::Relaxed)
        ));
        output.push_str(&format!(
            "job_orchestration_tick_duration_millis{{service=\"{service}\"}} {}\n",
            self.tick_millis.load(Ordering::Relaxed)
        ));
        let age = self.heartbeat_age.load(Ordering::Relaxed);
        output.push_str(
            "# HELP job_orchestration_heartbeat_age_seconds Age of the shared heartbeat.\n\
             # TYPE job_orchestration_heartbeat_age_seconds gauge\n",
        );
        if age >= 0 {
            output.push_str(&format!(
                "job_orchestration_heartbeat_age_seconds{{service=\"{service}\"}} {age}\n"
            ));
        }
        output.push_str(&format!(
            "job_orchestration_health{{service=\"{service}\"}} 1\n"
        ));
        output
    }
}

// ---------------------------------------------------------------------------
// Middleware
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
/// The correlation id.
pub struct CorrelationId(pub String);

#[derive(Debug, Clone)]
/// Validated claims.
pub struct ValidatedClaims(pub VerifiedClaims);

#[derive(Debug, Clone)]
/// The registered service caller.
pub struct ServicePrincipal(pub String, pub Vec<Operation>);

fn correlation_of(headers: &HeaderMap) -> String {
    headers
        .get("x-correlation-id")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty() && value.len() <= 128)
        .map(ToString::to_string)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
}

async fn require_jwt(
    State(state): State<AppState>,
    mut request: axum::extract::Request,
    next: middleware::Next,
) -> Response {
    let correlation = correlation_of(request.headers());
    request
        .extensions_mut()
        .insert(CorrelationId(correlation.clone()));
    let Some(token) = bearer(&request) else {
        return problem(
            &correlation,
            401,
            "unauthorized",
            "a bearer token is required",
        );
    };
    match state.token_validator.validate(&token).await {
        Ok(claims) => {
            request.extensions_mut().insert(ValidatedClaims(claims));
            next.run(request).await
        }
        Err(AuthError::Unavailable(detail)) => {
            problem(&correlation, 503, "identity-unavailable", &detail)
        }
        Err(error) => problem(&correlation, 401, "unauthorized", &error.to_string()),
    }
}

async fn require_service(
    State(state): State<AppState>,
    mut request: axum::extract::Request,
    next: middleware::Next,
) -> Response {
    let correlation = correlation_of(request.headers());
    request
        .extensions_mut()
        .insert(CorrelationId(correlation.clone()));
    let Some(token) = bearer(&request) else {
        return problem(
            &correlation,
            401,
            "unauthorized",
            "a service token is required",
        );
    };
    match state.service_tokens.service_for(&token).await {
        Ok(Some(service)) => {
            request
                .extensions_mut()
                .insert(ServicePrincipal(service.name, service.operations));
            next.run(request).await
        }
        Ok(None) => problem(&correlation, 401, "unauthorized", "unknown service token"),
        Err(error) => problem(
            &correlation,
            503,
            "registry-unavailable",
            &error.to_string(),
        ),
    }
}

async fn record_metrics(
    State(state): State<AppState>,
    request: axum::extract::Request,
    next: middleware::Next,
) -> Response {
    let response = next.run(request).await;
    state.metrics.record(response.status().as_u16());
    response
}

fn bearer(request: &axum::extract::Request) -> Option<String> {
    request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(|token| token.trim().to_string())
        .filter(|token| !token.is_empty())
}

fn problem(correlation: &str, status: u16, kind: &str, detail: &str) -> Response {
    (
        StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        [(header::CONTENT_TYPE, "application/problem+json")],
        Json(json!({
            "type": format!("https://errors.cpb/job-orchestration/{kind}"),
            "title": kind,
            "status": status,
            "detail": detail,
            "correlationId": correlation,
        })),
    )
        .into_response()
}

fn app_error(error: &AppError, correlation: &str) -> Response {
    match error {
        AppError::Unauthorized(detail) => problem(correlation, 401, "unauthorized", detail),
        AppError::Forbidden(detail) => problem(correlation, 403, "forbidden", detail),
        AppError::NotFound(what) => problem(correlation, 404, "not-found", what),
        AppError::Validation(detail) => problem(correlation, 422, "validation", detail),
        AppError::Conflict(detail) => problem(correlation, 409, "conflict", detail),
        AppError::Unavailable(detail) => problem(correlation, 503, "unavailable", detail),
        AppError::Internal(detail) => {
            tracing::error!(%detail, "job-orchestration request failed internally");
            problem(correlation, 500, "internal", "internal server error")
        }
    }
}

// ---------------------------------------------------------------------------
// DTOs and handlers
// ---------------------------------------------------------------------------

/// A run in the wire shape.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunDto {
    /// The attempt id.
    pub id: String,
    /// The job.
    pub job: String,
    /// The window.
    pub scheduled_for: DateTime<Utc>,
    /// The attempt number.
    pub attempt: u32,
    /// The state label.
    pub status: String,
    /// The trigger label.
    pub trigger: String,
    /// The actor, when one asked.
    pub actor_id: Option<String>,
    /// The reason, when one was given.
    pub reason: Option<String>,
    /// When it started.
    pub started_at: Option<DateTime<Utc>>,
    /// When it finished.
    pub finished_at: Option<DateTime<Utc>>,
    /// The provider's HTTP status.
    pub response_status: Option<u16>,
    /// A short reason.
    pub detail: Option<String>,
}

impl From<&JobRun> for RunDto {
    fn from(run: &JobRun) -> Self {
        Self {
            id: run.id.to_string(),
            job: run.job.clone(),
            scheduled_for: run.scheduled_for,
            attempt: run.attempt,
            status: run.status.as_str().to_string(),
            trigger: run.trigger.as_str().to_string(),
            actor_id: run.actor_id.map(|actor| actor.to_string()),
            reason: run.reason.clone(),
            started_at: run.started_at,
            finished_at: run.finished_at,
            response_status: run.response_status,
            detail: run.detail.clone(),
        }
    }
}

/// One job with its latest attempt.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobDto {
    /// The job.
    pub job: String,
    /// The latest attempt.
    pub latest: Option<RunDto>,
}

impl From<&JobView> for JobDto {
    fn from(view: &JobView) -> Self {
        Self {
            job: view.job.clone(),
            latest: view.latest.as_ref().map(RunDto::from),
        }
    }
}

/// The runs query string.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunsQuery {
    /// The lower bound.
    #[serde(default)]
    pub from: Option<DateTime<Utc>>,
    /// The upper bound.
    #[serde(default)]
    pub to: Option<DateTime<Utc>>,
    /// The page size.
    #[serde(default)]
    pub limit: Option<i64>,
}

/// The manual-run body.
#[derive(Debug, Clone, Deserialize)]
pub struct ManualRequest {
    /// The reason (1–200 characters).
    pub reason: String,
}

/// The backfill body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackfillRequest {
    /// The lower bound.
    pub from: DateTime<Utc>,
    /// The upper bound.
    pub to: DateTime<Utc>,
    /// The reason.
    pub reason: String,
}

/// The backfill response.
#[derive(Debug, Clone, Serialize)]
pub struct BackfillResponse {
    /// Windows accepted.
    pub accepted: usize,
    /// Windows already covered.
    pub covered: usize,
}

fn caller_of(claims: &VerifiedClaims, correlation: &str) -> Caller {
    Caller::from_claims(claims, correlation)
}

async fn health(State(state): State<AppState>) -> Response {
    Json(json!({ "status": "ok", "service": state.service_name })).into_response()
}

async fn ready(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
) -> Response {
    match state.readiness.check().await {
        Ok(()) => Json(json!({ "status": "ready" })).into_response(),
        Err(detail) => problem(&correlation.0, 503, "not-ready", &detail),
    }
}

async fn metrics(State(state): State<AppState>) -> Response {
    (
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        state.metrics.render(state.service_name),
    )
        .into_response()
}

async fn list_jobs(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
) -> Response {
    let caller = caller_of(&claims, &correlation.0);
    match state.operator.jobs(&caller).await {
        Ok(jobs) => {
            let body: Vec<JobDto> = jobs.iter().map(JobDto::from).collect();
            Json(body).into_response()
        }
        Err(error) => app_error(&error, &correlation.0),
    }
}

async fn list_runs(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(job): Path<String>,
    Query(query): Query<RunsQuery>,
) -> Response {
    let caller = caller_of(&claims, &correlation.0);
    match state
        .operator
        .runs(&caller, &job, query.from, query.to, query.limit)
        .await
    {
        Ok(runs) => {
            let body: Vec<RunDto> = runs.iter().map(RunDto::from).collect();
            Json(body).into_response()
        }
        Err(error) => app_error(&error, &correlation.0),
    }
}

async fn run_now(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(job): Path<String>,
    payload: Result<Json<ManualRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => {
            return problem(&correlation.0, 422, "validation", &rejection.body_text())
        }
    };
    let Some(schedule) = state.schedules.get(&job) else {
        return problem(
            &correlation.0,
            404,
            "not-found",
            "the job is not in the catalogue",
        );
    };
    let caller = caller_of(&claims, &correlation.0);
    match state
        .operator
        .run_now(&caller, &job, &request.reason, schedule)
        .await
    {
        Ok(queued) => (StatusCode::ACCEPTED, Json(RunDto::from(&queued.run))).into_response(),
        Err(error) => app_error(&error, &correlation.0),
    }
}

async fn backfill(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(job): Path<String>,
    payload: Result<Json<BackfillRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => {
            return problem(&correlation.0, 422, "validation", &rejection.body_text())
        }
    };
    let Some(schedule) = state.schedules.get(&job) else {
        return problem(
            &correlation.0,
            404,
            "not-found",
            "the job is not in the catalogue",
        );
    };
    let caller = caller_of(&claims, &correlation.0);
    match state
        .operator
        .backfill(
            &caller,
            &job,
            request.from,
            request.to,
            &request.reason,
            schedule,
        )
        .await
    {
        Ok(result) => (
            StatusCode::ACCEPTED,
            Json(BackfillResponse {
                accepted: result.accepted,
                covered: result.covered,
            }),
        )
            .into_response(),
        Err(error) => app_error(&error, &correlation.0),
    }
}

async fn smoke(
    State(state): State<AppState>,
    Extension(ServicePrincipal(name, operations)): Extension<ServicePrincipal>,
) -> Response {
    if !operations.contains(&Operation::Smoke) {
        return problem("smoke", 403, "forbidden", "the caller may not read smoke");
    }
    Json(json!({ "status": "ok", "service": state.service_name, "caller": name })).into_response()
}

async fn fallback(Extension(correlation): Extension<CorrelationId>) -> Response {
    problem(&correlation.0, 404, "not-found", "the route does not exist")
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Builds the router.
pub fn router(state: AppState) -> Router {
    let public = Router::new()
        .route("/healthz", get(health))
        .route("/readyz", get(ready))
        .route("/metrics", get(metrics));

    let operator = Router::new()
        .route("/v1/jobs", get(list_jobs))
        .route("/v1/jobs/{job}/runs", get(list_runs))
        .route("/v1/jobs/{job}/run", post(run_now))
        .route("/v1/jobs/{job}/backfill", post(backfill))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_jwt));

    let internal = Router::new()
        .route("/v1/internal/smoke", get(smoke))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_service,
        ));

    Router::new()
        .merge(public)
        .merge(operator)
        .merge(internal)
        .fallback(fallback)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            record_metrics,
        ))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::Metrics;

    #[test]
    fn the_exposition_carries_the_run_counters_and_heartbeat() {
        let metrics = Metrics::default();
        metrics.record_tick(2, 1, 0, 1, 12);
        metrics.record_heartbeat_age(Some(4));
        let text = metrics.render("job-orchestration");
        assert!(text.contains("outcome=\"succeeded\"} 2"));
        assert!(text.contains("job_orchestration_job_failures_total"));
        assert!(text.contains("job_orchestration_heartbeat_age_seconds"));
    }

    #[test]
    fn an_unknown_heartbeat_is_not_exported() {
        let metrics = Metrics::default();
        metrics.record_heartbeat_age(None);
        assert!(!metrics.render("x").contains("heartbeat_age_seconds{"));
    }
}
