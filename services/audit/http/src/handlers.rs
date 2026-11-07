//! Route handlers. Every failure is an RFC 7807 problem carrying the correlation
//! id; a validation refusal names the field paths and never the values.

use audit_application::append::{AppendError, Emitter};
use audit_application::export::ExportError;
use audit_application::ports::{Principal, RepositoryError};
use audit_application::privacy::PrivacyError;
use audit_application::query::QueryError;
use audit_application::retention::RetentionError;
use audit_application::support::SupportError;
use audit_application::FieldError;
use audit_domain::SupportSessionId;
use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use crate::dto::{
    AppendEventRequest, AppendEventResponse, CheckParams, CheckResponse, EventPageResponse,
    EventQueryParams, ExportRequest, PrivacyDeletionRequest, PrivacyDeletionResponse,
    RetentionReportRequest, RetentionRunResponse, StartSessionRequest, SupportSessionResponse,
};
use crate::export_stream::stream_export;
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

fn not_found(correlation: &str, detail: impl Into<String>) -> Response {
    problem(404, "not-found", "Not found", detail, correlation)
}

/// Liveness probe.
pub async fn health(State(state): State<AppState>) -> Response {
    Json(json!({ "status": "ok", "service": state.service_name })).into_response()
}

/// Readiness probe: the database and the migrations table.
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

/// WF1: append one audit event.
pub async fn append_event(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
    axum::extract::Extension(service): axum::extract::Extension<ServicePrincipal>,
    headers: HeaderMap,
    payload: Result<Json<AppendEventRequest>, JsonRejection>,
) -> Response {
    let idempotency_key = headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let Some(idempotency_key) = idempotency_key else {
        return validation(
            &correlation.0,
            vec![FieldError::new(
                "idempotencyKey",
                "the Idempotency-Key header is required",
            )],
        );
    };
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => {
            return validation(
                &correlation.0,
                vec![FieldError::new("body", rejection.body_text())],
            );
        }
    };

    let emitter = Emitter {
        service: service.0.clone(),
    };
    match state
        .append
        .execute(&emitter, request.into_command(Some(idempotency_key)))
        .await
    {
        Ok(result) => {
            state.metrics.record_append(false);
            (StatusCode::CREATED, Json(AppendEventResponse::from(result))).into_response()
        }
        Err(error) => {
            state
                .metrics
                .record_append(matches!(error, AppendError::Repository(_)));
            append_error(error, &correlation.0)
        }
    }
}

fn append_error(error: AppendError, correlation: &str) -> Response {
    match error {
        AppendError::Rejected(errors) => validation(correlation, errors),
        AppendError::PayloadMismatch => problem(
            409,
            "idempotency-conflict",
            "Conflict",
            "the Idempotency-Key was reused with a different payload",
            correlation,
        ),
        AppendError::Repository(repository) => repository_error(repository, correlation),
    }
}

/// WF4: query the trail (admin).
pub async fn list_events(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
    axum::extract::Extension(principal): axum::extract::Extension<Principal>,
    query: Result<Query<EventQueryParams>, QueryRejection>,
) -> Response {
    let Query(params) = match query {
        Ok(query) => query,
        Err(rejection) => {
            return validation(
                &correlation.0,
                vec![FieldError::new("query", rejection.body_text())],
            );
        }
    };
    state.metrics.record_query();
    match state.query.execute(&principal, params.into()).await {
        Ok(page) => Json(EventPageResponse {
            events: page.events.iter().map(Into::into).collect(),
            next_cursor: page.next_cursor,
        })
        .into_response(),
        Err(error) => query_error(error, &correlation.0),
    }
}

fn query_error(error: QueryError, correlation: &str) -> Response {
    match error {
        QueryError::Forbidden => problem(
            403,
            "forbidden",
            "Forbidden",
            "admin role required",
            correlation,
        ),
        QueryError::Rejected(errors) => validation(correlation, errors),
        QueryError::Repository(detail) => {
            if is_grant_refusal(&detail) {
                problem(
                    403,
                    "grant-refused",
                    "Refused by the database",
                    detail,
                    correlation,
                )
            } else {
                unavailable(correlation, detail)
            }
        }
    }
}

