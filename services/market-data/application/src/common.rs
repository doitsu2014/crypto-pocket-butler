//! Shared application types: the error surface and the authenticated internal
//! service caller. Nothing here imports a transport or storage crate.

use market_data_domain::{DomainError, Operation, RefreshRunId};

use crate::ports::{ExternalError, RepositoryError};

/// One offending field path and why it was refused, never its value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldError {
    /// The field path.
    pub field: String,
    /// Why it was refused.
    pub reason: String,
}

impl FieldError {
    /// Creates a field problem.
    #[must_use]
    pub fn new(field: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            reason: reason.into(),
        }
    }
}

/// The application's refusal surface. The HTTP boundary maps each variant to a
/// status code and an RFC 7807 problem type.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AppError {
    /// No valid authentication was presented.
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    /// The caller is authenticated but not permitted.
    #[error("forbidden: {0}")]
    Forbidden(String),
    /// No fresh reading exists for the request.
    #[error("not found: {0}")]
    NotFound(&'static str),
    /// One or more fields failed validation.
    #[error("validation failed")]
    Validation(Vec<FieldError>),
    /// The action conflicts with the current state; a held run id may travel
    /// with the refusal so a manual caller can correlate.
    #[error("conflict: {detail}")]
    Conflict {
        /// The plain reason, safe to show the caller.
        detail: String,
        /// The active run, when the conflict is a held lease.
        run_id: Option<RefreshRunId>,
    },
    /// A dependency could not serve the request.
    #[error("unavailable: {0}")]
    Unavailable(String),
    /// An invariant was violated; this is a defect, not a client error.
    #[error("internal error: {0}")]
    Internal(String),
}

impl AppError {
    /// Maps a domain refusal.
    #[must_use]
    pub fn from_domain(error: DomainError) -> Self {
        match error {
            DomainError::Denied { reason } => Self::Forbidden(reason),
            DomainError::Required { field } => {
                Self::Validation(vec![FieldError::new(field, "is required")])
            }
            DomainError::Invalid { field, reason } => {
                Self::Validation(vec![FieldError::new(field, reason)])
            }
            DomainError::TooLarge { field, limit } => {
                Self::Validation(vec![FieldError::new(field, format!("exceeds {limit}"))])
            }
            DomainError::InvalidTransition { entity, from, to } => Self::Conflict {
                detail: format!("{entity} cannot move from {from} to {to}"),
                run_id: None,
            },
            DomainError::InvalidId { entity } => {
                Self::Validation(vec![FieldError::new(entity, "is not a UUID")])
            }
            DomainError::Conflict { reason } => Self::Conflict {
                detail: reason,
                run_id: None,
            },
        }
    }

    /// Maps a persistence failure.
    #[must_use]
    pub fn from_repository(error: RepositoryError) -> Self {
        match error {
            RepositoryError::Unavailable(detail) => Self::Unavailable(detail),
            RepositoryError::Conflict(detail) => Self::Conflict {
                detail,
                run_id: None,
            },
            RepositoryError::Forbidden(detail) => Self::Forbidden(detail),
            RepositoryError::InvalidData(detail) => {
                Self::Internal(format!("stored data is invalid: {detail}"))
            }
        }
    }

    /// Maps an external dependency failure. Both variants become a 503: the
    /// caller can retry and the reason is preserved.
    #[must_use]
    pub fn from_external(error: ExternalError) -> Self {
        Self::Unavailable(error.to_string())
    }

    /// A conflict carrying the active run id.
    #[must_use]
    pub fn held_run(detail: impl Into<String>, run_id: RefreshRunId) -> Self {
        Self::Conflict {
            detail: detail.into(),
            run_id: Some(run_id),
        }
    }
}

/// The authenticated internal service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceCaller {
    /// The registered caller name.
    pub service: String,
    /// The operations the registry row allows. The registry is authoritative;
    /// the service does not keep a second allow-list at runtime.
    pub operations: Vec<Operation>,
    /// The request correlation id.
    pub correlation_id: String,
}

impl ServiceCaller {
    /// Creates a caller from the registry entry and the request.
    #[must_use]
    pub fn new(
        service: impl Into<String>,
        operations: Vec<Operation>,
        correlation_id: &str,
    ) -> Self {
        Self {
            service: service.into(),
            operations,
            correlation_id: correlation_id.to_string(),
        }
    }

    /// Refuses an operation not on the caller's registry allow-list.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] when the caller may not call the operation.
    pub fn require_operation(&self, operation: Operation) -> Result<(), AppError> {
        if self.operations.contains(&operation) {
            Ok(())
        } else {
            Err(AppError::Forbidden(format!(
                "service '{}' may not call {}",
                self.service,
                operation.as_str()
            )))
        }
    }
}
