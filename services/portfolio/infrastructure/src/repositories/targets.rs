//! The targets and guardrails repository.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use portfolio_application::ports::{GuardrailRepository, RepositoryError, TargetRepository};
use portfolio_domain::{Guardrail, GuardrailId, PortfolioId, TargetAllocation, TargetAllocationId};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use super::map_error;

/// sqlx-backed targets and guardrails.
#[derive(Debug, Clone)]
pub struct PgTargetRepository {
    pool: PgPool,
}

impl PgTargetRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct TargetRow {
    id: Uuid,
    portfolio_id: Uuid,
    asset_id: String,
    target_percent: Decimal,
    updated_at: DateTime<Utc>,
}

impl From<TargetRow> for TargetAllocation {
    fn from(row: TargetRow) -> Self {
        TargetAllocation::from_stored(
            TargetAllocationId::new(row.id),
            PortfolioId::new(row.portfolio_id),
            row.asset_id,
            row.target_percent,
            row.updated_at,
        )
    }
}

#[derive(sqlx::FromRow)]
struct GuardrailRow {
    id: Uuid,
    portfolio_id: Uuid,
    asset_id: String,
    threshold_points: Decimal,
    is_outside: bool,
    last_crossing_key: Option<String>,
    crossed_at: Option<DateTime<Utc>>,
    updated_at: DateTime<Utc>,
}

impl From<GuardrailRow> for Guardrail {
    fn from(row: GuardrailRow) -> Self {
        Guardrail::from_stored(
            GuardrailId::new(row.id),
            PortfolioId::new(row.portfolio_id),
            row.asset_id,
            row.threshold_points,
            row.is_outside,
            row.last_crossing_key,
            row.crossed_at,
            row.updated_at,
        )
    }
}

#[async_trait]
impl TargetRepository for PgTargetRepository {
    async fn list_targets(
        &self,
        portfolio_id: PortfolioId,
    ) -> Result<Vec<TargetAllocation>, RepositoryError> {
        let rows = sqlx::query_as::<_, TargetRow>(
            "SELECT id, portfolio_id, asset_id, target_percent, updated_at
             FROM target_allocations WHERE portfolio_id = $1 ORDER BY asset_id",
        )
        .bind(portfolio_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(map_error)?;
        Ok(rows.into_iter().map(TargetAllocation::from).collect())
    }

    async fn list_guardrails(
        &self,
        portfolio_id: PortfolioId,
    ) -> Result<Vec<Guardrail>, RepositoryError> {
        let rows = sqlx::query_as::<_, GuardrailRow>(
            "SELECT id, portfolio_id, asset_id, threshold_points, is_outside,
                    last_crossing_key, crossed_at, updated_at
             FROM guardrails WHERE portfolio_id = $1 ORDER BY asset_id",
        )
        .bind(portfolio_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(map_error)?;
        Ok(rows.into_iter().map(Guardrail::from).collect())
    }

    async fn replace_for_portfolio(
        &self,
        portfolio_id: PortfolioId,
        targets: &[TargetAllocation],
        guardrails: &[Guardrail],
    ) -> Result<(), RepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(map_error)?;
        sqlx::query("DELETE FROM target_allocations WHERE portfolio_id = $1")
            .bind(portfolio_id.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_error)?;
        sqlx::query("DELETE FROM guardrails WHERE portfolio_id = $1")
            .bind(portfolio_id.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_error)?;
        for target in targets {
            sqlx::query(
                "INSERT INTO target_allocations
                    (id, portfolio_id, asset_id, target_percent, updated_at)
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(target.id().as_uuid())
            .bind(target.portfolio_id().as_uuid())
            .bind(target.asset_id())
            .bind(target.target_percent())
            .bind(target.updated_at())
            .execute(&mut *transaction)
            .await
            .map_err(map_error)?;
        }
        for guardrail in guardrails {
            sqlx::query(
                "INSERT INTO guardrails
                    (id, portfolio_id, asset_id, threshold_points, is_outside,
                     last_crossing_key, crossed_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(guardrail.id().as_uuid())
            .bind(guardrail.portfolio_id().as_uuid())
            .bind(guardrail.asset_id())
            .bind(guardrail.threshold_points())
            .bind(guardrail.is_outside())
            .bind(guardrail.last_crossing_key())
            .bind(guardrail.crossed_at())
            .bind(guardrail.updated_at())
            .execute(&mut *transaction)
            .await
            .map_err(map_error)?;
        }
        transaction.commit().await.map_err(map_error)
    }
}

#[async_trait]
impl GuardrailRepository for PgTargetRepository {
    async fn update(&self, guardrail: &Guardrail) -> Result<(), RepositoryError> {
        sqlx::query(
            "UPDATE guardrails
             SET is_outside = $2, last_crossing_key = $3, crossed_at = $4, updated_at = $5
             WHERE id = $1",
        )
        .bind(guardrail.id().as_uuid())
        .bind(guardrail.is_outside())
        .bind(guardrail.last_crossing_key())
        .bind(guardrail.crossed_at())
        .bind(guardrail.updated_at())
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(map_error)
    }
}