/// WF5: record and stream an export (admin). The response body is the export.
pub async fn create_export(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
    axum::extract::Extension(principal): axum::extract::Extension<Principal>,
    payload: Result<Json<ExportRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => {
            return validation(
                &correlation.0,
                vec![FieldError::new("body", rejection.body_text())],
            );
        }
    };
    let filter = match request.into_filter() {
        Ok(filter) => filter,
        Err(error) => return validation(&correlation.0, vec![FieldError::from(error)]),
    };
    match state.export.start(&principal, filter).await {
        Ok(session) => {
            state.metrics.record_export();
            stream_export(session)
        }
        Err(error) => export_error(error, &correlation.0),
    }
}

fn export_error(error: ExportError, correlation: &str) -> Response {
    match error {
        ExportError::Forbidden => problem(
            403,
            "forbidden",
            "Forbidden",
            "admin role required",
            correlation,
        ),
        ExportError::Rejected(errors) => validation(correlation, errors),
        ExportError::TooLarge { cap } => problem(
            422,
            "export-too-large",
            "The export is too large",
            format!("more than {cap} rows match; narrow the filter"),
            correlation,
        ),
        ExportError::Failed(detail) => problem(
            502,
            "export-failed",
            "The export could not be produced",
            detail,
            correlation,
        ),
        ExportError::Repository(detail) => {
            if is_grant_refusal(&detail) {
                problem(
                    403,
                    "grant-refused",
                    "Refused by the database",
                    detail,
                    correlation,
                )
            } else {
                unavailable(correlation, detail)
            }
        }
    }
}

/// WF2: start a support session (support).
pub async fn create_support_session(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
    axum::extract::Extension(principal): axum::extract::Extension<Principal>,
    payload: Result<Json<StartSessionRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => {
            return validation(
                &correlation.0,
                vec![FieldError::new("body", rejection.body_text())],
            );
        }
    };
    match state.support.request(&principal, request.into()).await {
        Ok(session) => (
            StatusCode::CREATED,
            Json(SupportSessionResponse::from(&session)),
        )
            .into_response(),
        Err(error) => support_error(error, &correlation.0),
    }
}

fn support_error(error: SupportError, correlation: &str) -> Response {
    match error {
        SupportError::Forbidden => problem(
            403,
            "forbidden",
            "Forbidden",
            "support role required",
            correlation,
        ),
        SupportError::Rejected(errors) => validation(correlation, errors),
        SupportError::Denied { denial, .. } => problem(
            403,
            "support-session-denied",
            "Support session denied",
            format!("{} ({})", denial, denial.code()),
            correlation,
        ),
        SupportError::NotFound => not_found(correlation, "support session"),
        SupportError::Conflict(detail) => problem(409, "conflict", "Conflict", detail, correlation),
        SupportError::Repository(detail) => {
            if is_grant_refusal(&detail) {
                problem(
                    403,
                    "grant-refused",
                    "Refused by the database",
                    detail,
                    correlation,
                )
            } else {
                unavailable(correlation, detail)
            }
        }
    }
}

/// WF2: the named second approver activates a requested support session.
pub async fn approve_support_session(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
    axum::extract::Extension(principal): axum::extract::Extension<Principal>,
    path: Result<Path<String>, PathRejection>,
) -> Response {
    let Path(id) = match path {
        Ok(path) => path,
        Err(rejection) => {
            return validation(
                &correlation.0,
                vec![FieldError::new("id", rejection.body_text())],
            );
        }
    };
    let Ok(session_id) = SupportSessionId::parse(&id) else {
        return not_found(&correlation.0, "support session");
    };
    match state.support.approve(&principal, session_id).await {
        Ok(session) => Json(SupportSessionResponse::from(&session)).into_response(),
        Err(error) => support_error(error, &correlation.0),
    }
}

/// WF3: answer a session check for the gateway or the owning service.
pub async fn check_support_session(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
    Path(id): Path<String>,
    axum::extract::Extension(service): axum::extract::Extension<ServicePrincipal>,
    query: Result<Query<CheckParams>, QueryRejection>,
) -> Response {
    let Ok(session_id) = SupportSessionId::parse(&id) else {
        return not_found(&correlation.0, "support session");
    };
    let Query(params) = match query {
        Ok(query) => query,
        Err(rejection) => {
            return validation(
                &correlation.0,
                vec![FieldError::new("scope", rejection.body_text())],
            );
        }
    };
    let scope = match params.into_scope() {
        Ok(scope) => scope,
        Err(error) => return validation(&correlation.0, vec![FieldError::from(error)]),
    };
    state.metrics.record_support_check();
    let decision = state.support.check(session_id, scope, &service.0).await;
    Json(CheckResponse::from(decision)).into_response()
}

