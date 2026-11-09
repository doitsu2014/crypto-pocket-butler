//! Supabase JWT validation against a cached JWKS.
//!
//! Signature, issuer, audience, and expiry are all checked. The subject is the
//! user id Portfolio stores.

use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use portfolio_application::common::{AssuranceLevel, VerifiedClaims};
use portfolio_application::ports::{AuthError, TokenValidator};
use portfolio_domain::UserId;
use serde::Deserialize;
use tokio::sync::RwLock;

const DEFAULT_REFRESH_AFTER: Duration = Duration::from_secs(3600);
const DEFAULT_MAX_STALENESS: Duration = Duration::from_secs(600);
const DEFAULT_LEEWAY_SECONDS: u64 = 30;

/// One RSA signing key from a JWKS document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JwkKey {
    /// Key identifier matched against the JWT header.
    pub kid: String,
    /// Base64url-encoded RSA modulus.
    pub n: String,
    /// Base64url-encoded RSA exponent.
    pub e: String,
}

impl JwkKey {
    /// Creates a JWK key reference.
    #[must_use]
    pub fn new(kid: impl Into<String>, n: impl Into<String>, e: impl Into<String>) -> Self {
        Self {
            kid: kid.into(),
            n: n.into(),
            e: e.into(),
        }
    }

    fn decoding_key(&self) -> Result<DecodingKey, AuthError> {
        DecodingKey::from_rsa_components(&self.n, &self.e)
            .map_err(|error| AuthError::Invalid(format!("invalid JWKS key: {error}")))
    }
}

/// Fetches the current signing keys.
#[async_trait]
pub trait JwksSource: Send + Sync {
    /// Fetches the JWKS document's RSA keys.
    async fn fetch(&self) -> Result<Vec<JwkKey>, String>;
}

/// Fetches JWKS over HTTPS.
pub struct HttpJwksSource {
    client: reqwest::Client,
    url: String,
}

impl HttpJwksSource {
    /// Creates an HTTPS JWKS source.
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(1))
            .timeout(Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            url: url.into(),
        }
    }

    async fn fetch_once(&self) -> Result<Vec<JwkKey>, String> {
        let document = self
            .client
            .get(&self.url)
            .send()
            .await
            .map_err(|error| error.to_string())?
            .error_for_status()
            .map_err(|error| error.to_string())?
            .json::<JwksDocument>()
            .await
            .map_err(|error| error.to_string())?;
        let keys = document
            .keys
            .into_iter()
            .enumerate()
            .filter(|(_, key)| key.kty == "RSA")
            .filter_map(|(index, key)| {
                let n = key.n?;
                let e = key.e?;
                Some(JwkKey::new(
                    key.kid.unwrap_or_else(|| format!("key-{index}")),
                    n,
                    e,
                ))
            })
            .collect::<Vec<_>>();
        if keys.is_empty() {
            return Err("JWKS document contained no RSA signing keys".to_string());
        }
        Ok(keys)
    }
}

#[async_trait]
impl JwksSource for HttpJwksSource {
    async fn fetch(&self) -> Result<Vec<JwkKey>, String> {
        let mut last_error = String::new();
        for attempt in 0..3u32 {
            match self.fetch_once().await {
                Ok(keys) => return Ok(keys),
                Err(error) => {
                    last_error = error;
                    if attempt < 2 {
                        crate::http_retry::pause_before_retry(attempt).await;
                    }
                }
            }
        }
        Err(last_error)
    }
}

#[derive(Debug, Deserialize)]
struct JwksDocument {
    keys: Vec<RawJwk>,
}

#[derive(Debug, Deserialize)]
struct RawJwk {
    #[serde(default)]
    kid: Option<String>,
    kty: String,
    #[serde(default)]
    n: Option<String>,
    #[serde(default)]
    e: Option<String>,
}

#[derive(Debug, Clone)]
struct CachedKeys {
    keys: Vec<JwkKey>,
    fetched_at: Instant,
}

/// Validates Supabase JWTs against a cached JWKS.
pub struct JwksValidator {
    source: Arc<dyn JwksSource>,
    issuer: String,
    audience: String,
    cache: RwLock<Option<CachedKeys>>,
    refresh_after: Duration,
    max_staleness: Duration,
    leeway_seconds: u64,
}

