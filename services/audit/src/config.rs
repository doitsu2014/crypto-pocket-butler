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
    /// Postgres connection string for `audit_app` (secret).
    pub database_url: String,
    /// The service-token registry file (secret).
    pub service_tokens_path: PathBuf,
    /// Supabase JWKS URL (public).
    pub jwks_url: String,
    /// Expected JWT issuer.
    pub jwt_issuer: String,
    /// Expected JWT audience.
    pub jwt_audience: String,
    /// Appends per minute per emitting service, per replica.
    pub appends_per_minute: u32,
    /// Admin queries per minute per caller, per replica.
    pub queries_per_minute: u32,
    /// Exports per hour per caller, per replica.
    pub exports_per_hour: u32,
}

impl ApiConfig {
    /// Reads configuration from the environment.
    ///
    /// # Errors
    /// Returns an error naming the missing variable when a required value is absent.
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            port: env_or("AUDIT_PORT", "8084")
                .parse()
                .context("AUDIT_PORT must be a valid port")?,
            database_url: secret("DATABASE_URL")?,
            service_tokens_path: PathBuf::from(required("SERVICE_TOKENS_FILE")?),
            jwks_url: required("JWT_JWKS_URL")?,
            jwt_issuer: required("JWT_ISSUER")?,
            jwt_audience: required("JWT_AUDIENCE")?,
            appends_per_minute: env_or("AUDIT_APPENDS_PER_MINUTE", "1000")
                .parse()
                .context("AUDIT_APPENDS_PER_MINUTE must be a number")?,
            queries_per_minute: env_or("AUDIT_QUERIES_PER_MINUTE", "30")
                .parse()
                .context("AUDIT_QUERIES_PER_MINUTE must be a number")?,
            exports_per_hour: env_or("AUDIT_EXPORTS_PER_HOUR", "5")
                .parse()
                .context("AUDIT_EXPORTS_PER_HOUR must be a number")?,
        })
    }
}

/// The retention worker's configuration.
#[derive(Debug, Clone)]
pub struct RetentionConfig {
    /// Postgres connection string for `audit_retention` (secret).
    pub database_url: String,
    /// How long the supervisor sleeps between sweeps.
    pub sweep_interval_seconds: u64,
    /// Rows deleted per batch.
    pub batch_size: u32,
    /// The maximum batches one sweep may run.
    pub max_batches: u32,
    /// Milliseconds to pause between batches.
    pub pause_millis: u64,
    /// How many months of partitions to keep ahead.
    pub partition_horizon_months: u32,
    /// How many privacy deletions one tick executes.
    pub deletion_batch: u32,
}

impl RetentionConfig {
    /// Reads configuration from the environment.
    ///
    /// # Errors
    /// Returns an error naming the missing variable when a required value is absent.
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: secret("DATABASE_URL")?,
            sweep_interval_seconds: env_or("AUDIT_SWEEP_INTERVAL_SECONDS", "3600")
                .parse()
                .context("AUDIT_SWEEP_INTERVAL_SECONDS must be a number")?,
            batch_size: env_or("AUDIT_RETENTION_BATCH_SIZE", "10000")
                .parse()
                .context("AUDIT_RETENTION_BATCH_SIZE must be a number")?,
            max_batches: env_or("AUDIT_RETENTION_MAX_BATCHES", "1000")
                .parse()
                .context("AUDIT_RETENTION_MAX_BATCHES must be a number")?,
            pause_millis: env_or("AUDIT_RETENTION_PAUSE_MS", "200")
                .parse()
                .context("AUDIT_RETENTION_PAUSE_MS must be a number")?,
            partition_horizon_months: env_or("AUDIT_PARTITION_HORIZON_MONTHS", "3")
                .parse()
                .context("AUDIT_PARTITION_HORIZON_MONTHS must be a number")?,
            deletion_batch: env_or("AUDIT_DELETION_BATCH", "20")
                .parse()
                .context("AUDIT_DELETION_BATCH must be a number")?,
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