/// Revoke an active support session (admin).
pub async fn revoke_support_session(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
    axum::extract::Extension(principal): axum::extract::Extension<Principal>,
    path: Result<Path<String>, PathRejection>,
) -> Response {
    let Path(id) = match path {
        Ok(path) => path,
        Err(rejection) => {
            return validation(
                &correlation.0,
                vec![FieldError::new("id", rejection.body_text())],
            );
        }
    };
    let Ok(session_id) = SupportSessionId::parse(&id) else {
        return not_found(&correlation.0, "support session");
    };
    match state.support.revoke(&principal, session_id).await {
        Ok(session) => Json(SupportSessionResponse::from(&session)).into_response(),
        Err(error) => support_error(error, &correlation.0),
    }
}

/// WF7: record Identity's deletion request.
pub async fn create_privacy_deletion(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
    axum::extract::Extension(service): axum::extract::Extension<ServicePrincipal>,
    payload: Result<Json<PrivacyDeletionRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => {
            return validation(
                &correlation.0,
                vec![FieldError::new("body", rejection.body_text())],
            );
        }
    };
    match state.privacy.request(&service.0, request.into()).await {
        Ok(deletion) => (
            StatusCode::ACCEPTED,
            Json(PrivacyDeletionResponse::from(&deletion)),
        )
            .into_response(),
        Err(error) => privacy_error(error, &correlation.0),
    }
}

fn privacy_error(error: PrivacyError, correlation: &str) -> Response {
    match error {
        PrivacyError::Forbidden(detail) => {
            problem(403, "forbidden", "Forbidden", detail, correlation)
        }
        PrivacyError::Rejected(errors) => validation(correlation, errors),
        PrivacyError::Repository(detail) => {
            if is_grant_refusal(&detail) {
                problem(
                    403,
                    "grant-refused",
                    "Refused by the database",
                    detail,
                    correlation,
                )
            } else {
                unavailable(correlation, detail)
            }
        }
        PrivacyError::Inconsistent(detail) => problem(
            500,
            "deletion-inconsistent",
            "The deletion could not be completed",
            detail,
            correlation,
        ),
    }
}

/// WF6: record another service's completed retention run (C11).
pub async fn report_retention_run(
    State(state): State<AppState>,
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
    axum::extract::Extension(service): axum::extract::Extension<ServicePrincipal>,
    payload: Result<Json<RetentionReportRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => {
            return validation(
                &correlation.0,
                vec![FieldError::new("body", rejection.body_text())],
            );
        }
    };
    match state.retention.report(&service.0, request.into()).await {
        Ok(run) => {
            state.metrics.record_retention_run();
            (StatusCode::CREATED, Json(RetentionRunResponse::from(&run))).into_response()
        }
        Err(error) => retention_error(error, &correlation.0),
    }
}

fn retention_error(error: RetentionError, correlation: &str) -> Response {
    match error {
        RetentionError::Rejected(errors) => validation(correlation, errors),
        RetentionError::Forbidden(detail) => {
            problem(403, "forbidden", "Forbidden", detail, correlation)
        }
        RetentionError::CouldNotStart(detail) => unavailable(correlation, detail),
        RetentionError::RunFailed(detail) => problem(
            503,
            "retention-failed",
            "The retention run failed",
            detail,
            correlation,
        ),
        RetentionError::Repository(detail) => {
            if is_grant_refusal(&detail) {
                problem(
                    403,
                    "grant-refused",
                    "Refused by the database",
                    detail,
                    correlation,
                )
            } else {
                unavailable(correlation, detail)
            }
        }
    }
}

fn is_grant_refusal(detail: &str) -> bool {
    detail.contains("refused:")
}

/// The deny-by-default fallback.
pub async fn fallback(
    axum::extract::Extension(correlation): axum::extract::Extension<CorrelationId>,
) -> Response {
    problem(
        404,
        "not-found",
        "Not found",
        "no route matches this path",
        &correlation.0,
    )
}
