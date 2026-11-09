//! Route handlers. Every failure is an RFC 7807 problem carrying the correlation
//! id; a validation refusal names the field paths and never the values.

use std::time::Instant;

use analytics_application::{
    AggregateError, EraseError, FieldError, FlagError, IngestError, Principal, RepositoryError,
    RetentionError,
};
use analytics_domain::{event_class, EventClass, Metric, Period};
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, Request, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use crate::dto::{
    AggregateDto, AggregatePageResponse, AggregateQueryParams, AggregateRunResponse,
    CreateFlagRequest, EraseRequest, FlagDto, FlagResponse, IngestRequest, IngestResponse,
    ListFlagsParams, RetentionRunResponse, UpdateFlagRequest,
};
use crate::middleware::{CorrelationId, ServicePrincipal};
use crate::problem::{FieldProblem, ProblemDetails};
use crate::state::AppState;

fn problem(
    status: u16,
    problem_type: &str,
    title: &str,
    detail: impl Into<String>,
    correlation: &str,
) -> Response {
    ProblemDetails::new(
        status,
        problem_type,
        title,
        Some(detail.into()),
        correlation,
    )
    .into_response()
}

fn fields(errors: Vec<FieldError>) -> Vec<FieldProblem> {
    errors
        .into_iter()
        .map(|error| FieldProblem {
            field: error.field,
            reason: error.reason,
        })
        .collect()
}

fn validation(correlation: &str, errors: Vec<FieldError>) -> Response {
    ProblemDetails::new(
        422,
        "validation",
        "Invalid request",
        Some("one or more fields failed validation".to_string()),
        correlation,
    )
    .with_fields(fields(errors))
    .into_response()
}

fn unavailable(correlation: &str, detail: impl Into<String>) -> Response {
    problem(
        503,
        "unavailable",
        "Service unavailable",
        detail,
        correlation,
    )
}

fn repository_error(error: RepositoryError, correlation: &str) -> Response {
    match error {
        RepositoryError::Forbidden(detail) => problem(
            403,
            "grant-refused",
            "Refused by the database",
            detail,
            correlation,
        ),
        other => unavailable(correlation, other.to_string()),
    }
}

fn body_error(rejection: JsonRejection, correlation: &str) -> Response {
    validation(
        correlation,
        vec![FieldError::new("body", rejection.body_text())],
    )
}

/// Liveness probe.
pub async fn health(State(state): State<AppState>) -> Response {
    Json(json!({ "status": "ok", "service": state.service_name })).into_response()
}

/// Readiness probe: Postgres, migrations, and the current salt.
pub async fn ready(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
) -> Response {
    match state.readiness.check().await {
        Ok(()) => Json(json!({ "status": "ready" })).into_response(),
        Err(detail) => problem(
            503,
            "not-ready",
            "Service not ready",
            detail,
            &correlation.0,
        ),
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

fn consent_result_for(event_name: &str) -> &'static str {
    match event_class(event_name) {
        Ok(EventClass::Essential) => "skipped",
        Ok(EventClass::Analytics) => "granted",
        Err(_) => "unknown",
    }
}

/// Only a catalogue name becomes a metric label; anything else is `unknown`, so
/// a caller cannot inject an unbounded or PII-shaped label.
fn metric_event_name(event_name: &str) -> &'static str {
    analytics_domain::definition(event_name).map_or("unknown", |entry| entry.name)
}

