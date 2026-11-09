//! Correlation ids, security headers, request metrics, JWT identity with the
//! `X-Auth-Subject` agreement check, and service-token authorization.

use std::time::Instant;

use axum::extract::{MatchedPath, Request, State};
use axum::http::{header, HeaderValue};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use ledger_application::{AuthError, Principal, ServiceCaller};
use uuid::Uuid;

use crate::problem::ProblemDetails;
use crate::state::AppState;

/// The correlation id attached to a request.
#[derive(Debug, Clone)]
pub struct CorrelationId(pub String);

/// The authenticated internal service and its allowed operations.
#[derive(Debug, Clone)]
pub struct ServicePrincipal(pub ServiceCaller);

const CORRELATION_HEADER: &str = "x-correlation-id";
const SUBJECT_HEADER: &str = "x-auth-subject";

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

/// The matched route template, for a low-cardinality metric label.
fn route_of(request: &Request) -> String {
    request
        .extensions()
        .get::<MatchedPath>()
        .map(|path| path.as_str().to_string())
        .unwrap_or_else(|| "unmatched".to_string())
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

/// Records each response's route, status, and duration.
pub async fn record_metrics(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let route = route_of(&request);
    let started = Instant::now();
    let response = next.run(request).await;
    state.metrics.record_request(
        &route,
        response.status().as_u16(),
        started.elapsed().as_secs_f64(),
    );
    response
}

fn bearer_token(request: &Request) -> Option<&str> {
    request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| !token.is_empty())
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

fn unavailable(correlation: &str, detail: String) -> Response {
    ProblemDetails::new(
        503,
        "unavailable",
        "Service unavailable",
        Some(detail),
        correlation,
    )
    .into_response()
}

async fn principal_for(
    state: &AppState,
    token: &str,
    correlation: &str,
) -> Result<Principal, Box<Response>> {
    state
        .token_validator
        .validate(token)
        .await
        .map_err(|error| match error {
            AuthError::Missing => Box::new(unauthorized(correlation, "missing bearer token")),
            AuthError::Invalid(detail) => Box::new(unauthorized(correlation, &detail)),
            AuthError::Unavailable(detail) => Box::new(unavailable(correlation, detail)),
        })
}

fn required_token(request: &Request, correlation: &str) -> Result<String, Box<Response>> {
    bearer_token(request)
        .map(ToString::to_string)
        .ok_or_else(|| Box::new(unauthorized(correlation, "missing bearer token")))
}

fn subject_matches(
    request: &Request,
    principal: &Principal,
    correlation: &str,
) -> Result<(), Box<Response>> {
    let header_subject = request
        .headers()
        .get(SUBJECT_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    match header_subject {
        None => Err(Box::new(unauthorized(
            correlation,
            "missing X-Auth-Subject header",
        ))),
        Some(subject) if subject != principal.subject => Err(Box::new(forbidden(
            correlation,
            "X-Auth-Subject does not match the token subject",
        ))),
        Some(_) => Ok(()),
    }
}

/// Validates the JWT and requires `X-Auth-Subject` to equal its `sub`.
pub async fn require_user(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let correlation = correlation_of(&request);
    let token = match required_token(&request, &correlation) {
        Ok(token) => token,
        Err(response) => return *response,
    };
    let principal = match principal_for(&state, &token, &correlation).await {
        Ok(principal) => principal,
        Err(response) => return *response,
    };
    if let Err(response) = subject_matches(&request, &principal, &correlation) {
        return *response;
    }
    request.extensions_mut().insert(principal);
    next.run(request).await
}

async fn service_for(
    state: &AppState,
    token: &str,
    correlation: &str,
) -> Result<ServiceCaller, Box<Response>> {
    match state.service_tokens.resolve(token).await {
        Ok(Some(caller)) => Ok(caller),
        Ok(None) => Err(Box::new(unauthorized(correlation, "unknown service token"))),
        Err(error) => Err(Box::new(unavailable(correlation, error.to_string()))),
    }
}

async fn require_service_impl(
    state: AppState,
    mut request: Request,
    next: Next,
    operation: Option<&str>,
) -> Response {
    let correlation = correlation_of(&request);
    let token = match bearer_token(&request).map(ToString::to_string) {
        Some(token) => token,
        None => return unauthorized(&correlation, "missing service token"),
    };
    match service_for(&state, &token, &correlation).await {
        Ok(caller) => {
            if let Some(operation) = operation {
                if !caller.allows(operation) {
                    return forbidden(
                        &correlation,
                        "the caller is not allowed to perform this operation",
                    );
                }
            }
            request.extensions_mut().insert(ServicePrincipal(caller));
            next.run(request).await
        }
        Err(response) => *response,
    }
}

/// C7: the SPA reads a user's P&L with the forwarded JWT; Reporting reads it
/// with a service token that holds `pnl.read` (the C7 consumer list includes
/// U8). Either identity is attached; the handler enforces the owner rule for
/// the JWT path.
pub async fn require_user_or_pnl_service(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let correlation = correlation_of(&request);
    let token = match bearer_token(&request).map(ToString::to_string) {
        Some(token) => token,
        None => return unauthorized(&correlation, "missing bearer token"),
    };
    match state.service_tokens.resolve(&token).await {
        Ok(Some(caller)) => {
            if !caller.allows("pnl.read") {
                return forbidden(
                    &correlation,
                    "the caller is not allowed to read profit and loss",
                );
            }
            request.extensions_mut().insert(ServicePrincipal(caller));
            return next.run(request).await;
        }
        Ok(None) => {}
        Err(error) => return unavailable(&correlation, error.to_string()),
    }
    let principal = match principal_for(&state, &token, &correlation).await {
        Ok(principal) => principal,
        Err(response) => return *response,
    };
    if let Err(response) = subject_matches(&request, &principal, &correlation) {
        return *response;
    }
    request.extensions_mut().insert(principal);
    next.run(request).await
}

/// Requires a valid service token, with no operation check (privacy enforces
/// the exact operation in the use case).
pub async fn require_service(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    require_service_impl(state, request, next, None).await
}

/// Requires a token allowed to record Portfolio's trades.
pub async fn require_trades(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    require_service_impl(state, request, next, Some("trades.write")).await
}

/// Requires a token allowed to invoke C11 run endpoints.
pub async fn require_run(State(state): State<AppState>, request: Request, next: Next) -> Response {
    require_service_impl(state, request, next, Some("runs")).await
}

/// Requires a token allowed to read the release smoke endpoint.
pub async fn require_smoke(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    require_service_impl(state, request, next, Some("smoke")).await
}

/// The authenticated caller, when the middleware has run.
pub fn principal_of(request: &Request) -> Option<&Principal> {
    request.extensions().get::<Principal>()
}

/// The authenticated internal caller, when the middleware has run.
pub fn service_of(request: &Request) -> Option<&ServicePrincipal> {
    request.extensions().get::<ServicePrincipal>()
}
