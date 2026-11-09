//! The Postgres-backed service-token registry.
//!
//! Only a SHA-256 hash of each token is stored. The registry resolves a token to
//! the service it was issued to and the operations that service may invoke.

use async_trait::async_trait;
use ledger_application::hashing::sha256_hex;
use ledger_application::ports::{RepositoryError, ServiceCaller, ServiceTokenRegistry};
use sqlx::PgPool;

/// Resolves tokens from `ledger.service_token_registry`, plus any extra
/// in-process token supplied from a Docker secret (the release smoke reader).
#[derive(Debug, Clone)]
pub struct PgServiceTokenRegistry {
    pool: PgPool,
    extra: Vec<(String, ServiceCaller)>,
}

impl PgServiceTokenRegistry {
    /// Creates the registry over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            extra: Vec::new(),
        }
    }

    /// Adds a token that is not stored in the database.
    #[must_use]
    pub fn with_extra_token(mut self, token: &str, service: &str, operations: &[&str]) -> Self {
        let token = token.trim();
        if token.is_empty() {
            return self;
        }
        self.extra.push((
            sha256_hex(token),
            ServiceCaller {
                service: service.to_string(),
                operations: operations
                    .iter()
                    .map(|operation| (*operation).to_string())
                    .collect(),
            },
        ));
        self
    }
}

#[async_trait]
impl ServiceTokenRegistry for PgServiceTokenRegistry {
    async fn resolve(&self, token: &str) -> Result<Option<ServiceCaller>, RepositoryError> {
        let presented = token.trim();
        if presented.is_empty() {
            return Ok(None);
        }
        let hash = sha256_hex(presented);
        if let Some((_, caller)) = self.extra.iter().find(|(candidate, _)| candidate == &hash) {
            return Ok(Some(caller.clone()));
        }
        let row: Option<(String, Vec<String>)> = sqlx::query_as(
            "SELECT service, allowed_operations FROM service_token_registry WHERE token_sha256 = $1",
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| RepositoryError::Unavailable(format!("resolve service token: {error}")))?;
        Ok(row.map(|(service, operations)| ServiceCaller {
            service,
            operations,
        }))
    }
}