impl JwksValidator {
    /// Creates a validator with the production cache windows.
    #[must_use]
    pub fn new(
        source: Arc<dyn JwksSource>,
        issuer: impl Into<String>,
        audience: impl Into<String>,
    ) -> Self {
        Self::with_timings(
            source,
            issuer,
            audience,
            DEFAULT_REFRESH_AFTER,
            DEFAULT_MAX_STALENESS,
            DEFAULT_LEEWAY_SECONDS,
        )
    }

    /// Creates a validator with explicit cache windows, used by tests.
    #[must_use]
    pub fn with_timings(
        source: Arc<dyn JwksSource>,
        issuer: impl Into<String>,
        audience: impl Into<String>,
        refresh_after: Duration,
        max_staleness: Duration,
        leeway_seconds: u64,
    ) -> Self {
        Self {
            source,
            issuer: issuer.into(),
            audience: audience.into(),
            cache: RwLock::new(None),
            refresh_after,
            max_staleness,
            leeway_seconds,
        }
    }

    async fn keys(&self, force_refresh: bool) -> Result<Vec<JwkKey>, AuthError> {
        let cached = self.cache.read().await.clone();
        if !force_refresh {
            if let Some(entry) = &cached {
                if entry.fetched_at.elapsed() < self.refresh_after {
                    return Ok(entry.keys.clone());
                }
            }
        }
        match self.source.fetch().await {
            Ok(keys) => {
                let entry = CachedKeys {
                    keys: keys.clone(),
                    fetched_at: Instant::now(),
                };
                *self.cache.write().await = Some(entry);
                Ok(keys)
            }
            Err(error) => {
                if let Some(entry) = cached {
                    if entry.fetched_at.elapsed() <= self.max_staleness {
                        tracing::warn!(
                            error = %error,
                            "JWKS refresh failed; serving cached keys inside the staleness window"
                        );
                        return Ok(entry.keys);
                    }
                }
                tracing::error!(error = %error, "JWKS unavailable and no fresh cache; failing closed");
                Err(AuthError::Unavailable(error))
            }
        }
    }

    fn validate_with_keys(
        &self,
        token: &str,
        keys: &[JwkKey],
    ) -> Result<VerifiedClaims, AuthError> {
        let header = decode_header(token).map_err(|error| AuthError::Invalid(error.to_string()))?;
        let key = match &header.kid {
            Some(kid) => keys.iter().find(|candidate| &candidate.kid == kid),
            None => keys.first(),
        }
        .ok_or_else(|| AuthError::Invalid("no matching signing key".to_string()))?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.set_audience(&[self.audience.as_str()]);
        validation.leeway = self.leeway_seconds;
        validation.validate_exp = true;

        let data = decode::<RawClaims>(token, &key.decoding_key()?, &validation)
            .map_err(|error| AuthError::Invalid(error.to_string()))?;
        let claims = data.claims;
        if claims.sub.trim().is_empty() {
            return Err(AuthError::Invalid("token subject is empty".to_string()));
        }
        let user_id = UserId::parse(&claims.sub)
            .map_err(|_| AuthError::Invalid("the token subject is not a user id".to_string()))?;
        Ok(VerifiedClaims {
            user_id,
            assurance: AssuranceLevel::parse(claims.aal.as_deref()),
        })
    }
}

#[derive(Debug, Deserialize)]
struct RawClaims {
    sub: String,
    #[serde(default)]
    aal: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    exp: Option<DateTime<Utc>>,
}

#[async_trait]
impl TokenValidator for JwksValidator {
    async fn validate(&self, token: &str) -> Result<VerifiedClaims, AuthError> {
        let token = token.trim();
        if token.is_empty() {
            return Err(AuthError::Missing);
        }
        let keys = self.keys(false).await?;
        match self.validate_with_keys(token, &keys) {
            Ok(claims) => Ok(claims),
            Err(AuthError::Invalid(detail)) if detail.contains("no matching signing key") => {
                let keys = self.keys(true).await?;
                self.validate_with_keys(token, &keys)
            }
            Err(error) => Err(error),
        }
    }
}
