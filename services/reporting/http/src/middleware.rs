//! Correlation ids, security headers, request metrics, JWT validation, and the
//! internal service-token check.

use axum::extract::{Request, State};
use axum::http::{header, HeaderValue};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use reporting_application::common::VerifiedClaims;
use reporting_application::ports::{AuthError, RegisteredService};
use uuid::Uuid;

use crate::problem::ProblemDetails;
use crate::state::AppState;

/// The correlation id attached to a request.
#[derive(Debug, Clone)]
pub struct CorrelationId(pub String);

/// The validated JWT facts attached to a request.
#[derive(Debug, Clone)]
pub struct ValidatedClaims(pub VerifiedClaims);

/// The authenticated internal service and its allow-list.
#[derive(Debug, Clone)]
pub struct ServicePrincipal(pub RegisteredService);

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

/// Records every response status in the metrics registry.
pub async fn record_metrics(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let response = next.run(request).await;
    state.metrics.record(response.status().as_u16());
    response
}

/// Records every response status for the worker's listener.
pub async fn record_worker_metrics(
    State(state): State<crate::state::WorkerState>,
    request: Request,
    next: Next,
) -> Response {
    let response = next.run(request).await;
    state.metrics.record(response.status().as_u16());
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
        Ok(claims) => {
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

/// Validates the internal service token and attaches the registered caller.
pub async fn require_service(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let correlation = correlation_of(&request);
    let Some(token) = bearer_token(&request).map(ToString::to_string) else {
        return unauthorized(&correlation, "missing service token");
    };
    match state.service_tokens.service_for(&token).await {
        Ok(Some(service)) => {
            request.extensions_mut().insert(ServicePrincipal(service));
            next.run(request).await
        }
        Ok(None) => unauthorized(&correlation, "unknown service token"),
        Err(error) => ProblemDetails::new(
            503,
            "registry-unavailable",
            "Service-token registry unavailable",
            Some(error.to_string()),
            &correlation,
        )
        .into_response(),
    }
}

/// Accepts either the forwarded Supabase JWT (the SPA path) or a registered
/// service token (the C8 Reporting path) on the snapshot history route.
pub async fn require_jwt_or_service(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let correlation = correlation_of(&request);
    let Some(token) = bearer_token(&request).map(ToString::to_string) else {
        return unauthorized(&correlation, "missing bearer token or service token");
    };
    match state.service_tokens.service_for(&token).await {
        Ok(Some(service)) => {
            request.extensions_mut().insert(ServicePrincipal(service));
            return next.run(request).await;
        }
        Ok(None) => {}
        Err(error) => {
            return ProblemDetails::new(
                503,
                "registry-unavailable",
                "Service-token registry unavailable",
                Some(error.to_string()),
                &correlation,
            )
            .into_response()
        }
    }
    match state.token_validator.validate(&token).await {
        Ok(claims) => {
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

/// The worker's service-token check.
pub async fn require_worker_service(
    State(state): State<crate::state::WorkerState>,
    mut request: Request,
    next: Next,
) -> Response {
    let correlation = correlation_of(&request);
    let Some(token) = bearer_token(&request).map(ToString::to_string) else {
        return unauthorized(&correlation, "missing service token");
    };
    match state.service_tokens.service_for(&token).await {
        Ok(Some(service)) => {
            request.extensions_mut().insert(ServicePrincipal(service));
            next.run(request).await
        }
        Ok(None) => unauthorized(&correlation, "unknown service token"),
        Err(error) => ProblemDetails::new(
            503,
            "registry-unavailable",
            "Service-token registry unavailable",
            Some(error.to_string()),
            &correlation,
        )
        .into_response(),
    }
}
