//! Deterministic digests used by idempotency comparison and export proof.

use sha2::{Digest, Sha256};

/// The SHA-256 of `input`, as lowercase hex.
#[must_use]
pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

/// Incremental SHA-256 for a streamed export body.
#[derive(Debug, Default)]
pub struct StreamDigest {
    hasher: Sha256,
}

impl StreamDigest {
    /// Adds a chunk to the digest.
    pub fn update(&mut self, chunk: &[u8]) {
        self.hasher.update(chunk);
    }

    /// Finishes the digest as lowercase hex.
    #[must_use]
    pub fn finish(self) -> String {
        hex::encode(self.hasher.finalize())
    }
}
