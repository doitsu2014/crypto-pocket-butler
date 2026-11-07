//! Non-secret configuration. Every credential is read from a secret file
//! (`*_FILE`) — a Docker secret mounted from Supabase Vault — and never from an
//! env file committed to the repository.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

const DEFAULT_IDENTITY_URL: &str = "http://identity-api:8083";
const DEFAULT_AUDIT_URL: &str = "http://audit-api:8084";

/// The API service's configuration.
#[derive(Debug, Clone)]
pub struct ApiConfig {
    /// HTTP port.
    pub port: u16,
    /// Postgres connection string for `analytics_svc` (secret).
    pub database_url: String,
    /// The outbound service token for C3 and C10 (secret).
    pub outbound_token: String,
    /// Identity base URL (C3).
    pub identity_url: String,
    /// Audit base URL (C10).
    pub audit_url: String,
    /// Supabase JWKS URL (public).
    pub jwks_url: String,
    /// Expected JWT issuer.
    pub jwt_issuer: String,
    /// Expected JWT audience.
    pub jwt_audience: String,
    /// The release smoke token (secret), when mounted.
    pub smoke_token_path: Option<PathBuf>,
    /// Maximum pooled connections.
    pub pool_connections: u32,
}

impl ApiConfig {
    /// Reads configuration from the environment.
    ///
    /// # Errors
    /// Returns an error naming the missing variable when a required value is absent.
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            port: env_or("ANALYTICS_PORT", "8095")
                .parse()
                .context("ANALYTICS_PORT must be a valid port")?,
            database_url: secret("DATABASE_URL")?,
            outbound_token: secret("OUTBOUND_TOKEN")?,
            identity_url: env_or("IDENTITY_URL", DEFAULT_IDENTITY_URL),
            audit_url: env_or("AUDIT_URL", DEFAULT_AUDIT_URL),
            jwks_url: required("JWT_JWKS_URL")?,
            jwt_issuer: required("JWT_ISSUER")?,
            jwt_audience: required("JWT_AUDIENCE")?,
            smoke_token_path: optional_path("SMOKE_TOKEN_FILE"),
            pool_connections: env_or("ANALYTICS_API_POOL", "10")
                .parse()
                .context("ANALYTICS_API_POOL must be a number")?,
        })
    }
}

/// The worker's configuration.
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// HTTP port for probes and C11 run paths.
    pub port: u16,
    /// Postgres connection string for `analytics_svc` (secret).
    pub database_url: String,
    /// The outbound service token (secret); the worker makes no outbound call.
    pub outbound_token: String,
    /// Identity base URL (C3), unused by the worker but wired for symmetry.
    pub identity_url: String,
    /// Audit base URL (C10), unused by the worker but wired for symmetry.
    pub audit_url: String,
    /// The release smoke token (secret), when mounted.
    pub smoke_token_path: Option<PathBuf>,
    /// Maximum pooled connections.
    pub pool_connections: u32,
}

impl WorkerConfig {
    /// Reads configuration from the environment.
    ///
    /// # Errors
    /// Returns an error naming the missing variable when a required value is absent.
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            port: env_or("ANALYTICS_WORKER_PORT", "8096")
                .parse()
                .context("ANALYTICS_WORKER_PORT must be a valid port")?,
            database_url: secret("DATABASE_URL")?,
            outbound_token: secret("OUTBOUND_TOKEN")?,
            identity_url: env_or("IDENTITY_URL", DEFAULT_IDENTITY_URL),
            audit_url: env_or("AUDIT_URL", DEFAULT_AUDIT_URL),
            smoke_token_path: optional_path("SMOKE_TOKEN_FILE"),
            pool_connections: env_or("ANALYTICS_WORKER_POOL", "6")
                .parse()
                .context("ANALYTICS_WORKER_POOL must be a number")?,
        })
    }
}

/// The migration job's configuration.
#[derive(Debug, Clone)]
pub struct MigrateConfig {
    /// Postgres connection string for `analytics_migrator` (secret).
    pub database_url: String,
}

impl MigrateConfig {
    /// Reads configuration from the environment.
    ///
    /// # Errors
    /// Returns an error naming the missing variable when a required value is absent.
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: secret("DATABASE_URL")?,
        })
    }
}

fn required(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("{name} must be set"))
}

fn env_or(name: &str, fallback: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| fallback.to_string())
}

fn optional_path(name: &str) -> Option<PathBuf> {
    std::env::var(name).ok().map(PathBuf::from)
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
