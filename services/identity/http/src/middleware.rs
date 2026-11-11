//! Correlation ids, security headers, request metrics, JWT validation, and the
//! internal service-token check.

use axum::extract::{Request, State};
use axum::http::{header, HeaderValue};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use identity_application::ports::AuthError;
use identity_domain::VerifiedClaims;
use uuid::Uuid;

use crate::problem::ProblemDetails;
use crate::state::AppState;

/// The correlation id attached to a request.
#[derive(Debug, Clone)]
pub struct CorrelationId(pub String);

/// The validated JWT facts attached to a request.
#[derive(Debug, Clone)]
pub struct ValidatedClaims(pub VerifiedClaims);

/// The authenticated internal service.
#[derive(Debug, Clone)]
pub struct ServicePrincipal(pub String);

const CORRELATION_HEADER: &str = "x-correlation-id";

fn valid_correlation_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | ':')
        })
}

/// The correlation id of the request, when the middleware has run.
pub(crate) fn correlation_of(request: &Request) -> String {
    request
        .extensions()
        .get::<CorrelationId>()
        .map(|correlation| correlation.0.clone())
        .unwrap_or_default()
}

/// Honours a valid caller-supplied correlation id, generates one otherwise, and
/// echoes it on every response.
pub async fn correlation_id(mut request: Request, next: Next) -> Response {
    let supplied = request
        .headers()
        .get(CORRELATION_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(ToString::to_string);

    let id = match supplied {
        None => Uuid::new_v4().to_string(),
        Some(value) if valid_correlation_id(&value) => value,
        Some(_) => {
            let generated = Uuid::new_v4().to_string();
            let mut response = ProblemDetails::new(
                400,
                "invalid-correlation-id",
                "Invalid correlation id",
                Some(
                    "x-correlation-id must be 1-128 ASCII letters, digits, '-', '_', '.', or ':'"
                        .to_string(),
                ),
                &generated,
            )
            .into_response();
            if let Ok(value) = HeaderValue::from_str(&generated) {
                response.headers_mut().insert(CORRELATION_HEADER, value);
            }
            return response;
        }
    };

    request.extensions_mut().insert(CorrelationId(id.clone()));
    let mut response = next.run(request).await;
    if let Ok(value) = HeaderValue::from_str(&id) {
        response.headers_mut().insert(CORRELATION_HEADER, value);
    }
    response
}

/// Adds the security headers required on every API response.
pub async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=63072000; includeSubDomains"),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'none'"),
    );
    response
}

/// The route label for the metrics series. A path that is not named here folds
/// into `other`, so the aggregate series keeps working for dashboards that do
/// not select a route.
fn route_label(path: &str) -> &'static str {
    match path {
        "/internal/authorize" => "internal.authorize",
        "/v1/me/preferences" => "me.preferences",
        "/v1/me/consent/state" => "me.consent.state",
        "/v1/me/consent" => "me.consent",
        _ if path.starts_with("/v1/me/consent/") && path.ends_with("/withdraw") => {
            "me.consent.withdraw"
        }
        _ if path.starts_with("/internal/users/") && path.ends_with("/consent/state") => {
            "internal.consent.state"
        }
        _ => "other",
    }
}

/// Records every response status in the metrics registry, isolating the C4
/// authorisation route and the U4 consent routes under their own `route` label.
pub async fn record_metrics(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let route = route_label(request.uri().path());
    let response = next.run(request).await;
    state
        .metrics
        .record_routed(response.status().as_u16(), route);
    response
}

pub(crate) fn bearer_token(request: &Request) -> Option<&str> {
    request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| !token.is_empty())
}

fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

fn unauthorized(correlation: &str, detail: &str) -> Response {
    ProblemDetails::new(
        401,
        "unauthorized",
        "Unauthorized",
        Some(detail.to_string()),
        correlation,
    )
    .into_response()
}

fn forbidden(correlation: &str, detail: &str) -> Response {
    ProblemDetails::new(
        403,
        "forbidden",
        "Forbidden",
        Some(detail.to_string()),
        correlation,
    )
    .into_response()
}

