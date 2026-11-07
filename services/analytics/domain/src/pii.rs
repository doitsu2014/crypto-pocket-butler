//! BR2.2: the shared personal-data and credential detector.
//!
//! Every ingested property value passes through this one helper. A match drops
//! the property; a match on `eventName` or `occurredAt` refuses the event. The
//! detector never returns or logs the offending value.

/// The kind of personal data or credential a value looks like.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PiiKind {
    /// An address-shaped value containing `@` and a dot in the domain.
    Email,
    /// `0x` followed by exactly forty hexadecimal characters.
    EvmAddress,
    /// A 32-to-44 character string from the base58 alphabet.
    Base58Key,
    /// An IPv4 or IPv6 literal.
    IpLiteral,
    /// A long token-shaped value, including the `sk-`, `pk-`, `Bearer`, and
    /// `eyJ` families.
    SecretShaped,
}

impl PiiKind {
    /// A stable label for logs; it names the shape, never the value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::EvmAddress => "full-address",
            Self::Base58Key => "base58-key",
            Self::IpLiteral => "ip-address",
            Self::SecretShaped => "secret",
        }
    }
}

const BASE58_ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/// Returns the first PII or credential shape in `value`, if any.
#[must_use]
pub fn detect_pii(value: &str) -> Option<PiiKind> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if is_email(trimmed) {
        return Some(PiiKind::Email);
    }
    if contains_evm_address(trimmed) {
        return Some(PiiKind::EvmAddress);
    }
    if is_ip_literal(trimmed) {
        return Some(PiiKind::IpLiteral);
    }
    if is_secret_shaped(trimmed) {
        return Some(PiiKind::SecretShaped);
    }
    if trimmed
        .split_whitespace()
        .any(|token| is_base58_key(clean_token(token)))
    {
        return Some(PiiKind::Base58Key);
    }
    None
}

/// Whether a property value must be dropped instead of stored.
#[must_use]
pub fn is_pii(value: &str) -> bool {
    detect_pii(value).is_some()
}

fn clean_token(token: &str) -> &str {
    token.trim_matches(|character: char| {
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
    })
}

fn is_email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.trim().is_empty()
        && !local.contains(char::is_whitespace)
        && domain.contains('.')
        && !domain.contains(char::is_whitespace)
        && !domain.starts_with('.')
        && !domain.ends_with('.')
}

fn contains_evm_address(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut index = 0;
    while let Some(offset) = value[index..].find("0x") {
        let start = index + offset;
        let after_prefix = start + 2;
        let end = after_prefix + 40;
        if end <= bytes.len() && bytes[after_prefix..end].iter().all(u8::is_ascii_hexdigit) {
            let next_is_hex = bytes.get(end).is_some_and(u8::is_ascii_hexdigit);
            if !next_is_hex {
                return true;
            }
        }
        index = start + 2;
    }
    false
}

fn is_base58_key(token: &str) -> bool {
    if token.len() < 32 || token.len() > 44 {
        return false;
    }
    token.bytes().all(|byte| BASE58_ALPHABET.contains(&byte))
}

fn is_ip_literal(value: &str) -> bool {
    is_ipv4(value) || is_ipv6(value)
}

fn is_ipv4(value: &str) -> bool {
    let parts: Vec<&str> = value.split('.').collect();
    parts.len() == 4
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.len() <= 3
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && part.parse::<u16>().is_ok_and(|octet| octet <= 255)
        })
}

fn is_ipv6(value: &str) -> bool {
    if !value.contains(':') || value.contains(char::is_whitespace) {
        return false;
    }
    let compressed = value.contains("::");
    let groups: Vec<&str> = value.split(':').filter(|group| !group.is_empty()).collect();
    if groups.is_empty()
        || !groups
            .iter()
            .all(|group| group.len() <= 4 && group.bytes().all(|byte| byte.is_ascii_hexdigit()))
    {
        return false;
    }
    if compressed {
        groups.len() <= 7
    } else {
        groups.len() == 8
    }
}

fn is_secret_shaped(value: &str) -> bool {
    if value.contains(char::is_whitespace) {
        let lower = value.to_ascii_lowercase();
        return lower.starts_with("bearer ") || lower.contains("bearer ");
    }
    let lower = value.to_ascii_lowercase();
    if lower.starts_with("sk-") || lower.starts_with("pk-") || lower.starts_with("eyj") {
        return true;
    }
    value.len() >= 20
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        && value.bytes().any(|byte| byte.is_ascii_digit())
}
