//! U4 HTTP boundary: the `/v1/me/preferences` and `/v1/me/consent` DTOs and
//! handlers, and the C6 internal consent-state read.
//!
//! The caller's identity always comes from the validated JWT; the C6 path id is
//! only a check that the answer is for the token's own user. Another user's id
//! answers 404, indistinguishable from absent (BR4.3).

use std::collections::BTreeMap;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Utc};
use identity_application::common::{AppError, Caller, FieldError};
use identity_application::consent::ports::{HistoryCursor, PreferenceWriteOutcome};
use identity_application::consent::WithdrawResult;
use identity_domain::consent::{
    parse_iana_timezone, BaseCurrencyCode, ConsentEvent, ConsentKind, ConsentPolicy,
    ConsentStateView, EffectivePreferences, PreferenceUpdate, PreferenceValue,
};
use identity_domain::{Locale, UserId, VerifiedClaims};
use serde::Deserialize;
use serde_json::Value;

use crate::handlers::caller_of;
use crate::middleware::{CorrelationId, ServicePrincipal, ValidatedClaims};
use crate::problem::{from_app_error, validation, ProblemDetails};
use crate::state::AppState;

fn app_error(error: AppError, correlation: &str) -> Response {
    from_app_error(&error, correlation).into_response()
}

fn field(error: identity_domain::DomainError) -> FieldError {
    FieldError::new(error.field(), error.to_string())
}

fn invalid_body(rejection: JsonRejection, correlation: &str) -> Response {
    validation(
        correlation,
        vec![FieldError::new("body", rejection.body_text())],
    )
    .into_response()
}

// ---------------------------------------------------------------------------
// Preferences
// ---------------------------------------------------------------------------

/// `PUT /v1/me/preferences` body. Every field is optional; only the supplied
/// ones change.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreferencesPatch {
    /// `en` or `vi`.
    #[serde(default)]
    pub locale: Option<String>,
    /// `USD`, `EUR`, `GBP`, or `VND`.
    #[serde(default)]
    pub base_currency: Option<String>,
    /// A known IANA timezone name.
    #[serde(default)]
    pub timezone: Option<String>,
    /// `email` and `push` only; an unknown key is refused.
    #[serde(default)]
    pub notifications: Option<BTreeMap<String, bool>>,
}

impl PreferencesPatch {
    /// Validates the supplied fields and builds the partial update.
    ///
    /// # Errors
    /// Returns every field problem at once, naming the field and never its value.
    pub fn into_update(self) -> Result<PreferenceUpdate, Vec<FieldError>> {
        let mut errors = Vec::new();
        let locale = match self.locale {
            Some(raw) => match Locale::parse(&raw) {
                Ok(locale) => Some(locale),
                Err(error) => {
                    errors.push(field(error));
                    None
                }
            },
            None => None,
        };
        let base_currency = match self.base_currency {
            Some(raw) => match BaseCurrencyCode::parse(&raw) {
                Ok(code) => Some(code),
                Err(error) => {
                    errors.push(field(error));
                    None
                }
            },
            None => None,
        };
        let timezone = match self.timezone {
            Some(raw) => match parse_iana_timezone(&raw) {
                Ok(zone) => Some(zone),
                Err(error) => {
                    errors.push(field(error));
                    None
                }
            },
            None => None,
        };
        let (notification_email, notification_push) = match self.notifications {
            Some(channels) => {
                let mut email = None;
                let mut push = None;
                for (key, value) in channels {
                    match key.as_str() {
                        "email" => email = Some(value),
                        "push" => push = Some(value),
                        other => errors.push(FieldError::new(
                            format!("notifications.{other}"),
                            "is not a notification channel",
                        )),
                    }
                }
                (email, push)
            }
            None => (None, None),
        };
        if errors.is_empty() {
            Ok(PreferenceUpdate {
                locale,
                base_currency,
                timezone,
                notification_email,
                notification_push,
            })
        } else {
            Err(errors)
        }
    }
}

/// One effective value and its source.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreferenceValueResponse {
    /// The effective value.
    pub value: Value,
    /// `user` for an explicit choice, `default` otherwise.
    pub source: String,
}

impl PreferenceValueResponse {
    fn from_value<T>(value: &PreferenceValue<T>, as_json: impl Fn(&T) -> Value) -> Self {
        Self {
            value: as_json(value.value()),
            source: value.source().as_str().to_string(),
        }
    }
}

