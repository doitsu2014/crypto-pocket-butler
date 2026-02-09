use jsonwebtoken::{decode, decode_header, DecodingKey, Validation};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use thiserror::Error;
use tokio::sync::RwLock;

use super::KeycloakConfig;

#[derive(Debug, Error)]
pub enum JwtError {
    #[error("Missing authorization header")]
    MissingAuthHeader,
    
    #[error("Invalid authorization header format")]
    InvalidAuthHeader,
    
    #[error("Failed to decode JWT header: {0}")]
    DecodeHeader(String),
    
    #[error("Failed to validate JWT: {0}")]
    ValidationError(String),
    
    #[error("JWKS fetch failed: {0}")]
    JwksFetchError(String),
    
    #[error("Key not found in JWKS")]
    KeyNotFound,
    
    #[error("Invalid issuer")]
    InvalidIssuer,
    
    #[error("Invalid audience")]
    InvalidAudience,
}

/// Standard JWT claims with Keycloak-specific fields
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Subject (user ID) - stable identifier from Keycloak
    pub sub: String,
    /// Issuer
    pub iss: String,
    /// Audience
    pub aud: serde_json::Value,
    /// Expiration time (unix timestamp)
    pub exp: u64,
    /// Issued at (unix timestamp)
    pub iat: u64,
    /// Preferred username
    #[serde(default)]
    pub preferred_username: Option<String>,
    /// Email
    #[serde(default)]
    pub email: Option<String>,
    /// Email verified
    #[serde(default)]
    pub email_verified: Option<bool>,
}

impl Claims {
    /// Get the user_id (same as sub)
    pub fn user_id(&self) -> &str {
        &self.sub
    }
}

#[derive(Debug, Deserialize)]
struct JwksResponse {
    keys: Vec<Jwk>,
}

#[derive(Debug, Deserialize)]
struct Jwk {
    kid: String,
    kty: String,
    n: String,
    e: String,
}

struct JwksCache {
    keys: HashMap<String, DecodingKey>,
    last_fetch: SystemTime,
    cache_duration: Duration,
}

/// JWT validator that fetches and caches JWKS from Keycloak
pub struct JwtValidator {
    config: KeycloakConfig,
    http_client: Client,
    jwks_cache: Arc<RwLock<Option<JwksCache>>>,
}

impl JwtValidator {
    /// Create a new JWT validator
    pub fn new(config: KeycloakConfig) -> Self {
        Self {
            config,
            http_client: Client::new(),
            jwks_cache: Arc::new(RwLock::new(None)),
        }
    }

    /// Validate a JWT token and return the claims
    pub async fn validate(&self, token: &str) -> Result<Claims, JwtError> {
        // Decode header to get the key ID (kid)
        let header = decode_header(token)
            .map_err(|e| JwtError::DecodeHeader(e.to_string()))?;

        let kid = header
            .kid
            .ok_or_else(|| JwtError::DecodeHeader("Missing kid in token header".to_string()))?;

        // Get the decoding key from JWKS
        let decoding_key = self.get_decoding_key(&kid).await?;

        // Set up validation
        let mut validation = Validation::new(
            header.alg.try_into()
                .map_err(|_| JwtError::ValidationError("Unsupported algorithm".to_string()))?
        );
        validation.set_issuer(&[&self.config.issuer]);
        validation.set_audience(&[&self.config.audience]);

        // Validate and decode the token
        let token_data = decode::<Claims>(token, &decoding_key, &validation)
            .map_err(|e| JwtError::ValidationError(e.to_string()))?;

        Ok(token_data.claims)
    }

    /// Extract token from Authorization header
    pub fn extract_token(auth_header: &str) -> Result<&str, JwtError> {
        if !auth_header.starts_with("Bearer ") {
            return Err(JwtError::InvalidAuthHeader);
        }
        Ok(&auth_header[7..])
    }

    async fn get_decoding_key(&self, kid: &str) -> Result<DecodingKey, JwtError> {
        // Check if we have a cached key
        {
            let cache = self.jwks_cache.read().await;
            if let Some(jwks_cache) = cache.as_ref() {
                // Check if cache is still valid
                let now = SystemTime::now();
                if now.duration_since(jwks_cache.last_fetch).unwrap_or(Duration::MAX)
                    < jwks_cache.cache_duration
                {
                    if let Some(key) = jwks_cache.keys.get(kid) {
                        return Ok(key.clone());
                    }
                }
            }
        }

        // Cache miss or expired, fetch fresh JWKS
        self.refresh_jwks().await?;

        // Try again with the fresh cache
        let cache = self.jwks_cache.read().await;
        cache
            .as_ref()
            .and_then(|c| c.keys.get(kid).cloned())
            .ok_or(JwtError::KeyNotFound)
    }

    async fn refresh_jwks(&self) -> Result<(), JwtError> {
        let response = self
            .http_client
            .get(&self.config.jwks_url)
            .send()
            .await
            .map_err(|e| JwtError::JwksFetchError(e.to_string()))?;

        let jwks: JwksResponse = response
            .json()
            .await
            .map_err(|e| JwtError::JwksFetchError(e.to_string()))?;

        let mut keys = HashMap::new();
        for jwk in jwks.keys {
            if jwk.kty == "RSA" {
                let decoding_key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e)
                    .map_err(|e| JwtError::JwksFetchError(e.to_string()))?;
                keys.insert(jwk.kid, decoding_key);
            }
        }

        let mut cache = self.jwks_cache.write().await;
        *cache = Some(JwksCache {
            keys,
            last_fetch: SystemTime::now(),
            cache_duration: Duration::from_secs(3600), // Cache for 1 hour
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_token() {
        let auth_header = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        let token = JwtValidator::extract_token(auth_header).unwrap();
        assert_eq!(token, "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9");
    }

    #[test]
    fn test_extract_token_invalid() {
        let auth_header = "Basic dXNlcjpwYXNz";
        let result = JwtValidator::extract_token(auth_header);
        assert!(result.is_err());
    }

    #[test]
    fn test_claims_user_id() {
        let claims = Claims {
            sub: "user-123".to_string(),
            iss: "https://keycloak.example.com/realms/test".to_string(),
            aud: serde_json::Value::String("account".to_string()),
            exp: 1234567890,
            iat: 1234567800,
            preferred_username: Some("testuser".to_string()),
            email: Some("test@example.com".to_string()),
            email_verified: Some(true),
        };
        
        assert_eq!(claims.user_id(), "user-123");
    }
}
