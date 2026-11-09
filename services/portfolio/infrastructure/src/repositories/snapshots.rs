//! The immutable snapshot repository. The runtime role has SELECT/INSERT only;
//! a duplicate hour bucket is a no-op that the use case treats as "already
//! taken".

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use portfolio_application::ports::{RepositoryError, SnapshotRepository};
use portfolio_domain::{PortfolioId, UserId, ValuationSnapshot, ValuationSnapshotId};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use super::map_error;

/// sqlx-backed snapshots.
#[derive(Debug, Clone)]
pub struct PgSnapshotRepository {
    pool: PgPool,
}

impl PgSnapshotRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct SnapshotRow {
    id: Uuid,
    portfolio_id: Uuid,
    as_of: DateTime<Utc>,
    hour_bucket: DateTime<Utc>,
    total_value: Decimal,
    currency: String,
    inputs_digest: String,
    inputs_json: String,
    lines_json: String,
}

impl From<SnapshotRow> for ValuationSnapshot {
    fn from(row: SnapshotRow) -> Self {
        ValuationSnapshot::from_stored(
            ValuationSnapshotId::new(row.id),
            PortfolioId::new(row.portfolio_id),
            row.as_of,
            row.hour_bucket,
            row.total_value,
            row.currency,
            row.inputs_digest,
            row.inputs_json,
            row.lines_json,
        )
    }
}

const SELECT: &str = "SELECT id, portfolio_id, as_of, hour_bucket, total_value, currency, \
     inputs_digest, inputs::text AS inputs_json, lines::text AS lines_json FROM valuation_snapshots";

#[async_trait]
impl SnapshotRepository for PgSnapshotRepository {
    async fn insert(&self, snapshot: &ValuationSnapshot) -> Result<(), RepositoryError> {
        let result = sqlx::query(
            "INSERT INTO valuation_snapshots
                (id, portfolio_id, as_of, hour_bucket, total_value, currency,
                 inputs_digest, inputs, lines)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8::jsonb, $9::jsonb)
             ON CONFLICT (portfolio_id, hour_bucket) DO NOTHING",
        )
        .bind(snapshot.id().as_uuid())
        .bind(snapshot.portfolio_id().as_uuid())
        .bind(snapshot.as_of())
        .bind(snapshot.hour_bucket())
        .bind(snapshot.total_value())
        .bind(snapshot.currency())
        .bind(snapshot.inputs_digest())
        .bind(snapshot.inputs_json())
        .bind(snapshot.lines_json())
        .execute(&self.pool)
        .await
        .map_err(map_error)?;
        if result.rows_affected() == 0 {
            return Err(RepositoryError::Conflict(
                "the hour bucket already has a snapshot".to_string(),
            ));
        }
        Ok(())
    }

    async fn latest(
        &self,
        portfolio_id: PortfolioId,
    ) -> Result<Option<ValuationSnapshot>, RepositoryError> {
        let row = sqlx::query_as::<_, SnapshotRow>(&format!(
            "{SELECT} WHERE portfolio_id = $1 ORDER BY as_of DESC LIMIT 1"
        ))
        .bind(portfolio_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_error)?;
        Ok(row.map(ValuationSnapshot::from))
    }

    async fn list(
        &self,
        portfolio_id: PortfolioId,
        from: Option<DateTime<Utc>>,
        limit: i64,
        cursor: Option<DateTime<Utc>>,
    ) -> Result<Vec<ValuationSnapshot>, RepositoryError> {
        let rows = sqlx::query_as::<_, SnapshotRow>(&format!(
            "{SELECT} WHERE portfolio_id = $1
               AND ($2::timestamptz IS NULL OR as_of >= $2)
               AND ($3::timestamptz IS NULL OR as_of < $3)
             ORDER BY as_of DESC LIMIT $4"
        ))
        .bind(portfolio_id.as_uuid())
        .bind(from)
        .bind(cursor)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_error)?;
        Ok(rows.into_iter().map(ValuationSnapshot::from).collect())
    }

    async fn find_by_id(
        &self,
        id: portfolio_domain::ValuationSnapshotId,
    ) -> Result<Option<ValuationSnapshot>, RepositoryError> {
        let row = sqlx::query_as::<_, SnapshotRow>(&format!("{SELECT} WHERE id = $1"))
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_error)?;
        Ok(row.map(ValuationSnapshot::from))
    }

    async fn exists_for_hour(
        &self,
        portfolio_id: PortfolioId,
        hour_bucket: DateTime<Utc>,
    ) -> Result<bool, RepositoryError> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM valuation_snapshots
             WHERE portfolio_id = $1 AND hour_bucket = $2)",
        )
        .bind(portfolio_id.as_uuid())
        .bind(hour_bucket)
        .fetch_one(&self.pool)
        .await
        .map_err(map_error)?;
        Ok(exists)
    }

    async fn list_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<ValuationSnapshot>, RepositoryError> {
        let rows = sqlx::query_as::<_, SnapshotRow>(&format!(
            "{SELECT} WHERE portfolio_id IN (SELECT id FROM portfolios WHERE user_id = $1)
             ORDER BY as_of DESC"
        ))
        .bind(user_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(map_error)?;
        Ok(rows.into_iter().map(ValuationSnapshot::from).collect())
    }
}
