//! Postgres implementation of [`FxRepository`].

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use market_data_application::ports::{FxRepository, RepositoryError};
use market_data_domain::{FxRate, FxRateId};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use super::{invalid_data, map_sqlx_error};

const COLUMNS: &str = "id, base, quote, rate, source, observed_at, received_at";

/// sqlx-backed FX readings.
#[derive(Debug, Clone)]
pub struct PgFxRepository {
    pool: PgPool,
}

impl PgFxRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct FxRow {
    id: Uuid,
    base: String,
    quote: String,
    rate: Decimal,
    source: String,
    observed_at: DateTime<Utc>,
    received_at: DateTime<Utc>,
}

impl FxRow {
    fn into_domain(self) -> Result<FxRate, RepositoryError> {
        FxRate::from_stored(
            FxRateId::new(self.id),
            self.base,
            self.quote,
            self.rate,
            self.source,
            self.observed_at,
            self.received_at,
        )
        .map_err(|error| invalid_data("read fx rate", error))
    }
}

#[async_trait]
impl FxRepository for PgFxRepository {
    async fn insert(&self, rate: &FxRate) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO fx_rates
                 (id, base, quote, rate, source, observed_at, received_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(rate.id().as_uuid())
        .bind(rate.base())
        .bind(rate.quote())
        .bind(rate.rate())
        .bind(rate.source())
        .bind(rate.observed_at())
        .bind(rate.received_at())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("insert fx rate", error))?;
        Ok(())
    }

    async fn latest(&self, base: &str, quote: &str) -> Result<Option<FxRate>, RepositoryError> {
        let row = sqlx::query_as::<_, FxRow>(&format!(
            "SELECT {COLUMNS} FROM fx_rates
             WHERE base = $1 AND quote = $2
             ORDER BY observed_at DESC LIMIT 1"
        ))
        .bind(base)
        .bind(quote)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("read fx rate", error))?;
        row.map(FxRow::into_domain).transpose()
    }

    async fn prune_before(&self, cutoff: DateTime<Utc>) -> Result<u64, RepositoryError> {
        let removed = sqlx::query(
            "DELETE FROM fx_rates target
             WHERE target.received_at < $1
               AND EXISTS (
                   SELECT 1 FROM fx_rates newer
                   WHERE newer.base = target.base
                     AND newer.quote = target.quote
                     AND (newer.observed_at > target.observed_at
                          OR (newer.observed_at = target.observed_at AND newer.id > target.id))
               )",
        )
        .bind(cutoff)
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("prune fx rates", error))?
        .rows_affected();
        Ok(removed)
    }
}
