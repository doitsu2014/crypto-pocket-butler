//! Non-secret configuration. Every credential is read from a secret file
//! (`*_FILE`) — a Docker secret mounted from Supabase Vault — and never from an
//! env file committed to the repository.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

const DEFAULT_BINANCE_BASE: &str = "https://api.binance.com";
const DEFAULT_OKX_BASE: &str = "https://www.okx.com";

/// The API service's configuration.
#[derive(Debug, Clone)]
pub struct ApiConfig {
    /// HTTP port.
    pub port: u16,
    /// Postgres connection string for the `account` schema (secret).
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
    /// Supabase service-role key (secret); used only for Auth admin and Vault.
    pub service_role_key: String,
    /// Audit service base URL.
    pub audit_url: String,
    /// Notification service base URL.
    pub notification_url: String,
    /// Asset Registry base URL.
    pub asset_registry_url: String,
    /// Account worker base URL.
    pub worker_url: String,
    /// The service name whose token the API presents to other services.
    pub token_service: String,
    /// EVM JSON-RPC endpoint.
    pub evm_rpc_url: String,
    /// Solana JSON-RPC endpoint.
    pub solana_rpc_url: String,
    /// Binance REST base.
    pub binance_base_url: String,
    /// OKX REST base.
    pub okx_base_url: String,
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
            port: env_or("ACCOUNT_PORT", "8091")
                .parse()
                .context("ACCOUNT_PORT must be a valid port")?,
            database_url: secret("DATABASE_URL")?,
            service_tokens_path: PathBuf::from(required("SERVICE_TOKENS_FILE")?),
            jwks_url: required("JWT_JWKS_URL")?,
            jwt_issuer: required("JWT_ISSUER")?,
            jwt_audience: required("JWT_AUDIENCE")?,
            supabase_auth_url: required("SUPABASE_AUTH_URL")?,
            service_role_key: secret("SUPABASE_SERVICE_ROLE_KEY")?,
            audit_url: env_or("AUDIT_URL", "http://audit-api:8084"),
            notification_url: env_or("NOTIFICATION_URL", "http://notification-api:8099"),
            asset_registry_url: env_or("ASSET_REGISTRY_URL", "http://asset-registry:8081"),
            worker_url: env_or("ACCOUNT_WORKER_URL", "http://account-worker:8092"),
            token_service: env_or("ACCOUNT_TOKEN_SERVICE", "account"),
            evm_rpc_url: required("EVM_RPC_URL")?,
            solana_rpc_url: required("SOLANA_RPC_URL")?,
            binance_base_url: env_or("BINANCE_BASE_URL", DEFAULT_BINANCE_BASE),
            okx_base_url: env_or("OKX_BASE_URL", DEFAULT_OKX_BASE),
            pool_connections: env_or("ACCOUNT_API_POOL", "12")
                .parse()
                .context("ACCOUNT_API_POOL must be a number")?,
        })
    }
}

/// The sync worker's configuration.
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// HTTP port for the internal wake endpoint and probes.
    pub port: u16,
    /// Postgres connection string for the `account` schema (secret).
    pub database_url: String,
    /// The service-token registry file (secret).
    pub service_tokens_path: PathBuf,
    /// Supabase Auth admin base URL.
    pub supabase_auth_url: String,
    /// Supabase service-role key (secret).
    pub service_role_key: String,
    /// Audit service base URL.
    pub audit_url: String,
    /// Notification service base URL.
    pub notification_url: String,
    /// Asset Registry base URL.
    pub asset_registry_url: String,
    /// The service name whose token the worker presents to other services.
    pub token_service: String,
    /// EVM JSON-RPC endpoint.
    pub evm_rpc_url: String,
    /// Solana JSON-RPC endpoint.
    pub solana_rpc_url: String,
    /// Binance REST base.
    pub binance_base_url: String,
    /// OKX REST base.
    pub okx_base_url: String,
    /// How long the supervisor sleeps between native sweeps.
    pub sync_interval_seconds: u64,
    /// How often credentials are re-verified.
    pub verify_interval_seconds: u64,
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
            port: env_or("ACCOUNT_WORKER_PORT", "8092")
                .parse()
                .context("ACCOUNT_WORKER_PORT must be a valid port")?,
            database_url: secret("DATABASE_URL")?,
            service_tokens_path: PathBuf::from(required("SERVICE_TOKENS_FILE")?),
            supabase_auth_url: required("SUPABASE_AUTH_URL")?,
            service_role_key: secret("SUPABASE_SERVICE_ROLE_KEY")?,
            audit_url: env_or("AUDIT_URL", "http://audit-api:8084"),
            notification_url: env_or("NOTIFICATION_URL", "http://notification-api:8099"),
            asset_registry_url: env_or("ASSET_REGISTRY_URL", "http://asset-registry:8081"),
            token_service: env_or("ACCOUNT_TOKEN_SERVICE", "account"),
            evm_rpc_url: required("EVM_RPC_URL")?,
            solana_rpc_url: required("SOLANA_RPC_URL")?,
            binance_base_url: env_or("BINANCE_BASE_URL", DEFAULT_BINANCE_BASE),
            okx_base_url: env_or("OKX_BASE_URL", DEFAULT_OKX_BASE),
            sync_interval_seconds: env_or("ACCOUNT_SYNC_INTERVAL_SECONDS", "60")
                .parse()
                .context("ACCOUNT_SYNC_INTERVAL_SECONDS must be a number")?,
            verify_interval_seconds: env_or("ACCOUNT_VERIFY_INTERVAL_SECONDS", "21600")
                .parse()
                .context("ACCOUNT_VERIFY_INTERVAL_SECONDS must be a number")?,
            pool_connections: env_or("ACCOUNT_WORKER_POOL", "8")
                .parse()
                .context("ACCOUNT_WORKER_POOL must be a number")?,
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
