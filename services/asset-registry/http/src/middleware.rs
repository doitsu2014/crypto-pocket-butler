//! Correlation ids, security headers, request metrics, and JWT enforcement.

use asset_registry_application::audit::AuditEvent;
use asset_registry_application::ports::{AuthError, Principal, Role};
use axum::extract::{Request, State};
use axum::http::{header, HeaderValue};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use uuid::Uuid;

use crate::problem::ProblemDetails;
use crate::state::AppState;

/// The correlation id attached to a request.
#[derive(Debug, Clone)]
pub struct CorrelationId(pub String);

const CORRELATION_HEADER: &str = "x-correlation-id";

fn valid_correlation_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | ':')
        })
}

fn correlation_of(request: &Request) -> String {
    request
        .extensions()
        .get::<CorrelationId>()
        .map(|correlation| correlation.0.clone())
        .unwrap_or_default()
}

/// Honours a valid caller-supplied correlation id, generates one otherwise, and
/// echoes it on every response (BR4.3).
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

/// Validates the forwarded Supabase JWT before any use case runs (BR4.1).
pub async fn require_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let correlation = correlation_of(&request);
    let Some(token) = bearer_token(&request).map(ToString::to_string) else {
        return unauthorized(&correlation, "missing bearer token");
    };
    match state.token_validator.validate(&token).await {
        Ok(principal) => {
            request.extensions_mut().insert(principal);
            next.run(request).await
        }
        Err(AuthError::Missing) => unauthorized(&correlation, "missing bearer token"),
        Err(AuthError::Invalid(detail)) => unauthorized(&correlation, &detail),
        Err(AuthError::Unavailable(detail)) => ProblemDetails::new(
            503,
            "identity-unavailable",
            "Identity provider unavailable",
            Some(detail),
            &correlation,
        )
        .into_response(),
    }
}

/// Validates the JWT and requires the admin role (BR3.1, security design).
pub async fn require_admin(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let correlation = correlation_of(&request);
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let Some(token) = bearer_token(&request).map(ToString::to_string) else {
        return unauthorized(&correlation, "missing bearer token");
    };
    match state.token_validator.validate(&token).await {
        Ok(principal) if principal.role == Role::Admin => {
            request.extensions_mut().insert(principal);
            next.run(request).await
        }
        Ok(principal) => {
            let target = format!("{method} {path}");
            let event = AuditEvent::new(
                &principal.subject,
                "registry.admin.denied",
                target,
                "denied",
                Some(correlation.clone()),
                state.clock.now(),
            );
            if let Ok(event) = event {
                if let Err(error) = state.audit.record(&event).await {
                    tracing::error!(%error, "failed to audit an admin denial");
                }
            }
            ProblemDetails::new(
                403,
                "forbidden",
                "Forbidden",
                Some("admin role required".to_string()),
                &correlation,
            )
            .into_response()
        }
        Err(AuthError::Missing) => unauthorized(&correlation, "missing bearer token"),
        Err(AuthError::Invalid(detail)) => unauthorized(&correlation, &detail),
        Err(AuthError::Unavailable(detail)) => ProblemDetails::new(
            503,
            "identity-unavailable",
            "Identity provider unavailable",
            Some(detail),
            &correlation,
        )
        .into_response(),
    }
}

/// The authenticated caller, when present.
pub fn principal_of(request: &Request) -> Option<&Principal> {
    request.extensions().get::<Principal>()
}
