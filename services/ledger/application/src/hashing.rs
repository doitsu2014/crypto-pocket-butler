//! Hashing helpers for audit context. Only a reason hash is sent to C10; the
//! free-text reason stays on the correction row (BR4.1).

use sha2::{Digest, Sha256};

/// The lowercase hex SHA-256 of a value.
#[must_use]
pub fn sha256_hex(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    hex::encode(hasher.finalize())
}
