//! Non-secret configuration. The database credential is read from a secret file
//! (`DATABASE_URL_FILE`) or, for local development only, from the environment
//! with a warning. It never comes from an env file committed to the repository.

use std::path::Path;

use anyhow::{Context, Result};

/// Runtime configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// HTTP port.
    pub port: u16,
    /// Postgres connection string (secret).
    pub database_url: String,
    /// Supabase JWKS URL (public).
    pub jwks_url: String,
    /// Expected JWT issuer.
    pub jwt_issuer: String,
    /// Expected JWT audience.
    pub jwt_audience: String,
}

impl Config {
    /// Reads configuration from the environment.
    ///
    /// # Errors
    /// Returns an error naming the missing variable when a required value is absent.
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            port: env_or("ASSET_REGISTRY_PORT", "8081")
                .parse()
                .context("ASSET_REGISTRY_PORT must be a valid port")?,
            database_url: secret("DATABASE_URL")?,
            jwks_url: required("JWT_JWKS_URL")?,
            jwt_issuer: required("JWT_ISSUER")?,
            jwt_audience: required("JWT_AUDIENCE")?,
        })
    }
}

fn required(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("{name} must be set"))
}

fn env_or(name: &str, fallback: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| fallback.to_string())
}

fn secret(name: &str) -> Result<String> {
    let file_variable = format!("{name}_FILE");
    if let Ok(path) = std::env::var(&file_variable) {
        let value = std::fs::read_to_string(Path::new(&path))
            .with_context(|| format!("{file_variable} points at an unreadable file"))?;
        let value = value.trim().to_string();
        if value.is_empty() {
            anyhow::bail!("{file_variable} points at an empty secret file");
        }
        return Ok(value);
    }
    if let Ok(value) = std::env::var(name) {
        tracing::warn!("reading {name} from the environment; use {file_variable} in deployment");
        return Ok(value);
    }
    anyhow::bail!(
        "{file_variable} must be set (the secret lives in Supabase Vault or a Docker secret)"
    )
}
