//! The C4 `/internal/authorize` adapter and the admin role-change endpoint.
//!
//! Both take the caller's service token from `x-service-token` and the acting
//! user's Supabase JWT from `Authorization`, validated again against JWKS. The
//! subject and role come from that token only: a body carrying `callerSubject`
//! or `roleClaim` is rejected with 400, never ignored and never trusted.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, Path, Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use identity_application::authorisation::decision::DecisionError;
use identity_application::common::FieldError;
use identity_application::ports::AuthError;
use identity_application::AppError;
use identity_domain::authorisation::{
    CallerIdentity, Decision, DenialReason, ResourceOwnership, Role,
};
use identity_domain::{UserId, VerifiedClaims};
use serde::{Deserialize, Serialize};

use crate::middleware::{bearer_token, correlation_of, ServicePrincipal, ValidatedClaims};
use crate::problem::{from_app_error, validation, ProblemDetails};
use crate::state::AppState;

/// The header that carries the calling service's own token on the internal
/// authorisation paths. The user's JWT stays in `Authorization`.
pub const SERVICE_TOKEN_HEADER: &str = "x-service-token";

/// The one service that may invoke the admin role change (the same-credential
/// admin surface). Every other service is refused before the use case runs.
const ROLE_CHANGE_SERVICE: &str = "gateway";

/// `POST /internal/authorize` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizeRequest {
    /// The catalogue operation name.
    pub operation: String,
    /// The resource's domain name, when the operation touches one.
    #[serde(default)]
    pub resource_kind: Option<String>,
    /// `resource` (one row) or `collection` (a list, search, or export).
    #[serde(default)]
    pub scope: Option<String>,
    /// The one row's owner, when the scope is `resource`.
    #[serde(default)]
    pub resource_owner_id: Option<String>,
    /// A retired body field: identity comes from the validated JWT. Present is a
    /// 400.
    #[serde(default)]
    pub caller_subject: Option<serde_json::Value>,
    /// A retired body field: the role comes from the validated JWT. Present is a
    /// 400.
    #[serde(default)]
    pub role_claim: Option<serde_json::Value>,
}

impl AuthorizeRequest {
    /// Whether the body tried to supply the caller's identity.
    #[must_use]
    pub fn carries_identity(&self) -> bool {
        self.caller_subject.is_some() || self.role_claim.is_some()
    }

    /// Builds the ownership assertion from the request body.
    ///
    /// # Errors
    /// Returns the offending field paths for an unknown scope or a non-UUID
    /// owner id.
    pub fn ownership(&self) -> Result<Option<ResourceOwnership>, Vec<FieldError>> {
        let scope = self
            .scope
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        if let Some(scope) = scope {
            if !matches!(scope, "resource" | "collection") {
                return Err(vec![FieldError::new(
                    "scope",
                    "must be resource or collection",
                )]);
            }
        }
        let owner = match self.resource_owner_id.as_deref() {
            None => None,
            Some(raw) if raw.trim().is_empty() => None,
            Some(raw) => Some(
                UserId::parse(raw)
                    .map_err(|_| vec![FieldError::new("resourceOwnerId", "is not a UUID")])?,
            ),
        };
        let kind = self
            .resource_kind
            .as_deref()
            .map(str::trim)
            .filter(|kind| !kind.is_empty())
            .unwrap_or("resource");

        match scope {
            Some("collection") => Ok(Some(ResourceOwnership::collection(kind))),
            // The only other accepted scope is `resource`; the validation above
            // rejected every unknown value.
            Some(_) => Ok(Some(ResourceOwnership::row(kind, owner))),
            // An owner id without a scope means one row; neither means no
            // ownership assertion is needed.
            None if owner.is_some() => Ok(Some(ResourceOwnership::row(kind, owner))),
            None => Ok(None),
        }
    }
}

