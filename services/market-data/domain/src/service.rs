//! Internal service identities and the operations each may call (BR5.1).
//!
//! Each caller holds its own token; the registry maps a token hash to the caller
//! name and a per-operation allow-list.

use crate::error::DomainError;

/// The internal operations Market Data exposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// `GET /v1/prices` (C6).
    PricesRead,
    /// `GET /v1/fx` (C6).
    FxRead,
    /// `POST /v1/internal/run/{jobName}` (C11).
    RunRefresh,
}

impl Operation {
    /// The stored label used by `market_data.service_token_registry`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PricesRead => "prices_read",
            Self::FxRead => "fx_read",
            Self::RunRefresh => "run_refresh",
        }
    }

    /// Parses a stored operation label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown operation.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "prices_read" => Ok(Self::PricesRead),
            "fx_read" => Ok(Self::FxRead),
            "run_refresh" => Ok(Self::RunRefresh),
            _ => Err(DomainError::Invalid {
                field: "operation",
                reason: "is not a registered Market Data operation".to_string(),
            }),
        }
    }
}
