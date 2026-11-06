//! Correlation ids, security headers, request metrics, service-token auth, JWT
//! roles, and per-replica rate limiting.

use std::time::Duration;

use audit_application::hashing::sha256_hex;
use audit_application::ports::{AuthError, Principal, Role};
use audit_application::Context;
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

/// The authenticated emitting service.
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

fn unavailable(correlation: &str, detail: String) -> Response {
    ProblemDetails::new(
        503,
        "identity-unavailable",
        "Identity provider unavailable",
        Some(detail),
        correlation,
    )
    .into_response()
}

/// Validates the internal service token and resolves the emitting service.
/// Only a caller holding a token issued to a registered service may append.
pub async fn require_service(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let correlation = correlation_of(&request);
    let Some(token) = bearer_token(&request).map(ToString::to_string) else {
        return unauthorized(&correlation, "missing service token");
    };
    match state.service_tokens.service_for(&token) {
        Some(service) => {
            request.extensions_mut().insert(ServicePrincipal(service));
            next.run(request).await
        }
        None => unauthorized(&correlation, "unknown service token"),
    }
}

/// Validates the JWT and requires the admin role.
pub async fn require_admin(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    require_role(state, request, next, Role::Admin, "admin").await
}

/// Validates the JWT and requires the support role.
pub async fn require_support(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    require_role(state, request, next, Role::Support, "support").await
}

async fn require_role(
    state: AppState,
    mut request: Request,
    next: Next,
    required: Role,
    label: &'static str,
) -> Response {
    let correlation = correlation_of(&request);
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let Some(token) = bearer_token(&request).map(ToString::to_string) else {
        return unauthorized(&correlation, "missing bearer token");
    };
    match state.token_validator.validate(&token).await {
        Ok(principal) if principal.role == required => {
            request.extensions_mut().insert(principal);
            next.run(request).await
        }
        Ok(principal) => {
            record_denial(&state, &principal, &method, &path, &correlation).await;
            ProblemDetails::new(
                403,
                "forbidden",
                "Forbidden",
                Some(format!("{label} role required")),
                &correlation,
            )
            .into_response()
        }
        Err(AuthError::Missing) => unauthorized(&correlation, "missing bearer token"),
        Err(AuthError::Invalid(detail)) => unauthorized(&correlation, &detail),
        Err(AuthError::Unavailable(detail)) => unavailable(&correlation, detail),
    }
}

async fn record_denial(
    state: &AppState,
    principal: &Principal,
    method: &axum::http::Method,
    path: &str,
    correlation: &str,
) {
    let mut context = Context::empty();
    let _ = context.insert("correlationId", correlation);
    if let Err(error) = state
        .trail
        .record_denial(
            audit_domain::UserId::parse(&principal.subject).ok(),
            audit_domain::ActorKind::User,
            "audit.operator.denied",
            &format!("{method} {path}"),
            context,
        )
        .await
    {
        tracing::error!(%error, "failed to audit an operator denial");
    }
}

fn limited(correlation: &str) -> Response {
    let mut response = ProblemDetails::new(
        429,
        "rate-limited",
        "Too many requests",
        Some("the per-replica limit for this path is exhausted".to_string()),
        correlation,
    )
    .into_response();
    response
        .headers_mut()
        .insert(header::RETRY_AFTER, HeaderValue::from_static("60"));
    response
}

fn limiter_key(request: &Request) -> String {
    match bearer_token(request) {
        Some(token) => sha256_hex(token),
        None => "anonymous".to_string(),
    }
}

/// Limits appends per emitting token per minute.
pub async fn limit_appends(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let correlation = correlation_of(&request);
    let key = format!("append:{}", limiter_key(&request));
    if !state.limiter.allow(
        &key,
        state.limits.appends_per_minute,
        Duration::from_secs(60),
    ) {
        return limited(&correlation);
    }
    next.run(request).await
}

/// Limits admin queries per caller per minute.
pub async fn limit_queries(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let correlation = correlation_of(&request);
    let key = format!("query:{}", limiter_key(&request));
    if !state.limiter.allow(
        &key,
        state.limits.queries_per_minute,
        Duration::from_secs(60),
    ) {
        return limited(&correlation);
    }
    next.run(request).await
}

/// Limits exports per caller per hour.
pub async fn limit_exports(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let correlation = correlation_of(&request);
    let key = format!("export:{}", limiter_key(&request));
    if !state.limiter.allow(
        &key,
        state.limits.exports_per_hour,
        Duration::from_secs(3_600),
    ) {
        return limited(&correlation);
    }
    next.run(request).await
}

/// The authenticated caller, when present.
pub fn principal_of(request: &Request) -> Option<&Principal> {
    request.extensions().get::<Principal>()
}

/// The authenticated emitting service, when present.
pub fn service_of(request: &Request) -> Option<&ServicePrincipal> {
    request.extensions().get::<ServicePrincipal>()
}
