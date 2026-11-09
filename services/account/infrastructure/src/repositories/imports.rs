//! Postgres implementation of [`ImportRepository`].

use account_application::ports::{ImportRepository, RepositoryError};
use account_domain::{
    AccountId, ImportBatch, ImportBatchId, ImportBatchStatus, ImportRowError, ImportSource,
    ImportedTransaction, ImportedTransactionId, TransactionKind, UserId,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use super::{invalid_data, map_sqlx_error};

const BATCH_COLUMNS: &str = "id, account_id, status, source, file_name, imported_count, \
     skipped_count, failed_count, created_at, completed_at";

/// sqlx-backed CSV imports.
#[derive(Debug, Clone)]
pub struct PgImportRepository {
    pool: PgPool,
}

impl PgImportRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct BatchRow {
    id: Uuid,
    account_id: Uuid,
    status: String,
    source: String,
    file_name: String,
    imported_count: i32,
    skipped_count: i32,
    failed_count: i32,
    created_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
}

impl BatchRow {
    fn into_domain(self) -> Result<ImportBatch, RepositoryError> {
        Ok(ImportBatch::from_stored(
            ImportBatchId::new(self.id),
            AccountId::new(self.account_id),
            ImportSource::parse(&self.source)
                .map_err(|error| invalid_data("read import batch", error))?,
            self.file_name,
            ImportBatchStatus::parse(&self.status)
                .map_err(|error| invalid_data("read import batch", error))?,
            self.imported_count as u32,
            self.skipped_count as u32,
            self.failed_count as u32,
            self.created_at,
            self.completed_at,
        ))
    }
}

#[derive(sqlx::FromRow)]
struct TransactionRow {
    id: Uuid,
    batch_id: Uuid,
    account_id: Uuid,
    external_id: Option<String>,
    dedupe_key: String,
    asset_id: String,
    kind: String,
    quantity: Decimal,
    price: Option<Decimal>,
    executed_at: DateTime<Utc>,
}

impl TransactionRow {
    fn into_domain(self) -> Result<ImportedTransaction, RepositoryError> {
        Ok(ImportedTransaction::from_stored(
            ImportedTransactionId::new(self.id),
            ImportBatchId::new(self.batch_id),
            AccountId::new(self.account_id),
            self.external_id,
            self.dedupe_key,
            self.asset_id,
            TransactionKind::parse(&self.kind)
                .map_err(|error| invalid_data("read imported transaction", error))?,
            self.quantity,
            self.price,
            self.executed_at,
        ))
    }
}

#[async_trait]
impl ImportRepository for PgImportRepository {
    async fn create_batch(&self, batch: &ImportBatch, bytes: &[u8]) -> Result<(), RepositoryError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| map_sqlx_error("begin import staging", error))?;
        sqlx::query(
            "INSERT INTO import_batches
                 (id, account_id, status, source, file_name, imported_count, skipped_count,
                  failed_count, created_at, completed_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(batch.id().as_uuid())
        .bind(batch.account_id().as_uuid())
        .bind(batch.status().as_str())
        .bind(batch.source().as_str())
        .bind(batch.file_name())
        .bind(i32::try_from(batch.imported_count()).unwrap_or(i32::MAX))
        .bind(i32::try_from(batch.skipped_count()).unwrap_or(i32::MAX))
        .bind(i32::try_from(batch.failed_count()).unwrap_or(i32::MAX))
        .bind(batch.created_at())
        .bind(batch.completed_at())
        .execute(&mut *transaction)
        .await
        .map_err(|error| map_sqlx_error("insert import batch", error))?;
        sqlx::query(
            "INSERT INTO import_upload (batch_id, bytes, byte_size, created_at)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(batch.id().as_uuid())
        .bind(bytes)
        .bind(i32::try_from(bytes.len()).unwrap_or(i32::MAX))
        .bind(batch.created_at())
        .execute(&mut *transaction)
        .await
        .map_err(|error| map_sqlx_error("stage import upload", error))?;
        transaction
            .commit()
            .await
            .map_err(|error| map_sqlx_error("commit import staging", error))
    }

    async fn find_batch(&self, id: ImportBatchId) -> Result<Option<ImportBatch>, RepositoryError> {
        let row = sqlx::query_as::<_, BatchRow>(&format!(
            "SELECT {BATCH_COLUMNS} FROM import_batches WHERE id = $1"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find import batch", error))?;
        row.map(BatchRow::into_domain).transpose()
    }

    async fn update_batch(&self, batch: &ImportBatch) -> Result<(), RepositoryError> {
        let affected = sqlx::query(
            "UPDATE import_batches
             SET status = $2, imported_count = $3, skipped_count = $4, failed_count = $5,
                 completed_at = $6
             WHERE id = $1",
        )
        .bind(batch.id().as_uuid())
        .bind(batch.status().as_str())
        .bind(i32::try_from(batch.imported_count()).unwrap_or(i32::MAX))
        .bind(i32::try_from(batch.skipped_count()).unwrap_or(i32::MAX))
        .bind(i32::try_from(batch.failed_count()).unwrap_or(i32::MAX))
        .bind(batch.completed_at())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("update import batch", error))?
        .rows_affected();
        if affected == 0 {
            return Err(RepositoryError::Conflict(format!(
                "import batch '{}' does not exist",
                batch.id()
            )));
        }
        Ok(())
    }

    async fn claim_pending(&self) -> Result<Option<ImportBatch>, RepositoryError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| map_sqlx_error("begin import claim", error))?;
        let row = sqlx::query_as::<_, BatchRow>(&format!(
            "SELECT {BATCH_COLUMNS} FROM import_batches
             WHERE status = 'pending'
             ORDER BY created_at
             LIMIT 1
             FOR UPDATE SKIP LOCKED"
        ))
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|error| map_sqlx_error("claim pending import", error))?;
        let Some(row) = row else {
            transaction
                .commit()
                .await
                .map_err(|error| map_sqlx_error("commit empty import claim", error))?;
            return Ok(None);
        };
        sqlx::query("UPDATE import_batches SET status = 'processing' WHERE id = $1")
            .bind(row.id)
            .execute(&mut *transaction)
            .await
            .map_err(|error| map_sqlx_error("mark import processing", error))?;
        transaction
            .commit()
            .await
            .map_err(|error| map_sqlx_error("commit import claim", error))?;
        let mut batch = row.into_domain()?;
        batch.begin_processing();
        Ok(Some(batch))
    }

    async fn staged_bytes(&self, id: ImportBatchId) -> Result<Option<Vec<u8>>, RepositoryError> {
        sqlx::query_scalar("SELECT bytes FROM import_upload WHERE batch_id = $1")
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| map_sqlx_error("read staged upload", error))
    }

    async fn delete_staging(&self, id: ImportBatchId) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM import_upload WHERE batch_id = $1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await
            .map_err(|error| map_sqlx_error("delete staged upload", error))?;
        Ok(())
    }

    async fn dedupe_exists(
        &self,
        account_id: AccountId,
        dedupe_key: &str,
    ) -> Result<bool, RepositoryError> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                 SELECT 1 FROM imported_transactions
                 WHERE account_id = $1 AND dedupe_key = $2
             )",
        )
        .bind(account_id.as_uuid())
        .bind(dedupe_key)
        .fetch_one(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("check imported transaction", error))?;
        Ok(exists)
    }

    async fn insert_transactions(
        &self,
        rows: &[ImportedTransaction],
    ) -> Result<u64, RepositoryError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| map_sqlx_error("begin import rows", error))?;
        let mut inserted = 0u64;
        for row in rows {
            inserted += sqlx::query(
                "INSERT INTO imported_transactions
                     (id, batch_id, account_id, external_id, dedupe_key, asset_id, kind, quantity,
                      price, executed_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                 ON CONFLICT (account_id, dedupe_key) DO NOTHING",
            )
            .bind(row.id().as_uuid())
            .bind(row.batch_id().as_uuid())
            .bind(row.account_id().as_uuid())
            .bind(row.external_id())
            .bind(row.dedupe_key())
            .bind(row.asset_id())
            .bind(row.kind().as_str())
            .bind(row.quantity())
            .bind(row.price())
            .bind(row.executed_at())
            .execute(&mut *transaction)
            .await
            .map_err(|error| map_sqlx_error("insert imported transaction", error))?
            .rows_affected();
        }
        transaction
            .commit()
            .await
            .map_err(|error| map_sqlx_error("commit import rows", error))?;
        Ok(inserted)
    }

    async fn insert_row_errors(&self, errors: &[ImportRowError]) -> Result<(), RepositoryError> {
        for error in errors {
            sqlx::query(
                "INSERT INTO import_row_errors (id, batch_id, line_number, reason)
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(error.id().as_uuid())
            .bind(error.batch_id().as_uuid())
            .bind(i32::try_from(error.line_number()).unwrap_or(i32::MAX))
            .bind(error.reason())
            .execute(&self.pool)
            .await
            .map_err(|error| map_sqlx_error("insert import row error", error))?;
        }
        Ok(())
    }

    async fn transactions_for_account(
        &self,
        account_id: AccountId,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<ImportedTransaction>, RepositoryError> {
        let rows = sqlx::query_as::<_, TransactionRow>(
            "SELECT id, batch_id, account_id, external_id, dedupe_key, asset_id, kind, quantity,
                    price, executed_at
             FROM imported_transactions
             WHERE account_id = $1
               AND ($2::timestamptz IS NULL OR executed_at >= $2)
             ORDER BY executed_at, id",
        )
        .bind(account_id.as_uuid())
        .bind(since)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("read account transactions", error))?;
        rows.into_iter().map(TransactionRow::into_domain).collect()
    }

    async fn transactions_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<ImportedTransaction>, RepositoryError> {
        let rows = sqlx::query_as::<_, TransactionRow>(
            "SELECT t.id, t.batch_id, t.account_id, t.external_id, t.dedupe_key, t.asset_id,
                    t.kind, t.quantity, t.price, t.executed_at
             FROM imported_transactions t
             JOIN accounts a ON a.id = t.account_id
             WHERE a.user_id = $1
             ORDER BY t.executed_at, t.id",
        )
        .bind(user_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("read user transactions", error))?;
        rows.into_iter().map(TransactionRow::into_domain).collect()
    }
}
