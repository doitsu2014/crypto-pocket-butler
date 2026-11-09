//! Gateway errors, each with a stable HTTP status and problem type.

/// A request the gateway refuses or cannot complete.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GatewayError {
    /// BR4.1: the token is missing, malformed, expired, or fails validation.
    #[error("unauthorized: {0}")]
    Unauthorized(String),

    /// The caller lacks the role a route requires.
    #[error("forbidden: {0}")]
    Forbidden(String),

    /// BR4.2: no declared route matches.
    #[error("not found: {0}")]
    NotFound(String),

    /// BR4.3: the request is malformed, including an invalid correlation id.
    #[error("bad request: {0}")]
    BadRequest(String),

    /// JWKS is unavailable beyond the staleness window.
    #[error("service unavailable: {0}")]
    ServiceUnavailable(String),

    /// The owning service could not be reached or returned a transport failure.
    #[error("bad gateway: {0}")]
    BadGateway(String),

    /// The owning service did not answer in time.
    #[error("gateway timeout: {0}")]
    GatewayTimeout(String),
}

impl GatewayError {
    /// The HTTP status code.
    #[must_use]
    pub fn status(&self) -> u16 {
        match self {
            Self::Unauthorized(_) => 401,
            Self::Forbidden(_) => 403,
            Self::NotFound(_) => 404,
            Self::BadRequest(_) => 400,
            Self::ServiceUnavailable(_) => 503,
            Self::BadGateway(_) => 502,
            Self::GatewayTimeout(_) => 504,
        }
    }

    /// The stable problem-type slug.
    #[must_use]
    pub fn problem_type(&self) -> &'static str {
        match self {
            Self::Unauthorized(_) => "unauthorized",
            Self::Forbidden(_) => "forbidden",
            Self::NotFound(_) => "not-found",
            Self::BadRequest(_) => "bad-request",
            Self::ServiceUnavailable(_) => "service-unavailable",
            Self::BadGateway(_) => "bad-gateway",
            Self::GatewayTimeout(_) => "gateway-timeout",
        }
    }

    /// The short problem title.
    #[must_use]
    pub fn title(&self) -> &'static str {
        match self {
            Self::Unauthorized(_) => "Unauthorized",
            Self::Forbidden(_) => "Forbidden",
            Self::NotFound(_) => "Not found",
            Self::BadRequest(_) => "Bad request",
            Self::ServiceUnavailable(_) => "Service unavailable",
            Self::BadGateway(_) => "Bad gateway",
            Self::GatewayTimeout(_) => "Gateway timeout",
        }
    }

    /// The human-readable detail.
    #[must_use]
    pub fn detail(&self) -> &str {
        match self {
            Self::Unauthorized(detail)
            | Self::Forbidden(detail)
            | Self::NotFound(detail)
            | Self::BadRequest(detail)
            | Self::ServiceUnavailable(detail)
            | Self::BadGateway(detail)
            | Self::GatewayTimeout(detail) => detail,
        }
    }
}
