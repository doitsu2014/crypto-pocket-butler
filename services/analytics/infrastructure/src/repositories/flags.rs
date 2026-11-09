//! Postgres implementation of [`FlagRepository`].

use analytics_application::ports::{FlagRepository, RepositoryError};
use analytics_domain::{FeatureFlag, FeatureFlagId};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{invalid_data, map_sqlx_error};

const FLAG_COLUMNS: &str =
    "id, key, owner, expires_at, rollout_percentage, status, created_at, updated_at";

/// sqlx-backed flag store.
#[derive(Debug, Clone)]
pub struct PgFlagRepository {
    pool: PgPool,
}

impl PgFlagRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct FlagRow {
    id: Uuid,
    key: String,
    owner: String,
    expires_at: DateTime<Utc>,
    rollout_percentage: i16,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl FlagRow {
    fn into_domain(self) -> Result<FeatureFlag, RepositoryError> {
        FeatureFlag::from_stored(
            FeatureFlagId::new(self.id),
            &self.key,
            &self.owner,
            self.expires_at,
            u16::try_from(self.rollout_percentage)
                .map_err(|error| invalid_data("read flag", error))?,
            &self.status,
            self.created_at,
            self.updated_at,
        )
        .map_err(|error| invalid_data("read flag", error))
    }
}

#[async_trait]
impl FlagRepository for PgFlagRepository {
    async fn insert(&self, flag: &FeatureFlag) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO feature_flags
                 (id, key, owner, expires_at, rollout_percentage, status, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(flag.id().as_uuid())
        .bind(flag.key())
        .bind(flag.owner())
        .bind(flag.expires_at())
        .bind(i16::from(flag.rollout_percentage()))
        .bind(flag.status().as_str())
        .bind(flag.created_at())
        .bind(flag.updated_at())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("insert flag", error))?;
        Ok(())
    }

    async fn update(&self, flag: &FeatureFlag) -> Result<(), RepositoryError> {
        sqlx::query(
            "UPDATE feature_flags
             SET owner = $2, expires_at = $3, rollout_percentage = $4, status = $5, updated_at = $6
             WHERE key = $1",
        )
        .bind(flag.key())
        .bind(flag.owner())
        .bind(flag.expires_at())
        .bind(i16::from(flag.rollout_percentage()))
        .bind(flag.status().as_str())
        .bind(flag.updated_at())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("update flag", error))?;
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool, RepositoryError> {
        let result = sqlx::query("DELETE FROM feature_flags WHERE key = $1")
            .bind(key)
            .execute(&self.pool)
            .await
            .map_err(|error| map_sqlx_error("delete flag", error))?;
        Ok(result.rows_affected() > 0)
    }

    async fn find_by_key(&self, key: &str) -> Result<Option<FeatureFlag>, RepositoryError> {
        let row = sqlx::query_as::<_, FlagRow>(&format!(
            "SELECT {FLAG_COLUMNS} FROM feature_flags WHERE key = $1"
        ))
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("read flag", error))?;
        row.map(FlagRow::into_domain).transpose()
    }

    async fn list(&self) -> Result<Vec<FeatureFlag>, RepositoryError> {
        let rows = sqlx::query_as::<_, FlagRow>(&format!(
            "SELECT {FLAG_COLUMNS} FROM feature_flags ORDER BY key"
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("list flags", error))?;
        rows.into_iter().map(FlagRow::into_domain).collect()
    }
}
