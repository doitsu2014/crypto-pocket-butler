//! Shared application types: the error surface, the caller, and the principal
//! response. Nothing here imports a transport or storage crate.

use identity_domain::claims::{NextStep, Role, VerifiedClaims};
use identity_domain::{DomainError, User, UserId, UserStatus};

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
    /// The window the action needed has closed; the resource is gone or due to
    /// be erased.
    #[error("gone: {0}")]
    Gone(String),
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
            RepositoryError::Validation(detail) => {
                Self::Validation(vec![FieldError::new("subject", detail)])
            }
            RepositoryError::InvalidData(detail) => {
                Self::Internal(format!("stored data is invalid: {detail}"))
            }
        }
    }

    /// Maps an external dependency failure. Both variants become a 503: the
    /// caller can retry, and the refusal reason is preserved.
    #[must_use]
    pub fn from_external(error: ExternalError) -> Self {
        Self::Unavailable(error.to_string())
    }
}

/// The authenticated caller: the stored user and the claims it was derived from.
#[derive(Debug, Clone)]
pub struct Caller {
    /// The internal user record.
    pub user: User,
    /// The validated JWT facts.
    pub claims: VerifiedClaims,
    /// The request correlation id, propagated to the audit trail.
    pub correlation_id: String,
}

impl Caller {
    /// The internal user identifier.
    #[must_use]
    pub fn user_id(&self) -> UserId {
        self.user.id()
    }

    /// The capability role, taken from the validated token (never the stored
    /// row and never a request header).
    #[must_use]
    pub fn role(&self) -> Role {
        self.claims.role
    }

    /// Whether the caller holds the admin capability.
    #[must_use]
    pub fn is_admin(&self) -> bool {
        self.role().is_admin()
    }

    /// Whether the caller holds the support capability.
    #[must_use]
    pub fn is_support(&self) -> bool {
        self.role().is_support()
    }

    /// Refuses unless the caller owns the record (BR1.4).
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for another user's record.
    pub fn require_owner(&self, user_id: UserId) -> Result<(), AppError> {
        if self.user.id() == user_id {
            Ok(())
        } else {
            Err(AppError::Forbidden(
                "the resource belongs to another user".to_string(),
            ))
        }
    }

    /// Refuses unless the caller owns the record or holds the admin capability.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] otherwise.
    pub fn require_owner_or_admin(&self, user_id: UserId) -> Result<(), AppError> {
        if self.user.id() == user_id || self.is_admin() {
            Ok(())
        } else {
            Err(AppError::Forbidden(
                "the resource belongs to another user".to_string(),
            ))
        }
    }
}

/// The principal returned to the gateway (C3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrincipalView {
    user_id: UserId,
    role: Role,
    status: UserStatus,
    next_step: NextStep,
}

impl PrincipalView {
    /// Creates the view.
    #[must_use]
    pub fn new(user_id: UserId, role: Role, status: UserStatus, next_step: NextStep) -> Self {
        Self {
            user_id,
            role,
            status,
            next_step,
        }
    }

    /// The internal user identifier.
    #[must_use]
    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    /// The capability role.
    #[must_use]
    pub fn role(&self) -> Role {
        self.role
    }

    /// The lifecycle state.
    #[must_use]
    pub fn status(&self) -> UserStatus {
        self.status
    }

    /// What the caller must do next before portfolio data is reachable.
    #[must_use]
    pub fn next_step(&self) -> NextStep {
        self.next_step
    }
}
