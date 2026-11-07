//! HMAC-SHA256 pseudonymisation and the deterministic flag bucket.
//!
//! A subject is hashed with the Vault salt for `userRef` (BR2.3). A flag bucket
//! is `SHA-256(subject) mod 100` from the raw subject, held only in memory and
//! never stored (BR4.2).

use sha2::{Digest, Sha256};
use uuid::Uuid;

/// The HMAC block size for SHA-256.
const BLOCK_SIZE: usize = 64;

/// Returns the lowercase hex SHA-256 digest of `value`.
#[must_use]
pub fn sha256_hex(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

/// Computes HMAC-SHA256 and returns the raw 32 bytes.
#[must_use]
pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut normalised = [0u8; BLOCK_SIZE];
    if key.len() > BLOCK_SIZE {
        normalised[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        normalised[..key.len()].copy_from_slice(key);
    }

    let mut inner_pad = [0x36u8; BLOCK_SIZE];
    let mut outer_pad = [0x5cu8; BLOCK_SIZE];
    for index in 0..BLOCK_SIZE {
        inner_pad[index] ^= normalised[index];
        outer_pad[index] ^= normalised[index];
    }

    let mut inner = Sha256::new();
    inner.update(inner_pad);
    inner.update(message);
    let inner_digest = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(outer_pad);
    outer.update(inner_digest);
    let digest = outer.finalize();

    let mut output = [0u8; 32];
    output.copy_from_slice(&digest);
    output
}

/// Returns the lowercase hex HMAC-SHA256 of `message` under `key`.
#[must_use]
pub fn hmac_sha256_hex(key: &[u8], message: &str) -> String {
    hex::encode(hmac_sha256(key, message.as_bytes()))
}

/// Derives a stable, pseudonymous UUID from a user-reference hash.
///
/// C10 stores `actorId` as a UUID and refuses the raw subject from Analytics, so
/// an audit record names the hash instead: the first 16 bytes of its SHA-256,
/// with the UUID version and variant bits set. The same `userRef` always yields
/// the same actor id for the life of that salt version.
#[must_use]
pub fn pseudonymous_actor_id(actor_ref: &str) -> String {
    let digest = Sha256::digest(actor_ref.as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes).to_string()
}

/// BR4.2: the deterministic per-user rollout bucket, 0 to 99.
#[must_use]
pub fn flag_bucket(subject: &str) -> u8 {
    let digest = Sha256::digest(subject.as_bytes());
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    (u64::from_be_bytes(bytes) % 100) as u8
}