/// The decision document an allowed request returns.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionResponse {
    /// Whether the request is permitted.
    pub allowed: bool,
    /// The reason vocabulary.
    pub reason: String,
    /// The status the caller may answer with.
    pub http_status: u16,
    /// The owner filter the owning service must apply, on a collection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_owner_id: Option<String>,
}

impl From<&Decision> for DecisionResponse {
    fn from(decision: &Decision) -> Self {
        Self {
            allowed: decision.allowed(),
            reason: decision.reason().as_str().to_string(),
            http_status: decision.http_status(),
            filter_owner_id: decision.filter_owner_id().map(|id| id.to_string()),
        }
    }
}

/// The admin role-change body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleChangeRequest {
    /// The target user's internal id.
    pub user_id: String,
    /// `owner`, `admin`, or `support`; `system` is never assignable.
    pub new_role: String,
    /// Why the change is made; required and bounded.
    pub reason: String,
}

impl RoleChangeRequest {
    /// Parses the target id and the new role.
    ///
    /// # Errors
    /// Returns the offending field paths for a non-UUID id or an unknown role.
    pub fn parse(&self) -> Result<(UserId, Role), Vec<FieldError>> {
        let user_id = UserId::parse(&self.user_id)
            .map_err(|_| vec![FieldError::new("userId", "is not a UUID")])?;
        Ok((user_id, parse_role(&self.new_role)?))
    }
}

/// The admin role-change body on the gateway's `/v1` surface, where the path
/// carries the target id.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleChangeBody {
    /// `owner`, `admin`, or `support`; `system` is never assignable.
    pub new_role: String,
    /// Why the change is made; required and bounded.
    pub reason: String,
}

/// Parses the contract's role vocabulary. `system` and any unknown value are
/// refused.
fn parse_role(raw: &str) -> Result<Role, Vec<FieldError>> {
    match raw.trim() {
        "owner" | "user" => Ok(Role::User),
        "admin" => Ok(Role::Admin),
        "support" => Ok(Role::Support),
        _ => Err(vec![FieldError::new(
            "newRole",
            "must be owner, admin, or support",
        )]),
    }
}

/// The role-change response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleChangeResponse {
    /// The target user.
    pub user_id: String,
    /// The role before the change.
    pub previous_role: String,
    /// The role after the change.
    pub new_role: String,
}

/// Authenticates an internal authorisation call: the service token names the
/// caller, and the user's forwarded JWT names the user. A service token alone
/// never authorises a user-scoped action.
pub async fn require_internal_caller(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let correlation = correlation_of(&request);
    let service_token = request
        .headers()
        .get(SERVICE_TOKEN_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|token| !token.is_empty());
    let Some(service) = service_token.and_then(|token| state.service_tokens.service_for(token))
    else {
        return problem(
            401,
            "unauthorized",
            "Unauthorized",
            "missing or unknown service token",
            &correlation,
        );
    };

    let Some(jwt) = bearer_token(&request).map(ToString::to_string) else {
        return problem(
            401,
            "unauthorized",
            "Unauthorized",
            "missing bearer token",
            &correlation,
        );
    };
    match state.token_validator.validate(&jwt).await {
        Ok(claims) => {
            request.extensions_mut().insert(ServicePrincipal(service));
            request.extensions_mut().insert(ValidatedClaims(claims));
            next.run(request).await
        }
        Err(AuthError::Unavailable(detail)) => problem(
            503,
            "identity-unavailable",
            "Identity provider unavailable",
            &detail,
            &correlation,
        ),
        Err(error) => problem(
            401,
            "unauthorized",
            "Unauthorized",
            &error.to_string(),
            &correlation,
        ),
    }
}

