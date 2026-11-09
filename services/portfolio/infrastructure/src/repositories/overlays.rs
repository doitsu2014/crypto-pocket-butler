//! The trade-overlay repository.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use portfolio_application::ports::{OverlayRepository, RepositoryError};
use portfolio_domain::{
    AccountId, PortfolioId, RecordedTradeId, TradeOverlay, TradeOverlayId, UserId,
};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use super::map_error;

/// sqlx-backed trade overlays.
#[derive(Debug, Clone)]
pub struct PgOverlayRepository {
    pool: PgPool,
}

impl PgOverlayRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct OverlayRow {
    id: Uuid,
    recorded_trade_id: Uuid,
    portfolio_id: Uuid,
    account_id: Option<Uuid>,
    asset_id: String,
    quantity_delta: Decimal,
    applied_at: DateTime<Utc>,
}

impl From<OverlayRow> for TradeOverlay {
    fn from(row: OverlayRow) -> Self {
        TradeOverlay::from_stored(
            TradeOverlayId::new(row.id),
            RecordedTradeId::new(row.recorded_trade_id),
            PortfolioId::new(row.portfolio_id),
            row.account_id.map(AccountId::new),
            row.asset_id,
            row.quantity_delta,
            row.applied_at,
        )
    }
}

const SELECT: &str = "SELECT id, recorded_trade_id, portfolio_id, account_id, asset_id, \
     quantity_delta, applied_at FROM trade_overlays";

#[async_trait]
impl OverlayRepository for PgOverlayRepository {
    async fn insert(&self, overlay: &TradeOverlay) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO trade_overlays
                (id, recorded_trade_id, portfolio_id, account_id, asset_id,
                 quantity_delta, applied_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(overlay.id().as_uuid())
        .bind(overlay.recorded_trade_id().as_uuid())
        .bind(overlay.portfolio_id().as_uuid())
        .bind(overlay.account_id().map(|id| id.as_uuid()))
        .bind(overlay.asset_id())
        .bind(overlay.quantity_delta())
        .bind(overlay.applied_at())
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(map_error)
    }

    async fn list_for_portfolio(
        &self,
        portfolio_id: PortfolioId,
    ) -> Result<Vec<TradeOverlay>, RepositoryError> {
        let rows = sqlx::query_as::<_, OverlayRow>(&format!(
            "{SELECT} WHERE portfolio_id = $1 ORDER BY applied_at"
        ))
        .bind(portfolio_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(map_error)?;
        Ok(rows.into_iter().map(TradeOverlay::from).collect())
    }

    async fn delete_for_recorded_trade(
        &self,
        recorded_trade_id: RecordedTradeId,
    ) -> Result<u64, RepositoryError> {
        let result = sqlx::query("DELETE FROM trade_overlays WHERE recorded_trade_id = $1")
            .bind(recorded_trade_id.as_uuid())
            .execute(&self.pool)
            .await
            .map_err(map_error)?;
        Ok(result.rows_affected())
    }

    async fn expire_before(&self, cutoff: DateTime<Utc>) -> Result<u64, RepositoryError> {
        let result = sqlx::query("DELETE FROM trade_overlays WHERE applied_at < $1")
            .bind(cutoff)
            .execute(&self.pool)
            .await
            .map_err(map_error)?;
        Ok(result.rows_affected())
    }

    async fn list_for_user(&self, user_id: UserId) -> Result<Vec<TradeOverlay>, RepositoryError> {
        let rows = sqlx::query_as::<_, OverlayRow>(&format!(
            "{SELECT} WHERE portfolio_id IN (SELECT id FROM portfolios WHERE user_id = $1)
             ORDER BY applied_at"
        ))
        .bind(user_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(map_error)?;
        Ok(rows.into_iter().map(TradeOverlay::from).collect())
    }
}
