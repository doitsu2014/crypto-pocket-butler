//! Supabase JWT validation against a cached JWKS.
//!
//! The gateway forwards the user's JWT unchanged; Notification validates it
//! itself (signature, issuer, audience, expiry) and derives the subject and role
//! from `app_metadata.role`. Keys are cached in memory for five minutes and may
//! serve stale for at most ten before the validator fails closed.

use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use notification_application::ports::{AuthError, Principal, Role, TokenValidator};
use serde::Deserialize;
use tokio::sync::RwLock;

const DEFAULT_REFRESH_AFTER: Duration = Duration::from_secs(300);
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
    /// Creates a JWK reference.
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
            .timeout(Duration::from_secs(3))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            url: url.into(),
        }
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

#[async_trait]
impl JwksSource for HttpJwksSource {
    async fn fetch(&self) -> Result<Vec<JwkKey>, String> {
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
                Some(JwkKey::new(
                    key.kid.unwrap_or_else(|| format!("key-{index}")),
                    key.n?,
                    key.e?,
                ))
            })
            .collect::<Vec<_>>();
        if keys.is_empty() {
            return Err("JWKS document contained no RSA signing keys".to_string());
        }
        Ok(keys)
    }
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
        Self {
            source,
            issuer: issuer.into(),
            audience: audience.into(),
            cache: RwLock::new(None),
            refresh_after: DEFAULT_REFRESH_AFTER,
            max_staleness: DEFAULT_MAX_STALENESS,
            leeway_seconds: DEFAULT_LEEWAY_SECONDS,
        }
    }

    async fn keys(&self) -> Result<Vec<JwkKey>, AuthError> {
        let cached = self.cache.read().await.clone();
        if let Some(entry) = &cached {
            if entry.fetched_at.elapsed() < self.refresh_after {
                return Ok(entry.keys.clone());
            }
        }
        match self.source.fetch().await {
            Ok(keys) => {
                *self.cache.write().await = Some(CachedKeys {
                    keys: keys.clone(),
                    fetched_at: Instant::now(),
                });
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
                tracing::error!(
                    error = %error,
                    "JWKS unavailable and no fresh cache; failing closed"
                );
                Err(AuthError::Unavailable(error))
            }
        }
    }

    fn validate_with_keys(&self, token: &str, keys: &[JwkKey]) -> Result<Principal, AuthError> {
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
        let role = match claims
            .app_metadata
            .and_then(|metadata| metadata.role)
            .as_deref()
        {
            Some("admin") => Role::Admin,
            Some("support") => Role::Support,
            Some("user") | None => Role::User,
            Some(other) => {
                return Err(AuthError::Invalid(format!("unknown role '{other}'")));
            }
        };
        Ok(Principal {
            subject: claims.sub,
            role,
        })
    }
}

#[derive(Debug, Deserialize)]
struct RawClaims {
    sub: String,
    #[serde(default)]
    app_metadata: Option<AppMetadata>,
}

#[derive(Debug, Deserialize)]
struct AppMetadata {
    #[serde(default)]
    role: Option<String>,
}

#[async_trait]
impl TokenValidator for JwksValidator {
    async fn validate(&self, token: &str) -> Result<Principal, AuthError> {
        let token = token.trim();
        if token.is_empty() {
            return Err(AuthError::Missing);
        }
        let keys = self.keys().await?;
        self.validate_with_keys(token, &keys)
    }
}
