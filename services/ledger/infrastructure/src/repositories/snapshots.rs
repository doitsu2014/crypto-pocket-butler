//! Postgres implementation of [`SnapshotRepository`].

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ledger_application::ports::{RepositoryError, SnapshotRepository};
use ledger_domain::{LedgerSubjectId, PnlSnapshot, PnlSnapshotId};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use super::map_sqlx_error;

const COLUMNS: &str = "id, subject_id, portfolio_id, as_of, currency, fx_as_of, fx_estimated, \
     net_invested, realized, unrealized, total_return, total_acquisition_cost, coverage, \
     coverage_gap, computed_at";

/// sqlx-backed derived snapshot store.
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
    subject_id: Uuid,
    portfolio_id: Option<Uuid>,
    as_of: DateTime<Utc>,
    currency: String,
    fx_as_of: Option<DateTime<Utc>>,
    fx_estimated: bool,
    net_invested: Option<Decimal>,
    realized: Option<Decimal>,
    unrealized: Option<Decimal>,
    total_return: Option<Decimal>,
    total_acquisition_cost: Option<Decimal>,
    coverage: Decimal,
    coverage_gap: Vec<String>,
    computed_at: DateTime<Utc>,
}

impl SnapshotRow {
    fn into_domain(self) -> PnlSnapshot {
        PnlSnapshot::from_stored(
            PnlSnapshotId::new(self.id),
            LedgerSubjectId::new(self.subject_id),
            self.portfolio_id,
            self.as_of,
            self.currency,
            self.fx_as_of,
            self.fx_estimated,
            self.net_invested,
            self.realized,
            self.unrealized,
            self.total_return,
            self.total_acquisition_cost,
            self.coverage,
            self.coverage_gap,
            self.computed_at,
        )
    }
}

#[async_trait]
impl SnapshotRepository for PgSnapshotRepository {
    async fn upsert(&self, snapshot: &PnlSnapshot) -> Result<(), RepositoryError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| map_sqlx_error("begin snapshot upsert", error))?;
        let updated = sqlx::query(
            "UPDATE pnl_snapshots
             SET id = $3, as_of = $4, currency = $5, fx_as_of = $6, fx_estimated = $7,
                 net_invested = $8, realized = $9, unrealized = $10, total_return = $11,
                 total_acquisition_cost = $12, coverage = $13, coverage_gap = $14,
                 computed_at = $15
             WHERE subject_id = $1 AND portfolio_id IS NOT DISTINCT FROM $2",
        )
        .bind(snapshot.subject_id().as_uuid())
        .bind(snapshot.portfolio_id())
        .bind(snapshot.id().as_uuid())
        .bind(snapshot.as_of())
        .bind(snapshot.currency())
        .bind(snapshot.fx_as_of())
        .bind(snapshot.fx_estimated())
        .bind(snapshot.net_invested())
        .bind(snapshot.realized())
        .bind(snapshot.unrealized())
        .bind(snapshot.total_return())
        .bind(snapshot.total_acquisition_cost())
        .bind(snapshot.coverage())
        .bind(snapshot.coverage_gap().to_vec())
        .bind(snapshot.computed_at())
        .execute(&mut *transaction)
        .await
        .map_err(|error| map_sqlx_error("update snapshot", error))?;
        if updated.rows_affected() == 0 {
            sqlx::query(
                "INSERT INTO pnl_snapshots
                     (id, subject_id, portfolio_id, as_of, currency, fx_as_of, fx_estimated,
                      net_invested, realized, unrealized, total_return, total_acquisition_cost,
                      coverage, coverage_gap, computed_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
            )
            .bind(snapshot.id().as_uuid())
            .bind(snapshot.subject_id().as_uuid())
            .bind(snapshot.portfolio_id())
            .bind(snapshot.as_of())
            .bind(snapshot.currency())
            .bind(snapshot.fx_as_of())
            .bind(snapshot.fx_estimated())
            .bind(snapshot.net_invested())
            .bind(snapshot.realized())
            .bind(snapshot.unrealized())
            .bind(snapshot.total_return())
            .bind(snapshot.total_acquisition_cost())
            .bind(snapshot.coverage())
            .bind(snapshot.coverage_gap().to_vec())
            .bind(snapshot.computed_at())
            .execute(&mut *transaction)
            .await
            .map_err(|error| map_sqlx_error("insert snapshot", error))?;
        }
        transaction
            .commit()
            .await
            .map_err(|error| map_sqlx_error("commit snapshot upsert", error))?;
        Ok(())
    }

    async fn find_latest(
        &self,
        subject: LedgerSubjectId,
        portfolio_id: Option<Uuid>,
    ) -> Result<Option<PnlSnapshot>, RepositoryError> {
        let row = sqlx::query_as::<_, SnapshotRow>(&format!(
            "SELECT {COLUMNS} FROM pnl_snapshots
             WHERE subject_id = $1 AND portfolio_id IS NOT DISTINCT FROM $2
             ORDER BY computed_at DESC LIMIT 1"
        ))
        .bind(subject.as_uuid())
        .bind(portfolio_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("read snapshot", error))?;
        Ok(row.map(SnapshotRow::into_domain))
    }
}
