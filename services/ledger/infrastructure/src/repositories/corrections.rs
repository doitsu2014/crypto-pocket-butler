//! Postgres implementation of [`CorrectionRepository`].

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ledger_application::ports::{CorrectionRepository, RepositoryError};
use ledger_domain::{Correction, CorrectionField, CorrectionId, LedgerSubjectId, TransactionId};
use sqlx::PgPool;
use uuid::Uuid;

use super::{invalid_data, map_sqlx_error};

const COLUMNS: &str =
    "id, transaction_id, field, old_value, new_value, reason, actor_id, corrected_at";

/// sqlx-backed append-only correction store.
#[derive(Debug, Clone)]
pub struct PgCorrectionRepository {
    pool: PgPool,
}

impl PgCorrectionRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct CorrectionRow {
    id: Uuid,
    transaction_id: Uuid,
    field: String,
    old_value: String,
    new_value: String,
    reason: String,
    actor_id: String,
    corrected_at: DateTime<Utc>,
}

impl CorrectionRow {
    fn into_domain(self) -> Result<Correction, RepositoryError> {
        let field = CorrectionField::parse(&self.field)
            .map_err(|error| invalid_data("read correction field", error))?;
        Ok(Correction::from_stored(
            CorrectionId::new(self.id),
            TransactionId::new(self.transaction_id),
            field,
            self.old_value,
            self.new_value,
            self.reason,
            self.actor_id,
            self.corrected_at,
        ))
    }
}

#[async_trait]
impl CorrectionRepository for PgCorrectionRepository {
    async fn insert(&self, correction: &Correction) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO corrections
                 (id, transaction_id, field, old_value, new_value, reason, actor_id, corrected_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(correction.id().as_uuid())
        .bind(correction.transaction_id().as_uuid())
        .bind(correction.field().as_str())
        .bind(correction.old_value())
        .bind(correction.new_value())
        .bind(correction.reason())
        .bind(correction.actor_id())
        .bind(correction.corrected_at())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("insert correction", error))?;
        Ok(())
    }

    async fn list_for_transaction(
        &self,
        transaction: TransactionId,
    ) -> Result<Vec<Correction>, RepositoryError> {
        let rows = sqlx::query_as::<_, CorrectionRow>(&format!(
            "SELECT {COLUMNS} FROM corrections
             WHERE transaction_id = $1
             ORDER BY corrected_at ASC, id ASC"
        ))
        .bind(transaction.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("list corrections", error))?;
        rows.into_iter().map(CorrectionRow::into_domain).collect()
    }

    async fn list_for_subject(
        &self,
        subject: LedgerSubjectId,
    ) -> Result<Vec<Correction>, RepositoryError> {
        let rows = sqlx::query_as::<_, CorrectionRow>(
            "SELECT c.id, c.transaction_id, c.field, c.old_value, c.new_value, c.reason,
                    c.actor_id, c.corrected_at
             FROM corrections c
             JOIN transactions t ON t.id = c.transaction_id
             WHERE t.subject_id = $1
             ORDER BY c.corrected_at ASC, c.id ASC",
        )
        .bind(subject.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("list subject corrections", error))?;
        rows.into_iter().map(CorrectionRow::into_domain).collect()
    }

    async fn last_corrected_at(
        &self,
        subject: LedgerSubjectId,
    ) -> Result<Option<DateTime<Utc>>, RepositoryError> {
        sqlx::query_scalar::<_, Option<DateTime<Utc>>>(
            "SELECT max(c.corrected_at)
             FROM corrections c
             JOIN transactions t ON t.id = c.transaction_id
             WHERE t.subject_id = $1",
        )
        .bind(subject.as_uuid())
        .fetch_one(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("read last correction time", error))
    }
}