/// Validates the forwarded Supabase JWT and attaches the claims. A missing,
/// invalid, expired, or wrongly-signed token is refused with 401; an unavailable
/// identity provider is refused with 503.
pub async fn require_jwt(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let correlation = correlation_of(&request);
    let Some(token) = bearer_token(&request).map(ToString::to_string) else {
        return unauthorized(&correlation, "missing bearer token");
    };
    match state.token_validator.validate(&token).await {
        Ok(mut claims) => {
            // The session view needs a device label and the gateway's coarse
            // geolocation. The gateway never supplies the full IP (BR8.1).
            claims.user_agent = request
                .headers()
                .get("x-device-label")
                .and_then(|value| value.to_str().ok())
                .map(ToString::to_string)
                .or_else(|| {
                    request
                        .headers()
                        .get(header::USER_AGENT)
                        .and_then(|value| value.to_str().ok())
                        .map(ToString::to_string)
                })
                .map(|value| truncate(&value, 120));
            claims.location = request
                .headers()
                .get("x-geo-location")
                .and_then(|value| value.to_str().ok())
                .map(ToString::to_string)
                .map(|value| truncate(&value, 120));
            // The self-declared country and the observed client address come
            // from the trusted caller, never from the token's own claims. The
            // jurisdiction gate refuses provisioning when either is absent.
            if let Some(country) = request
                .headers()
                .get("x-country-code")
                .and_then(|value| value.to_str().ok())
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                claims.country_code = Some(truncate(country, 8));
            }
            if let Some(ip) = request
                .headers()
                .get("x-client-ip")
                .and_then(|value| value.to_str().ok())
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                claims.client_ip = Some(truncate(ip, 64));
            }
            request.extensions_mut().insert(ValidatedClaims(claims));
            next.run(request).await
        }
        Err(AuthError::Unavailable(detail)) => ProblemDetails::new(
            503,
            "identity-unavailable",
            "Identity provider unavailable",
            Some(detail),
            &correlation,
        )
        .into_response(),
        Err(error) => unauthorized(&correlation, &error.to_string()),
    }
}

/// Validates the internal service token and attaches the issuing service.
///
/// Authorization is per caller: each internal route names the one service that
/// may invoke it, so a valid token issued to any other service is refused (403).
async fn require_caller(
    state: AppState,
    mut request: Request,
    next: Next,
    allowed: &'static [&'static str],
    refusal: &'static str,
) -> Response {
    let correlation = correlation_of(&request);
    let Some(token) = bearer_token(&request).map(ToString::to_string) else {
        return unauthorized(&correlation, "missing service token");
    };
    match state.service_tokens.service_for(&token) {
        Some(service) if allowed.contains(&service.as_str()) => {
            request.extensions_mut().insert(ServicePrincipal(service));
            next.run(request).await
        }
        Some(_) => forbidden(&correlation, refusal),
        None => unauthorized(&correlation, "unknown service token"),
    }
}

/// C3: only Notification may read a user's verified contact at delivery time.
pub async fn require_contact(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    require_caller(
        state,
        request,
        next,
        &["notification"],
        "only Notification may read a user's verified contact",
    )
    .await
}

/// C3: only Reporting may ask which users are due for a period report.
pub async fn require_due_users(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    require_caller(
        state,
        request,
        next,
        &["reporting"],
        "only Reporting may read the due-user list",
    )
    .await
}

/// C11: only Job Orchestration may invoke a run endpoint.
pub async fn require_run(State(state): State<AppState>, request: Request, next: Next) -> Response {
    require_caller(
        state,
        request,
        next,
        &["job-orchestration"],
        "only Job Orchestration may invoke a run endpoint",
    )
    .await
}

/// C2/C3: only the API Gateway may check a support grant on the metadata path.
pub async fn require_support_check(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    require_caller(
        state,
        request,
        next,
        &["gateway"],
        "only the API Gateway may check a support grant",
    )
    .await
}

/// C3: the API Gateway may read a user record; the release smoke check may
/// resolve a principal by subject hash with its read-only service token.
pub async fn require_registry_read(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    require_caller(
        state,
        request,
        next,
        &["gateway", "identity-smoke"],
        "only the API Gateway or the release smoke check may read a user record",
    )
    .await
}

/// C3: only the API Gateway may provision or refresh a user record.
pub async fn require_registry_write(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    require_caller(
        state,
        request,
        next,
        &["gateway"],
        "only the API Gateway may provision a user record",
    )
    .await
}

/// C8: only the User Registry (inside Identity) or the API Gateway may ask for a
/// jurisdiction decision. A direct client call has no service token and is
/// refused before the body is read.
pub async fn require_jurisdiction_check(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    require_caller(
        state,
        request,
        next,
        &["gateway", "identity"],
        "only the User Registry or the API Gateway may request a jurisdiction check",
    )
    .await
}
