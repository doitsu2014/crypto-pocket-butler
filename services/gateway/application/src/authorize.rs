//! The gateway request path: validate the JWT, deny by default, attach the
//! correlation id, and forward the request with the JWT unchanged (BR4.1–BR4.3).
//!
//! `route_budget` exposes the declared per-route limit for the support-access
//! paths (FR16.3–FR16.6, NFR11.1); the HTTP layer applies it after the address
//! budget.

use std::sync::Arc;

use gateway_domain::{CorrelationId, GatewayError, Method, Role, RouteTable};
use uuid::Uuid;

use crate::ports::{
    AuditEvent, AuditSink, AuthError, Clock, ForwardRequest, IncomingRequest, TokenValidator,
    UpstreamClient, UpstreamError,
};

/// What the gateway returns to the caller.
#[derive(Debug, Clone)]
pub struct RoutedResponse {
    /// The status code.
    pub status: u16,
    /// Response headers.
    pub headers: Vec<(String, String)>,
    /// The response body.
    pub body: Vec<u8>,
    /// The content type.
    pub content_type: Option<String>,
    /// The correlation id, echoed to the caller.
    pub correlation_id: String,
}

/// Authenticates, authorizes, and routes one request.
pub struct AuthorizeAndRoute {
    validator: Arc<dyn TokenValidator>,
    upstream: Arc<dyn UpstreamClient>,
    audit: Arc<dyn AuditSink>,
    clock: Arc<dyn Clock>,
    routes: RouteTable,
}

impl AuthorizeAndRoute {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        validator: Arc<dyn TokenValidator>,
        upstream: Arc<dyn UpstreamClient>,
        audit: Arc<dyn AuditSink>,
        clock: Arc<dyn Clock>,
        routes: RouteTable,
    ) -> Self {
        Self {
            validator,
            upstream,
            audit,
            clock,
            routes,
        }
    }

    /// The matched route's label and its per-route budget, when the table
    /// declares one (NFR11.1). The middleware applies it before the request
    /// reaches the use case.
    #[must_use]
    pub fn route_budget(&self, method: Method, path: &str) -> Option<(&str, u32)> {
        let route = self.routes.match_route(method, path)?;
        let limit = route.max_requests_per_minute?;
        Some((route.pattern_label(), limit))
    }

    /// Runs the gateway request path.
    ///
    /// # Errors
    /// Returns [`GatewayError`] for an invalid correlation id, a missing or invalid
    /// token, an undeclared route, a missing role, an unavailable identity
    /// provider, or an upstream failure.
    pub async fn execute(&self, request: IncomingRequest) -> Result<RoutedResponse, GatewayError> {
        let correlation = self.correlation(&request)?;

        let token = request
            .header("authorization")
            .and_then(|value| value.strip_prefix("Bearer "))
            .map(str::trim)
            .filter(|token| !token.is_empty());
        let Some(token) = token.map(ToString::to_string) else {
            self.deny("anonymous", &request, &correlation).await;
            return Err(GatewayError::Unauthorized(
                "missing bearer token".to_string(),
            ));
        };

        let principal = match self.validator.validate(&token).await {
            Ok(principal) => principal,
            Err(AuthError::Missing) => {
                self.deny("anonymous", &request, &correlation).await;
                return Err(GatewayError::Unauthorized(
                    "missing bearer token".to_string(),
                ));
            }
            Err(AuthError::Invalid(detail)) => {
                self.deny("anonymous", &request, &correlation).await;
                return Err(GatewayError::Unauthorized(detail));
            }
            Err(AuthError::Unavailable(detail)) => {
                return Err(GatewayError::ServiceUnavailable(detail));
            }
        };

        let Some(route) = self
            .routes
            .match_route(request.method, &request.path)
            .cloned()
        else {
            self.deny(&principal.subject, &request, &correlation).await;
            return Err(GatewayError::NotFound(format!(
                "no declared route matches {} {}",
                request.method.as_str(),
                request.path
            )));
        };

        if route.requires_admin && principal.role != Role::Admin {
            self.deny(&principal.subject, &request, &correlation).await;
            return Err(GatewayError::Forbidden("admin role required".to_string()));
        }

        self.forward(&route, &request, &correlation).await
    }

    fn correlation(&self, request: &IncomingRequest) -> Result<CorrelationId, GatewayError> {
        match request.header("x-correlation-id") {
            Some(raw) => CorrelationId::parse(raw),
            None => CorrelationId::parse(&Uuid::new_v4().to_string()),
        }
    }

    async fn forward(
        &self,
        route: &gateway_domain::Route,
        request: &IncomingRequest,
        correlation: &CorrelationId,
    ) -> Result<RoutedResponse, GatewayError> {
        let mut headers = request.headers.clone();
        headers.retain(|(name, _)| !name.eq_ignore_ascii_case("x-correlation-id"));
        headers.push((
            "x-correlation-id".to_string(),
            correlation.as_str().to_string(),
        ));
        if !request.method.is_idempotent() {
            let has_key = headers
                .iter()
                .any(|(name, _)| name.eq_ignore_ascii_case("idempotency-key"));
            if !has_key {
                headers.push(("idempotency-key".to_string(), Uuid::new_v4().to_string()));
            }
        }

        let path_and_query = match &request.query {
            Some(query) if !query.is_empty() => format!("{}?{}", request.path, query),
            _ => request.path.clone(),
        };

        let forward = ForwardRequest {
            service: route.service.clone(),
            method: request.method,
            path_and_query,
            headers,
            body: request.body.clone(),
            correlation_id: correlation.as_str().to_string(),
        };

        let max_attempts = if request.method.is_idempotent() { 3 } else { 1 };
        let mut attempt = 1;
        loop {
            match self.upstream.forward(forward.clone()).await {
                Ok(response) => {
                    return Ok(RoutedResponse {
                        status: response.status,
                        headers: response.headers,
                        body: response.body,
                        content_type: response.content_type,
                        correlation_id: correlation.as_str().to_string(),
                    });
                }
                Err(error) => {
                    let retryable = matches!(
                        error,
                        UpstreamError::Timeout | UpstreamError::Unavailable(_)
                    );
                    if retryable && attempt < max_attempts {
                        attempt += 1;
                        continue;
                    }
                    return Err(match error {
                        UpstreamError::Timeout => GatewayError::GatewayTimeout(
                            "the owning service did not answer in time".to_string(),
                        ),
                        UpstreamError::Unavailable(detail) | UpstreamError::Rejected(detail) => {
                            GatewayError::BadGateway(detail)
                        }
                    });
                }
            }
        }
    }

    async fn deny(&self, actor: &str, request: &IncomingRequest, correlation: &CorrelationId) {
        let event = AuditEvent::new(
            actor,
            "gateway.denied",
            format!("{} {}", request.method.as_str(), request.path),
            "denied",
            Some(correlation.as_str().to_string()),
            self.clock.now(),
        );
        if let Ok(event) = event {
            if let Err(error) = self.audit.record(&event).await {
                tracing::warn!(%error, "failed to audit a gateway denial");
            }
        }
    }
}

/// Whether a method is retried without an idempotency key.
#[must_use]
pub fn is_idempotent(method: Method) -> bool {
    method.is_idempotent()
}
