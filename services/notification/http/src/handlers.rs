//! Route handlers. Every failure is an RFC 7807 problem carrying the correlation
//! id; a validation refusal names the field paths and never the values.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, Request, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use notification_application::{
    DeliveryError, EraseError, FeedError, FieldError, PreferenceError, Principal, RepositoryError,
    RetentionError, SubmitError, SubmitStatus,
};
use notification_domain::{UserId, MAX_IDEMPOTENCY_KEY_LEN};
use serde_json::json;

use crate::dto::{
    delivery_detail, AlertAccepted, AlertRequest, AnonymiseRequest, FeedQuery, FeedResponse,
    PreferencesRequest, PreferencesResponse, RunResponse,
};
use crate::middleware::CorrelationId;
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

fn forbidden(correlation: &str, detail: &str) -> Response {
    problem(403, "forbidden", "Forbidden", detail, correlation)
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

/// Readiness probe: Postgres and its migrations only (NFR1.7).
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
        state
            .metrics
            .render(state.service_name, state.trail.failures()),
    )
        .into_response()
}

/// C9: submit one alert.
pub async fn submit_alert(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
    payload: Result<Json<AlertRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => return body_error(rejection, &correlation.0),
    };
    let kind_label = request.r#type.clone();
    let command = request.into_command(Some(correlation.0.clone()));
    match state.submit.execute(command).await {
        Ok(result) => {
            match result.status {
                SubmitStatus::Queued => state.metrics.record_alert(&kind_label, "queued"),
                SubmitStatus::Deduplicated => {
                    state.metrics.record_duplicate(&kind_label);
                }
            }
            let status = match result.status {
                SubmitStatus::Queued => "queued",
                SubmitStatus::Deduplicated => "deduplicated",
            };
            (
                StatusCode::ACCEPTED,
                Json(AlertAccepted {
                    alert_id: result.alert_id.as_uuid(),
                    status,
                }),
            )
                .into_response()
        }
        Err(SubmitError::Rejected(errors)) => validation(&correlation.0, errors),
        Err(SubmitError::Repository(error)) => repository_error(error, &correlation.0),
    }
}

/// The owner check every user-facing path performs (NFR9.2).
fn owner(principal: &Principal, user_id: &str, correlation: &str) -> Result<(), Box<Response>> {
    if let Err(error) = UserId::parse(user_id) {
        return Err(Box::new(validation(
            correlation,
            vec![FieldError::from(error)],
        )));
    }
    if principal.subject != user_id {
        return Err(Box::new(forbidden(
            correlation,
            "a user may only read their own notifications",
        )));
    }
    Ok(())
}

/// C1: the caller's in-app feed.
pub async fn feed(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
    axum::extract::Extension(principal): axum::extract::Extension<Principal>,
    Path(user_id): Path<String>,
    Query(query): Query<FeedQuery>,
) -> Response {
    if let Err(response) = owner(&principal, &user_id, &correlation.0) {
        return *response;
    }
    match state
        .feed
        .list(&user_id, query.cursor.as_deref(), query.limit)
        .await
    {
        Ok(page) => Json(FeedResponse::from(page)).into_response(),
        Err(FeedError::Rejected(errors)) => validation(&correlation.0, errors),
        Err(FeedError::Repository(error)) => repository_error(error, &correlation.0),
    }
}

/// C1: read preferences.
pub async fn get_preferences(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
    axum::extract::Extension(principal): axum::extract::Extension<Principal>,
    Path(user_id): Path<String>,
) -> Response {
    if let Err(response) = owner(&principal, &user_id, &correlation.0) {
        return *response;
    }
    match state.preferences.view(&user_id).await {
        Ok(view) => Json(PreferencesResponse::from(view)).into_response(),
        Err(PreferenceError::Rejected(errors)) => validation(&correlation.0, errors),
        Err(PreferenceError::Repository(error)) => repository_error(error, &correlation.0),
    }
}

/// C1: save preferences.
pub async fn put_preferences(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
    axum::extract::Extension(principal): axum::extract::Extension<Principal>,
    Path(user_id): Path<String>,
    payload: Result<Json<PreferencesRequest>, JsonRejection>,
) -> Response {
    if let Err(response) = owner(&principal, &user_id, &correlation.0) {
        return *response;
    }
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => return body_error(rejection, &correlation.0),
    };
    match state
        .preferences
        .save(&user_id, request.into_changes(), Some(&correlation.0))
        .await
    {
        Ok(saved) => {
            state.metrics.record_audit_failures(saved.audit_failures);
            Json(PreferencesResponse::from(saved.preferences)).into_response()
        }
        Err(PreferenceError::Rejected(errors)) => validation(&correlation.0, errors),
        Err(PreferenceError::Repository(error)) => repository_error(error, &correlation.0),
    }
}

/// C9: Identity erases a user's notification rows.
pub async fn anonymise(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
    payload: Result<Json<AnonymiseRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => return body_error(rejection, &correlation.0),
    };
    let key = request.idempotency_key.trim();
    if key.is_empty() {
        return validation(
            &correlation.0,
            vec![FieldError::new("idempotencyKey", "must not be empty")],
        );
    }
    if key.chars().count() > MAX_IDEMPOTENCY_KEY_LEN {
        return validation(
            &correlation.0,
            vec![FieldError::new(
                "idempotencyKey",
                format!("must be at most {MAX_IDEMPOTENCY_KEY_LEN} characters"),
            )],
        );
    }
    match state
        .erase
        .anonymise(&request.user_id, Some(&correlation.0))
        .await
    {
        Ok(report) => {
            if report.audit_failed {
                state.metrics.record_audit_failures(1);
            }
            StatusCode::NO_CONTENT.into_response()
        }
        Err(EraseError::Rejected(errors)) => validation(&correlation.0, errors),
        Err(EraseError::Repository(error)) => repository_error(error, &correlation.0),
    }
}

/// C10 smoke: the service is ready.
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

/// C11: run one delivery pass (the fallback path; the worker also polls).
pub async fn run_deliver(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
) -> Response {
    let started_at = state.clock.now();
    match state.delivery.run().await {
        Ok(report) => {
            crate::metrics::record_delivery_report(&state.metrics, &report);
            Json(RunResponse::succeeded(
                "notification-deliver",
                started_at,
                Some(report.run_id.to_string()),
                delivery_detail(&report.alerts),
            ))
            .into_response()
        }
        Err(DeliveryError::Busy) => problem(
            409,
            "run-in-progress",
            "A delivery run is in progress",
            "another delivery run holds the single-flight lease",
            &correlation.0,
        ),
        Err(DeliveryError::Repository(error)) => repository_error(error, &correlation.0),
        Err(DeliveryError::Domain(detail)) => unavailable(&correlation.0, detail),
    }
}

/// C11: run the retention sweep.
pub async fn run_retention(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
) -> Response {
    let started_at = state.clock.now();
    match state.retention.run().await {
        Ok(report) => Json(RunResponse::succeeded(
            "notification-retention",
            started_at,
            None,
            format!(
                "{} alerts and {} attempts deleted",
                report.alerts_deleted, report.attempts_deleted
            ),
        ))
        .into_response(),
        Err(RetentionError::Repository(error)) => repository_error(error, &correlation.0),
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
