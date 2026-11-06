//! Postgres implementation of [`RegistryMetadataRepository`].

use asset_registry_application::ports::{RegistryMetadataRepository, RepositoryError};
use async_trait::async_trait;
use sqlx::PgPool;

use super::map_sqlx_error;

/// sqlx-backed registry metadata repository.
#[derive(Debug, Clone)]
pub struct PgMetadataRepository {
    pool: PgPool,
}

impl PgMetadataRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RegistryMetadataRepository for PgMetadataRepository {
    async fn current_version(&self) -> Result<i64, RepositoryError> {
        sqlx::query_scalar::<_, i64>("SELECT version FROM registry_meta WHERE id = true")
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)
    }

    async fn bump_version(&self) -> Result<i64, RepositoryError> {
        sqlx::query_scalar::<_, i64>(
            "UPDATE registry_meta
             SET version = version + 1, updated_at = now()
             WHERE id = true
             RETURNING version",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error)
    }
}
