//! Supabase JWT validation against a cached JWKS.
//!
//! - Keys are cached in memory and refreshed after five minutes, or immediately
//!   when an unknown `kid` arrives.
//! - If a refresh fails, cached keys serve for at most ten minutes; after that
//!   the validator fails closed with `AuthError::Unavailable` (HTTP 503).
//! - Signature, issuer, audience, and expiry are all checked. The role, the
//!   verified-email fact, the assurance level, and the exchange-credential fact
//!   are read from the validated token (BR1.2, BR1.5, BR2.1).

use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use identity_application::ports::{AuthError, TokenValidator};
use identity_domain::claims::{AssuranceLevel, Role, VerifiedClaims};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
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

impl HttpJwksSource {
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
        // At most three attempts, 1 s connect / 5 s read (reliability design).
        let mut last_error = String::new();
        for attempt in 0..3u32 {
            match self.fetch_once().await {
                Ok(keys) => return Ok(keys),
                Err(error) => {
                    last_error = error;
                    if attempt < 2 {
                        tracing::warn!(%last_error, "JWKS fetch failed; retrying");
                        crate::http_retry::pause_before_retry(attempt).await;
                    }
                }
            }
        }
        Err(last_error)
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
}

#[derive(Debug, Deserialize)]
struct RawClaims {
    sub: String,
    #[serde(default)]
    app_metadata: Option<AppMetadata>,
    #[serde(default)]
    email_confirmed: Option<bool>,
    #[serde(default)]
    email_confirmed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    aal: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AppMetadata {
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    has_exchange_credentials: Option<bool>,
}

impl JwksValidator {
    fn validate_with_keys(
        &self,
        token: &str,
        keys: &[JwkKey],
    ) -> Result<(VerifiedClaims, String), AuthError> {
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
        let metadata = claims.app_metadata.unwrap_or(AppMetadata {
            role: None,
            has_exchange_credentials: None,
        });
        let kid = key.kid.clone();
        Ok((
            VerifiedClaims {
                subject: claims.sub,
                role: Role::from_claim(metadata.role.as_deref()),
                email_confirmed: claims
                    .email_confirmed
                    .unwrap_or(claims.email_confirmed_at.is_some()),
                email: claims
                    .email
                    .map(|email| email.trim().to_string())
                    .filter(|email| !email.is_empty()),
                display_name: None,
                assurance: AssuranceLevel::parse(claims.aal.as_deref()),
                has_exchange_credentials: metadata.has_exchange_credentials.unwrap_or(false),
                session_id: claims.session_id,
                user_agent: None,
                location: None,
                country_code: None,
                client_ip: None,
            },
            kid,
        ))
    }
}

#[async_trait]
impl TokenValidator for JwksValidator {
    async fn validate(&self, token: &str) -> Result<VerifiedClaims, AuthError> {
        let token = token.trim();
        if token.is_empty() {
            return Err(AuthError::Missing);
        }
        let keys = self.keys(false).await?;
        let kid = decode_header(token)
            .ok()
            .and_then(|header| header.kid)
            .unwrap_or_default();
        match self.validate_with_keys(token, &keys) {
            Ok((claims, _)) => Ok(claims),
            Err(AuthError::Invalid(detail)) if detail.contains("no matching signing key") => {
                // An unknown kid triggers exactly one refetch (performance design).
                let keys = self.keys(true).await?;
                let _ = kid;
                self.validate_with_keys(token, &keys)
                    .map(|(claims, _)| claims)
            }
            Err(error) => Err(error),
        }
    }
}
