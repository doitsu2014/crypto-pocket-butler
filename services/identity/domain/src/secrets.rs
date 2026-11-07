//! A conservative credential-shaped field scan for the export archive (BR5.2).
//!
//! The scan looks at JSON field names, not prose values, so a sentence that uses
//! the word "token" does not fail an export.

/// The field names that mark a credential-shaped value.
const FORBIDDEN_FIELDS: &[&str] = &[
    "password",
    "passwd",
    "secret",
    "apikey",
    "api_key",
    "token",
    "accesstoken",
    "access_token",
    "refreshtoken",
    "refresh_token",
    "privatekey",
    "private_key",
    "seedphrase",
    "seed_phrase",
    "mnemonic",
];

/// Returns the offending field name when the JSON carries a credential-shaped
/// field, or `None` when it is clean.
#[must_use]
pub fn find_secret_field(json: &str) -> Option<String> {
    let lowered = json.to_ascii_lowercase();
    for field in FORBIDDEN_FIELDS {
        let needle = format!("\"{field}\"");
        let mut offset = 0;
        while let Some(found) = lowered[offset..].find(&needle) {
            let start = offset + found;
            let end = start + needle.len();
            if lowered[end..].trim_start().starts_with(':') {
                return Some(json[start + 1..end - 1].to_string());
            }
            offset = end;
        }
    }
    None
}
