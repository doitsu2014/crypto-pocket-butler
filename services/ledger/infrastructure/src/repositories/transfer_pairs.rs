//! Postgres implementation of [`TransferPairRepository`].

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ledger_application::ports::{RepositoryError, TransferPairRepository};
use ledger_domain::{LedgerSubjectId, TransactionId, TransferPair, TransferPairId, TransferStatus};
use sqlx::PgPool;
use uuid::Uuid;

use super::{invalid_data, map_sqlx_error};

const COLUMNS: &str =
    "id, subject_id, out_transaction_id, in_transaction_id, asset_id, status, matched_at";

/// sqlx-backed transfer-pair store.
#[derive(Debug, Clone)]
pub struct PgTransferPairRepository {
    pool: PgPool,
}

impl PgTransferPairRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct PairRow {
    id: Uuid,
    subject_id: Uuid,
    out_transaction_id: Option<Uuid>,
    in_transaction_id: Option<Uuid>,
    asset_id: String,
    status: String,
    matched_at: Option<DateTime<Utc>>,
}

impl PairRow {
    fn into_domain(self) -> Result<TransferPair, RepositoryError> {
        let status = TransferStatus::parse(&self.status)
            .map_err(|error| invalid_data("read transfer status", error))?;
        Ok(TransferPair::from_stored(
            TransferPairId::new(self.id),
            LedgerSubjectId::new(self.subject_id),
            self.out_transaction_id.map(TransactionId::new),
            self.in_transaction_id.map(TransactionId::new),
            self.asset_id,
            status,
            self.matched_at,
        ))
    }
}

#[async_trait]
impl TransferPairRepository for PgTransferPairRepository {
    async fn insert(&self, pair: &TransferPair) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO transfer_pairs
                 (id, subject_id, out_transaction_id, in_transaction_id, asset_id, status,
                  matched_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(pair.id().as_uuid())
        .bind(pair.subject_id().as_uuid())
        .bind(pair.out_transaction_id().map(|id| id.as_uuid()))
        .bind(pair.in_transaction_id().map(|id| id.as_uuid()))
        .bind(pair.asset_id())
        .bind(pair.status().as_str())
        .bind(pair.matched_at())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("insert transfer pair", error))?;
        Ok(())
    }

    async fn list_for_subject(
        &self,
        subject: LedgerSubjectId,
    ) -> Result<Vec<TransferPair>, RepositoryError> {
        let rows = sqlx::query_as::<_, PairRow>(&format!(
            "SELECT {COLUMNS} FROM transfer_pairs WHERE subject_id = $1 ORDER BY id"
        ))
        .bind(subject.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("list transfer pairs", error))?;
        rows.into_iter().map(PairRow::into_domain).collect()
    }

    async fn mark_matched(
        &self,
        out_transaction: TransactionId,
        in_transaction: TransactionId,
        at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        sqlx::query(
            "UPDATE transfer_pairs
             SET in_transaction_id = $2, status = 'matched', matched_at = $3
             WHERE out_transaction_id = $1",
        )
        .bind(out_transaction.as_uuid())
        .bind(in_transaction.as_uuid())
        .bind(at)
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("match transfer pair", error))?;
        Ok(())
    }

    async fn mark_rejected(&self, out_transaction: TransactionId) -> Result<(), RepositoryError> {
        sqlx::query("UPDATE transfer_pairs SET status = 'rejected' WHERE out_transaction_id = $1")
            .bind(out_transaction.as_uuid())
            .execute(&self.pool)
            .await
            .map_err(|error| map_sqlx_error("reject transfer pair", error))?;
        Ok(())
    }

    async fn match_pending_in(
        &self,
        in_transaction: TransactionId,
        out_transaction: TransactionId,
        at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        sqlx::query(
            "UPDATE transfer_pairs
             SET out_transaction_id = $2, status = 'matched', matched_at = $3
             WHERE in_transaction_id = $1",
        )
        .bind(in_transaction.as_uuid())
        .bind(out_transaction.as_uuid())
        .bind(at)
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("match pending transfer pair", error))?;
        Ok(())
    }
}
