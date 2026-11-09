//! The recorded-trade repository. A trade and its overlay are written in one
//! transaction.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use portfolio_application::ports::{RepositoryError, TradeRepository};
use portfolio_domain::{
    OrderSide, PortfolioId, PriceSource, RecordedTrade, RecordedTradeId, TradeOverlay, UserId,
};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use super::{map_error, UNIQUE_VIOLATION};

/// sqlx-backed recorded trades.
#[derive(Debug, Clone)]
pub struct PgTradeRepository {
    pool: PgPool,
}

impl PgTradeRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct TradeRow {
    id: Uuid,
    portfolio_id: Uuid,
    user_id: Uuid,
    asset_id: String,
    side: String,
    quantity: Decimal,
    price: Decimal,
    price_source: String,
    executed_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    idempotency_key: String,
    ledger_transaction_id: Option<String>,
}

fn parse_trade(row: TradeRow) -> Result<RecordedTrade, RepositoryError> {
    let side = OrderSide::parse(&row.side)
        .map_err(|error| RepositoryError::InvalidData(error.to_string()))?;
    let price_source = PriceSource::parse(&row.price_source)
        .map_err(|error| RepositoryError::InvalidData(error.to_string()))?;
    Ok(RecordedTrade::from_stored(
        RecordedTradeId::new(row.id),
        PortfolioId::new(row.portfolio_id),
        UserId::new(row.user_id),
        row.asset_id,
        side,
        row.quantity,
        row.price,
        price_source,
        row.executed_at,
        row.created_at,
        row.idempotency_key,
        row.ledger_transaction_id,
    ))
}

const SELECT: &str = "SELECT id, portfolio_id, user_id, asset_id, side, quantity, price, \
     price_source, executed_at, created_at, idempotency_key, ledger_transaction_id \
     FROM recorded_trades";

#[async_trait]
impl TradeRepository for PgTradeRepository {
    async fn insert_with_overlay(
        &self,
        trade: &RecordedTrade,
        overlay: &TradeOverlay,
    ) -> Result<(), RepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(map_error)?;
        let inserted = sqlx::query(
            "INSERT INTO recorded_trades
                (id, portfolio_id, user_id, asset_id, side, quantity, price, price_source,
                 executed_at, created_at, idempotency_key, ledger_transaction_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(trade.id().as_uuid())
        .bind(trade.portfolio_id().as_uuid())
        .bind(trade.user_id().as_uuid())
        .bind(trade.asset_id())
        .bind(trade.side().as_str())
        .bind(trade.quantity())
        .bind(trade.price())
        .bind(trade.price_source().as_str())
        .bind(trade.executed_at())
        .bind(trade.created_at())
        .bind(trade.idempotency_key())
        .bind(trade.ledger_transaction_id())
        .execute(&mut *transaction)
        .await;
        if let Err(error) = inserted {
            return Err(match &error {
                sqlx::Error::Database(database)
                    if database.code().as_deref() == Some(UNIQUE_VIOLATION) =>
                {
                    RepositoryError::Conflict("the idempotency key is already used".to_string())
                }
                _ => map_error(error),
            });
        }
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
        .execute(&mut *transaction)
        .await
        .map_err(map_error)?;
        transaction.commit().await.map_err(map_error)
    }

    async fn find_by_idempotency_key(
        &self,
        portfolio_id: PortfolioId,
        idempotency_key: &str,
    ) -> Result<Option<RecordedTrade>, RepositoryError> {
        let row = sqlx::query_as::<_, TradeRow>(&format!(
            "{SELECT} WHERE portfolio_id = $1 AND idempotency_key = $2"
        ))
        .bind(portfolio_id.as_uuid())
        .bind(idempotency_key)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_error)?;
        row.map(parse_trade).transpose()
    }

    async fn find_by_id(
        &self,
        id: RecordedTradeId,
    ) -> Result<Option<RecordedTrade>, RepositoryError> {
        let row = sqlx::query_as::<_, TradeRow>(&format!("{SELECT} WHERE id = $1"))
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_error)?;
        row.map(parse_trade).transpose()
    }

    async fn list_for_portfolio(
        &self,
        portfolio_id: PortfolioId,
    ) -> Result<Vec<RecordedTrade>, RepositoryError> {
        let rows = sqlx::query_as::<_, TradeRow>(&format!(
            "{SELECT} WHERE portfolio_id = $1 ORDER BY created_at DESC"
        ))
        .bind(portfolio_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(map_error)?;
        rows.into_iter().map(parse_trade).collect()
    }

    async fn list_for_user(&self, user_id: UserId) -> Result<Vec<RecordedTrade>, RepositoryError> {
        let rows = sqlx::query_as::<_, TradeRow>(&format!(
            "{SELECT} WHERE portfolio_id IN (SELECT id FROM portfolios WHERE user_id = $1)
             ORDER BY created_at DESC"
        ))
        .bind(user_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(map_error)?;
        rows.into_iter().map(parse_trade).collect()
    }

    async fn anonymise_for_user(
        &self,
        user_id: UserId,
        anonymised_ref: UserId,
    ) -> Result<u64, RepositoryError> {
        let result = sqlx::query("UPDATE recorded_trades SET user_id = $2 WHERE user_id = $1")
            .bind(user_id.as_uuid())
            .bind(anonymised_ref.as_uuid())
            .execute(&self.pool)
            .await
            .map_err(map_error)?;
        Ok(result.rows_affected())
    }
}
