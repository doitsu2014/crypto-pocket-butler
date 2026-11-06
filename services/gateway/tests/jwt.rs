//! JWKS validation tests: signature, issuer, audience, expiry, cache staleness,
//! and fail-closed behaviour (BR4.1).

mod common;

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use common::jwt::{sign, test_key, TestClaims};
use gateway_application::ports::{AuthError, TokenValidator};
use gateway_domain::Role;
use gateway_infrastructure::auth::{JwkKey, JwksSource, JwksValidator};

struct FakeSource {
    keys: Mutex<Vec<JwkKey>>,
    fail: AtomicBool,
    calls: AtomicUsize,
}

impl FakeSource {
    fn with_key() -> Self {
        let key = test_key();
        Self {
            keys: Mutex::new(vec![JwkKey::new(
                key.kid.clone(),
                key.n.clone(),
                key.e.clone(),
            )]),
            fail: AtomicBool::new(false),
            calls: AtomicUsize::new(0),
        }
    }

    fn set_failing(&self, fail: bool) {
        self.fail.store(fail, Ordering::SeqCst);
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl JwksSource for FakeSource {
    async fn fetch(&self) -> Result<Vec<JwkKey>, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fail.load(Ordering::SeqCst) {
            return Err("jwks endpoint down".to_string());
        }
        Ok(self.keys.lock().unwrap().clone())
    }
}

fn validator(
    source: Arc<FakeSource>,
    refresh_after: Duration,
    max_staleness: Duration,
) -> JwksValidator {
    let key = test_key();
    JwksValidator::with_timings(
        source,
        key.issuer.clone(),
        key.audience.clone(),
        refresh_after,
        max_staleness,
        30,
    )
}

#[tokio::test]
async fn a_valid_token_yields_the_caller_and_role() {
    let source = Arc::new(FakeSource::with_key());
    let validator = validator(source, Duration::from_secs(300), Duration::from_secs(600));
    let key = test_key();

    let principal = validator
        .validate(&sign(key, &TestClaims::valid(key, "user")))
        .await
        .expect("valid token");

    assert_eq!(principal.subject, "user-1");
    assert_eq!(principal.role, Role::User);
}

#[tokio::test]
async fn an_admin_role_is_read_from_app_metadata() {
    let source = Arc::new(FakeSource::with_key());
    let validator = validator(source, Duration::from_secs(300), Duration::from_secs(600));
    let key = test_key();
    let mut claims = TestClaims::valid(key, "admin");
    claims.sub = "admin-1".to_string();

    let principal = validator
        .validate(&sign(key, &claims))
        .await
        .expect("valid admin token");
    assert_eq!(principal.role, Role::Admin);
    assert_eq!(principal.subject, "admin-1");
}

#[tokio::test]
async fn an_expired_token_is_invalid() {
    let source = Arc::new(FakeSource::with_key());
    let validator = validator(source, Duration::from_secs(300), Duration::from_secs(600));
    let key = test_key();
    let mut claims = TestClaims::valid(key, "user");
    claims.exp = 1_000_000_000; // 2001

    let result = validator.validate(&sign(key, &claims)).await;
    assert!(matches!(result, Err(AuthError::Invalid(_))));
}

#[tokio::test]
async fn a_wrong_issuer_or_audience_is_invalid() {
    let source = Arc::new(FakeSource::with_key());
    let validator = validator(source, Duration::from_secs(300), Duration::from_secs(600));
    let key = test_key();

    let mut wrong_issuer = TestClaims::valid(key, "user");
    wrong_issuer.iss = "https://attacker.example/auth/v1".to_string();
    assert!(matches!(
        validator.validate(&sign(key, &wrong_issuer)).await,
        Err(AuthError::Invalid(_))
    ));

    let mut wrong_audience = TestClaims::valid(key, "user");
    wrong_audience.aud = "service-role".to_string();
    assert!(matches!(
        validator.validate(&sign(key, &wrong_audience)).await,
        Err(AuthError::Invalid(_))
    ));
}

#[tokio::test]
async fn an_unknown_key_id_or_malformed_token_is_invalid() {
    let source = Arc::new(FakeSource::with_key());
    let validator = validator(source, Duration::from_secs(300), Duration::from_secs(600));
    let key = test_key();

    let mut claims = TestClaims::valid(key, "user");
    claims.sub = "someone".to_string();
    // Sign with a header that names a key id absent from the JWKS.
    let token = {
        use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("missing-key".to_string());
        encode(
            &header,
            &claims,
            &EncodingKey::from_rsa_pem(key.private_pem.as_bytes()).unwrap(),
        )
        .unwrap()
    };
    assert!(matches!(
        validator.validate(&token).await,
        Err(AuthError::Invalid(_))
    ));
    assert!(matches!(
        validator.validate("not-a-jwt").await,
        Err(AuthError::Invalid(_))
    ));
}

#[tokio::test]
async fn jwks_failure_with_no_cache_fails_closed() {
    let source = Arc::new(FakeSource::with_key());
    source.set_failing(true);
    let validator = validator(source, Duration::from_secs(300), Duration::from_secs(600));
    let key = test_key();

    let result = validator
        .validate(&sign(key, &TestClaims::valid(key, "user")))
        .await;
    assert!(matches!(result, Err(AuthError::Unavailable(_))));
}

#[tokio::test]
async fn cached_keys_serve_inside_the_staleness_window() {
    let source = Arc::new(FakeSource::with_key());
    let validator = validator(source.clone(), Duration::ZERO, Duration::from_secs(60));
    let key = test_key();
    let token = sign(key, &TestClaims::valid(key, "user"));

    validator
        .validate(&token)
        .await
        .expect("first fetch succeeds");
    source.set_failing(true);
    validator
        .validate(&token)
        .await
        .expect("cached key serves inside the window");
    assert_eq!(source.calls(), 2, "the second call attempted a refresh");
}

#[tokio::test]
async fn keys_beyond_the_staleness_window_fail_closed() {
    let source = Arc::new(FakeSource::with_key());
    let validator = validator(source.clone(), Duration::ZERO, Duration::ZERO);
    let key = test_key();
    let token = sign(key, &TestClaims::valid(key, "user"));

    validator
        .validate(&token)
        .await
        .expect("first fetch succeeds");
    source.set_failing(true);
    let result = validator.validate(&token).await;
    assert!(matches!(result, Err(AuthError::Unavailable(_))));
}

#[tokio::test]
async fn a_blank_token_is_missing_not_invalid() {
    let source = Arc::new(FakeSource::with_key());
    let validator = validator(source, Duration::from_secs(300), Duration::from_secs(600));
    assert!(matches!(
        validator.validate("   ").await,
        Err(AuthError::Missing)
    ));
}