/// The two notification channels.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationsResponse {
    /// Email notifications.
    pub email: PreferenceValueResponse,
    /// Push notifications.
    pub push: PreferenceValueResponse,
}

/// `GET|PUT /v1/me/preferences` response.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreferencesResponse {
    /// The owner.
    pub user_id: String,
    /// The effective locale.
    pub locale: PreferenceValueResponse,
    /// The effective base currency.
    pub base_currency: PreferenceValueResponse,
    /// The effective timezone.
    pub timezone: PreferenceValueResponse,
    /// The effective notification channels.
    pub notifications: NotificationsResponse,
}

impl From<EffectivePreferences> for PreferencesResponse {
    fn from(effective: EffectivePreferences) -> Self {
        let notifications = NotificationsResponse {
            email: PreferenceValueResponse::from_value(effective.notification_email(), |value| {
                Value::Bool(*value)
            }),
            push: PreferenceValueResponse::from_value(effective.notification_push(), |value| {
                Value::Bool(*value)
            }),
        };
        Self {
            user_id: effective.user_id().to_string(),
            locale: PreferenceValueResponse::from_value(effective.locale(), |value| {
                Value::String(value.as_str().to_string())
            }),
            base_currency: PreferenceValueResponse::from_value(
                effective.base_currency(),
                |value| Value::String(value.as_str().to_string()),
            ),
            timezone: PreferenceValueResponse::from_value(effective.timezone(), |value| {
                Value::String(value.as_str().to_string())
            }),
            notifications,
        }
    }
}

