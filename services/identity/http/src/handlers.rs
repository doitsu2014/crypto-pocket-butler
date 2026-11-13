//! Route handlers. Every refusal is an RFC 7807 problem carrying the correlation
//! id; a validation refusal names the field paths and never the values.
//!
//! The support-access paths (FR16.3–FR16.6, BR3.9–BR3.13) record their outcome
//! and latency in the `support_*` metric families before the response is built.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use identity_application::common::{AppError, Caller, FieldError};
use identity_application::user_registry::subject_hash;
use identity_domain::{
    DeletionRequestId, DeviceSessionId, ExportRequestId, SupportGrantId, UserId, VerifiedClaims,
};
use serde::Deserialize;
use serde_json::json;

use crate::dto::{
    AccountStatusReadResponse, AccountStatusResponse, AdminLookupRequest, AdminStatusRequest,
    AdminUserResponse, ConsentAppendRequest, ConsentAppendResponse, ConsentResponse,
    ContactResponse, DeletionResponse, ExportResponse, InternalProvisionRequest,
    JurisdictionCheckRequest, JurisdictionCheckResponse, JurisdictionPublishRequest,
    JurisdictionPublishResponse, PendingGrantResponse, PreferencesRequest, PreferencesResponse,
    PrincipalResponse, ProfileAction, ProfileResponse, ProfileUpdateRequest, SessionView,
    SessionsResponse, SignupGateRequest, SignupGateResponse, SupportGrantRequest,
    SupportGrantResponse, SupportMetadataResponse, UserResponse,
};
use crate::middleware::{CorrelationId, ServicePrincipal, ValidatedClaims};
use crate::problem::{bad_request, from_app_error, no_record, validation, ProblemDetails};
use crate::state::AppState;

fn app_error(error: AppError, correlation: &str) -> Response {
    from_app_error(&error, correlation).into_response()
}

// Axum's `Response` is the idiomatic error type for handler helpers; boxing it
// would only move the allocation into every refusal path, so allow the lint here.
#[allow(clippy::result_large_err)]
pub(crate) async fn caller_of(
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

/// The support-access metric outcome for one write: `failed` is the 503 class,
/// every other refusal is `refused`, and success is `accepted`.
fn support_write_outcome(error: Option<&AppError>) -> &'static str {
    match error {
        None => "accepted",
        Some(AppError::Unavailable(_)) => "failed",
        Some(_) => "refused",
    }
}