/// WF1: ingest one event.
pub async fn ingest_event(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
    axum::extract::Extension(principal): axum::extract::Extension<Principal>,
    payload: Result<Json<IngestRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => return body_error(rejection, &correlation.0),
    };
    let event_name = request.event_name.clone();
    let command = request.into_command(principal.subject, Some(correlation.0.clone()));
    match state.ingest.execute(command).await {
        Ok(_) => {
            state.metrics.record_event(
                metric_event_name(&event_name),
                consent_result_for(&event_name),
                "accepted",
            );
            (StatusCode::ACCEPTED, Json(IngestResponse::accepted())).into_response()
        }
        Err(IngestError::Rejected(errors)) => {
            state
                .metrics
                .record_event(metric_event_name(&event_name), "unknown", "rejected");
            validation(&correlation.0, errors)
        }
        Err(IngestError::ConsentNotGranted) => {
            state
                .metrics
                .record_event(metric_event_name(&event_name), "not-granted", "refused");
            problem(
                403,
                "consent-required",
                "Analytics consent is required",
                "no granted analytics consent is recorded for this user",
                &correlation.0,
            )
        }
        Err(IngestError::ConsentUnavailable(detail)) => {
            state
                .metrics
                .record_event(metric_event_name(&event_name), "unavailable", "error");
            unavailable(&correlation.0, detail)
        }
        Err(IngestError::SaltUnavailable(detail)) => {
            state
                .metrics
                .record_event(metric_event_name(&event_name), "unavailable", "error");
            unavailable(&correlation.0, detail)
        }
        Err(IngestError::Repository(error)) => {
            state
                .metrics
                .record_event(metric_event_name(&event_name), "error", "error");
            repository_error(error, &correlation.0)
        }
    }
}

/// WF3: evaluate one flag for the caller.
pub async fn get_flag(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
    axum::extract::Extension(principal): axum::extract::Extension<Principal>,
    Path(key): Path<String>,
) -> Response {
    match state.flags.evaluate(&key, &principal.subject).await {
        Ok(evaluation) => Json(FlagResponse {
            key: evaluation.key,
            enabled: evaluation.enabled,
        })
        .into_response(),
        Err(error) => flag_failure(error, &correlation.0),
    }
}

fn flag_failure(error: FlagError, correlation: &str) -> Response {
    match error {
        FlagError::Rejected(errors) => validation(correlation, errors),
        FlagError::NotFound => problem(404, "not-found", "Not found", "no such flag", correlation),
        FlagError::Repository(error) => repository_error(error, correlation),
    }
}

/// WF4: list flags, optionally only expired ones.
pub async fn list_flags(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
    Query(params): Query<ListFlagsParams>,
) -> Response {
    match state.flags.list(params.expired.unwrap_or(false)).await {
        Ok(flags) => {
            let now = state.clock.now();
            let body: Vec<FlagDto> = flags
                .iter()
                .map(|flag| FlagDto::from_flag(flag, now))
                .collect();
            Json(body).into_response()
        }
        Err(error) => flag_failure(error, &correlation.0),
    }
}

/// WF4: create a flag.
pub async fn create_flag(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
    axum::extract::Extension(principal): axum::extract::Extension<Principal>,
    payload: Result<Json<CreateFlagRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => return body_error(rejection, &correlation.0),
    };
    match state.flags.create(request.into(), &principal.subject).await {
        Ok(flag) => {
            let now = state.clock.now();
            (StatusCode::CREATED, Json(FlagDto::from_flag(&flag, now))).into_response()
        }
        Err(error) => flag_failure(error, &correlation.0),
    }
}

/// WF4: update a flag.
pub async fn update_flag(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
    axum::extract::Extension(principal): axum::extract::Extension<Principal>,
    Path(key): Path<String>,
    payload: Result<Json<UpdateFlagRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => return body_error(rejection, &correlation.0),
    };
    match state
        .flags
        .update(&key, request.into(), &principal.subject)
        .await
    {
        Ok(flag) => {
            let now = state.clock.now();
            Json(FlagDto::from_flag(&flag, now)).into_response()
        }
        Err(error) => flag_failure(error, &correlation.0),
    }
}

/// WF4: delete a flag.
pub async fn delete_flag(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
    axum::extract::Extension(principal): axum::extract::Extension<Principal>,
    Path(key): Path<String>,
) -> Response {
    match state.flags.delete(&key, &principal.subject).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => flag_failure(error, &correlation.0),
    }
}

