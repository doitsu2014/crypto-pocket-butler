//! A disposable RSA signing key and JWT builder for validator tests.
//!
//! The key is generated per test process; nothing is committed and no real
//! Supabase key is ever used.

use std::sync::OnceLock;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use rand::thread_rng;
use rsa::pkcs8::{EncodePrivateKey, LineEnding};
use rsa::traits::PublicKeyParts;
use rsa::RsaPrivateKey;
use serde::Serialize;

pub struct TestKey {
    pub kid: String,
    pub private_pem: String,
    pub n: String,
    pub e: String,
    pub issuer: String,
    pub audience: String,
}

pub fn test_key() -> &'static TestKey {
    static KEY: OnceLock<TestKey> = OnceLock::new();
    KEY.get_or_init(|| {
        let private = RsaPrivateKey::new(&mut thread_rng(), 2048).expect("generate a test RSA key");
        let public = rsa::RsaPublicKey::from(&private);
        let private_pem = private
            .to_pkcs8_pem(LineEnding::LF)
            .expect("encode the test key")
            .to_string();
        TestKey {
            kid: "test-key".to_string(),
            private_pem,
            n: URL_SAFE_NO_PAD.encode(public.n().to_bytes_be()),
            e: URL_SAFE_NO_PAD.encode(public.e().to_bytes_be()),
            issuer: "https://test.supabase.co/auth/v1".to_string(),
            audience: "authenticated".to_string(),
        }
    })
}

#[derive(Debug, Serialize)]
pub struct TestClaims {
    pub sub: String,
    pub iss: String,
    pub aud: String,
    pub exp: usize,
    pub app_metadata: TestAppMetadata,
}

#[derive(Debug, Serialize)]
pub struct TestAppMetadata {
    pub role: String,
}

impl TestClaims {
    pub fn valid(key: &TestKey, role: &str) -> Self {
        Self {
            sub: "user-1".to_string(),
            iss: key.issuer.clone(),
            aud: key.audience.clone(),
            exp: u64::MAX as usize / 4,
            app_metadata: TestAppMetadata {
                role: role.to_string(),
            },
        }
    }
}

pub fn sign(key: &TestKey, claims: &TestClaims) -> String {
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(key.kid.clone());
    encode(
        &header,
        claims,
        &EncodingKey::from_rsa_pem(key.private_pem.as_bytes()).expect("encoding key"),
    )
    .expect("sign the test token")
}
