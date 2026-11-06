//! Shared shapes for a refused request.

use audit_domain::DomainError;

/// One offending field path, never its value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldError {
    /// The field path, e.g. `target` or `context`.
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
