//! Correlation ids travel from the gateway to the owning service and back (BR4.3).

use crate::error::GatewayError;

/// A validated correlation id.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CorrelationId(String);

impl CorrelationId {
    /// Parses and validates a caller-supplied correlation id.
    ///
    /// # Errors
    /// Returns [`GatewayError::BadRequest`] when the value is empty, longer than
    /// 128 characters, or contains a character outside `[A-Za-z0-9._:-]`.
    pub fn parse(raw: &str) -> Result<Self, GatewayError> {
        if raw.is_empty() {
            return Err(GatewayError::BadRequest(
                "correlation id must not be empty".to_string(),
            ));
        }
        if raw.len() > 128 {
            return Err(GatewayError::BadRequest(
                "correlation id must be at most 128 characters".to_string(),
            ));
        }
        if !raw.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | ':')
        }) {
            return Err(GatewayError::BadRequest(
                "correlation id contains an invalid character".to_string(),
            ));
        }
        Ok(Self(raw.to_string()))
    }

    /// The id as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
