//! Shared application types: the error surface, the authenticated user, and the
//! internal service caller. Nothing here imports a transport or storage crate.

use reporting_domain::service::{service_allowed, Operation};
use reporting_domain::{DomainError, UserId};

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

    /// Maps an external dependency failure.
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
    /// Two factors (MFA).
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

    /// BR3.1: a brief belongs to one user and only that user reads it.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for another user's brief.
    pub fn require_owner(&self, user_id: UserId) -> Result<(), AppError> {
        if self.user_id == user_id {
            Ok(())
        } else {
            Err(AppError::Forbidden(
                "the brief belongs to another user".to_string(),
            ))
        }
    }
}

/// The authenticated internal service.
#[derive(Debug, Clone)]
pub struct ServiceCaller {
    /// The registered caller name.
    pub service: String,
    /// The operations the registry grants this caller. `None` means a
    /// system-constructed caller, which falls back to the static allow-list.
    pub operations: Option<Vec<Operation>>,
    /// The request correlation id.
    pub correlation_id: String,
}

impl ServiceCaller {
    /// Creates a caller from the registry entry and the request.
    #[must_use]
    pub fn new(service: impl Into<String>, correlation_id: &str) -> Self {
        Self {
            service: service.into(),
            operations: None,
            correlation_id: correlation_id.to_string(),
        }
    }

    /// Creates a caller that honours exactly the operations the registry grants
    /// it, so narrowing or rotating a caller's allow-list takes effect.
    #[must_use]
    pub fn from_registered(
        registered: &crate::ports::RegisteredService,
        correlation_id: &str,
    ) -> Self {
        Self {
            service: registered.name.clone(),
            operations: Some(registered.operations.clone()),
            correlation_id: correlation_id.to_string(),
        }
    }

    /// Refuses an operation not on the caller's allow-list.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] when the caller may not call the
    /// operation.
    pub fn require_operation(&self, operation: Operation) -> Result<(), AppError> {
        let allowed = match &self.operations {
            Some(operations) => operations.contains(&operation),
            None => service_allowed(&self.service, operation),
        };
        if allowed {
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

#[cfg(test)]
mod tests {
    use super::*;
    use reporting_domain::DomainError;

    #[test]
    fn a_denied_domain_error_becomes_forbidden() {
        let mapped = AppError::from_domain(DomainError::Denied {
            reason: "no".to_string(),
        });
        assert!(matches!(mapped, AppError::Forbidden(_)));
    }

    #[test]
    fn a_required_field_becomes_a_validation_error_naming_the_field() {
        let mapped = AppError::from_domain(DomainError::Required { field: "period" });
        match mapped {
            AppError::Validation(errors) => {
                assert_eq!(errors.len(), 1);
                assert_eq!(errors[0].field, "period");
                assert_eq!(errors[0].reason, "is required");
            }
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    #[test]
    fn a_transition_refusal_becomes_a_conflict() {
        let mapped = AppError::from_domain(DomainError::InvalidTransition {
            entity: "brief",
            from: "generated",
            to: "generated",
        });
        assert!(matches!(mapped, AppError::Conflict(_)));
    }

    #[test]
    fn a_repository_outage_becomes_unavailable() {
        let mapped = AppError::from_repository(RepositoryError::Unavailable("down".to_string()));
        assert!(matches!(mapped, AppError::Unavailable(_)));
    }

    #[test]
    fn an_invalid_stored_row_becomes_internal() {
        let mapped = AppError::from_repository(RepositoryError::InvalidData("bad row".to_string()));
        assert!(matches!(mapped, AppError::Internal(_)));
    }

    #[test]
    fn an_external_refusal_becomes_unavailable() {
        let mapped = AppError::from_external(ExternalError::Refused("no".to_string()));
        assert!(matches!(mapped, AppError::Unavailable(_)));
    }

    #[test]
    fn the_assurance_level_defaults_to_aal1() {
        assert_eq!(AssuranceLevel::parse(None), AssuranceLevel::Aal1);
        assert_eq!(AssuranceLevel::parse(Some("aal2")), AssuranceLevel::Aal2);
        assert_eq!(
            AssuranceLevel::parse(Some("unexpected")),
            AssuranceLevel::Aal1
        );
        assert_eq!(AssuranceLevel::Aal2.as_str(), "aal2");
    }

    #[test]
    fn a_caller_may_open_its_own_brief_and_not_anothers() {
        let user = UserId::generate();
        let caller = Caller {
            user_id: user,
            assurance: AssuranceLevel::Aal1,
            correlation_id: "c-1".to_string(),
        };
        assert!(caller.require_owner(user).is_ok());
        assert!(caller.require_owner(UserId::generate()).is_err());
    }

    #[test]
    fn a_service_caller_is_refused_an_operation_it_does_not_hold() {
        let caller = ServiceCaller::new("identity", "c-1");
        assert!(caller.require_operation(Operation::PrivacyExport).is_ok());
        assert!(caller
            .require_operation(Operation::RunBriefGeneration)
            .is_err());
    }
}

#[cfg(test)]
mod registry_tests {
    use super::*;
    use crate::ports::RegisteredService;

    #[test]
    fn a_registered_caller_is_limited_to_its_stored_operations() {
        let caller = ServiceCaller::from_registered(
            &RegisteredService {
                name: "job-orchestration".to_string(),
                operations: vec![Operation::RunBriefGeneration],
            },
            "c-1",
        );
        assert!(caller
            .require_operation(Operation::RunBriefGeneration)
            .is_ok());
        assert!(caller.require_operation(Operation::PrivacyExport).is_err());
    }

    #[test]
    fn a_registered_caller_cannot_exceed_its_stored_list_even_when_the_table_would_allow_it() {
        let caller = ServiceCaller::from_registered(
            &RegisteredService {
                name: "identity".to_string(),
                operations: vec![],
            },
            "c-1",
        );
        assert!(caller
            .require_operation(Operation::PrivacyAnonymise)
            .is_err());
    }
}
