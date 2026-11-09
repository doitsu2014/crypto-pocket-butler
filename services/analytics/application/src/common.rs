//! Shared shapes for a refused request.

use analytics_domain::DomainError;

/// One offending field path, never its value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldError {
    /// The field path, e.g. `eventName`.
    pub field: String,
    /// Why it was refused, phrased without echoing the value.
    pub reason: String,
}

impl FieldError {
    /// Creates a field error.
    #[must_use]
    pub fn new(field: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            reason: reason.into(),
        }
    }
}

impl From<DomainError> for FieldError {
    fn from(error: DomainError) -> Self {
        Self {
            field: error.field().to_string(),
            reason: error.to_string(),
        }
    }
}

/// A use-case failure surfaced without transport detail.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AppError {
    /// The request failed validation.
    #[error("the request is invalid")]
    Rejected(Vec<FieldError>),
    /// Consent was absent or withdrawn.
    #[error("analytics consent was not granted")]
    ConsentNotGranted,
    /// The consent service could not answer.
    #[error("the consent service is unavailable: {0}")]
    ConsentUnavailable(String),
    /// The pseudonymisation salt could not be read.
    #[error("the pseudonymisation salt is unavailable: {0}")]
    SaltUnavailable(String),
    /// The record does not exist.
    #[error("not found")]
    NotFound,
    /// The caller may not perform the operation.
    #[error("forbidden")]
    Forbidden,
    /// A storage failure.
    #[error("storage failure: {0}")]
    Repository(#[from] crate::ports::RepositoryError),
}
