//! BR5.1: the content guard.
//!
//! Every allow-listed payload value passes through this one helper. A match
//! drops the value and records only its field name; the value itself is never
//! returned, logged, or stored.
//!
//! The guard detects an email address, a full `0x` EVM address, a 32-to-44
//! character base58 key, an IPv4 or IPv6 literal, a token-shaped value, and the
//! `sk-`, `pk-`, `Bearer`, and `eyJ` families.
//!
//! A canonical UUID is deliberately not treated as token-shaped: the payload
//! allow-lists carry `portfolioId`, `assetId`, `accountId`, and `briefId`, which
//! are exactly those identifiers, and dropping them would make every alert
//! contentless.

const BASE58_ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/// Whether a payload value must be dropped.
#[must_use]
pub fn is_sensitive(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    if is_canonical_uuid(trimmed) {
        return false;
    }
    if has_secret_prefix(trimmed) {
        return true;
    }
    if is_email(trimmed)
        || is_evm_address(trimmed)
        || is_ip_literal(trimmed)
        || is_token_shaped(trimmed)
    {
        return true;
    }
    trimmed.split_whitespace().any(|token| {
        let token = clean_token(token);
        !token.is_empty()
            && !is_canonical_uuid(token)
            && (is_email(token)
                || is_evm_address(token)
                || is_ip_literal(token)
                || is_base58_key(token))
    })
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

/// A canonical 8-4-4-4-12 hexadecimal UUID.
#[must_use]
pub fn is_canonical_uuid(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 36 {
        return false;
    }
    for (index, byte) in bytes.iter().enumerate() {
        if matches!(index, 8 | 13 | 18 | 23) {
            if *byte != b'-' {
                return false;
            }
        } else if !byte.is_ascii_hexdigit() {
            return false;
        }
    }
    true
}

fn has_secret_prefix(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.starts_with("sk-")
        || lower.starts_with("pk-")
        || lower.starts_with("bearer ")
        || lower.contains(" bearer ")
        || lower.starts_with("eyj")
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

fn is_evm_address(value: &str) -> bool {
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

fn is_token_shaped(value: &str) -> bool {
    value.len() >= 20
        && !value.contains(char::is_whitespace)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}
