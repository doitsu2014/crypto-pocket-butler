//! Correlation ids, security headers, request metrics, JWT identity with the
//! `X-Auth-Subject` agreement check, and service-token authorization.

use std::time::Instant;

use axum::extract::{Request, State};
use axum::http::{header, HeaderValue};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use notification_application::ports::{AuthError, Principal, ServiceCaller};
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

/// Records each response against the matched route pattern and its status. The
/// route pattern (not the raw path) keeps the label bounded and free of ids.
pub async fn record_metrics(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let route = request
        .extensions()
        .get::<axum::extract::MatchedPath>()
        .map_or_else(|| "unmatched".to_string(), |path| path.as_str().to_string());
    let started = Instant::now();
    let response = next.run(request).await;
    state.metrics.record_route(
        &route,
        response.status().as_u16(),
        started.elapsed().as_secs_f64(),
    );
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

/// Validates the JWT and requires `X-Auth-Subject` to equal its `sub`.
pub async fn require_user(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let correlation = correlation_of(&request);
    let token = match bearer_token(&request).map(ToString::to_string) {
        Some(token) => token,
        None => return unauthorized(&correlation, "missing bearer token"),
    };
    let principal = match state.token_validator.validate(&token).await {
        Ok(principal) => principal,
        Err(AuthError::Missing) => return unauthorized(&correlation, "missing bearer token"),
        Err(AuthError::Invalid(detail)) => return unauthorized(&correlation, &detail),
        Err(AuthError::Unavailable(detail)) => return unavailable(&correlation, detail),
    };
    let header_subject = request
        .headers()
        .get(SUBJECT_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    match header_subject {
        None => unauthorized(&correlation, "missing X-Auth-Subject header"),
        Some(subject) if subject != principal.subject => forbidden(
            &correlation,
            "X-Auth-Subject does not match the token subject",
        ),
        Some(_) => {
            request.extensions_mut().insert(principal);
            next.run(request).await
        }
    }
}

async fn require_operation(
    state: AppState,
    mut request: Request,
    next: Next,
    operation: &str,
) -> Response {
    let correlation = correlation_of(&request);
    let token = match bearer_token(&request).map(ToString::to_string) {
        Some(token) => token,
        None => return unauthorized(&correlation, "missing service token"),
    };
    match state.service_tokens.resolve(&token).await {
        Ok(Some(caller)) if caller.allows(operation) => {
            request.extensions_mut().insert(ServicePrincipal(caller));
            next.run(request).await
        }
        Ok(Some(_)) => forbidden(
            &correlation,
            "the caller is not allowed to perform this operation",
        ),
        Ok(None) => unauthorized(&correlation, "unknown service token"),
        Err(error) => unavailable(&correlation, error.to_string()),
    }
}

/// Requires a token allowed to submit alerts through C9.
pub async fn require_submit(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    require_operation(state, request, next, "alerts.submit").await
}

/// Requires a token allowed to invoke C11 run endpoints.
pub async fn require_run(State(state): State<AppState>, request: Request, next: Next) -> Response {
    require_operation(state, request, next, "runs").await
}

/// Requires a token allowed to read the release smoke endpoint.
pub async fn require_smoke(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    require_operation(state, request, next, "smoke").await
}

/// Requires Identity's token and its `privacy.anonymise` operation.
pub async fn require_anonymise(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let correlation = correlation_of(&request);
    let token = match bearer_token(&request).map(ToString::to_string) {
        Some(token) => token,
        None => return unauthorized(&correlation, "missing service token"),
    };
    match state.service_tokens.resolve(&token).await {
        Ok(Some(caller)) if caller.service == "identity" && caller.allows("privacy.anonymise") => {
            request.extensions_mut().insert(ServicePrincipal(caller));
            next.run(request).await
        }
        Ok(Some(_)) => forbidden(
            &correlation,
            "only Identity may anonymise a user's notification rows",
        ),
        Ok(None) => unauthorized(&correlation, "unknown service token"),
        Err(error) => unavailable(&correlation, error.to_string()),
    }
}

/// The authenticated caller, when the middleware has run.
pub fn principal_of(request: &Request) -> Option<&Principal> {
    request.extensions().get::<Principal>()
}
