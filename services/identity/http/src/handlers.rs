//! Route handlers. Every refusal is an RFC 7807 problem carrying the correlation
//! id; a validation refusal names the field paths and never the values.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use identity_application::common::{AppError, Caller, FieldError};
use identity_domain::{
    DeletionRequestId, DeviceSessionId, ExportRequestId, SupportGrantId, UserId, VerifiedClaims,
};
use serde::Deserialize;
use serde_json::json;

use crate::dto::{
    ConsentAppendRequest, ConsentAppendResponse, ConsentResponse, ContactResponse,
    DeletionResponse, ExportResponse, PreferencesRequest, PreferencesResponse, PrincipalResponse,
    SessionView, SessionsResponse, SignupGateRequest, SignupGateResponse, SupportCheckResponse,
    SupportGrantRequest, SupportGrantResponse,
};
use crate::middleware::{CorrelationId, ServicePrincipal, ValidatedClaims};
use crate::problem::{from_app_error, validation, ProblemDetails};
use crate::state::AppState;

fn app_error(error: AppError, correlation: &str) -> Response {
    from_app_error(&error, correlation).into_response()
}

async fn caller_of(
    state: &AppState,
    claims: VerifiedClaims,
    correlation: &str,
) -> Result<Caller, Response> {
    state
        .principal
        .load_caller(claims, correlation)
        .await
        .map_err(|error| app_error(error, correlation))
}

fn parse_user_id(raw: &str) -> Result<UserId, FieldError> {
    UserId::parse(raw).map_err(|_| FieldError::new("userId", "is not a UUID"))
}

/// Liveness probe.
pub async fn health(State(state): State<AppState>) -> Response {
    Json(json!({ "status": "ok", "service": state.service_name })).into_response()
}

/// Readiness probe: the database and its migration table.
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