/// `POST /internal/authorize`: evaluate one operation for the token's user.
pub async fn authorize(
    State(state): State<AppState>,
    Extension(correlation): Extension<crate::middleware::CorrelationId>,
    Extension(ServicePrincipal(service)): Extension<ServicePrincipal>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    payload: Result<Json<AuthorizeRequest>, JsonRejection>,
) -> Response {
    if !state
        .authorize_services
        .iter()
        .any(|allowed| allowed == &service)
    {
        return problem(
            403,
            "service-not-allowed",
            "Forbidden",
            "the service is not allowed on the authorisation endpoint",
            &correlation.0,
        );
    }

    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => return invalid_body(rejection, &correlation.0),
    };

    if request.carries_identity() {
        return ProblemDetails::new(
            400,
            "caller-identity-not-accepted",
            "Invalid request body",
            Some(
                "the caller's subject and role come from the validated JWT; callerSubject and roleClaim are not accepted"
                    .to_string(),
            ),
            &correlation.0,
        )
        .into_response();
    }

    let operation = state.authorisation.catalogue().lookup(&request.operation);
    if !may_ask(&service, operation.name()) {
        return problem(
            403,
            "operation-not-allowed",
            "Forbidden",
            "the calling service may not ask for this operation",
            &correlation.0,
        );
    }

    let caller = match caller_of(&state, &claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };

    let ownership = match request.ownership() {
        Ok(ownership) => ownership,
        Err(errors) => return validation(&correlation.0, errors).into_response(),
    };

    match state
        .authorisation
        .decide(&caller, &operation, ownership.as_ref(), &correlation.0)
        .await
    {
        Ok(decision) if decision.allowed() => {
            Json(DecisionResponse::from(&decision)).into_response()
        }
        Ok(decision) => denial_problem(&decision, &correlation.0),
        Err(DecisionError::StatusUnavailable(detail)) => problem(
            503,
            "status_unavailable",
            "Service unavailable",
            &detail,
            &correlation.0,
        ),
    }
}

/// `PUT /internal/admin/role`: an admin changes one user's provider role.
pub async fn change_role(
    State(state): State<AppState>,
    Extension(correlation): Extension<crate::middleware::CorrelationId>,
    Extension(ServicePrincipal(service)): Extension<ServicePrincipal>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    payload: Result<Json<RoleChangeRequest>, JsonRejection>,
) -> Response {
    if service != ROLE_CHANGE_SERVICE {
        return problem(
            403,
            "service-not-allowed",
            "Forbidden",
            "only the API Gateway may change a role",
            &correlation.0,
        );
    }

    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => return invalid_body(rejection, &correlation.0),
    };

    let (target, new_role) = match request.parse() {
        Ok(parsed) => parsed,
        Err(errors) => return validation(&correlation.0, errors).into_response(),
    };

    let caller = match caller_of(&state, &claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };

    match state
        .role_change
        .change(&caller, target, new_role, &request.reason, &correlation.0)
        .await
    {
        Ok(view) => Json(RoleChangeResponse {
            user_id: view.user_id().to_string(),
            previous_role: view.previous_role().provider_value().to_string(),
            new_role: view.new_role().provider_value().to_string(),
        })
        .into_response(),
        Err(error) => from_app_error(&error, &correlation.0).into_response(),
    }
}

/// `PUT /v1/admin/users/{userId}/role`: the admin surface through the gateway.
///
/// The gateway already enforces the admin role at the edge; the role change
/// re-checks it from the same forwarded JWT, so a direct call to Identity
/// cannot bypass the rule. The target id travels in the path.
pub async fn change_role_for_gateway(
    State(state): State<AppState>,
    Extension(correlation): Extension<crate::middleware::CorrelationId>,
    Extension(ValidatedClaims(claims)): Extension<ValidatedClaims>,
    Path(user_id): Path<String>,
    payload: Result<Json<RoleChangeBody>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(rejection) => return invalid_body(rejection, &correlation.0),
    };
    let target = match UserId::parse(&user_id) {
        Ok(target) => target,
        Err(_) => {
            return validation(
                &correlation.0,
                vec![FieldError::new("userId", "is not a UUID")],
            )
            .into_response()
        }
    };
    let new_role = match parse_role(&request.new_role) {
        Ok(role) => role,
        Err(errors) => return validation(&correlation.0, errors).into_response(),
    };
    let caller = match caller_of(&state, &claims, &correlation.0).await {
        Ok(caller) => caller,
        Err(response) => return response,
    };

    match state
        .role_change
        .change(&caller, target, new_role, &request.reason, &correlation.0)
        .await
    {
        Ok(view) => Json(RoleChangeResponse {
            user_id: view.user_id().to_string(),
            previous_role: view.previous_role().provider_value().to_string(),
            new_role: view.new_role().provider_value().to_string(),
        })
        .into_response(),
        Err(error) => from_app_error(&error, &correlation.0).into_response(),
    }
}

