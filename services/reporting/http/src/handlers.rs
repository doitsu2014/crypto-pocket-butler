//! Route handlers. Every refusal is an RFC 7807 problem carrying the correlation
//! id; a validation refusal names the field paths and never the values.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use reporting_application::common::{AppError, Caller, FieldError, ServiceCaller};
use reporting_application::ports::RegisteredService;
use reporting_domain::{BriefId, UserId};
use serde_json::json;

use crate::dto::{
    AnonymiseRequest, AnonymiseResponse, BriefDto, BriefSummaryDto, ExportQuery, ListQuery,
};
use crate::middleware::{CorrelationId, ServicePrincipal, ValidatedClaims};
use crate::problem::{from_app_error, validation, ProblemDetails};
use crate::state::AppState;

fn field(field: &'static str, reason: impl Into<String>) -> FieldError {
    FieldError::new(field, reason)
}

fn caller_of(claims: &reporting_application::common::VerifiedClaims, correlation: &str) -> Caller {
    Caller::from_claims(claims, correlation)
}

fn service_caller(principal: &RegisteredService, correlation: &str) -> ServiceCaller {
    ServiceCaller::from_registered(principal, correlation)
}

fn parse_brief_id(raw: &str) -> Result<BriefId, FieldError> {
    BriefId::parse(raw).map_err(|_| field("briefId", "is not a UUID"))
}

fn parse_user_id(raw: &str) -> Result<UserId, FieldError> {
    UserId::parse(raw).map_err(|_| field("userId", "is not a UUID"))
}

fn body_error(correlation: &str, rejection: JsonRejection) -> Response {
    validation(correlation, vec![field("body", rejection.body_text())]).into_response()
}

/// Liveness probe.
pub async fn health(State(state): State<AppState>) -> Response {
    Json(json!({ "status": "ok", "service": state.service_name })).into_response()
}

/// Readiness: the service's own database only (NFR1.7).
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

/// Prometheus exposition.
pub async fn metrics(State(state): State<AppState>) -> Response {
    (
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        state.metrics.render(state.service_name),
    )
        .into_response()
}

/// BR3.1: the caller's briefs, newest first, optionally narrowed to a period.
pub async fn list_briefs(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(user_id): Path<String>,
    Query(query): Query<ListQuery>,
) -> Response {
    let user_id = match parse_user_id(&user_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let caller = caller_of(&claims, &correlation.0);
    if caller.user_id != user_id {
        return from_app_error(
            &AppError::Forbidden("the brief list belongs to another user".to_string()),
            &correlation.0,
        )
        .into_response();
    }
    let period = match query.period.as_deref() {
        None => None,
        Some(raw) => match reporting_domain::Period::parse(raw) {
            Ok(period) => Some(period),
            Err(_) => {
                return validation(
                    &correlation.0,
                    vec![field("period", "must be daily or weekly")],
                )
                .into_response()
            }
        },
    };
    match state
        .briefs
        .list(&caller, period, query.limit, query.cursor)
        .await
    {
        Ok(briefs) => {
            let body: Vec<BriefSummaryDto> = briefs.iter().map(BriefSummaryDto::from).collect();
            Json(body).into_response()
        }
        Err(error) => from_app_error(&error, &correlation.0).into_response(),
    }
}

/// BR3.1: one brief with its sections and delivery attempts. Another user's
/// brief is reported as not found.
pub async fn get_brief(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path((user_id, brief_id)): Path<(String, String)>,
) -> Response {
    let user_id = match parse_user_id(&user_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let brief_id = match parse_brief_id(&brief_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let caller = caller_of(&claims, &correlation.0);
    if caller.user_id != user_id {
        return ProblemDetails::not_found(&correlation.0, "brief was not found").into_response();
    }
    match state.briefs.open(&caller, brief_id).await {
        Ok(view) => Json(BriefDto::from(view)).into_response(),
        Err(error) => from_app_error(&error, &correlation.0).into_response(),
    }
}

/// C1/internal: the user's briefs for Identity's privacy export.
pub async fn privacy_export(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ServicePrincipal(principal)): Extension<ServicePrincipal>,
    Query(query): Query<ExportQuery>,
) -> Response {
    let user_id = match parse_user_id(&query.user_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let caller = service_caller(&principal, &correlation.0);
    match state.privacy.export(&caller, user_id).await {
        Ok(export) => Json(export).into_response(),
        Err(error) => from_app_error(&error, &correlation.0).into_response(),
    }
}

/// Internal: Identity's erasure calls this to replace the brief subject.
pub async fn privacy_anonymise(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ServicePrincipal(principal)): Extension<ServicePrincipal>,
    payload: Result<Json<AnonymiseRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => return body_error(&correlation.0, rejection),
    };
    let user_id = match parse_user_id(&request.user_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let caller = service_caller(&principal, &correlation.0);
    match state
        .privacy
        .anonymise(&caller, user_id, &request.idempotency_key)
        .await
    {
        Ok(result) => (
            StatusCode::OK,
            Json(AnonymiseResponse {
                anonymised_ref: result.anonymised_ref.to_string(),
                briefs_changed: result.briefs_changed,
                completed_at: result.completed_at,
                already_applied: result.already_applied,
            }),
        )
            .into_response(),
        Err(error) => from_app_error(&error, &correlation.0).into_response(),
    }
}

/// Internal smoke read for the deploy job.
pub async fn smoke(
    State(state): State<AppState>,
    Extension(ServicePrincipal(principal)): Extension<ServicePrincipal>,
) -> Response {
    let correlation = "smoke";
    let caller = service_caller(&principal, correlation);
    if let Err(error) = caller.require_operation(reporting_domain::Operation::RunBriefGeneration) {
        return from_app_error(&error, correlation).into_response();
    }
    Json(json!({ "status": "ok", "service": state.service_name, "caller": principal.name }))
        .into_response()
}

/// A path with no rule is not served.
pub async fn fallback(Extension(correlation): Extension<CorrelationId>) -> Response {
    ProblemDetails::not_found(&correlation.0, "the route does not exist").into_response()
}
