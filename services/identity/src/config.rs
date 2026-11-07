//! Non-secret configuration. Every credential is read from a secret file
//! (`*_FILE`) — a Docker secret mounted from Supabase Vault — and never from an
//! env file committed to the repository.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// The API service's configuration.
#[derive(Debug, Clone)]
pub struct ApiConfig {
    /// HTTP port.
    pub port: u16,
    /// Postgres connection string for the `identity` schema (secret).
    pub database_url: String,
    /// The service-token registry file (secret).
    pub service_tokens_path: PathBuf,
    /// Supabase JWKS URL (public).
    pub jwks_url: String,
    /// Expected JWT issuer.
    pub jwt_issuer: String,
    /// Expected JWT audience.
    pub jwt_audience: String,
    /// Supabase Auth admin base URL.
    pub supabase_auth_url: String,
    /// Supabase service-role key (secret); used only for Auth admin and Storage.
    pub service_role_key: String,
    /// Supabase Storage base URL.
    pub storage_url: String,
    /// The private export bucket.
    pub export_bucket: String,
    /// Audit service base URL.
    pub audit_url: String,
    /// Account service base URL (privacy contributions).
    pub account_url: String,
    /// Ledger service base URL (privacy contributions).
    pub ledger_url: String,
    /// Portfolio service base URL (privacy contributions).
    pub portfolio_url: String,
    /// Identity worker wake URL.
    pub worker_url: String,
    /// The service name whose token the API presents to other services.
    pub token_service: String,
    /// The HMAC key that signs the short-lived pre-sign-up gate token (secret).
    pub gate_token_key: String,
    /// Maximum pooled connections for the API.
    pub pool_connections: u32,
}

impl ApiConfig {
    /// Reads configuration from the environment.
    ///
    /// # Errors
    /// Returns an error naming the missing variable when a required value is absent.
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            port: env_or("IDENTITY_PORT", "8083")
                .parse()
                .context("IDENTITY_PORT must be a valid port")?,
            database_url: secret("DATABASE_URL")?,
            service_tokens_path: PathBuf::from(required("SERVICE_TOKENS_FILE")?),
            jwks_url: required("JWT_JWKS_URL")?,
            jwt_issuer: required("JWT_ISSUER")?,
            jwt_audience: required("JWT_AUDIENCE")?,
            supabase_auth_url: required("SUPABASE_AUTH_URL")?,
            service_role_key: secret("SUPABASE_SERVICE_ROLE_KEY")?,
            storage_url: required("SUPABASE_STORAGE_URL")?,
            export_bucket: env_or("IDENTITY_EXPORT_BUCKET", "identity-exports"),
            audit_url: env_or("AUDIT_URL", "http://audit-api:8084"),
            account_url: env_or("ACCOUNT_URL", "http://account:8091"),
            ledger_url: env_or("LEDGER_URL", "http://ledger:8092"),
            portfolio_url: env_or("PORTFOLIO_URL", "http://portfolio:8093"),
            worker_url: env_or("IDENTITY_WORKER_URL", "http://identity-worker:8085"),
            token_service: env_or("IDENTITY_TOKEN_SERVICE", "identity"),
            gate_token_key: secret("GATE_TOKEN_KEY")?,
            pool_connections: env_or("IDENTITY_API_POOL", "12")
                .parse()
                .context("IDENTITY_API_POOL must be a number")?,
        })
    }
}

/// The privacy worker's configuration.
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// HTTP port for the internal wake endpoint and probes.
    pub port: u16,
    /// Postgres connection string for the `identity` schema (secret).
    pub database_url: String,
    /// The service-token registry file (secret).
    pub service_tokens_path: PathBuf,
    /// Supabase Auth admin base URL.
    pub supabase_auth_url: String,
    /// Supabase service-role key (secret).
    pub service_role_key: String,
    /// Supabase Storage base URL.
    pub storage_url: String,
    /// The private export bucket.
    pub export_bucket: String,
    /// Audit service base URL.
    pub audit_url: String,
    /// Account service base URL (privacy contributions).
    pub account_url: String,
    /// Ledger service base URL (privacy contributions).
    pub ledger_url: String,
    /// Portfolio service base URL (privacy contributions).
    pub portfolio_url: String,
    /// The service name whose token the worker presents to other services.
    pub token_service: String,
    /// How long the supervisor sleeps between sweeps.
    pub sweep_interval_seconds: u64,
    /// Maximum pooled connections for the worker.
    pub pool_connections: u32,
}

impl WorkerConfig {
    /// Reads configuration from the environment.
    ///
    /// # Errors
    /// Returns an error naming the missing variable when a required value is absent.
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            port: env_or("IDENTITY_WORKER_PORT", "8085")
                .parse()
                .context("IDENTITY_WORKER_PORT must be a valid port")?,
            database_url: secret("DATABASE_URL")?,
            service_tokens_path: PathBuf::from(required("SERVICE_TOKENS_FILE")?),
            supabase_auth_url: required("SUPABASE_AUTH_URL")?,
            service_role_key: secret("SUPABASE_SERVICE_ROLE_KEY")?,
            storage_url: required("SUPABASE_STORAGE_URL")?,
            export_bucket: env_or("IDENTITY_EXPORT_BUCKET", "identity-exports"),
            audit_url: env_or("AUDIT_URL", "http://audit-api:8084"),
            account_url: env_or("ACCOUNT_URL", "http://account:8091"),
            ledger_url: env_or("LEDGER_URL", "http://ledger:8092"),
            portfolio_url: env_or("PORTFOLIO_URL", "http://portfolio:8093"),
            token_service: env_or("IDENTITY_TOKEN_SERVICE", "identity"),
            sweep_interval_seconds: env_or("IDENTITY_SWEEP_INTERVAL_SECONDS", "60")
                .parse()
                .context("IDENTITY_SWEEP_INTERVAL_SECONDS must be a number")?,
            pool_connections: env_or("IDENTITY_WORKER_POOL", "8")
                .parse()
                .context("IDENTITY_WORKER_POOL must be a number")?,
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
