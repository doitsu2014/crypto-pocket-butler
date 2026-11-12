//! Internal service identities and the operations each may call.
//!
//! Each calling service holds its own token; the registry maps a token hash to
//! the caller name and a per-operation allow-list (security design). The
//! `support_metadata` operation lets only Identity read the support projection
//! (BR3.9).

use crate::error::DomainError;

/// The internal operations exposed by the Portfolio service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// `GET /v1/portfolios/{portfolioId}/snapshots` (C8).
    SnapshotsRead,
    /// `GET /v1/users/{userId}/portfolios` (C8, Reporting).
    PortfoliosRead,
    /// `GET /v1/portfolios/{portfolioId}/valuation` (C8, Reporting).
    ValuationRead,
    /// `GET /v1/internal/privacy/export`.
    PrivacyExport,
    /// `POST /v1/internal/privacy/anonymise`.
    PrivacyAnonymise,
    /// `POST /v1/internal/run/portfolio-valuation`.
    RunValuation,
    /// `GET /v1/internal/support-metadata/{userId}` (BR3.9).
    SupportMetadata,
}

impl Operation {
    /// The stored label used by `portfolio.service_token_registry`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SnapshotsRead => "snapshots_read",
            Self::PortfoliosRead => "portfolios_read",
            Self::ValuationRead => "valuation_read",
            Self::PrivacyExport => "privacy_export",
            Self::PrivacyAnonymise => "privacy_anonymise",
            Self::RunValuation => "run_valuation",
            Self::SupportMetadata => "support_metadata",
        }
    }

    /// Parses a stored operation label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown operation.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "snapshots_read" => Ok(Self::SnapshotsRead),
            "portfolios_read" => Ok(Self::PortfoliosRead),
            "valuation_read" => Ok(Self::ValuationRead),
            "privacy_export" => Ok(Self::PrivacyExport),
            "privacy_anonymise" => Ok(Self::PrivacyAnonymise),
            "run_valuation" => Ok(Self::RunValuation),
            "support_metadata" => Ok(Self::SupportMetadata),
            _ => Err(DomainError::Invalid {
                field: "operation",
                reason: "is not a registered Portfolio operation".to_string(),
            }),
        }
    }
}

/// Whether a registered caller may call one operation. An unknown caller is
/// refused for every operation; there is no shared super-token.
#[must_use]
pub fn service_allowed(service: &str, operation: Operation) -> bool {
    match service.trim().to_ascii_lowercase().as_str() {
        "reporting" | "smoke-reader" | "smoke_reader" => matches!(
            operation,
            Operation::SnapshotsRead | Operation::PortfoliosRead | Operation::ValuationRead
        ),
        "identity" => matches!(
            operation,
            Operation::PrivacyExport | Operation::PrivacyAnonymise | Operation::SupportMetadata
        ),
        "job-orchestration" | "job_orchestration" => {
            matches!(operation, Operation::RunValuation)
        }
        _ => false,
    }
}