/// The support-access metric outcome for one metadata read.
fn support_read_outcome(error: Option<&AppError>) -> &'static str {
    match error {
        None => "served",
        Some(AppError::Forbidden(_)) => "refused",
        Some(AppError::Unavailable(_)) => "unavailable",
        Some(_) => "failed",
    }
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
        Ok(view) => {
            state
                .metrics
                .record_principal_status(view.status().as_str());
            Json(PrincipalResponse::from(view)).into_response()
        }
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
    // The legacy path now writes `user_preferences` through the U4 use case, so
    // a write here is read back with source `user`, not `default`, and never
    // touches u1's provider-derived columns.
    if let Err(error) = caller.require_owner(user_id) {
        return app_error(error, &correlation.0);
    }
    let update = match request.into_update() {
        Ok(update) => update,
        Err(errors) => return validation(&correlation.0, errors).into_response(),
    };
    match state.preferences_write.write(&caller, update).await {
        Ok(effective) => Json(PreferencesResponse::from_effective(&effective)).into_response(),
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

/// WF9: confirm account deletion (C7/W2). A second open request is `409`; the
/// response carries `recoveryEndsAt`.
pub async fn request_deletion(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
) -> Response {
    let caller = match caller_for_deletion(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    match state.account_lifecycle.request_deletion(&caller).await {
        Ok(deletion) => {
            state
                .metrics
                .record_deletion_status(deletion.status().as_str());
            // The durable audit intent is the outbox row the transition wrote;
            // this C10 event is the operator-facing privacy record.
            if let Err(error) = state.deletions.record_requested(&caller, &deletion).await {
                tracing::warn!(%error, "failed to record the deletion privacy event");
            }
            (
                StatusCode::ACCEPTED,
                Json(DeletionResponse::from(&deletion)),
            )
                .into_response()
        }
        Err(error) => app_error(error, &correlation.0),
    }
}

/// WF9: the owner cancels an open deletion inside the recovery window (C7/W3).
/// After the deadline the answer is `410 Gone`.
pub async fn cancel_deletion(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(user_id): Path<String>,
) -> Response {
    let user_id = match parse_user_id(&user_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let caller = match caller_for_deletion(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    // Another user's id is `404`, so the route cannot confirm that it exists
    // (NFR9.3).
    if caller.user_id() != user_id {
        return app_error(AppError::NotFound("deletion"), &correlation.0);
    }
    match state.account_lifecycle.cancel_deletion(&caller).await {
        Ok(deletion) => Json(DeletionResponse::from(&deletion)).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// C7/W1: an admin disables an account with a recorded reason.
pub async fn disable_account(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(user_id): Path<String>,
    payload: Result<Json<AdminStatusRequest>, JsonRejection>,
) -> Response {
    admin_status_change(state, correlation, claims, user_id, payload, false).await
}

/// C7/W1: an admin restores a disabled account with a recorded reason.
pub async fn restore_account(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(user_id): Path<String>,
    payload: Result<Json<AdminStatusRequest>, JsonRejection>,
) -> Response {
    admin_status_change(state, correlation, claims, user_id, payload, true).await
}

async fn admin_status_change(
    state: AppState,
    correlation: CorrelationId,
    claims: VerifiedClaims,
    raw_user_id: String,
    payload: Result<Json<AdminStatusRequest>, JsonRejection>,
    restore: bool,
) -> Response {
    let user_id = match parse_user_id(&raw_user_id) {
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
    let answer = if restore {
        state
            .account_lifecycle
            .restore(&caller, user_id, &request.reason)
            .await
    } else {
        state
            .account_lifecycle
            .disable(&caller, user_id, &request.reason)
            .await
    };
    match answer {
        Ok(view) => Json(AccountStatusResponse::from(&view)).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// Builds a caller for the owner deletion paths.
///
/// The principal's normal resolution denies a `pending_deletion` account, which
/// is right for every other route but would make the restore path impossible.
/// The two owner operations therefore resolve the stored record directly; u2's
/// decision still governs every other request from that account (BR5.7).
#[allow(clippy::result_large_err)]
async fn caller_for_deletion(
    state: &AppState,
    claims: VerifiedClaims,
    correlation: &str,
) -> Result<Caller, Response> {
    let user = state
        .users
        .find_by_auth_subject(&claims.subject)
        .await
        .map_err(|error| app_error(AppError::from_repository(error), correlation))?
        .ok_or_else(|| app_error(AppError::NotFound("user"), correlation))?;
    Ok(Caller {
        user,
        claims,
        correlation_id: correlation.to_string(),
    })
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
    let caller = match caller_for_deletion(&state, claims, &correlation.0).await {
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

/// D0: the caller's own account status (U7).
///
/// The caller is resolved from the stored record by auth subject, never with the
/// denying `load_caller`, so a disabled, pending-deletion, or deleted person can
/// learn where their account stands. The route accepts no user id.
pub async fn my_account_status(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
) -> Response {
    let caller = match caller_for_deletion(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    Json(AccountStatusReadResponse::from(
        state.profile.account_status(&caller),
    ))
    .into_response()
}

/// P1: the caller's own profile (U7). A plain owner read, no audit event.
pub async fn my_profile(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
) -> Response {
    let caller = match caller_of(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    Json(ProfileResponse::from(&state.profile.view(&caller))).into_response()
}

/// P2: save a display name (W1) or adopt the provider name again (W2).
pub async fn update_my_profile(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    payload: Result<Json<ProfileUpdateRequest>, JsonRejection>,
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
    let action = match request.action() {
        Ok(action) => action,
        Err(fields) => return validation(&correlation.0, fields).into_response(),
    };
    let caller = match caller_of(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    let result = match action {
        ProfileAction::DisplayName(display_name) => {
            state
                .profile
                .update_display_name(&caller, &display_name)
                .await
        }
        ProfileAction::UseProviderName => state.profile.adopt_provider_name(&caller).await,
    };
    match result {
        Ok(view) => Json(ProfileResponse::from(&view)).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// D1: the caller's open deletion request (U7/W3). Read-only; no audit event.
///
/// An empty answer carries the pinned no-record type so the SPA can tell "there
/// is nothing open" from any other `404`.
pub async fn my_deletion_request(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
) -> Response {
    let caller = match caller_for_deletion(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    match state.account_lifecycle.open_deletion(&caller).await {
        Ok(deletion) => Json(DeletionResponse::from(&deletion)).into_response(),
        Err(AppError::NotFound(_)) => no_record(&correlation.0).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// D2: cancel the caller's open deletion request (U7/W4).
pub async fn cancel_my_deletion_request(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
) -> Response {
    let caller = match caller_for_deletion(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    match state.account_lifecycle.cancel_deletion(&caller).await {
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
        Err(_) => return bad_request(&correlation.0, "invalid-user-id").into_response(),
    };
    let caller = match caller_of(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    let started = std::time::Instant::now();
    let result = state
        .support
        .request(&caller, user_id, &request.reason)
        .await;
    state.metrics.record_support_write(
        "request",
        support_write_outcome(result.as_ref().err()),
        started.elapsed(),
    );
    match result {
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
        Err(_) => return bad_request(&correlation.0, "invalid-grant-id").into_response(),
    };
    let caller = match caller_of(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    let started = std::time::Instant::now();
    let result = state.support.approve(&caller, grant_id).await;
    state.metrics.record_support_write(
        "approve",
        support_write_outcome(result.as_ref().err()),
        started.elapsed(),
    );
    match result {
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
        Err(_) => return bad_request(&correlation.0, "invalid-grant-id").into_response(),
    };
    let caller = match caller_of(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    let started = std::time::Instant::now();
    let result = state.support.revoke(&caller, grant_id).await;
    state.metrics.record_support_write(
        "revoke",
        support_write_outcome(result.as_ref().err()),
        started.elapsed(),
    );
    match result {
        Ok(grant) => Json(SupportGrantResponse::from(&grant)).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// W4: the grant holder reads the target's closed metadata projection.
pub async fn support_grant_metadata(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(grant_id): Path<String>,
) -> Response {
    let grant_id = match SupportGrantId::parse(&grant_id) {
        Ok(id) => id,
        Err(_) => return bad_request(&correlation.0, "invalid-grant-id").into_response(),
    };
    let caller = match caller_of(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    let started = std::time::Instant::now();
    let result = state.support.metadata(&caller, grant_id).await;
    state.metrics.record_support_read(
        support_read_outcome(result.as_ref().err()),
        started.elapsed(),
    );
    if let Err(error) = &result {
        match error {
            AppError::Unavailable(detail) if detail == "audit-unavailable" => {
                state.metrics.record_support_audit_write_failure("read");
            }
            AppError::Internal(_) => state.metrics.record_support_grant_decision_error("lookup"),
            _ => {}
        }
    }
    match result {
        Ok(metadata) => Json(SupportMetadataResponse::from(&metadata)).into_response(),
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

/// C8: evaluate a self-declared country plus the caller-observed IP.
///
/// The endpoint is reachable only through the gateway or the User Registry
/// (service token). A client-supplied `x-forwarded-for` is never a trusted
/// address source, so a request carrying one is refused (NFR23.3).
pub async fn check_jurisdiction(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ServicePrincipal(_service)): Extension<ServicePrincipal>,
    headers: HeaderMap,
    payload: Result<Json<JurisdictionCheckRequest>, JsonRejection>,
) -> Response {
    if headers.contains_key("x-forwarded-for") {
        return validation(
            &correlation.0,
            vec![FieldError::new(
                "ip",
                "must be the address observed by the trusted caller, not a forwarded header",
            )],
        )
        .into_response();
    }
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
        .jurisdiction_check
        .check(&request.country_code, &request.ip, &correlation.0)
        .await
    {
        Ok(result) => {
            state
                .metrics
                .record_jurisdiction_outcome(if result.decision().is_allowed() {
                    "allowed"
                } else {
                    "denied"
                });
            Json(JurisdictionCheckResponse::from_result(&result)).into_response()
        }
        Err(error) => {
            state.metrics.record_jurisdiction_outcome("error");
            app_error(error, &correlation.0)
        }
    }
}

/// W3: publish a new immutable list version. Only an administrator may publish;
/// the role comes from the validated JWT (NFR23.2).
pub async fn publish_jurisdiction(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    payload: Result<Json<JurisdictionPublishRequest>, JsonRejection>,
) -> Response {
    if !claims.role.is_admin() {
        return app_error(
            AppError::Forbidden(
                "only an administrator may publish the jurisdiction list".to_string(),
            ),
            &correlation.0,
        );
    }
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
        .jurisdiction_publish
        .publish(request.into_application(), &correlation.0)
        .await
    {
        Ok(list) => Json(JurisdictionPublishResponse::from_published(&list)).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
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

/// U1: read one user record. The owner or an admin may read it.
pub async fn get_user(
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
    match state.user_registry.by_id(&caller, user_id).await {
        Ok(record) => Json(UserResponse::from(record)).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// The `email` query on the admin user search.
#[derive(Debug, Deserialize)]
pub struct AdminUsersQuery {
    /// The exact address to look up.
    pub email: String,
}

/// U1: an admin looks a user up by exact email (BR1.9).
///
/// U10/W1 (D-S1): the address arrives in the body of `POST
/// /v1/admin/users/lookup`, and the answer carries the role the provider
/// currently reports.
pub async fn lookup_admin_user(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    payload: Result<Json<AdminLookupRequest>, JsonRejection>,
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
    let caller = match caller_of(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    match state
        .user_registry
        .by_email_admin(&caller, &request.email)
        .await
    {
        Ok(view) => Json(AdminUserResponse::from(view)).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// The `status` query on the pending-grant list.
#[derive(Debug, Deserialize)]
pub struct PendingGrantsQuery {
    /// The only accepted value is `pending_approval`.
    #[serde(default)]
    pub status: Option<String>,
}

/// U10/W5: an admin lists the grants awaiting approval.
pub async fn list_pending_grants(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Query(query): Query<PendingGrantsQuery>,
) -> Response {
    if query.status.as_deref() != Some("pending_approval") {
        return validation(
            &correlation.0,
            vec![FieldError::new("status", "must be 'pending_approval'")],
        )
        .into_response();
    }
    let caller = match caller_of(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    match state.support.pending_grants(&caller).await {
        Ok(grants) => Json(
            grants
                .iter()
                .map(PendingGrantResponse::from)
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// S2: a support agent reads their own grants, newest first, at most twenty.
pub async fn my_support_grants(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
) -> Response {
    let caller = match caller_of(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    match state.support.my_grants(&caller).await {
        Ok(grants) => Json(
            grants
                .iter()
                .map(PendingGrantResponse::from)
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// C3: read one user record by internal id for a trusted service caller.
pub async fn internal_get_user(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ServicePrincipal(_service)): Extension<ServicePrincipal>,
    Path(user_id): Path<String>,
) -> Response {
    let user_id = match parse_user_id(&user_id) {
        Ok(id) => id,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    match state.user_registry.internal_by_id(user_id).await {
        Ok(record) => Json(UserResponse::from(record)).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// C3: read one user record by hashed provider subject.
pub async fn internal_get_by_subject(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ServicePrincipal(_service)): Extension<ServicePrincipal>,
    Path(expected_hash): Path<String>,
) -> Response {
    match state
        .user_registry
        .internal_by_subject_hash(&expected_hash)
        .await
    {
        Ok(record) => Json(UserResponse::from(record)).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// C3: read one user record by exact email for a trusted service caller.
pub async fn internal_lookup(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ServicePrincipal(_service)): Extension<ServicePrincipal>,
    Query(query): Query<AdminUsersQuery>,
) -> Response {
    match state.user_registry.internal_by_email(&query.email).await {
        Ok(record) => Json(UserResponse::from(record)).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// C3: provision or refresh a user from validated claims.
///
/// The path carries the subject hash; the body carries the plaintext subject
/// the caller validated. The two must agree, and the jurisdiction gate must
/// clear the provider before anything is written.
pub async fn internal_provision(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ServicePrincipal(_service)): Extension<ServicePrincipal>,
    Path(expected_hash): Path<String>,
    headers: HeaderMap,
    payload: Result<Json<InternalProvisionRequest>, JsonRejection>,
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
    if subject_hash(&request.subject) != expected_hash {
        return validation(
            &correlation.0,
            vec![FieldError::new(
                "subjectHash",
                "does not match the supplied subject",
            )],
        )
        .into_response();
    }
    let country = header_of(&headers, "x-country-code");
    let ip = header_of(&headers, "x-client-ip");
    let claims = match request.into_claims(country.as_deref(), ip.as_deref()) {
        Ok(claims) => claims,
        Err(error) => {
            return validation(
                &correlation.0,
                vec![FieldError::new(error.field(), error.to_string())],
            )
            .into_response();
        }
    };
    match state
        .principal
        .resolve_with_origin(claims, &correlation.0)
        .await
    {
        Ok((view, created)) => {
            state
                .metrics
                .record_principal_status(view.status().as_str());
            match state.user_registry.internal_by_id(view.user_id()).await {
                Ok(record) => {
                    let status = if created {
                        StatusCode::CREATED
                    } else {
                        StatusCode::OK
                    };
                    (status, Json(UserResponse::from(record))).into_response()
                }
                Err(error) => app_error(error, &correlation.0),
            }
        }
        Err(error) => app_error(error, &correlation.0),
    }
}

/// A trimmed trusted-caller header value, when present.
fn header_of(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
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
