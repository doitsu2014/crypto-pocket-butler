//! BR1.3: the shared secret-shape detector.
//!
//! Every free-text value that can reach the audit trail is scanned by this one
//! helper, so no emitter can store a credential under a field the others check.
//! A refusal names the field path and never the offending value.
//!
//! The shapes are the ones in the security design: EVM/BSC addresses, Solana
//! base58 public keys, JSON Web Tokens, the project secret-prefix list, twelve-
//! or twenty-four-word seed phrases, and connection strings.

use crate::error::DomainError;

/// The kind of secret a value looks like.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretKind {
    /// `0x` followed by forty hexadecimal characters (EVM and BNB Smart Chain).
    EvmAddress,
    /// A base58 string that decodes to a 32-byte Solana public key.
    SolanaAddress,
    /// A three-segment base64url JSON Web Token.
    Jwt,
    /// A value carrying one of the project's secret prefixes.
    ApiKey,
    /// Twelve or twenty-four space-separated words from the seed word list.
    SeedPhrase,
    /// A database or service connection string.
    ConnectionString,
}

impl SecretKind {
    /// A stable label used in logs and redaction markers.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EvmAddress => "full-address",
            Self::SolanaAddress => "solana-address",
            Self::Jwt => "token",
            Self::ApiKey => "api-key",
            Self::SeedPhrase => "seed-phrase",
            Self::ConnectionString => "connection-string",
        }
    }
}

/// The project secret-prefix list. Values are public prefixes, never credentials.
const SECRET_PREFIXES: &[&str] = &[
    "cpb_",
    "sk_live_",
    "sk_test_",
    "pk_live_",
    "pk_test_",
    "rk_live_",
    "rk_test_",
    "whsec_",
    "ghp_",
    "gho_",
    "github_pat_",
    "xoxb-",
    "xoxp-",
    "AKIA",
    "ASIA",
    "vault://",
    "pgsodium:",
];

/// A representative slice of the BIP-39 English list. The production build
/// replaces it with the full list; the shape of the check is unchanged.
const SEED_WORDS: &[&str] = &[
    "abandon", "ability", "able", "about", "above", "absent", "absorb", "abstract", "absurd",
    "abuse", "access", "accident", "account", "accuse", "achieve", "acid", "acoustic", "acquire",
    "across", "act", "action", "actor", "actress", "actual", "adapt", "add", "addict", "address",
    "adjust", "admit", "adult", "advance", "advice", "aerobic", "affair", "afford", "afraid",
    "again", "age", "agent", "agree", "ahead", "aim", "air", "airport", "aisle", "alarm", "album",
    "alcohol", "alert", "alien", "all", "alley", "allow", "almost", "alone", "alpha", "already",
    "also", "alter", "always", "amateur", "amazing", "among", "amount", "amused", "analyst",
    "anchor", "ancient", "anger", "angle", "angry", "animal", "ankle", "announce", "annual",
    "another", "answer", "antenna", "antique", "anxiety", "any", "apart", "apology", "appear",
    "apple", "approve", "april", "arch", "arctic", "area", "arena", "argue", "arm", "armed",
    "armor", "army", "around", "arrange", "arrest", "arrive", "arrow", "art", "artefact", "artist",
    "artwork", "ask", "aspect", "assault", "asset", "assist", "assume", "asthma", "athlete",
    "atom", "attack", "attend", "attitude", "attract", "auction", "audit", "august", "aunt",
    "author", "auto", "autumn", "average", "avocado", "avoid", "awake", "aware", "away", "awesome",
    "awful", "awkward", "axis",
];

const BASE58_ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/// Detects the first secret shape in `value`, if any.
#[must_use]
pub fn detect_secret(value: &str) -> Option<SecretKind> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    if looks_like_connection_string(trimmed) {
        return Some(SecretKind::ConnectionString);
    }

    if is_seed_phrase(trimmed) {
        return Some(SecretKind::SeedPhrase);
    }

    if contains_evm_address(trimmed) {
        return Some(SecretKind::EvmAddress);
    }

    for token in trimmed.split_whitespace() {
        let token = token.trim_matches(|character: char| {
            matches!(
                character,
                '"' | '\''
                    | '`'
                    | ','
                    | ';'
                    | ':'
                    | ')'
                    | '('
                    | '['
                    | ']'
                    | '{'
                    | '}'
                    | '<'
                    | '>'
                    | '='
            )
        });
        if token.is_empty() {
            continue;
        }
        if is_jwt(token) {
            return Some(SecretKind::Jwt);
        }
        if has_secret_prefix(token) {
            return Some(SecretKind::ApiKey);
        }
        if is_solana_key(token) {
            return Some(SecretKind::SolanaAddress);
        }
    }

    None
}

