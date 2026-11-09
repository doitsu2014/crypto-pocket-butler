//! Postgres implementation of [`TierRepository`].

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use market_data_application::ports::{RepositoryError, TierRepository};
use market_data_domain::{AssetPriceTier, PriceTier};
use sqlx::PgPool;

use super::{invalid_data, map_sqlx_error};

/// sqlx-backed asset tiers.
#[derive(Debug, Clone)]
pub struct PgTierRepository {
    pool: PgPool,
}

impl PgTierRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct TierRow {
    asset_id: String,
    rank: i32,
    tier: String,
    bound_seconds: i32,
    updated_at: DateTime<Utc>,
}

impl TierRow {
    fn into_domain(self) -> Result<AssetPriceTier, RepositoryError> {
        AssetPriceTier::from_stored(
            self.asset_id,
            u32::try_from(self.rank).map_err(|error| invalid_data("read asset tier", error))?,
            PriceTier::parse(&self.tier).map_err(|error| invalid_data("read asset tier", error))?,
            i64::from(self.bound_seconds),
            self.updated_at,
        )
        .map_err(|error| invalid_data("read asset tier", error))
    }
}

#[async_trait]
impl TierRepository for PgTierRepository {
    async fn upsert(&self, tier: &AssetPriceTier) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO asset_price_tiers (asset_id, rank, tier, bound_seconds, updated_at)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (asset_id) DO UPDATE
             SET rank = EXCLUDED.rank,
                 tier = EXCLUDED.tier,
                 bound_seconds = EXCLUDED.bound_seconds,
                 updated_at = EXCLUDED.updated_at",
        )
        .bind(tier.asset_id())
        .bind(i32::try_from(tier.rank()).unwrap_or(i32::MAX))
        .bind(tier.tier().as_str())
        .bind(i32::try_from(tier.bound_seconds()).unwrap_or(i32::MAX))
        .bind(tier.updated_at())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("upsert asset tier", error))?;
        Ok(())
    }

    async fn find(&self, asset_id: &str) -> Result<Option<AssetPriceTier>, RepositoryError> {
        let row = sqlx::query_as::<_, TierRow>(
            "SELECT asset_id, rank, tier, bound_seconds, updated_at
             FROM asset_price_tiers WHERE asset_id = $1",
        )
        .bind(asset_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("read asset tier", error))?;
        row.map(TierRow::into_domain).transpose()
    }

    async fn bounds_for(
        &self,
        asset_ids: &[String],
    ) -> Result<Vec<(String, i64)>, RepositoryError> {
        let rows = sqlx::query_as::<_, (String, i32)>(
            "SELECT asset_id, bound_seconds FROM asset_price_tiers WHERE asset_id = ANY($1)",
        )
        .bind(asset_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("read asset tier bounds", error))?;
        Ok(rows
            .into_iter()
            .map(|(asset_id, bound_seconds)| (asset_id, i64::from(bound_seconds)))
            .collect())
    }
}
