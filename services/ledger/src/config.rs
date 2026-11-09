//! Non-secret configuration. Every credential is read from a secret file
//! (`*_FILE`) — a Docker secret mounted from Supabase Vault — and never from an
//! env file committed to the repository.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

const DEFAULT_ACCOUNT_URL: &str = "http://account-api:8091";
const DEFAULT_ASSET_REGISTRY_URL: &str = "http://asset-registry:8081";
const DEFAULT_AUDIT_URL: &str = "http://audit-api:8084";
const DEFAULT_MARKET_DATA_URL: &str = "http://market-data-api:8093";

/// The API service's configuration.
#[derive(Debug, Clone)]
pub struct ApiConfig {
    /// HTTP port.
    pub port: u16,
    /// Postgres connection string for `ledger_svc` (secret).
    pub database_url: String,
    /// Postgres connection string for `ledger_anonymiser` (secret); opened only
    /// inside the C7 anonymise handler.
    pub anonymiser_database_url: String,
    /// The outbound service token for C4, C5, and C10 (secret).
    pub outbound_token: String,
    /// Account base URL (C5).
    pub account_url: String,
    /// Asset Registry base URL (C4).
    pub asset_registry_url: String,
    /// Market Data base URL (C6).
    pub market_data_url: String,
    /// The user's base currency for P&L.
    pub base_currency: String,
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
            port: env_or("LEDGER_PORT", "8097")
                .parse()
                .context("LEDGER_PORT must be a valid port")?,
            database_url: secret("DATABASE_URL")?,
            anonymiser_database_url: secret("ANONYMISER_DATABASE_URL")?,
            outbound_token: secret("OUTBOUND_TOKEN")?,
            account_url: env_or("ACCOUNT_URL", DEFAULT_ACCOUNT_URL),
            asset_registry_url: env_or("ASSET_REGISTRY_URL", DEFAULT_ASSET_REGISTRY_URL),
            market_data_url: env_or("MARKET_DATA_URL", DEFAULT_MARKET_DATA_URL),
            base_currency: env_or("LEDGER_BASE_CURRENCY", "USD"),
            audit_url: env_or("AUDIT_URL", DEFAULT_AUDIT_URL),
            jwks_url: required("JWT_JWKS_URL")?,
            jwt_issuer: required("JWT_ISSUER")?,
            jwt_audience: required("JWT_AUDIENCE")?,
            smoke_token_path: optional_path("SMOKE_TOKEN_FILE"),
            pool_connections: env_or("LEDGER_API_POOL", "12")
                .parse()
                .context("LEDGER_API_POOL must be a number")?,
        })
    }
}

/// The worker's configuration.
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// HTTP port for probes and C11 run paths.
    pub port: u16,
    /// Postgres connection string for `ledger_svc` (secret).
    pub database_url: String,
    /// Postgres connection string for `ledger_anonymiser` (secret).
    pub anonymiser_database_url: String,
    /// The outbound service token (secret).
    pub outbound_token: String,
    /// Account base URL (C5).
    pub account_url: String,
    /// Asset Registry base URL (C4).
    pub asset_registry_url: String,
    /// Market Data base URL (C6).
    pub market_data_url: String,
    /// The user's base currency for P&L.
    pub base_currency: String,
    /// Audit base URL (C10).
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
            port: env_or("LEDGER_WORKER_PORT", "8098")
                .parse()
                .context("LEDGER_WORKER_PORT must be a valid port")?,
            database_url: secret("DATABASE_URL")?,
            anonymiser_database_url: secret("ANONYMISER_DATABASE_URL")?,
            outbound_token: secret("OUTBOUND_TOKEN")?,
            account_url: env_or("ACCOUNT_URL", DEFAULT_ACCOUNT_URL),
            asset_registry_url: env_or("ASSET_REGISTRY_URL", DEFAULT_ASSET_REGISTRY_URL),
            market_data_url: env_or("MARKET_DATA_URL", DEFAULT_MARKET_DATA_URL),
            base_currency: env_or("LEDGER_BASE_CURRENCY", "USD"),
            audit_url: env_or("AUDIT_URL", DEFAULT_AUDIT_URL),
            smoke_token_path: optional_path("SMOKE_TOKEN_FILE"),
            pool_connections: env_or("LEDGER_WORKER_POOL", "8")
                .parse()
                .context("LEDGER_WORKER_POOL must be a number")?,
        })
    }
}

/// The migration job's configuration.
#[derive(Debug, Clone)]
pub struct MigrateConfig {
    /// Postgres connection string for `ledger_migrator` (secret).
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
