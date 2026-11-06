//! Postgres implementation of [`ChainRepository`].

use asset_registry_application::ports::{ChainRepository, RepositoryError};
use asset_registry_domain::{Chain, ChainFamily, ChainId};
use async_trait::async_trait;
use sqlx::PgPool;

use super::map_sqlx_error;

/// sqlx-backed chain repository.
#[derive(Debug, Clone)]
pub struct PgChainRepository {
    pool: PgPool,
}

impl PgChainRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct ChainRow {
    id: String,
    name: String,
    family: String,
    launch_coverage: bool,
}

impl ChainRow {
    fn into_domain(self) -> Result<Chain, RepositoryError> {
        let family = ChainFamily::parse(&self.family)
            .map_err(|error| RepositoryError::InvalidData(error.to_string()))?;
        Chain::new(self.id, &self.name, family, self.launch_coverage)
            .map_err(|error| RepositoryError::InvalidData(error.to_string()))
    }
}

#[async_trait]
impl ChainRepository for PgChainRepository {
    async fn find_by_id(&self, id: &ChainId) -> Result<Option<Chain>, RepositoryError> {
        let row = sqlx::query_as::<_, ChainRow>(
            "SELECT id, name, family, launch_coverage FROM chains WHERE id = $1",
        )
        .bind(id.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        row.map(ChainRow::into_domain).transpose()
    }

    async fn list(&self) -> Result<Vec<Chain>, RepositoryError> {
        let rows = sqlx::query_as::<_, ChainRow>(
            "SELECT id, name, family, launch_coverage FROM chains ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        rows.into_iter().map(ChainRow::into_domain).collect()
    }

    async fn upsert(&self, chain: &Chain) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO chains (id, name, family, launch_coverage)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (id) DO UPDATE
             SET name = EXCLUDED.name,
                 family = EXCLUDED.family,
                 launch_coverage = EXCLUDED.launch_coverage",
        )
        .bind(chain.id().as_str())
        .bind(chain.name())
        .bind(chain.family().as_str())
        .bind(chain.launch_coverage())
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        Ok(())
    }
}
