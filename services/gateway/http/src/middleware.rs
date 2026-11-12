//! Correlation ids, security headers, request metrics, the trusted client
//! address, and rate limiting.
//!
//! Rate limiting runs the address budget first and then the declared per-route
//! budget for the four support-access paths (FR16.3–FR16.6, NFR11.1).

use std::net::{IpAddr, SocketAddr};

use axum::extract::connect_info::ConnectInfo;
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

/// The header the gateway sets for the client address it observed. It is the only
/// address source a downstream service may trust (NFR23.3).
pub const CLIENT_IP_HEADER: &str = "x-client-ip";

/// A client-supplied forwarding header. It is stripped at the edge and never
/// forwarded.
const FORWARDED_FOR_HEADER: &str = "x-forwarded-for";

/// The header the trusted edge (CDN/WAF) sets from the real peer.
const EDGE_CLIENT_IP_HEADER: &str = "x-real-ip";

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

/// Adds the security headers required on every response.
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

/// Derives the client address from the observed connection, or from the trusted
/// edge header, and exposes it as `x-client-ip`.
///
/// Any inbound `x-forwarded-for` or `x-client-ip` is stripped first: a client can
/// write those headers, so they are never trusted. The derived value is parsed as
/// an IP address before it is forwarded, so a malformed edge header cannot inject
/// a header value (NFR23.3).
pub async fn client_ip(mut request: Request, next: Next) -> Response {
    let headers = request.headers_mut();
    headers.remove(FORWARDED_FOR_HEADER);
    headers.remove(CLIENT_IP_HEADER);

    let observed = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(address)| address.ip())
        .or_else(|| {
            request
                .headers()
                .get(EDGE_CLIENT_IP_HEADER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.trim().parse::<IpAddr>().ok())
        });

    let headers = request.headers_mut();
    headers.remove(EDGE_CLIENT_IP_HEADER);
    if let Some(address) = observed {
        if let Ok(value) = HeaderValue::from_str(&address.to_string()) {
            headers.insert(CLIENT_IP_HEADER, value);
        }
    }
    next.run(request).await
}

/// Enforces the shared per-caller budget and, on a route that declares one, the
/// route's own per-token budget (BR4 / security design, NFR11.1). The address
/// key is the address the gateway observed, never a caller-supplied header; the
/// route bucket is keyed on a hash of the bearer token, never the `sub` claim.
pub async fn rate_limit(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let correlation = correlation_of(&request);
    let key = request
        .headers()
        .get(CLIENT_IP_HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown")
        .to_string();
    let authenticated = request.headers().contains_key(header::AUTHORIZATION);

    if let Err(retry_after) = state.rate_limiter.check(&key, authenticated) {
        return rate_limited(&correlation, retry_after);
    }

    // The route budget is the stricter, per-token limit on a manual write or a
    // metadata read. It runs in a separate bucket from the address budget so
    // one cannot evict the other.
    let route_budget = gateway_domain::Method::parse(request.method().as_str())
        .ok()
        .and_then(|method| state.use_case.route_budget(method, request.uri().path()));
    if let Some((route, limit)) = route_budget {
        let token = request
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .map(str::trim)
            .filter(|token| !token.is_empty());
        if let Some(token) = token {
            if let Err(retry_after) = state.rate_limiter.check_route(route, token, limit) {
                return rate_limited(&correlation, retry_after);
            }
        }
    }
    next.run(request).await
}

fn rate_limited(correlation: &str, retry_after: u64) -> Response {
    let mut response = ProblemDetails::new(
        429,
        "rate-limited",
        "Too many requests",
        Some(format!("retry after {retry_after} seconds")),
        correlation,
    )
    .into_response();
    if let Ok(value) = HeaderValue::from_str(&retry_after.to_string()) {
        response.headers_mut().insert(header::RETRY_AFTER, value);
    }
    response
}
