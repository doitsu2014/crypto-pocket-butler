//! Ports defined inward and implemented outward by `gateway-infrastructure`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use gateway_domain::{Method, ServiceName};

pub use gateway_domain::Principal;

/// A request as it reached the gateway.
#[derive(Debug, Clone)]
pub struct IncomingRequest {
    /// The request method.
    pub method: Method,
    /// The request path, without the query string.
    pub path: String,
    /// The raw query string, without `?`.
    pub query: Option<String>,
    /// Request headers as name/value pairs.
    pub headers: Vec<(String, String)>,
    /// The request body.
    pub body: Vec<u8>,
}

impl IncomingRequest {
    /// Finds a header case-insensitively.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

/// A request to forward to the owning service.
#[derive(Debug, Clone)]
pub struct ForwardRequest {
    /// The owning service.
    pub service: ServiceName,
    /// The request method.
    pub method: Method,
    /// The path and query string.
    pub path_and_query: String,
    /// Headers to forward, including the unchanged user JWT.
    pub headers: Vec<(String, String)>,
    /// The request body.
    pub body: Vec<u8>,
    /// The correlation id.
    pub correlation_id: String,
}

impl ForwardRequest {
    /// Finds a header case-insensitively.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// Whether a header is already present.
    #[must_use]
    pub fn has_header(&self, name: &str) -> bool {
        self.header(name).is_some()
    }
}

/// A response returned by the owning service.
#[derive(Debug, Clone)]
pub struct UpstreamResponse {
    /// The status code.
    pub status: u16,
    /// Response headers.
    pub headers: Vec<(String, String)>,
    /// The response body.
    pub body: Vec<u8>,
    /// The content type, when one was returned.
    pub content_type: Option<String>,
}

/// A transport failure talking to the owning service.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UpstreamError {
    /// The owning service did not answer in time.
    #[error("upstream timed out")]
    Timeout,
    /// The owning service could not be reached.
    #[error("upstream unavailable: {0}")]
    Unavailable(String),
    /// The owning service refused the request at the transport layer.
    #[error("upstream rejected the request: {0}")]
    Rejected(String),
}

/// Forwards a request to the owning service with bounded retries.
#[async_trait]
pub trait UpstreamClient: Send + Sync {
    /// Forwards one attempt.
    async fn forward(&self, request: ForwardRequest) -> Result<UpstreamResponse, UpstreamError>;
}

/// A token-validation failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AuthError {
    /// No bearer token was presented.
    #[error("missing bearer token")]
    Missing,
    /// The token failed signature, issuer, audience, expiry, or shape checks.
    #[error("invalid token: {0}")]
    Invalid(String),
    /// JWKS could not be refreshed inside the staleness window.
    #[error("identity provider unavailable: {0}")]
    Unavailable(String),
}

/// Validates a Supabase JWT against JWKS before any routing (BR4.1).
#[async_trait]
pub trait TokenValidator: Send + Sync {
    /// Validates the token and returns the caller.
    async fn validate(&self, token: &str) -> Result<Principal, AuthError>;
}

/// An audit failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AuditError {
    /// The audit destination is unavailable.
    #[error("audit sink unavailable: {0}")]
    Unavailable(String),
    /// The audit destination rejected the event.
    #[error("audit sink rejected the event: {0}")]
    Rejected(String),
}

/// One gateway denial.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEvent {
    /// The actor, or `anonymous`.
    pub actor: String,
    /// What happened.
    pub action: String,
    /// The refused request target.
    pub target: String,
    /// `denied` or `success`.
    pub outcome: String,
    /// The request correlation id.
    pub correlation_id: Option<String>,
    /// When it happened.
    pub at: DateTime<Utc>,
}

impl AuditEvent {
    /// Creates an audit event.
    ///
    /// # Errors
    /// Returns [`AuditError::Rejected`] when a required field is blank.
    pub fn new(
        actor: impl Into<String>,
        action: impl Into<String>,
        target: impl Into<String>,
        outcome: impl Into<String>,
        correlation_id: Option<String>,
        at: DateTime<Utc>,
    ) -> Result<Self, AuditError> {
        let actor = actor.into();
        let action = action.into();
        let target = target.into();
        let outcome = outcome.into();
        if actor.trim().is_empty()
            || action.trim().is_empty()
            || target.trim().is_empty()
            || outcome.trim().is_empty()
        {
            return Err(AuditError::Rejected(
                "audit event requires actor, action, target, and outcome".to_string(),
            ));
        }
        Ok(Self {
            actor,
            action,
            target,
            outcome,
            correlation_id,
            at,
        })
    }
}

/// Records gateway denials.
#[async_trait]
pub trait AuditSink: Send + Sync {
    /// Records one event.
    async fn record(&self, event: &AuditEvent) -> Result<(), AuditError>;
}

/// An injectable clock.
pub trait Clock: Send + Sync {
    /// The current UTC time.
    fn now(&self) -> DateTime<Utc>;
}

/// Reports whether the gateway can serve traffic.
#[async_trait]
pub trait ReadinessProbe: Send + Sync {
    /// Checks the gateway's critical dependency.
    ///
    /// # Errors
    /// Returns a human-readable reason when the gateway is not ready.
    async fn check(&self) -> Result<(), String>;
}