/// WF1: resolve a validated JWT to an internal principal.
pub async fn resolve_principal(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(auth_user_id): Path<String>,
) -> Response {
    if auth_user_id != claims.subject {
        return app_error(
            AppError::Forbidden("the path subject does not match the validated token".to_string()),
            &correlation.0,
        );
    }
    match state.principal.resolve(claims, &correlation.0).await {
        Ok(view) => Json(PrincipalResponse::from(view)).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// C3: a service caller reads the user's verified contact at delivery time.
pub async fn get_contact(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ServicePrincipal(_service)): Extension<ServicePrincipal>,
    Path(user_id): Path<String>,
) -> Response {
    let user_id = match parse_user_id(&user_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    match state.principal.contact(user_id).await {
        Ok(view) => Json(ContactResponse::from(view)).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// WF6: the pre-authentication jurisdiction and age gate.
pub async fn signup_gate(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    payload: Result<Json<SignupGateRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => {
            return validation(
                &correlation.0,
                vec![FieldError::new("body", rejection.body_text())],
            )
            .into_response();
        }
    };
    match state
        .signup_gate
        .evaluate(
            &request.jurisdiction,
            request.declared_age,
            request.disclosures_acknowledged,
            &correlation.0,
        )
        .await
    {
        Ok(answer) => Json(SignupGateResponse::from_tokens(
            answer.gate_token().map(ToString::to_string),
            answer.disclosures_required(),
        ))
        .into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// WF4: append one consent decision.
pub async fn append_consent(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    payload: Result<Json<ConsentAppendRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => {
            return validation(
                &correlation.0,
                vec![FieldError::new("body", rejection.body_text())],
            )
            .into_response();
        }
    };
    let purpose = match request.purpose() {
        Ok(purpose) => purpose,
        Err(error) => {
            return validation(
                &correlation.0,
                vec![FieldError::new(error.field(), error.to_string())],
            )
            .into_response();
        }
    };
    let source = match request.source() {
        Ok(source) => source,
        Err(error) => {
            return validation(
                &correlation.0,
                vec![FieldError::new(error.field(), error.to_string())],
            )
            .into_response();
        }
    };
    let caller = match caller_of(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    match state
        .consent
        .append(&caller, purpose, request.granted, source)
        .await
    {
        Ok(record) => (
            StatusCode::CREATED,
            Json(ConsentAppendResponse::from(&record)),
        )
            .into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// WF4: read the newest consent decision per purpose.
pub async fn get_consent(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(user_id): Path<String>,
) -> Response {
    let user_id = match parse_user_id(&user_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let caller = match caller_of(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    match state.consent.get(&caller, user_id).await {
        Ok(view) => Json(ConsentResponse::from(view)).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// WF4: change locale, base currency, or timezone.
pub async fn update_preferences(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(user_id): Path<String>,
    payload: Result<Json<PreferencesRequest>, JsonRejection>,
) -> Response {
    let user_id = match parse_user_id(&user_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => {
            return validation(
                &correlation.0,
                vec![FieldError::new("body", rejection.body_text())],
            )
            .into_response();
        }
    };
    let caller = match caller_of(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    let preferences = match request.into_preferences(&caller.user) {
        Ok(preferences) => preferences,
        Err(errors) => return validation(&correlation.0, errors).into_response(),
    };
    match state
        .preferences
        .update(&caller, user_id, preferences)
        .await
    {
        Ok(user) => Json(PreferencesResponse::from(&user)).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// WF3: list the user's device sessions.
pub async fn list_sessions(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(user_id): Path<String>,
) -> Response {
    let user_id = match parse_user_id(&user_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let caller = match caller_of(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    let current = caller.claims.session_id.clone();
    match state.sessions.list(&caller, user_id).await {
        Ok(sessions) => Json(SessionsResponse {
            sessions: sessions
                .iter()
                .map(|session| SessionView::from_session(session, current.as_deref()))
                .collect(),
        })
        .into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// The `confirmCurrent` query on a session revoke.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmQuery {
    /// Whether revoking the current session was confirmed in the UI.
    #[serde(default)]
    pub confirm_current: bool,
}

/// WF3: revoke one session.
pub async fn revoke_session(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path((user_id, session_id)): Path<(String, String)>,
    Query(query): Query<ConfirmQuery>,
) -> Response {
    let user_id = match parse_user_id(&user_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let session_id = match DeviceSessionId::parse(&session_id) {
        Ok(id) => id,
        Err(_) => {
            return validation(
                &correlation.0,
                vec![FieldError::new("sessionId", "is not a UUID")],
            )
            .into_response();
        }
    };
    let caller = match caller_of(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    match state
        .sessions
        .revoke(&caller, user_id, session_id, query.confirm_current)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// WF8: request an export.
pub async fn request_export(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
) -> Response {
    let caller = match caller_of(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    match state.exports.request(&caller).await {
        Ok(export) => {
            state.metrics.record_export_status(export.status().as_str());
            (
                StatusCode::ACCEPTED,
                Json(ExportResponse::from_export(&export, None)),
            )
                .into_response()
        }
        Err(error) => app_error(error, &correlation.0),
    }
}

/// WF8: poll export progress and the signed download.
pub async fn get_export(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(export_id): Path<String>,
) -> Response {
    let export_id = match ExportRequestId::parse(&export_id) {
        Ok(id) => id,
        Err(_) => {
            return validation(
                &correlation.0,
                vec![FieldError::new("exportId", "is not a UUID")],
            )
            .into_response();
        }
    };
    let caller = match caller_of(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    match state.exports.status(&caller, export_id).await {
        Ok(view) => {
            state
                .metrics
                .record_export_status(view.export().status().as_str());
            Json(ExportResponse::from_export(
                view.export(),
                view.signed_download().map(ToString::to_string),
            ))
            .into_response()
        }
        Err(error) => app_error(error, &correlation.0),
    }
}

/// WF9: confirm account deletion.
pub async fn request_deletion(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
) -> Response {
    let caller = match caller_of(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    match state.deletions.request(&caller).await {
        Ok(deletion) => {
            state
                .metrics
                .record_deletion_status(deletion.status().as_str());
            (
                StatusCode::ACCEPTED,
                Json(DeletionResponse::from(&deletion)),
            )
                .into_response()
        }
        Err(error) => app_error(error, &correlation.0),
    }
}

/// WF9: poll deletion state.
pub async fn get_deletion(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(deletion_id): Path<String>,
) -> Response {
    let deletion_id = match DeletionRequestId::parse(&deletion_id) {
        Ok(id) => id,
        Err(_) => {
            return validation(
                &correlation.0,
                vec![FieldError::new("deletionId", "is not a UUID")],
            )
            .into_response();
        }
    };
    let caller = match caller_of(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    match state.deletions.status(&caller, deletion_id).await {
        Ok(deletion) => {
            state
                .metrics
                .record_deletion_status(deletion.status().as_str());
            Json(DeletionResponse::from(&deletion)).into_response()
        }
        Err(error) => app_error(error, &correlation.0),
    }
}

/// WF7: request a support grant.
pub async fn request_support_grant(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    payload: Result<Json<SupportGrantRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => {
            return validation(
                &correlation.0,
                vec![FieldError::new("body", rejection.body_text())],
            )
            .into_response();
        }
    };
    let user_id = match parse_user_id(&request.user_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let caller = match caller_of(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    match state
        .support
        .request(&caller, user_id, &request.reason)
        .await
    {
        Ok(grant) => (
            StatusCode::CREATED,
            Json(SupportGrantResponse::from(&grant)),
        )
            .into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// WF7: an admin approves a grant.
pub async fn approve_support_grant(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(grant_id): Path<String>,
) -> Response {
    let grant_id = match SupportGrantId::parse(&grant_id) {
        Ok(id) => id,
        Err(_) => {
            return validation(
                &correlation.0,
                vec![FieldError::new("grantId", "is not a UUID")],
            )
            .into_response();
        }
    };
    let caller = match caller_of(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    match state.support.approve(&caller, grant_id).await {
        Ok(grant) => Json(SupportGrantResponse::from(&grant)).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// WF7: an admin revokes a grant.
pub async fn revoke_support_grant(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(grant_id): Path<String>,
) -> Response {
    let grant_id = match SupportGrantId::parse(&grant_id) {
        Ok(id) => id,
        Err(_) => {
            return validation(
                &correlation.0,
                vec![FieldError::new("grantId", "is not a UUID")],
            )
            .into_response();
        }
    };
    let caller = match caller_of(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    match state.support.revoke(&caller, grant_id).await {
        Ok(grant) => Json(SupportGrantResponse::from(&grant)).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// The target user on the support-grant check.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckQuery {
    /// The user whose metadata the read covers.
    pub user_id: String,
}

/// WF7: verify a grant on the Account metadata read path.
pub async fn check_support_grant(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ServicePrincipal(_service)): Extension<ServicePrincipal>,
    Path(grant_id): Path<String>,
    Query(query): Query<CheckQuery>,
) -> Response {
    let grant_id = match SupportGrantId::parse(&grant_id) {
        Ok(id) => id,
        Err(_) => {
            return validation(
                &correlation.0,
                vec![FieldError::new("grantId", "is not a UUID")],
            )
            .into_response();
        }
    };
    let user_id = match parse_user_id(&query.user_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    match state.support.check(grant_id, user_id, &correlation.0).await {
        Ok(grant) => Json(SupportCheckResponse::from_grant(&grant)).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// C11: Job Orchestration asks Identity to resume the privacy sweep.
pub async fn run_privacy_sweep(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ServicePrincipal(_service)): Extension<ServicePrincipal>,
) -> Response {
    match state.wake.wake().await {
        Ok(()) => (StatusCode::ACCEPTED, Json(json!({ "status": "accepted" }))).into_response(),
        Err(detail) => ProblemDetails::new(
            503,
            "worker-unavailable",
            "Privacy worker unavailable",
            Some(detail),
            &correlation.0,
        )
        .into_response(),
    }
}

/// Any unregistered path is a problem document.
pub async fn fallback(Extension(correlation): Extension<CorrelationId>) -> Response {
    ProblemDetails::not_found(&correlation.0, "the route does not exist").into_response()
}

/// `GET /v1/internal/users/due` query string (C3).
#[derive(Debug, Deserialize)]
pub struct DueQuery {
    /// `daily` or `weekly`.
    #[serde(default)]
    pub period: Option<String>,
    /// The instant the read is measured at; defaults to now.
    #[serde(rename = "at", default)]
    pub at: Option<String>,
}

/// C3: the users Reporting should consider for a period.
pub async fn get_due_users(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ServicePrincipal(_service)): Extension<ServicePrincipal>,
    Query(query): Query<DueQuery>,
) -> Response {
    let period = match query.period.as_deref() {
        Some(period) if !period.trim().is_empty() => period.to_string(),
        _ => {
            return validation(
                &correlation.0,
                vec![FieldError::new("period", "must be daily or weekly")],
            )
            .into_response()
        }
    };
    let at = match query.at.as_deref() {
        None => chrono::Utc::now(),
        Some(raw) => match chrono::DateTime::parse_from_rfc3339(raw) {
            Ok(value) => value.with_timezone(&chrono::Utc),
            Err(_) => {
                return validation(
                    &correlation.0,
                    vec![FieldError::new("at", "is not an RFC 3339 timestamp")],
                )
                .into_response()
            }
        },
    };
    match state.due_users.due(&period, at).await {
        Ok(due) => {
            let body: Vec<serde_json::Value> = due
                .into_iter()
                .map(|user| {
                    json!({
                        "userId": user.user_id.to_string(),
                        "timezone": user.timezone,
                    })
                })
                .collect();
            Json(body).into_response()
        }
        Err(error) => app_error(error, &correlation.0),
    }
}
