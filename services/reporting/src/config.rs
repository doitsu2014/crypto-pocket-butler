//! Non-secret configuration. Every credential is read from a secret file
//! (`*_FILE`) — a Docker secret mounted from Supabase Vault — and never from an
//! env file committed to the repository.

use anyhow::{Context, Result};

/// The API service's configuration.
#[derive(Debug, Clone)]
pub struct ApiConfig {
    /// HTTP port.
    pub port: u16,
    /// Postgres connection string for the `reporting` schema (secret).
    pub database_url: String,
    /// The outbound service token Reporting presents to C3, C7–C10 (secret file).
    pub outbound_token: String,
    /// Supabase JWKS URL (public).
    pub jwks_url: String,
    /// Expected JWT issuer.
    pub jwt_issuer: String,
    /// Expected JWT audience.
    pub jwt_audience: String,
    /// Identity base URL (C3).
    pub identity_url: String,
    /// Portfolio base URL (C8).
    pub portfolio_url: String,
    /// Ledger base URL (C7).
    pub ledger_url: String,
    /// Notification base URL (C9).
    pub notification_url: String,
    /// Audit base URL (C10).
    pub audit_url: String,
    /// The service name whose token the API presents.
    pub token_service: String,
    /// Maximum pooled connections for the API.
    pub pool_connections: u32,
}

impl ApiConfig {
    /// Reads configuration from the environment.
    ///
    /// # Errors
    /// Returns an error naming the missing variable when a required value is
    /// absent.
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            port: env_or("REPORTING_PORT", "8103")
                .parse()
                .context("REPORTING_PORT must be a valid port")?,
            database_url: secret("DATABASE_URL")?,
            outbound_token: secret("OUTBOUND_TOKEN")?,
            jwks_url: required("JWT_JWKS_URL")?,
            jwt_issuer: required("JWT_ISSUER")?,
            jwt_audience: required("JWT_AUDIENCE")?,
            identity_url: env_or("IDENTITY_URL", "http://identity-api:8089"),
            portfolio_url: env_or("PORTFOLIO_URL", "http://portfolio-api:8101"),
            ledger_url: env_or("LEDGER_URL", "http://ledger-api:8097"),
            notification_url: env_or("NOTIFICATION_URL", "http://notification-api:8099"),
            audit_url: env_or("AUDIT_URL", "http://audit-api:8084"),
            token_service: env_or("REPORTING_TOKEN_SERVICE", "reporting"),
            pool_connections: env_or("REPORTING_API_POOL", "10")
                .parse()
                .context("REPORTING_API_POOL must be a number")?,
        })
    }
}

/// The generation worker's configuration.
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// HTTP port for the run endpoint and probes.
    pub port: u16,
    /// Postgres connection string for the `reporting` schema (secret).
    pub database_url: String,
    /// The outbound service token Reporting presents (secret file).
    pub outbound_token: String,
    /// Identity base URL (C3).
    pub identity_url: String,
    /// Portfolio base URL (C8).
    pub portfolio_url: String,
    /// Ledger base URL (C7).
    pub ledger_url: String,
    /// Notification base URL (C9).
    pub notification_url: String,
    /// Audit base URL (C10).
    pub audit_url: String,
    /// The service name whose token the worker presents.
    pub token_service: String,
    /// How long the supervisor sleeps between native runs.
    pub run_interval_seconds: u64,
    /// Maximum pooled connections for the worker.
    pub pool_connections: u32,
}

impl WorkerConfig {
    /// Reads configuration from the environment.
    ///
    /// # Errors
    /// Returns an error naming the missing variable when a required value is
    /// absent.
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            port: env_or("REPORTING_WORKER_PORT", "8104")
                .parse()
                .context("REPORTING_WORKER_PORT must be a valid port")?,
            database_url: secret("DATABASE_URL")?,
            outbound_token: secret("OUTBOUND_TOKEN")?,
            identity_url: env_or("IDENTITY_URL", "http://identity-api:8089"),
            portfolio_url: env_or("PORTFOLIO_URL", "http://portfolio-api:8101"),
            ledger_url: env_or("LEDGER_URL", "http://ledger-api:8097"),
            notification_url: env_or("NOTIFICATION_URL", "http://notification-api:8099"),
            audit_url: env_or("AUDIT_URL", "http://audit-api:8084"),
            token_service: env_or("REPORTING_TOKEN_SERVICE", "reporting"),
            run_interval_seconds: env_or("REPORTING_RUN_INTERVAL_SECONDS", "3600")
                .parse()
                .context("REPORTING_RUN_INTERVAL_SECONDS must be a number")?,
            pool_connections: env_or("REPORTING_WORKER_POOL", "8")
                .parse()
                .context("REPORTING_WORKER_POOL must be a number")?,
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
        let value = std::fs::read_to_string(std::path::Path::new(&path))
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