/// `GET /v1/me/preferences`.
pub async fn read_preferences(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
) -> Response {
    let caller = match caller_of(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    match state.preferences_read.read(&caller).await {
        Ok(effective) => Json(PreferencesResponse::from(effective)).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// `PUT /v1/me/preferences`.
pub async fn write_preferences(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    payload: Result<Json<PreferencesPatch>, JsonRejection>,
) -> Response {
    let Json(patch) = match payload {
        Ok(payload) => payload,
        Err(rejection) => return invalid_body(rejection, &correlation.0),
    };
    let update = match patch.into_update() {
        Ok(update) => update,
        Err(errors) => {
            state
                .consent_observation
                .record_preference_write(PreferenceWriteOutcome::Refused);
            return validation(&correlation.0, errors).into_response();
        }
    };
    let caller = match caller_of(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    match state.preferences_write.write(&caller, update).await {
        Ok(effective) => Json(PreferencesResponse::from(effective)).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

// ---------------------------------------------------------------------------
// Consent state, grant, withdrawal, history
// ---------------------------------------------------------------------------

/// The consent state per kind plus the required kinds still missing.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentStateResponse {
    /// The owner.
    pub user_id: String,
    /// Every kind's current state.
    pub kinds: BTreeMap<String, String>,
    /// Every required kind that is not granted.
    pub missing: Vec<String>,
}

impl From<ConsentStateView> for ConsentStateResponse {
    fn from(view: ConsentStateView) -> Self {
        Self {
            user_id: view.user_id().to_string(),
            kinds: view
                .kinds()
                .iter()
                .map(|(kind, state)| (kind.as_str().to_string(), state.as_str().to_string()))
                .collect(),
            missing: view
                .missing()
                .into_iter()
                .map(|kind| kind.as_str().to_string())
                .collect(),
        }
    }
}

/// `GET /v1/me/consent/state`.
pub async fn my_consent_state(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
) -> Response {
    let caller = match caller_of(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    match state.consent_state.state(caller.user_id()).await {
        Ok(view) => Json(ConsentStateResponse::from(view)).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// `GET /internal/users/{userId}/consent/state` (C6).
pub async fn internal_consent_state(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ServicePrincipal(service)): Extension<ServicePrincipal>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(user_id): Path<String>,
) -> Response {
    let caller = match internal_caller(&state, &service, claims, &user_id, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    match state.consent_state.state(caller.user_id()).await {
        Ok(view) => Json(ConsentStateResponse::from(view)).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// Authenticates a C6 call: the allowlisted service token, the forwarded JWT,
/// and a path id that names the token's own user. Another user's id answers 404,
/// indistinguishable from absent (BR4.3).
// Axum's `Response` is the idiomatic error type for handler helpers; boxing it
// would only move the allocation into every refusal path, so allow the lint here.
#[allow(clippy::result_large_err)]
async fn internal_caller(
    state: &AppState,
    service: &str,
    claims: VerifiedClaims,
    path_user: &str,
    correlation: &str,
) -> Result<Caller, Response> {
    if !state
        .consent_services
        .iter()
        .any(|allowed| allowed == service)
    {
        return Err(ProblemDetails::new(
            403,
            "service-not-allowed",
            "Forbidden",
            Some("the service is not allowed on the consent endpoints".to_string()),
            correlation,
        )
        .into_response());
    }
    let target = UserId::parse(path_user).map_err(|_| {
        validation(
            correlation,
            vec![FieldError::new("userId", "is not a UUID")],
        )
        .into_response()
    })?;
    let caller = caller_of(state, claims, correlation).await?;
    if caller.user_id() != target {
        return Err(app_error(AppError::NotFound("consent record"), correlation));
    }
    Ok(caller)
}

/// `GET /internal/users/{userId}/preferences` (C6).
pub async fn internal_read_preferences(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ServicePrincipal(service)): Extension<ServicePrincipal>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(user_id): Path<String>,
) -> Response {
    let caller = match internal_caller(&state, &service, claims, &user_id, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    match state.preferences_read.read(&caller).await {
        Ok(effective) => Json(PreferencesResponse::from(effective)).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// `PUT /internal/users/{userId}/preferences` (C6).
pub async fn internal_write_preferences(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ServicePrincipal(service)): Extension<ServicePrincipal>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(user_id): Path<String>,
    payload: Result<Json<PreferencesPatch>, JsonRejection>,
) -> Response {
    let Json(patch) = match payload {
        Ok(payload) => payload,
        Err(rejection) => return invalid_body(rejection, &correlation.0),
    };
    let update = match patch.into_update() {
        Ok(update) => update,
        Err(errors) => {
            state
                .consent_observation
                .record_preference_write(PreferenceWriteOutcome::Refused);
            return validation(&correlation.0, errors).into_response();
        }
    };
    let caller = match internal_caller(&state, &service, claims, &user_id, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    match state.preferences_write.write(&caller, update).await {
        Ok(effective) => Json(PreferencesResponse::from(effective)).into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// `GET /internal/users/{userId}/consent` (C6): the user's event history.
pub async fn internal_consent_history(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ServicePrincipal(service)): Extension<ServicePrincipal>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(user_id): Path<String>,
    Query(query): Query<HistoryQuery>,
) -> Response {
    let cursor = match query.cursor() {
        Ok(cursor) => cursor,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let caller = match internal_caller(&state, &service, claims, &user_id, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    match state
        .consent_history
        .history(&caller, query.limit(), cursor)
        .await
    {
        Ok(events) => {
            let response = ConsentHistoryResponse::new(events, query.limit().clamp(1, 100));
            Json(response).into_response()
        }
        Err(error) => app_error(error, &correlation.0),
    }
}

/// `POST /internal/users/{userId}/consent` body (C6). `action` defaults to a
/// grant; a withdrawal resolves its policy version server-side.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalConsentRequest {
    /// The consent kind.
    pub kind: String,
    /// The policy version, required on a grant.
    #[serde(default)]
    pub policy_version: Option<String>,
    /// `granted` (the default) or `withdrawn`.
    #[serde(default)]
    pub action: Option<String>,
}

impl InternalConsentRequest {
    /// Parses the request into a grant or a withdrawal.
    ///
    /// # Errors
    /// Returns every field problem at once.
    pub fn parse(self) -> Result<InternalConsent, Vec<FieldError>> {
        let kind = ConsentKind::parse(&self.kind).map_err(|error| vec![field(error)])?;
        match self.action.as_deref().map(str::trim) {
            None | Some("") | Some("granted") => {
                let policy = match self.policy_version.as_deref() {
                    Some(raw) => ConsentPolicy::parse(raw).map_err(|error| vec![field(error)])?,
                    None => return Err(vec![FieldError::new("policyVersion", "is required")]),
                };
                Ok(InternalConsent::Grant(kind, policy))
            }
            Some("withdrawn") => Ok(InternalConsent::Withdraw(kind)),
            Some(other) => Err(vec![FieldError::new(
                "action",
                format!("must be granted or withdrawn, not {other}"),
            )]),
        }
    }
}

/// The parsed internal consent action.
pub enum InternalConsent {
    /// Grant the kind with the supplied policy version.
    Grant(ConsentKind, ConsentPolicy),
    /// Withdraw the kind; the server resolves the policy version.
    Withdraw(ConsentKind),
}

/// `POST /internal/users/{userId}/consent` (C6).
pub async fn internal_grant_or_withdraw(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ServicePrincipal(service)): Extension<ServicePrincipal>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(user_id): Path<String>,
    payload: Result<Json<InternalConsentRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => return invalid_body(rejection, &correlation.0),
    };
    let action = match request.parse() {
        Ok(action) => action,
        Err(errors) => return validation(&correlation.0, errors).into_response(),
    };
    let caller = match internal_caller(&state, &service, claims, &user_id, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    match action {
        InternalConsent::Grant(kind, policy) => {
            match state.consent_grant.grant(&caller, kind, policy).await {
                Ok(event) => (
                    StatusCode::CREATED,
                    Json(ConsentDecisionResponse {
                        kind: event.kind().as_str().to_string(),
                        state: identity_domain::consent::ConsentState::Granted
                            .as_str()
                            .to_string(),
                        changed: None,
                    }),
                )
                    .into_response(),
                Err(error) => app_error(error, &correlation.0),
            }
        }
        InternalConsent::Withdraw(kind) => {
            match state.consent_withdraw.withdraw(&caller, kind).await {
                Ok(WithdrawResult::Withdrawn(event)) => Json(ConsentDecisionResponse {
                    kind: event.kind().as_str().to_string(),
                    state: identity_domain::consent::ConsentState::Withdrawn
                        .as_str()
                        .to_string(),
                    changed: Some(true),
                })
                .into_response(),
                Ok(WithdrawResult::NoOp(state_now)) => Json(ConsentDecisionResponse {
                    kind: kind.as_str().to_string(),
                    state: state_now.as_str().to_string(),
                    changed: Some(false),
                })
                .into_response(),
                Err(error) => app_error(error, &correlation.0),
            }
        }
    }
}

/// `POST /v1/consent` is retired (410). The purpose-shaped append wrote rows the
/// event gate could not see; callers must use `POST /v1/me/consent`.
pub async fn retired_consent_append(Extension(correlation): Extension<CorrelationId>) -> Response {
    ProblemDetails::new(
        410,
        "consent-route-retired",
        "Gone",
        Some("use POST /v1/me/consent to record a consent event".to_string()),
        &correlation.0,
    )
    .into_response()
}

/// `POST /v1/me/consent` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentGrantRequest {
    /// The consent kind.
    pub kind: String,
    /// The policy version the person acted on; required on a grant.
    #[serde(default)]
    pub policy_version: Option<String>,
}

impl ConsentGrantRequest {
    /// Parses the kind and the required policy version.
    ///
    /// # Errors
    /// Returns every field problem at once.
    pub fn parse(self) -> Result<(ConsentKind, ConsentPolicy), Vec<FieldError>> {
        let mut errors = Vec::new();
        let kind = ConsentKind::parse(&self.kind).map_err(|error| vec![field(error)])?;
        let policy = match self.policy_version.as_deref() {
            Some(raw) => match ConsentPolicy::parse(raw) {
                Ok(policy) => policy,
                Err(error) => {
                    errors.push(field(error));
                    return Err(errors);
                }
            },
            None => {
                return Err(vec![FieldError::new("policyVersion", "is required")]);
            }
        };
        Ok((kind, policy))
    }
}

/// The grant and withdrawal response.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentDecisionResponse {
    /// The kind.
    pub kind: String,
    /// The kind's current state.
    pub state: String,
    /// Whether the call wrote an event; absent on a grant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changed: Option<bool>,
}

/// `POST /v1/me/consent`.
pub async fn grant_consent(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    payload: Result<Json<ConsentGrantRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => return invalid_body(rejection, &correlation.0),
    };
    let (kind, policy) = match request.parse() {
        Ok(parsed) => parsed,
        Err(errors) => return validation(&correlation.0, errors).into_response(),
    };
    let caller = match caller_of(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    match state.consent_grant.grant(&caller, kind, policy).await {
        Ok(event) => (
            StatusCode::CREATED,
            Json(ConsentDecisionResponse {
                kind: event.kind().as_str().to_string(),
                state: identity_domain::consent::ConsentState::Granted
                    .as_str()
                    .to_string(),
                changed: None,
            }),
        )
            .into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// `POST /v1/me/consent/{kind}/withdraw`.
pub async fn withdraw_consent(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(kind): Path<String>,
) -> Response {
    let kind = match ConsentKind::parse(&kind) {
        Ok(kind) => kind,
        Err(error) => {
            return validation(&correlation.0, vec![field(error)]).into_response();
        }
    };
    let caller = match caller_of(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    match state.consent_withdraw.withdraw(&caller, kind).await {
        Ok(WithdrawResult::Withdrawn(event)) => Json(ConsentDecisionResponse {
            kind: event.kind().as_str().to_string(),
            state: identity_domain::consent::ConsentState::Withdrawn
                .as_str()
                .to_string(),
            changed: Some(true),
        })
        .into_response(),
        Ok(WithdrawResult::NoOp(state_now)) => Json(ConsentDecisionResponse {
            kind: kind.as_str().to_string(),
            state: state_now.as_str().to_string(),
            changed: Some(false),
        })
        .into_response(),
        Err(error) => app_error(error, &correlation.0),
    }
}

/// `GET /v1/me/consent` query.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryQuery {
    /// The page size; capped at 100.
    #[serde(default)]
    pub limit: Option<u32>,
    /// The opaque cursor a previous page returned.
    #[serde(default)]
    pub before: Option<String>,
}

impl HistoryQuery {
    /// Decodes the keyset cursor.
    ///
    /// # Errors
    /// Returns a field problem when the cursor is not the value this service
    /// issued.
    pub fn cursor(&self) -> Result<Option<HistoryCursor>, FieldError> {
        let Some(raw) = self
            .before
            .as_deref()
            .map(str::trim)
            .filter(|raw| !raw.is_empty())
        else {
            return Ok(None);
        };
        let Some((occurred_at, seq)) = raw.split_once('|') else {
            return Err(FieldError::new(
                "before",
                "is not a cursor this service issued",
            ));
        };
        let occurred_at = DateTime::parse_from_rfc3339(occurred_at)
            .map(|value| value.with_timezone(&Utc))
            .map_err(|_| FieldError::new("before", "is not a cursor this service issued"))?;
        let seq = seq
            .parse::<i64>()
            .map_err(|_| FieldError::new("before", "is not a cursor this service issued"))?;
        Ok(Some(HistoryCursor { occurred_at, seq }))
    }

    /// The page size, defaulting to 50.
    #[must_use]
    pub fn limit(&self) -> u32 {
        self.limit.unwrap_or(50)
    }
}

/// One event in the history page.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentEventResponse {
    /// The event identifier.
    pub id: String,
    /// The monotonic sequence.
    pub seq: i64,
    /// The kind.
    pub kind: String,
    /// `granted` or `withdrawn`.
    pub action: String,
    /// The database timestamp.
    pub occurred_at: DateTime<Utc>,
    /// The policy version.
    pub policy_version: String,
}

impl From<&ConsentEvent> for ConsentEventResponse {
    fn from(event: &ConsentEvent) -> Self {
        Self {
            id: event.id().to_string(),
            seq: event.seq(),
            kind: event.kind().as_str().to_string(),
            action: event.action().as_str().to_string(),
            occurred_at: event.occurred_at(),
            policy_version: event.policy_version().to_string(),
        }
    }
}

/// `GET /v1/me/consent` response.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentHistoryResponse {
    /// The page, newest first.
    pub events: Vec<ConsentEventResponse>,
    /// The cursor for the next page, when one exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

impl ConsentHistoryResponse {
    fn new(events: Vec<ConsentEvent>, requested: u32) -> Self {
        let next_cursor = if events.len() as u32 >= requested {
            events.last().map(|event| {
                format!(
                    "{}|{}",
                    event
                        .occurred_at()
                        .to_rfc3339_opts(chrono::SecondsFormat::Micros, true),
                    event.seq()
                )
            })
        } else {
            None
        };
        Self {
            events: events.iter().map(ConsentEventResponse::from).collect(),
            next_cursor,
        }
    }
}

/// `GET /v1/me/consent`.
pub async fn consent_history(
    State(state): State<AppState>,
    Extension(correlation): Extension<CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Query(query): Query<HistoryQuery>,
) -> Response {
    let cursor = match query.cursor() {
        Ok(cursor) => cursor,
        Err(error) => return validation(&correlation.0, vec![error]).into_response(),
    };
    let caller = match caller_of(&state, claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    match state
        .consent_history
        .history(&caller, query.limit(), cursor)
        .await
    {
        Ok(events) => {
            let response = ConsentHistoryResponse::new(events, query.limit().clamp(1, 100));
            Json(response).into_response()
        }
        Err(error) => app_error(error, &correlation.0),
    }
}
