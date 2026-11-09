//! Postgres implementation of [`LotRepository`].

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ledger_application::ports::{LotRepository, RepositoryError};
use ledger_domain::{CostBasisLot, CostBasisLotId, LedgerSubjectId, TransactionId};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use super::map_sqlx_error;

const COLUMNS: &str = "id, subject_id, asset_id, acquired_at, quantity, remaining, unit_cost, \
     currency, source_transaction_id";

/// sqlx-backed derived lot store.
#[derive(Debug, Clone)]
pub struct PgLotRepository {
    pool: PgPool,
}

impl PgLotRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct LotRow {
    id: Uuid,
    subject_id: Uuid,
    asset_id: String,
    acquired_at: DateTime<Utc>,
    quantity: Decimal,
    remaining: Decimal,
    unit_cost: Decimal,
    currency: String,
    source_transaction_id: Uuid,
}

impl LotRow {
    fn into_domain(self) -> CostBasisLot {
        CostBasisLot::from_stored(
            CostBasisLotId::new(self.id),
            LedgerSubjectId::new(self.subject_id),
            self.asset_id,
            self.acquired_at,
            self.quantity,
            self.remaining,
            self.unit_cost,
            self.currency,
            TransactionId::new(self.source_transaction_id),
        )
    }
}

#[async_trait]
impl LotRepository for PgLotRepository {
    async fn replace_for_subject(
        &self,
        subject: LedgerSubjectId,
        lots: &[CostBasisLot],
    ) -> Result<(), RepositoryError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| map_sqlx_error("begin lot rebuild", error))?;
        sqlx::query("DELETE FROM cost_basis_lots WHERE subject_id = $1")
            .bind(subject.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(|error| map_sqlx_error("clear lots", error))?;
        for lot in lots {
            sqlx::query(
                "INSERT INTO cost_basis_lots
                     (id, subject_id, asset_id, acquired_at, quantity, remaining, unit_cost,
                      currency, source_transaction_id)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            )
            .bind(lot.id().as_uuid())
            .bind(lot.subject_id().as_uuid())
            .bind(lot.asset_id())
            .bind(lot.acquired_at())
            .bind(lot.quantity())
            .bind(lot.remaining())
            .bind(lot.unit_cost())
            .bind(lot.currency())
            .bind(lot.source_transaction_id().as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(|error| map_sqlx_error("insert lot", error))?;
        }
        transaction
            .commit()
            .await
            .map_err(|error| map_sqlx_error("commit lot rebuild", error))?;
        Ok(())
    }

    async fn list_open(
        &self,
        subject: LedgerSubjectId,
    ) -> Result<Vec<CostBasisLot>, RepositoryError> {
        let rows = sqlx::query_as::<_, LotRow>(&format!(
            "SELECT {COLUMNS} FROM cost_basis_lots
             WHERE subject_id = $1 AND remaining > 0
             ORDER BY asset_id ASC, acquired_at ASC, id ASC"
        ))
        .bind(subject.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("list open lots", error))?;
        Ok(rows.into_iter().map(LotRow::into_domain).collect())
    }
}
