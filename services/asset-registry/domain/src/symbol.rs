//! One documented symbol-normalization rule: trim, reject blank/whitespace/control
//! characters, uppercase, and cap the length. Every lookup uses this rule.

use crate::error::DomainError;

const MAX_SYMBOL_LEN: usize = 32;

/// Normalizes an asset or observation symbol.
///
/// # Errors
/// Returns [`DomainError::EmptySymbol`] for a blank value and
/// [`DomainError::InvalidSymbol`] when the value is too long or contains
/// whitespace or control characters.
pub fn normalize_symbol(raw: &str) -> Result<String, DomainError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(DomainError::EmptySymbol);
    }
    if trimmed.len() > MAX_SYMBOL_LEN
        || trimmed.chars().any(char::is_whitespace)
        || trimmed.chars().any(char::is_control)
    {
        return Err(DomainError::InvalidSymbol(trimmed.to_string()));
    }
    Ok(trimmed.to_uppercase())
}
