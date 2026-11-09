//! HTTP methods the route table can declare, kept free of the http crate.

use crate::error::GatewayError;

/// A supported request method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    /// GET.
    Get,
    /// POST.
    Post,
    /// PUT.
    Put,
    /// PATCH.
    Patch,
    /// DELETE.
    Delete,
    /// HEAD.
    Head,
    /// OPTIONS.
    Options,
}

impl Method {
    /// Parses an HTTP method.
    ///
    /// # Errors
    /// Returns [`GatewayError::BadRequest`] for an unsupported method.
    pub fn parse(raw: &str) -> Result<Self, GatewayError> {
        match raw.trim().to_ascii_uppercase().as_str() {
            "GET" => Ok(Self::Get),
            "POST" => Ok(Self::Post),
            "PUT" => Ok(Self::Put),
            "PATCH" => Ok(Self::Patch),
            "DELETE" => Ok(Self::Delete),
            "HEAD" => Ok(Self::Head),
            "OPTIONS" => Ok(Self::Options),
            other => Err(GatewayError::BadRequest(format!(
                "unsupported method '{other}'"
            ))),
        }
    }

    /// The wire representation.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
        }
    }

    /// Whether the method may be retried without an idempotency key.
    #[must_use]
    pub fn is_idempotent(self) -> bool {
        matches!(
            self,
            Self::Get | Self::Head | Self::Options | Self::Put | Self::Delete
        )
    }
}
