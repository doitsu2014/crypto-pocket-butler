//! SHA-256 helpers for the service-token registry. Only a hash is stored.

use sha2::{Digest, Sha256};

/// The lowercase hex SHA-256 of `value`.
#[must_use]
pub fn sha256_hex(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    hex::encode(hasher.finalize())
}
