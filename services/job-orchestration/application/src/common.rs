//! Shared application types: the refusal surface, the caller and the role check.

use job_orchestration_domain::{DomainError, UserId};

use crate::ports::{RepositoryError, VerifiedClaims};

/// The application's refusal surface.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AppError {
    /// No valid authentication was presented.
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    /// The caller is authenticated but not permitted.
    #[error("forbidden: {0}")]
    Forbidden(String),
    /// The resource does not exist.
    #[error("not found: {0}")]
    NotFound(&'static str),
    /// One or more fields failed validation.
    #[error("validation failed: {0}")]
    Validation(String),
    /// The action conflicts with the record's state.
    #[error("conflict: {0}")]
    Conflict(String),
    /// A dependency could not serve the request.
    #[error("unavailable: {0}")]
    Unavailable(String),
    /// An invariant was violated.
    #[error("internal error: {0}")]
    Internal(String),
}

impl AppError {
    /// Maps a domain refusal.
    #[must_use]
    pub fn from_domain(error: DomainError) -> Self {
        match error {
            DomainError::Denied { reason } => Self::Forbidden(reason),
            DomainError::Required { field } => Self::Validation(format!("{field} is required")),
            DomainError::Invalid { field, reason } => {
                Self::Validation(format!("{field}: {reason}"))
            }
            DomainError::InvalidTransition { entity, from, to } => {
                Self::Conflict(format!("{entity} cannot move from {from} to {to}"))
            }
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
}

/// The validated caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caller {
    /// The user id.
    pub user_id: UserId,
    /// The application role from `app_metadata.role`.
    pub role: String,
    /// The correlation id.
    pub correlation_id: String,
}

impl Caller {
    /// Builds a caller from validated claims.
    #[must_use]
    pub fn from_claims(claims: &VerifiedClaims, correlation_id: &str) -> Self {
        Self {
            user_id: claims.user_id,
            role: claims.role.clone(),
            correlation_id: correlation_id.to_string(),
        }
    }

    /// Enforces the operator role (BR7.1). Only `app_metadata.role` counts.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for any other role.
    pub fn require_operator(&self) -> Result<(), AppError> {
        if self.role == "admin" || self.role == "support" {
            Ok(())
        } else {
            Err(AppError::Forbidden(
                "the operator role is required".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caller(role: &str) -> Caller {
        Caller {
            user_id: UserId::generate(),
            role: role.to_string(),
            correlation_id: "c-1".to_string(),
        }
    }

    #[test]
    fn admin_and_support_are_operators() {
        assert!(caller("admin").require_operator().is_ok());
        assert!(caller("support").require_operator().is_ok());
    }

    #[test]
    fn a_user_role_is_refused() {
        assert!(caller("user").require_operator().is_err());
        assert!(caller("authenticated").require_operator().is_err());
        assert!(caller("").require_operator().is_err());
    }

    #[test]
    fn a_domain_validation_error_names_the_field() {
        let mapped = AppError::from_domain(DomainError::Invalid {
            field: "reason",
            reason: "exceeds 200 characters".to_string(),
        });
        assert!(matches!(mapped, AppError::Validation(_)));
    }

    #[test]
    fn a_repository_outage_is_unavailable() {
        assert!(matches!(
            AppError::from_repository(RepositoryError::Unavailable("down".to_string())),
            AppError::Unavailable(_)
        ));
    }
}
