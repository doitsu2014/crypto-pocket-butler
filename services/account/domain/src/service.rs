//! Internal service identities and the operations each may call.
//!
//! Each calling service holds its own token; the registry maps a token hash to
//! the caller name and a per-operation allow-list (security design).

use crate::error::DomainError;

/// The internal operations exposed by the Account service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// `GET /v1/internal/privacy/export`.
    PrivacyExport,
    /// `POST /v1/internal/privacy/anonymise`.
    PrivacyAnonymise,
    /// `GET /v1/users/{userId}/holdings`.
    HoldingsRead,
    /// `GET /v1/accounts/{accountId}/transactions`.
    TransactionsRead,
    /// `POST /v1/internal/run/account-sync`.
    RunScheduledSync,
}

impl Operation {
    /// The stored label used by `account.service_token_registry`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PrivacyExport => "privacy_export",
            Self::PrivacyAnonymise => "privacy_anonymise",
            Self::HoldingsRead => "holdings_read",
            Self::TransactionsRead => "transactions_read",
            Self::RunScheduledSync => "run_scheduled_sync",
        }
    }

    /// Parses a stored operation label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown operation.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "privacy_export" => Ok(Self::PrivacyExport),
            "privacy_anonymise" => Ok(Self::PrivacyAnonymise),
            "holdings_read" => Ok(Self::HoldingsRead),
            "transactions_read" => Ok(Self::TransactionsRead),
            "run_scheduled_sync" => Ok(Self::RunScheduledSync),
            _ => Err(DomainError::Invalid {
                field: "operation",
                reason: "is not a registered Account operation".to_string(),
            }),
        }
    }
}

/// Whether a registered caller may call one operation. An unknown caller is
/// refused for every operation; there is no shared super-token.
#[must_use]
pub fn service_allowed(service: &str, operation: Operation) -> bool {
    match service.trim().to_ascii_lowercase().as_str() {
        "identity" => matches!(
            operation,
            Operation::PrivacyExport | Operation::PrivacyAnonymise
        ),
        "portfolio" | "ledger" => matches!(
            operation,
            Operation::HoldingsRead | Operation::TransactionsRead
        ),
        "job-orchestration" | "job_orchestration" => {
            matches!(operation, Operation::RunScheduledSync)
        }
        _ => false,
    }
}