/// BR1.3: refuses a value that carries a secret shape.
///
/// # Errors
/// Returns [`DomainError::SecretShaped`] naming the field path, never the value.
pub fn ensure_secret_free(field: &'static str, value: &str) -> Result<(), DomainError> {
    match detect_secret(value) {
        Some(_) => Err(DomainError::SecretShaped { field }),
        None => Ok(()),
    }
}

/// Replaces a secret-shaped value with a marker instead of refusing it.
///
/// Used for a run's `failureReason`: the run still has to record that it failed,
/// so the reason is redacted rather than dropped.
#[must_use]
pub fn redact(value: &str) -> (String, Option<SecretKind>) {
    match detect_secret(value) {
        Some(kind) => (format!("[redacted:{}]", kind.as_str()), Some(kind)),
        None => (value.to_string(), None),
    }
}

fn contains_evm_address(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut index = 0;
    while let Some(offset) = value[index..].find("0x") {
        let start = index + offset;
        let after_prefix = start + 2;
        let end = after_prefix + 40;
        if end <= bytes.len() && bytes[after_prefix..end].iter().all(u8::is_ascii_hexdigit) {
            match bytes.get(end) {
                Some(next) if next.is_ascii_hexdigit() => {}
                _ => return true,
            }
        }
        index = start + 2;
    }
    false
}

fn is_jwt(token: &str) -> bool {
    let segments: Vec<&str> = token.split('.').collect();
    if segments.len() != 3 {
        return false;
    }
    if !segments.iter().all(|segment| {
        !segment.is_empty()
            && segment
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    }) {
        return false;
    }
    // A JWT header is the base64url of a JSON object and therefore starts "eyJ".
    segments[0].starts_with("eyJ") && segments[1].len() >= 8
}

fn has_secret_prefix(token: &str) -> bool {
    SECRET_PREFIXES
        .iter()
        .any(|prefix| token.starts_with(prefix) && token.len() >= prefix.len() + 8)
}

fn is_solana_key(token: &str) -> bool {
    if token.len() < 32 || token.len() > 44 {
        return false;
    }
    if !token.bytes().all(|byte| BASE58_ALPHABET.contains(&byte)) {
        return false;
    }
    matches!(decode_base58(token), Some(bytes) if bytes.len() == 32)
}

fn is_seed_phrase(value: &str) -> bool {
    let words: Vec<&str> = value.split_whitespace().collect();
    if words.len() != 12 && words.len() != 24 {
        return false;
    }
    words
        .iter()
        .all(|word| word.bytes().all(|byte| byte.is_ascii_lowercase()) && SEED_WORDS.contains(word))
}

fn looks_like_connection_string(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    if lower.contains("password=") || lower.contains("passwd=") {
        return true;
    }
    for token in value.split_whitespace() {
        let token = token.trim_matches(|character: char| {
            matches!(
                character,
                '"' | '\'' | '`' | ',' | ';' | ')' | '(' | '[' | ']'
            )
        });
        if let Some(scheme_end) = token.find("://") {
            let scheme = &token[..scheme_end];
            if !scheme.is_empty()
                && scheme
                    .bytes()
                    .next()
                    .is_some_and(|byte| byte.is_ascii_alphabetic())
                && scheme
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
            {
                return true;
            }
        }
        if let Some(at) = token.find('@') {
            let credentials = &token[..at];
            if let Some(colon) = credentials.find(':') {
                let user = &credentials[..colon];
                let secret = &credentials[colon + 1..];
                if !user.is_empty() && secret.len() >= 8 && !secret.contains('/') {
                    return true;
                }
            }
        }
    }
    false
}

fn decode_base58(value: &str) -> Option<Vec<u8>> {
    let mut bytes: Vec<u8> = Vec::with_capacity(value.len());
    let mut digits: Vec<u8> = vec![0];
    for character in value.bytes() {
        let mut carry = BASE58_ALPHABET
            .iter()
            .position(|candidate| *candidate == character)? as u32;
        for digit in digits.iter_mut() {
            let value = u32::from(*digit) * 58 + carry;
            *digit = (value & 0xff) as u8;
            carry = value >> 8;
        }
        while carry > 0 {
            digits.push((carry & 0xff) as u8);
            carry >>= 8;
        }
    }
    for character in value.bytes() {
        if character == b'1' {
            bytes.push(0);
        } else {
            break;
        }
    }
    while digits.last() == Some(&0) {
        digits.pop();
    }
    bytes.extend(digits.iter().rev());
    Some(bytes)
}
