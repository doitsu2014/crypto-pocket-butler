//! Postgres implementation of [`ObservationRepository`].
//!
//! The read path takes the newest accepted observation per asset with
//! `DISTINCT ON (asset_id) ... ORDER BY asset_id, observed_at DESC`, as the
//! performance design specifies.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use market_data_application::ports::{ObservationRepository, RepositoryError};
use market_data_domain::{ObservationId, ObservationStatus, PriceObservation};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use super::{invalid_data, map_sqlx_error};

const COLUMNS: &str = "id, asset_id, source_name, price, currency, observed_at, received_at, \
     status, rejection_reason";

/// sqlx-backed price observations.
#[derive(Debug, Clone)]
pub struct PgObservationRepository {
    pool: PgPool,
}

impl PgObservationRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct ObservationRow {
    id: Uuid,
    asset_id: String,
    source_name: String,
    price: Decimal,
    currency: String,
    observed_at: DateTime<Utc>,
    received_at: DateTime<Utc>,
    status: String,
    rejection_reason: Option<String>,
}

impl ObservationRow {
    fn into_domain(self) -> Result<PriceObservation, RepositoryError> {
        PriceObservation::from_stored(
            ObservationId::new(self.id),
            self.asset_id,
            self.source_name,
            self.price,
            self.currency,
            self.observed_at,
            self.received_at,
            ObservationStatus::parse(&self.status)
                .map_err(|error| invalid_data("read observation", error))?,
            self.rejection_reason,
        )
        .map_err(|error| invalid_data("read observation", error))
    }
}

#[async_trait]
impl ObservationRepository for PgObservationRepository {
    async fn insert(&self, observation: &PriceObservation) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO price_observations
                 (id, asset_id, source_name, price, currency, observed_at, received_at, status,
                  rejection_reason)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(observation.id().as_uuid())
        .bind(observation.asset_id())
        .bind(observation.source_name())
        .bind(observation.price())
        .bind(observation.currency())
        .bind(observation.observed_at())
        .bind(observation.received_at())
        .bind(observation.status().as_str())
        .bind(observation.rejection_reason())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("insert observation", error))?;
        Ok(())
    }

    async fn latest_accepted_for_source(
        &self,
        source_name: &str,
        asset_ids: &[String],
    ) -> Result<Vec<PriceObservation>, RepositoryError> {
        let rows = sqlx::query_as::<_, ObservationRow>(&format!(
            "SELECT DISTINCT ON (asset_id) {COLUMNS}
             FROM price_observations
             WHERE source_name = $1 AND status = 'accepted' AND asset_id = ANY($2)
             ORDER BY asset_id, observed_at DESC"
        ))
        .bind(source_name)
        .bind(asset_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("read newest observation per source", error))?;
        rows.into_iter().map(ObservationRow::into_domain).collect()
    }

    async fn latest_accepted_for_assets(
        &self,
        asset_ids: &[String],
    ) -> Result<Vec<PriceObservation>, RepositoryError> {
        let rows = sqlx::query_as::<_, ObservationRow>(&format!(
            "SELECT DISTINCT ON (asset_id) {COLUMNS}
             FROM price_observations
             WHERE status = 'accepted' AND asset_id = ANY($1)
             ORDER BY asset_id, observed_at DESC"
        ))
        .bind(asset_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("read newest observation per asset", error))?;
        rows.into_iter().map(ObservationRow::into_domain).collect()
    }

    async fn prune_before(&self, cutoff: DateTime<Utc>) -> Result<u64, RepositoryError> {
        let removed = sqlx::query(
            "DELETE FROM price_observations target
             WHERE target.received_at < $1
               AND EXISTS (
                   SELECT 1 FROM price_observations newer
                   WHERE newer.asset_id = target.asset_id
                     AND newer.source_name = target.source_name
                     AND (newer.observed_at > target.observed_at
                          OR (newer.observed_at = target.observed_at AND newer.id > target.id))
               )",
        )
        .bind(cutoff)
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("prune observations", error))?
        .rows_affected();
        Ok(removed)
    }
}
