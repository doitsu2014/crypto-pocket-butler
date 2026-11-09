//! Shared application types: the error surface, the authenticated user, and the
//! internal service caller. Nothing here imports a transport or storage crate.

use account_domain::service::{service_allowed, Operation};
use account_domain::{DomainError, UserId};

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
    /// The resource does not exist for this caller.
    #[error("not found: {0}")]
    NotFound(&'static str),
    /// One or more fields failed validation.
    #[error("validation failed")]
    Validation(Vec<FieldError>),
    /// The action conflicts with the record's state.
    #[error("conflict: {0}")]
    Conflict(String),
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
            DomainError::InvalidTransition { entity, from, to } => {
                Self::Conflict(format!("{entity} cannot move from {from} to {to}"))
            }
            DomainError::InvalidId { entity } => {
                Self::Validation(vec![FieldError::new(entity, "is not a UUID")])
            }
            DomainError::SecretLike { field } => Self::Validation(vec![FieldError::new(
                field,
                "looks like key material and is never accepted",
            )]),
            DomainError::Conflict { reason } => Self::Conflict(reason),
        }
    }

    /// Maps a persistence failure.
    #[must_use]
    pub fn from_repository(error: RepositoryError) -> Self {
        match error {
            RepositoryError::Unavailable(detail) => Self::Unavailable(detail),
            RepositoryError::Conflict(detail) => Self::Conflict(detail),
            RepositoryError::Forbidden(detail) => Self::Forbidden(detail),
            RepositoryError::InvalidData(detail) => {
                Self::Internal(format!("stored data is invalid: {detail}"))
            }
        }
    }

    /// Maps an external dependency failure. Both variants become a 503: the caller
    /// can retry and the reason is preserved.
    #[must_use]
    pub fn from_external(error: ExternalError) -> Self {
        Self::Unavailable(error.to_string())
    }
}

/// The assurance level carried by the validated Supabase JWT.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssuranceLevel {
    /// A single factor.
    Aal1,
    /// Two factors (MFA). Required to store an exchange credential (BR2.4).
    Aal2,
}

impl AssuranceLevel {
    /// Reads the `aal` claim. An absent or unknown value is AAL1.
    #[must_use]
    pub fn parse(raw: Option<&str>) -> Self {
        match raw.map(str::trim) {
            Some("aal2") => Self::Aal2,
            _ => Self::Aal1,
        }
    }

    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Aal1 => "aal1",
            Self::Aal2 => "aal2",
        }
    }
}

/// The facts the service derives from a validated JWT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedClaims {
    /// The Supabase subject, used directly as the internal user id.
    pub user_id: UserId,
    /// The assurance level.
    pub assurance: AssuranceLevel,
}

/// The authenticated user.
#[derive(Debug, Clone)]
pub struct Caller {
    /// The user id from the validated token.
    pub user_id: UserId,
    /// The assurance level.
    pub assurance: AssuranceLevel,
    /// The request correlation id.
    pub correlation_id: String,
}

impl Caller {
    /// Creates a caller from validated claims.
    #[must_use]
    pub fn from_claims(claims: &VerifiedClaims, correlation_id: &str) -> Self {
        Self {
            user_id: claims.user_id,
            assurance: claims.assurance,
            correlation_id: correlation_id.to_string(),
        }
    }

    /// Refuses unless the caller owns the record (BR1.1).
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for another user's record. A support grant
    /// is checked separately and never becomes a blanket admin exception.
    pub fn require_owner(&self, user_id: UserId) -> Result<(), AppError> {
        if self.user_id == user_id {
            Ok(())
        } else {
            Err(AppError::Forbidden(
                "the resource belongs to another user".to_string(),
            ))
        }
    }

    /// Refuses an exchange-credential write below AAL2 (BR2.4).
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] at AAL1.
    pub fn require_mfa_for_exchange(&self) -> Result<(), AppError> {
        if self.assurance == AssuranceLevel::Aal2 {
            Ok(())
        } else {
            Err(AppError::Forbidden(
                "enrolling MFA is required before an exchange key is stored".to_string(),
            ))
        }
    }
}

/// The authenticated internal service.
#[derive(Debug, Clone)]
pub struct ServiceCaller {
    /// The registered caller name.
    pub service: String,
    /// The request correlation id.
    pub correlation_id: String,
}

impl ServiceCaller {
    /// Creates a caller from the registry entry and the request.
    #[must_use]
    pub fn new(service: impl Into<String>, correlation_id: &str) -> Self {
        Self {
            service: service.into(),
            correlation_id: correlation_id.to_string(),
        }
    }

    /// Refuses an operation not on the caller's allow-list.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] when the caller may not call the operation.
    pub fn require_operation(&self, operation: Operation) -> Result<(), AppError> {
        if service_allowed(&self.service, operation) {
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
