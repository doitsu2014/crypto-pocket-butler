//! Postgres implementation of [`TransactionRepository`].

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ledger_application::ports::{RepositoryError, TransactionCursor, TransactionRepository};
use ledger_domain::{Kind, LedgerSubjectId, PriceSource, Source, Transaction, TransactionId};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use super::{invalid_data, map_sqlx_error};

const COLUMNS: &str = "id, subject_id, account_id, asset_id, kind, quantity, price, price_source, \
     currency, source, external_ref, canonical_key, external_key, executed_at, recorded_at";

/// sqlx-backed append-only transaction store.
#[derive(Debug, Clone)]
pub struct PgTransactionRepository {
    pool: PgPool,
}

impl PgTransactionRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct TransactionRow {
    id: Uuid,
    subject_id: Uuid,
    account_id: Option<Uuid>,
    asset_id: String,
    kind: String,
    quantity: Decimal,
    price: Option<Decimal>,
    price_source: Option<String>,
    currency: String,
    source: String,
    external_ref: Option<String>,
    canonical_key: String,
    external_key: Option<String>,
    executed_at: DateTime<Utc>,
    recorded_at: DateTime<Utc>,
}

impl TransactionRow {
    fn into_domain(self) -> Result<Transaction, RepositoryError> {
        let kind = Kind::parse(&self.kind)
            .map_err(|error| invalid_data("read transaction kind", error))?;
        let source = Source::parse(&self.source)
            .map_err(|error| invalid_data("read transaction source", error))?;
        let price_source = self
            .price_source
            .as_deref()
            .map(PriceSource::parse)
            .transpose()
            .map_err(|error| invalid_data("read transaction price source", error))?;
        Ok(Transaction::from_stored(
            TransactionId::new(self.id),
            LedgerSubjectId::new(self.subject_id),
            self.account_id,
            self.asset_id,
            kind,
            self.quantity,
            self.price,
            price_source,
            self.currency,
            source,
            self.external_ref,
            self.canonical_key,
            self.external_key,
            self.executed_at,
            self.recorded_at,
        ))
    }
}

#[async_trait]
impl TransactionRepository for PgTransactionRepository {
    async fn insert(&self, transaction: &Transaction) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO transactions
                 (id, subject_id, account_id, asset_id, kind, quantity, price, price_source,
                  currency, source, external_ref, canonical_key, external_key, executed_at,
                  recorded_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
        )
        .bind(transaction.id().as_uuid())
        .bind(transaction.subject_id().as_uuid())
        .bind(transaction.account_id())
        .bind(transaction.asset_id())
        .bind(transaction.kind().as_str())
        .bind(transaction.quantity())
        .bind(transaction.price())
        .bind(transaction.price_source().map(PriceSource::as_str))
        .bind(transaction.currency())
        .bind(transaction.source().as_str())
        .bind(transaction.external_ref())
        .bind(transaction.canonical_key())
        .bind(transaction.external_key())
        .bind(transaction.executed_at())
        .bind(transaction.recorded_at())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("insert transaction", error))?;
        Ok(())
    }

    async fn find_by_id(&self, id: TransactionId) -> Result<Option<Transaction>, RepositoryError> {
        let row = sqlx::query_as::<_, TransactionRow>(&format!(
            "SELECT {COLUMNS} FROM transactions WHERE id = $1"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("read transaction", error))?;
        row.map(TransactionRow::into_domain).transpose()
    }

    async fn canonical_exists(&self, key: &str) -> Result<bool, RepositoryError> {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM transactions WHERE canonical_key = $1)",
        )
        .bind(key)
        .fetch_one(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("check canonical key", error))
    }

    async fn external_exists(&self, key: &str) -> Result<bool, RepositoryError> {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM transactions WHERE external_key = $1)",
        )
        .bind(key)
        .fetch_one(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("check external key", error))
    }

    async fn list_page(
        &self,
        subject: LedgerSubjectId,
        after: Option<TransactionCursor>,
        limit: u32,
    ) -> Result<Vec<Transaction>, RepositoryError> {
        let rows = match after {
            Some(cursor) => {
                sqlx::query_as::<_, TransactionRow>(&format!(
                    "SELECT {COLUMNS} FROM transactions
                     WHERE subject_id = $1
                       AND (executed_at < $2 OR (executed_at = $2 AND id < $3))
                     ORDER BY executed_at DESC, id DESC
                     LIMIT $4"
                ))
                .bind(subject.as_uuid())
                .bind(cursor.executed_at)
                .bind(cursor.id.as_uuid())
                .bind(i64::from(limit))
                .fetch_all(&self.pool)
                .await
            }
            None => {
                sqlx::query_as::<_, TransactionRow>(&format!(
                    "SELECT {COLUMNS} FROM transactions
                     WHERE subject_id = $1
                     ORDER BY executed_at DESC, id DESC
                     LIMIT $2"
                ))
                .bind(subject.as_uuid())
                .bind(i64::from(limit))
                .fetch_all(&self.pool)
                .await
            }
        }
        .map_err(|error| map_sqlx_error("list transactions", error))?;
        rows.into_iter().map(TransactionRow::into_domain).collect()
    }

    async fn list_all(
        &self,
        subject: LedgerSubjectId,
    ) -> Result<Vec<Transaction>, RepositoryError> {
        let rows = sqlx::query_as::<_, TransactionRow>(&format!(
            "SELECT {COLUMNS} FROM transactions
             WHERE subject_id = $1
             ORDER BY executed_at ASC, id ASC"
        ))
        .bind(subject.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("list all transactions", error))?;
        rows.into_iter().map(TransactionRow::into_domain).collect()
    }

    async fn last_recorded_at(
        &self,
        subject: LedgerSubjectId,
    ) -> Result<Option<DateTime<Utc>>, RepositoryError> {
        sqlx::query_scalar::<_, Option<DateTime<Utc>>>(
            "SELECT max(recorded_at) FROM transactions WHERE subject_id = $1",
        )
        .bind(subject.as_uuid())
        .fetch_one(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("read last transaction time", error))
    }

    async fn list_subjects(&self) -> Result<Vec<LedgerSubjectId>, RepositoryError> {
        let ids = sqlx::query_scalar::<_, Uuid>(
            "SELECT DISTINCT subject_id FROM transactions ORDER BY subject_id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("list transaction subjects", error))?;
        Ok(ids.into_iter().map(LedgerSubjectId::new).collect())
    }

    async fn count_for_subject(&self, subject: LedgerSubjectId) -> Result<u64, RepositoryError> {
        let count =
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM transactions WHERE subject_id = $1")
                .bind(subject.as_uuid())
                .fetch_one(&self.pool)
                .await
                .map_err(|error| map_sqlx_error("count transactions", error))?;
        Ok(u64::try_from(count).unwrap_or(0))
    }
}
