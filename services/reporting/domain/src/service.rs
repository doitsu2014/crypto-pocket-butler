//! Internal service identities and the operations each may call.
//!
//! Each calling service holds its own token; the registry maps a token hash to
//! the caller name and a per-operation allow-list (security design).

use crate::error::DomainError;

/// The internal operations exposed by the Reporting service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// `POST /v1/internal/run/brief-generation` (C11).
    RunBriefGeneration,
    /// `GET /v1/internal/privacy/export`.
    PrivacyExport,
    /// `POST /v1/internal/privacy/anonymise`.
    PrivacyAnonymise,
}

impl Operation {
    /// The stored label used by `reporting.service_token_registry`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RunBriefGeneration => "run_brief_generation",
            Self::PrivacyExport => "privacy_export",
            Self::PrivacyAnonymise => "privacy_anonymise",
        }
    }

    /// Parses a stored operation label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown operation.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "run_brief_generation" => Ok(Self::RunBriefGeneration),
            "privacy_export" => Ok(Self::PrivacyExport),
            "privacy_anonymise" => Ok(Self::PrivacyAnonymise),
            _ => Err(DomainError::Invalid {
                field: "operation",
                reason: "is not a registered Reporting operation".to_string(),
            }),
        }
    }
}

/// Whether a registered caller may call one operation. An unknown caller is
/// refused for every operation; there is no shared super-token.
#[must_use]
pub fn service_allowed(service: &str, operation: Operation) -> bool {
    match service.trim().to_ascii_lowercase().as_str() {
        "job-orchestration" | "job_orchestration" | "smoke-reader" | "smoke_reader" => {
            matches!(operation, Operation::RunBriefGeneration)
        }
        "identity" => matches!(
            operation,
            Operation::PrivacyExport | Operation::PrivacyAnonymise
        ),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{service_allowed, Operation};

    #[test]
    fn job_orchestration_may_run_the_brief_generation() {
        assert!(service_allowed(
            "job-orchestration",
            Operation::RunBriefGeneration
        ));
        assert!(!service_allowed(
            "job-orchestration",
            Operation::PrivacyExport
        ));
    }

    #[test]
    fn identity_may_export_and_anonymise_but_not_run() {
        assert!(service_allowed("identity", Operation::PrivacyExport));
        assert!(service_allowed("identity", Operation::PrivacyAnonymise));
        assert!(!service_allowed("identity", Operation::RunBriefGeneration));
    }

    #[test]
    fn an_unknown_caller_is_refused_everywhere() {
        for operation in [
            Operation::RunBriefGeneration,
            Operation::PrivacyExport,
            Operation::PrivacyAnonymise,
        ] {
            assert!(!service_allowed("gateway", operation));
            assert!(!service_allowed("reporting", operation));
        }
    }

    #[test]
    fn an_operation_round_trips_through_its_label() {
        for operation in [
            Operation::RunBriefGeneration,
            Operation::PrivacyExport,
            Operation::PrivacyAnonymise,
        ] {
            assert_eq!(Operation::parse(operation.as_str()), Ok(operation));
        }
        assert!(Operation::parse("run_valuation").is_err());
    }
}
