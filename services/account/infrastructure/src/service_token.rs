//! The inbound service-token registry.
//!
//! A token is hashed with SHA-256 and looked up in `account.service_token_registry`
//! (token hash → caller name → per-operation allow-list). Outbound tokens are
//! read from a Docker secret file; they never live in source or an env file.

use std::collections::BTreeMap;
use std::path::Path;

use account_application::ports::{RegisteredService, RepositoryError, ServiceTokenRegistry};
use account_domain::Operation;
use async_trait::async_trait;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

/// The SHA-256 hex digest of a presented token.
#[must_use]
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.trim().as_bytes());
    hex::encode(hasher.finalize())
}

/// Resolves inbound tokens from the database and outbound tokens from a secret
/// file.
#[derive(Debug, Clone)]
pub struct PgServiceTokenRegistry {
    pool: PgPool,
    outbound: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct TokenFile {
    services: BTreeMap<String, String>,
}

#[derive(sqlx::FromRow)]
struct TokenRow {
    caller: String,
    operations: Vec<String>,
}

impl PgServiceTokenRegistry {
    /// Loads the outbound tokens from a JSON secret file. The file is either
    /// `{"account":"<token>"}` or `{"services":{"account":"<token>"}}`.
    ///
    /// # Errors
    /// Returns a message when the file is unreadable, malformed, or empty.
    pub fn from_file(pool: PgPool, path: &Path) -> Result<Self, String> {
        let raw = std::fs::read_to_string(path)
            .map_err(|error| format!("{} is unreadable: {error}", path.display()))?;
        let outbound = parse_outbound(&raw)?;
        Ok(Self { pool, outbound })
    }

    /// Builds a registry with explicit outbound tokens, used by tests.
    #[must_use]
    pub fn with_outbound(pool: PgPool, outbound: &[(&str, &str)]) -> Self {
        Self {
            pool,
            outbound: outbound
                .iter()
                .map(|(service, token)| ((*service).to_string(), (*token).to_string()))
                .collect(),
        }
    }
}

fn parse_outbound(raw: &str) -> Result<BTreeMap<String, String>, String> {
    let value: serde_json::Value = serde_json::from_str(raw)
        .map_err(|error| format!("service-token file is not JSON: {error}"))?;
    let map: BTreeMap<String, String> = if value.get("services").is_some() {
        serde_json::from_value::<TokenFile>(value)
            .map_err(|error| format!("service-token file is malformed: {error}"))?
            .services
    } else {
        serde_json::from_value(value)
            .map_err(|error| format!("service-token file is malformed: {error}"))?
    };
    if map.is_empty() || map.values().any(|token| token.trim().is_empty()) {
        return Err("the service-token file has an empty service or token".to_string());
    }
    Ok(map)
}

#[async_trait]
impl ServiceTokenRegistry for PgServiceTokenRegistry {
    async fn service_for(&self, token: &str) -> Result<Option<RegisteredService>, RepositoryError> {
        if token.trim().is_empty() {
            return Ok(None);
        }
        let row = sqlx::query_as::<_, TokenRow>(
            "SELECT caller, operations
             FROM service_token_registry
             WHERE token_hash = $1 AND active",
        )
        .bind(hash_token(token))
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| RepositoryError::Unavailable(error.to_string()))?;
        let Some(row) = row else {
            return Ok(None);
        };
        let mut operations = Vec::with_capacity(row.operations.len());
        for operation in &row.operations {
            operations.push(Operation::parse(operation).map_err(|error| {
                RepositoryError::InvalidData(format!("stored service token operation: {error}"))
            })?);
        }
        Ok(Some(RegisteredService {
            name: row.caller,
            operations,
        }))
    }

    fn token_for(&self, service: &str) -> Option<String> {
        self.outbound.get(service).cloned()
    }
}
