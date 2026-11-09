//! The order-intent repository.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use portfolio_application::ports::{IntentRepository, RepositoryError};
use portfolio_domain::{IntentStatus, OrderIntent, OrderIntentId, OrderSide, PortfolioId, UserId};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use super::map_error;

/// sqlx-backed order intents.
#[derive(Debug, Clone)]
pub struct PgIntentRepository {
    pool: PgPool,
}

impl PgIntentRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct IntentRow {
    id: Uuid,
    portfolio_id: Uuid,
    asset_id: String,
    side: String,
    quantity: Decimal,
    projected_allocation: Decimal,
    projected_drift_points: Option<Decimal>,
    shortfall: Option<Decimal>,
    status: String,
    created_at: DateTime<Utc>,
    cancelled_at: Option<DateTime<Utc>>,
}

fn parse_intent(row: IntentRow) -> Result<OrderIntent, RepositoryError> {
    let side = OrderSide::parse(&row.side)
        .map_err(|error| RepositoryError::InvalidData(error.to_string()))?;
    let status = match row.status.as_str() {
        "pending" => IntentStatus::Pending,
        "cancelled" => IntentStatus::Cancelled,
        other => {
            return Err(RepositoryError::InvalidData(format!(
                "stored intent status {other} is unknown"
            )))
        }
    };
    Ok(OrderIntent::from_stored(
        OrderIntentId::new(row.id),
        PortfolioId::new(row.portfolio_id),
        row.asset_id,
        side,
        row.quantity,
        row.projected_allocation,
        row.projected_drift_points,
        row.shortfall,
        status,
        row.created_at,
        row.cancelled_at,
    ))
}

const SELECT: &str = "SELECT id, portfolio_id, asset_id, side, quantity, projected_allocation, \
     projected_drift_points, shortfall, status, created_at, cancelled_at FROM order_intents";

#[async_trait]
impl IntentRepository for PgIntentRepository {
    async fn insert(&self, intent: &OrderIntent) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO order_intents
                (id, portfolio_id, asset_id, side, quantity, projected_allocation,
                 projected_drift_points, shortfall, status, created_at, cancelled_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(intent.id().as_uuid())
        .bind(intent.portfolio_id().as_uuid())
        .bind(intent.asset_id())
        .bind(intent.side().as_str())
        .bind(intent.quantity())
        .bind(intent.projected_allocation())
        .bind(intent.projected_drift_points())
        .bind(intent.shortfall())
        .bind(intent.status().as_str())
        .bind(intent.created_at())
        .bind(intent.cancelled_at())
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(map_error)
    }

    async fn find_by_id(&self, id: OrderIntentId) -> Result<Option<OrderIntent>, RepositoryError> {
        let row = sqlx::query_as::<_, IntentRow>(&format!("{SELECT} WHERE id = $1"))
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_error)?;
        row.map(parse_intent).transpose()
    }

    async fn list_for_portfolio(
        &self,
        portfolio_id: PortfolioId,
    ) -> Result<Vec<OrderIntent>, RepositoryError> {
        let rows = sqlx::query_as::<_, IntentRow>(&format!(
            "{SELECT} WHERE portfolio_id = $1 ORDER BY created_at DESC"
        ))
        .bind(portfolio_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(map_error)?;
        rows.into_iter().map(parse_intent).collect()
    }

    async fn update(&self, intent: &OrderIntent) -> Result<(), RepositoryError> {
        sqlx::query(
            "UPDATE order_intents
             SET status = $2, cancelled_at = $3, projected_allocation = $4,
                 projected_drift_points = $5, shortfall = $6
             WHERE id = $1",
        )
        .bind(intent.id().as_uuid())
        .bind(intent.status().as_str())
        .bind(intent.cancelled_at())
        .bind(intent.projected_allocation())
        .bind(intent.projected_drift_points())
        .bind(intent.shortfall())
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(map_error)
    }

    async fn list_for_user(&self, user_id: UserId) -> Result<Vec<OrderIntent>, RepositoryError> {
        let rows = sqlx::query_as::<_, IntentRow>(&format!(
            "{SELECT} WHERE portfolio_id IN (SELECT id FROM portfolios WHERE user_id = $1)
             ORDER BY created_at DESC"
        ))
        .bind(user_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(map_error)?;
        rows.into_iter().map(parse_intent).collect()
    }
}