/// C1: read aggregates (admin).
pub async fn get_aggregates(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
    Query(params): Query<AggregateQueryParams>,
) -> Response {
    let mut errors = Vec::new();
    let metric = match params.metric.as_deref() {
        None => None,
        Some(raw) => match Metric::parse(raw) {
            Ok(metric) => Some(metric),
            Err(error) => {
                errors.push(FieldError::from(error));
                None
            }
        },
    };
    let period = match params.period.as_deref() {
        None => None,
        Some(raw) => match Period::parse(raw) {
            Ok(period) => Some(period),
            Err(error) => {
                errors.push(FieldError::from(error));
                None
            }
        },
    };
    if !errors.is_empty() {
        return validation(&correlation.0, errors);
    }
    let limit = params.limit.unwrap_or(100).clamp(1, 500);
    match state
        .aggregates
        .list(metric, period, params.from, params.to, limit)
        .await
    {
        Ok(aggregates) => Json(AggregatePageResponse {
            aggregates: aggregates.iter().map(AggregateDto::from).collect(),
        })
        .into_response(),
        Err(error) => aggregate_failure(error, &correlation.0),
    }
}

fn aggregate_failure(error: AggregateError, correlation: &str) -> Response {
    match error {
        AggregateError::Repository(error) => repository_error(error, correlation),
        AggregateError::InvalidData(detail) => unavailable(correlation, detail),
        AggregateError::WindowTooLarge { .. } => unavailable(correlation, error.to_string()),
    }
}

/// C11: run the nightly aggregation.
pub async fn run_aggregate(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
) -> Response {
    let started = Instant::now();
    match state.aggregates.run_nightly().await {
        Ok(report) => {
            state
                .metrics
                .record_aggregate_duration(started.elapsed().as_secs_f64());
            state.metrics.record_aggregate_run(
                true,
                u64::try_from(report.suppressed.len()).unwrap_or(u64::MAX),
            );
            Json(AggregateRunResponse {
                status: "success",
                period_start: report.period_start,
                stored: report.stored.len(),
                suppressed: report.suppressed.len(),
                scanned_daily: report.scanned.daily,
                scanned_cohort_d7: report.scanned.cohort_d7,
                scanned_cohort_d30: report.scanned.cohort_d30,
            })
            .into_response()
        }
        Err(error) => {
            state
                .metrics
                .record_aggregate_duration(started.elapsed().as_secs_f64());
            state.metrics.record_aggregate_run(false, 0);
            aggregate_failure(error, &correlation.0)
        }
    }
}

/// C11: run the retention sweep.
pub async fn run_retention(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
) -> Response {
    match state.retention.run().await {
        Ok(report) => Json(RetentionRunResponse {
            status: "success",
            events_deleted: report.events_deleted,
            consent_checks_deleted: report.consent_checks_deleted,
        })
        .into_response(),
        Err(RetentionError::Repository(error)) => repository_error(error, &correlation.0),
    }
}

/// C12: Identity erases a user's pseudonymous events.
pub async fn erase(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
    axum::extract::Extension(service): axum::extract::Extension<ServicePrincipal>,
    payload: Result<Json<EraseRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => return body_error(rejection, &correlation.0),
    };
    match state
        .erase
        .erase(&request.auth_user_id, &service.0.service)
        .await
    {
        Ok(_) => {
            state.metrics.record_erase(true);
            StatusCode::NO_CONTENT.into_response()
        }
        Err(EraseError::Rejected(errors)) => {
            state.metrics.record_erase(false);
            validation(&correlation.0, errors)
        }
        Err(EraseError::SaltUnavailable(detail)) => {
            state.metrics.record_erase(false);
            unavailable(&correlation.0, detail)
        }
        Err(EraseError::Repository(error)) => {
            state.metrics.record_erase(false);
            repository_error(error, &correlation.0)
        }
    }
}

/// C12: release smoke.
pub async fn smoke(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
) -> Response {
    match state.readiness.check().await {
        Ok(()) => Json(json!({ "status": "ready" })).into_response(),
        Err(detail) => problem(
            503,
            "not-ready",
            "Service not ready",
            detail,
            &correlation.0,
        ),
    }
}

/// Unknown path.
pub async fn fallback(request: Request) -> Response {
    let correlation = crate::middleware::correlation_of(&request);
    problem(
        404,
        "not-found",
        "Not found",
        "no route matches this path",
        &correlation,
    )
}
