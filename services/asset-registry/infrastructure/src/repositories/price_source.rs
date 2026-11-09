//! Postgres implementation of [`PriceSourceRepository`].

use asset_registry_application::ports::{PriceSourceRepository, RepositoryError};
use asset_registry_domain::PriceSource;
use async_trait::async_trait;
use sqlx::PgPool;

use super::{map_sqlx_error, priority_from_storage};

/// sqlx-backed price-source repository.
#[derive(Debug, Clone)]
pub struct PgPriceSourceRepository {
    pool: PgPool,
}

impl PgPriceSourceRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct PriceSourceRow {
    id: String,
    name: String,
    priority: i32,
    licence_ref: Option<String>,
    enabled: bool,
}

impl PriceSourceRow {
    fn into_domain(self) -> Result<PriceSource, RepositoryError> {
        let mut source = PriceSource::new(
            self.id,
            &self.name,
            priority_from_storage(self.priority)?,
            self.licence_ref.as_deref(),
        )
        .map_err(|error| RepositoryError::InvalidData(error.to_string()))?;
        if self.enabled {
            if source.licence_ref().is_none() {
                return Err(RepositoryError::InvalidData(format!(
                    "price source '{}' is enabled without a licence reference",
                    source.name()
                )));
            }
            source
                .enable()
                .map_err(|error| RepositoryError::InvalidData(error.to_string()))?;
        } else {
            source.disable();
        }
        Ok(source)
    }
}

const SELECT_COLUMNS: &str = "id, name, priority, licence_ref, enabled";

#[async_trait]
impl PriceSourceRepository for PgPriceSourceRepository {
    async fn list_enabled_by_priority(&self) -> Result<Vec<PriceSource>, RepositoryError> {
        let rows = sqlx::query_as::<_, PriceSourceRow>(&format!(
            "SELECT {SELECT_COLUMNS} FROM price_sources WHERE enabled ORDER BY priority ASC"
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        rows.into_iter().map(PriceSourceRow::into_domain).collect()
    }

    async fn list(&self) -> Result<Vec<PriceSource>, RepositoryError> {
        let rows = sqlx::query_as::<_, PriceSourceRow>(&format!(
            "SELECT {SELECT_COLUMNS} FROM price_sources ORDER BY priority ASC"
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        rows.into_iter().map(PriceSourceRow::into_domain).collect()
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<PriceSource>, RepositoryError> {
        let row = sqlx::query_as::<_, PriceSourceRow>(&format!(
            "SELECT {SELECT_COLUMNS} FROM price_sources WHERE name = $1"
        ))
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        row.map(PriceSourceRow::into_domain).transpose()
    }

    async fn upsert(&self, source: &PriceSource) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO price_sources (id, name, priority, licence_ref, enabled)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (name) DO UPDATE
             SET priority = EXCLUDED.priority,
                 licence_ref = EXCLUDED.licence_ref,
                 enabled = EXCLUDED.enabled,
                 updated_at = now()",
        )
        .bind(source.id().as_str())
        .bind(source.name())
        .bind(i32::try_from(source.priority()).map_err(|_| {
            RepositoryError::InvalidData(format!("priority out of range: {}", source.priority()))
        })?)
        .bind(source.licence_ref())
        .bind(source.enabled())
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn set_enabled(&self, name: &str, enabled: bool) -> Result<(), RepositoryError> {
        let result = sqlx::query(
            "UPDATE price_sources SET enabled = $1, updated_at = now() WHERE name = $2",
        )
        .bind(enabled)
        .bind(name)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        if result.rows_affected() == 0 {
            return Err(RepositoryError::Conflict(format!(
                "price source '{name}' does not exist"
            )));
        }
        Ok(())
    }

    async fn enabled_priority_exists(
        &self,
        priority: u32,
        except_name: Option<&str>,
    ) -> Result<bool, RepositoryError> {
        let priority = i32::try_from(priority).map_err(|_| {
            RepositoryError::InvalidData(format!("priority out of range: {priority}"))
        })?;
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (
                 SELECT 1 FROM price_sources
                 WHERE enabled
                   AND priority = $1
                   AND ($2::text IS NULL OR name <> $2)
             )",
        )
        .bind(priority)
        .bind(except_name)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        Ok(exists)
    }
}
