//! Non-secret configuration. Every credential is read from a secret file
//! (`*_FILE`) — a Docker secret mounted from Supabase Vault — and never from an
//! env file committed to the repository.

use std::path::PathBuf;

use anyhow::{Context, Result};

const DEFAULT_ASSET_REGISTRY_URL: &str = "http://asset-registry:8081";
const DEFAULT_AUDIT_URL: &str = "http://audit-api:8084";

/// The API service's configuration.
#[derive(Debug, Clone)]
pub struct ApiConfig {
    /// HTTP port.
    pub port: u16,
    /// Postgres connection string for the `market_data` schema (secret).
    pub database_url: String,
    /// The optional service-token registry file. When absent, the outbound
    /// token is read from Vault.
    pub service_tokens_path: Option<PathBuf>,
    /// Asset Registry base URL (C4).
    pub asset_registry_url: String,
    /// Audit service base URL (C10).
    pub audit_url: String,
    /// The service name whose token the API presents to other services.
    pub token_service: String,
    /// The shared C11 run token, read from Vault at startup (secret).
    pub run_token_path: Option<PathBuf>,
    /// Provider base URLs, `name=url` pairs.
    pub provider_urls: Vec<(String, String)>,
    /// The launch base currencies whose USD FX pairs the top100 job refreshes.
    pub fx_base_currencies: Vec<String>,
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
            port: env_or("MARKET_DATA_PORT", "8093")
                .parse()
                .context("MARKET_DATA_PORT must be a valid port")?,
            database_url: secret("DATABASE_URL")?,
            service_tokens_path: std::env::var("SERVICE_TOKENS_FILE").ok().map(PathBuf::from),
            asset_registry_url: env_or("ASSET_REGISTRY_URL", DEFAULT_ASSET_REGISTRY_URL),
            audit_url: env_or("AUDIT_URL", DEFAULT_AUDIT_URL),
            token_service: env_or("MARKET_DATA_TOKEN_SERVICE", "market-data"),
            run_token_path: std::env::var("RUN_TOKEN_FILE").ok().map(PathBuf::from),
            provider_urls: parse_provider_urls(&env_or("MARKET_DATA_PROVIDER_URLS", "")),
            fx_base_currencies: parse_currencies(&env_or(
                "MARKET_DATA_FX_BASE_CURRENCIES",
                "EUR,GBP,VND,USDT",
            )),
            pool_connections: env_or("MARKET_DATA_API_POOL", "12")
                .parse()
                .context("MARKET_DATA_API_POOL must be a number")?,
        })
    }
}

/// The refresh worker's configuration.
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// HTTP port for probes.
    pub port: u16,
    /// Postgres connection string for the `market_data` schema (secret).
    pub database_url: String,
    /// The optional service-token registry file. When absent, the outbound
    /// token is read from Vault.
    pub service_tokens_path: Option<PathBuf>,
    /// Asset Registry base URL (C4).
    pub asset_registry_url: String,
    /// Audit service base URL (C10).
    pub audit_url: String,
    /// The service name whose token the worker presents to other services.
    pub token_service: String,
    /// Provider base URLs, `name=url` pairs, comma separated.
    pub provider_urls: Vec<(String, String)>,
    /// The launch base currencies whose USD FX pairs the top100 job refreshes.
    pub fx_base_currencies: Vec<String>,
    /// How often the supervisor sweeps expired leases and retention.
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
            port: env_or("MARKET_DATA_WORKER_PORT", "8094")
                .parse()
                .context("MARKET_DATA_WORKER_PORT must be a valid port")?,
            database_url: secret("DATABASE_URL")?,
            service_tokens_path: std::env::var("SERVICE_TOKENS_FILE").ok().map(PathBuf::from),
            asset_registry_url: env_or("ASSET_REGISTRY_URL", DEFAULT_ASSET_REGISTRY_URL),
            audit_url: env_or("AUDIT_URL", DEFAULT_AUDIT_URL),
            token_service: env_or("MARKET_DATA_TOKEN_SERVICE", "market-data"),
            provider_urls: parse_provider_urls(&env_or("MARKET_DATA_PROVIDER_URLS", "")),
            fx_base_currencies: parse_currencies(&env_or(
                "MARKET_DATA_FX_BASE_CURRENCIES",
                "EUR,GBP,VND,USDT",
            )),
            sweep_interval_seconds: env_or("MARKET_DATA_SWEEP_INTERVAL_SECONDS", "60")
                .parse()
                .context("MARKET_DATA_SWEEP_INTERVAL_SECONDS must be a number")?,
            pool_connections: env_or("MARKET_DATA_WORKER_POOL", "8")
                .parse()
                .context("MARKET_DATA_WORKER_POOL must be a number")?,
        })
    }
}

fn parse_provider_urls(raw: &str) -> Vec<(String, String)> {
    raw.split(',')
        .filter_map(|entry| {
            let (name, url) = entry.split_once('=')?;
            let name = name.trim().to_ascii_lowercase();
            let url = url.trim();
            (!name.is_empty() && !url.is_empty()).then(|| (name, url.to_string()))
        })
        .collect()
}

fn parse_currencies(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_uppercase())
        .collect()
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
