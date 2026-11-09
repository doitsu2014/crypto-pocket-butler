//! Conservative detection of private keys and seed phrases (BR1.3, BR3.3).
//!
//! The check is deliberately conservative: a value shaped like key material is
//! refused even when it might have been intended as an ordinary string.

use crate::error::DomainError;

/// Returns `true` when a value is shaped like a private key or seed phrase.
#[must_use]
pub fn looks_like_secret(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    let lowered = trimmed.to_ascii_lowercase();
    let hex_body = lowered.strip_prefix("0x").unwrap_or(&lowered);
    if hex_body.len() == 64
        && hex_body
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return true;
    }
    if matches!(trimmed.len(), 51 | 52)
        && matches!(trimmed.chars().next(), Some('5' | 'K' | 'L'))
        && trimmed
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
    {
        return true;
    }
    let words: Vec<&str> = lowered.split_whitespace().collect();
    if matches!(words.len(), 12 | 15 | 18 | 21 | 24)
        && words
            .iter()
            .all(|word| word.chars().all(|character| character.is_ascii_lowercase()))
    {
        return true;
    }
    false
}

/// Refuses a field shaped like key material.
///
/// # Errors
/// Returns [`DomainError::SecretLike`] when the value looks like a private key or
/// seed phrase. The value is never echoed.
pub fn ensure_not_secret(field: &'static str, value: &str) -> Result<(), DomainError> {
    if looks_like_secret(value) {
        Err(DomainError::SecretLike { field })
    } else {
        Ok(())
    }
}