/// Resolves the validated token's subject to the internal caller identity.
// Axum's `Response` is the idiomatic error type for handler helpers; boxing it
// would only move the allocation into every refusal path.
#[allow(clippy::result_large_err)]
async fn caller_of(
    state: &AppState,
    claims: &VerifiedClaims,
    correlation: &str,
) -> Result<CallerIdentity, Response> {
    match state.users.find_by_auth_subject(&claims.subject).await {
        Ok(Some(user)) => Ok(CallerIdentity::user(
            claims.subject.clone(),
            user.id(),
            claims.role,
        )),
        Ok(None) => Err(problem(
            401,
            "unauthorized",
            "Unauthorized",
            "the token's subject is not provisioned",
            correlation,
        )),
        Err(error) => {
            Err(from_app_error(&AppError::from_repository(error), correlation).into_response())
        }
    }
}

/// The per-service, per-operation allowlist. It names operations, not classes, so
/// a caller cannot reach an administrative or confirmation operation by class.
fn may_ask(service: &str, operation: &str) -> bool {
    match service {
        // The Account and Portfolio services enforce BR2.13 through this
        // endpoint: owner-scoped and collection reads and writes only.
        "account-api" | "portfolio" => matches!(
            operation,
            "principal:read"
                | "profile:read"
                | "profile:write"
                | "resource:read"
                | "resource:write"
                | "collection:list"
                | "collection:search"
                | "collection:export"
        ),
        // The release smoke check proves one allow path and one denial.
        "identity-smoke" => operation == "principal:read",
        _ => false,
    }
}

/// The refusal body. `not_owner` and a nonexistent id produce the same 404 and
/// disclose nothing (BR2.6).
fn denial_problem(decision: &Decision, correlation: &str) -> Response {
    let (title, detail) = match decision.reason() {
        DenialReason::NotOwner => ("Not found", "the requested resource was not found"),
        DenialReason::AuditUnavailable => {
            ("Service unavailable", "the decision could not be recorded")
        }
        DenialReason::NoRole => (
            "Forbidden",
            "the caller's role does not permit this operation",
        ),
        DenialReason::StatusDenied => (
            "Forbidden",
            "the account's status does not permit this operation",
        ),
        DenialReason::NotDeclared => ("Forbidden", "the operation is not declared"),
        DenialReason::Allowed => ("OK", "the request is permitted"),
    };
    ProblemDetails::new(
        decision.http_status(),
        decision.reason().as_str(),
        title,
        Some(detail.to_string()),
        correlation,
    )
    .into_response()
}

fn problem(
    status: u16,
    problem_type: &str,
    title: &str,
    detail: &str,
    correlation: &str,
) -> Response {
    ProblemDetails::new(
        status,
        problem_type,
        title,
        Some(detail.to_string()),
        correlation,
    )
    .into_response()
}

/// A body that is not the declared JSON shape is a 400, not a decision.
fn invalid_body(rejection: JsonRejection, correlation: &str) -> Response {
    ProblemDetails::new(
        400,
        "invalid-body",
        "Invalid request body",
        Some(rejection.body_text()),
        correlation,
    )
    .into_response()
}
