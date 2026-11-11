//! Gateway configuration. The gateway holds no secret: it validates JWTs against
//! public JWKS and forwards requests to the internal registry.

use anyhow::{Context, Result};

/// Runtime configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// HTTP port.
    pub port: u16,
    /// Asset Registry base URL on the internal network.
    pub registry_url: String,
    /// Identity base URL on the internal network.
    pub identity_url: String,
    /// Supabase JWKS URL (public).
    pub jwks_url: String,
    /// Expected JWT issuer.
    pub jwt_issuer: String,
    /// Expected JWT audience.
    pub jwt_audience: String,
    /// Upstream request timeout in milliseconds.
    pub upstream_timeout_ms: u64,
    /// Authenticated requests per minute.
    pub rate_limit_per_minute: u32,
    /// Anonymous requests per minute.
    pub anonymous_rate_limit_per_minute: u32,
}

impl Config {
    /// Reads configuration from the environment.
    ///
    /// # Errors
    /// Returns an error naming the missing or malformed variable.
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            port: env_or("GATEWAY_PORT", "8080")
                .parse()
                .context("GATEWAY_PORT must be a valid port")?,
            registry_url: env_or("ASSET_REGISTRY_URL", "http://asset-registry:8081"),
            identity_url: env_or("IDENTITY_URL", "http://identity-api:8083"),
            jwks_url: required("JWT_JWKS_URL")?,
            jwt_issuer: required("JWT_ISSUER")?,
            jwt_audience: required("JWT_AUDIENCE")?,
            upstream_timeout_ms: env_or("UPSTREAM_TIMEOUT_MS", "2000")
                .parse()
                .context("UPSTREAM_TIMEOUT_MS must be a number")?,
            rate_limit_per_minute: env_or("GATEWAY_RATE_LIMIT_PER_MINUTE", "120")
                .parse()
                .context("GATEWAY_RATE_LIMIT_PER_MINUTE must be a number")?,
            anonymous_rate_limit_per_minute: env_or(
                "GATEWAY_ANONYMOUS_RATE_LIMIT_PER_MINUTE",
                "30",
            )
            .parse()
            .context("GATEWAY_ANONYMOUS_RATE_LIMIT_PER_MINUTE must be a number")?,
        })
    }
}

fn required(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("{name} must be set"))
}

fn env_or(name: &str, fallback: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| fallback.to_string())
}
