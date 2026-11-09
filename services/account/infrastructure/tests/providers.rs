//! Provider adapter tests that need no network: the HMAC-SHA256 signatures and
//! the service-token hash.

use account_infrastructure::exchange::{hmac_sha256_base64, hmac_sha256_hex};
use account_infrastructure::http_retry::is_transient;
use account_infrastructure::service_token::hash_token;
use base64::Engine;

#[test]
fn hmac_sha256_matches_the_published_test_vector() {
    let digest = hmac_sha256_hex(b"key", b"The quick brown fox jumps over the lazy dog");
    assert_eq!(
        digest,
        "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8"
    );
}

#[test]
fn the_base64_form_encodes_the_same_digest() {
    let message = b"The quick brown fox jumps over the lazy dog";
    let hex = hmac_sha256_hex(b"key", message);
    let base64 = hmac_sha256_base64(b"key", message);
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(base64)
        .expect("valid base64");
    assert_eq!(decoded, hex::decode(hex).unwrap());
}

#[test]
fn a_long_key_is_hashed_before_use() {
    let key = vec![b'a'; 200];
    let first = hmac_sha256_hex(&key, b"message");
    let second = hmac_sha256_hex(&key, b"message");
    assert_eq!(first, second);
    assert_eq!(first.len(), 64);
}

#[test]
fn token_hashes_are_stable_and_distinct() {
    let first = hash_token("service-token");
    assert_eq!(first, hash_token("service-token"));
    assert_eq!(first.len(), 64);
    assert!(first.chars().all(|character| character.is_ascii_hexdigit()));
    assert_ne!(first, hash_token("another-token"));
    assert_eq!(first, hash_token("  service-token  "));
}

#[test]
fn transient_statuses_cover_rate_limits_and_server_failures() {
    for status in [408, 425, 429, 500, 502, 503, 504] {
        assert!(is_transient(status), "{status} is transient");
    }
    for status in [200, 201, 400, 401, 403, 404, 422] {
        assert!(!is_transient(status), "{status} is not transient");
    }
}
