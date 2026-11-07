//! Detection of key material in free text. A provider body or exception message
//! is never stored, echoed, or audited verbatim.

use crate::constants::MAX_ERROR_MESSAGE_CHARS;

/// The word count at which a phrase is treated as a seed phrase.
const SEED_PHRASE_WORDS: usize = 12;
/// The hex length at which a bare token is treated as a private key.
const HEX_KEY_CHARS: usize = 64;

/// Whether a string is shaped like key material or a seed phrase.
#[must_use]
pub fn looks_like_secret(raw: &str) -> bool {
    let value = raw.trim();
    let without_prefix = value.strip_prefix("0x").unwrap_or(value);
    if without_prefix.len() >= HEX_KEY_CHARS
        && without_prefix
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return true;
    }
    if value.len() >= 51
        && value.starts_with('5')
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
    {
        return true;
    }
    let words = value.split_whitespace().count();
    words >= SEED_PHRASE_WORDS
        && value.split_whitespace().all(|word| {
            word.chars()
                .all(|character| character.is_ascii_alphabetic())
        })
}

/// Returns a bounded, non-secret summary. A secret-shaped value is replaced by
/// the caller's fallback.
#[must_use]
pub fn sanitise_message(raw: &str, fallback: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() || looks_like_secret(trimmed) {
        return fallback.to_string();
    }
    trimmed.chars().take(MAX_ERROR_MESSAGE_CHARS).collect()
}
