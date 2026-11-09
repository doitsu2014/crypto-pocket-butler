//! Shared shapes for a refused request.

use ledger_domain::DomainError;

use crate::ports::{ExternalError, RepositoryError};

/// One offending field path, never its value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldError {
    /// The field path, e.g. `quantity`.
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
    /// The record does not exist.
    #[error("not found")]
    NotFound,
    /// The caller may not perform the operation, or does not own the resource.
    #[error("forbidden")]
    Forbidden,
    /// A uniqueness or state rule was violated.
    #[error("conflict: {0}")]
    Conflict(String),
    /// The Asset Registry could not resolve the asset.
    #[error("the asset could not be resolved: {0}")]
    AssetUnavailable(String),
    /// A storage failure.
    #[error("storage failure: {0}")]
    Repository(#[from] RepositoryError),
    /// An external dependency failed.
    #[error("dependency failure: {0}")]
    External(#[from] ExternalError),
}

impl From<DomainError> for AppError {
    fn from(error: DomainError) -> Self {
        Self::Rejected(vec![FieldError::from(error)])
    }
}

impl AppError {
    /// A single-field refusal.
    #[must_use]
    pub fn field(field: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Rejected(vec![FieldError::new(field, reason)])
    }
}
