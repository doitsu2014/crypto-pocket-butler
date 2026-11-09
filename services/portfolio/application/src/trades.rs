//! WF6: record a confirmed trade (BR6.1–BR6.2). Ledger C7 records it with a
//! client idempotency key, a C10 audit event is written out of band, and a
//! TradeOverlay makes the allocation and drift reflect the trade immediately.

use std::sync::Arc;

use portfolio_domain::{
    AuditOutcome, OrderSide, Portfolio, PortfolioId, PriceSource, RecordedTrade, RecordedTradeId,
    TradeOverlay, TradeOverlayId, UserId,
};
use rust_decimal::Decimal;

use crate::common::{AppError, Caller, FieldError};
use crate::ports::{
    Clock, IdGenerator, LedgerClient, LedgerTradeCommand, PortfolioRepository, TradeRepository,
};
use crate::trail::Trail;

/// A trade the user confirms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeRequest {
    /// The asset.
    pub asset_id: String,
    /// The side.
    pub side: OrderSide,
    /// The quantity.
    pub quantity: Decimal,
    /// The unit price, supplied by the user when the venue has none.
    pub price: Option<Decimal>,
    /// Where the price came from.
    pub price_source: PriceSource,
    /// When the trade executed.
    pub executed_at: chrono::DateTime<chrono::Utc>,
    /// The client idempotency key.
    pub idempotency_key: String,
}

/// Records and lists confirmed trades.
pub struct TradeService {
    portfolios: Arc<dyn PortfolioRepository>,
    trades: Arc<dyn TradeRepository>,
    ledger: Arc<dyn LedgerClient>,
    trail: Arc<Trail>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
}

impl TradeService {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        portfolios: Arc<dyn PortfolioRepository>,
        trades: Arc<dyn TradeRepository>,
        ledger: Arc<dyn LedgerClient>,
        trail: Arc<Trail>,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            portfolios,
            trades,
            ledger,
            trail,
            clock,
            ids,
        }
    }

    /// Records a confirmed trade. A replay with the same idempotency key returns
    /// the existing trade without calling Ledger again.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for another user's portfolio, a
    /// validation refusal for a missing price or bad quantity, and
    /// [`AppError::Unavailable`] when Ledger refuses so no overlay is written.
    pub async fn record(
        &self,
        caller: &Caller,
        portfolio_id: PortfolioId,
        request: &TradeRequest,
    ) -> Result<RecordedTrade, AppError> {
        let portfolio = self.owned(caller, portfolio_id).await?;
        let key = request.idempotency_key.trim();
        if key.is_empty() {
            return Err(AppError::Validation(vec![FieldError::new(
                "idempotencyKey",
                "is required",
            )]));
        }
        if let Some(existing) = self
            .trades
            .find_by_idempotency_key(portfolio.id(), key)
            .await
            .map_err(AppError::from_repository)?
        {
            return Ok(existing);
        }
        let price = request.price.ok_or_else(|| {
            AppError::Validation(vec![FieldError::new(
                "price",
                "is required; a recorded trade without a venue price must have one supplied by the user",
            )])
        })?;
        let now = self.clock.now();
        let trade = RecordedTrade::new(
            RecordedTradeId::new(self.ids.next()),
            portfolio.id(),
            caller.user_id,
            &request.asset_id,
            request.side,
            request.quantity,
            price,
            request.price_source,
            request.executed_at,
            now,
            key,
        )
        .map_err(AppError::from_domain)?;

        let command = LedgerTradeCommand {
            asset_id: trade.asset_id().to_string(),
            side: trade.side(),
            quantity: trade.quantity(),
            price: trade.price(),
            price_source: trade.price_source(),
            currency: Some(portfolio.base_currency().to_string()),
            executed_at: trade.executed_at(),
        };
        let transaction_id = self
            .ledger
            .record_trade(caller.user_id, &command, key)
            .await
            .map_err(AppError::from_external)?;
        let trade = trade.with_ledger_transaction(Some(transaction_id.clone()));

        let overlay = TradeOverlay::new(
            TradeOverlayId::new(self.ids.next()),
            trade.id(),
            portfolio.id(),
            None,
            trade.asset_id(),
            trade.quantity_delta(),
            now,
        )
        .map_err(AppError::from_domain)?;
        self.trades
            .insert_with_overlay(&trade, &overlay)
            .await
            .map_err(AppError::from_repository)?;

        // NFR1.5: the audit write is out of band so the 900 ms trade budget is
        // untouched. A persistent failure is logged; it never fails the trade.
        let trail = Arc::clone(&self.trail);
        let actor = caller.user_id;
        let correlation = caller.correlation_id.clone();
        let target = trade.id().to_string();
        tokio::spawn(async move {
            if let Err(error) = trail
                .record_action(
                    Some(actor),
                    "portfolio.trade.recorded",
                    target,
                    AuditOutcome::Success,
                    &correlation,
                    "BR6.1",
                )
                .await
            {
                tracing::error!(%error, "the recorded-trade audit was abandoned after retries");
            }
        });

        Ok(trade)
    }

    /// Lists the portfolio's recorded trades, newest first.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for another user's portfolio and a
    /// repository refusal when the read fails.
    pub async fn list(
        &self,
        caller: &Caller,
        portfolio_id: PortfolioId,
    ) -> Result<Vec<RecordedTrade>, AppError> {
        let portfolio = self.owned(caller, portfolio_id).await?;
        self.trades
            .list_for_portfolio(portfolio.id())
            .await
            .map_err(AppError::from_repository)
    }

    /// Lists every recorded trade of the user, for privacy export.
    ///
    /// # Errors
    /// Returns a repository refusal when the read fails.
    pub async fn for_user(&self, user_id: UserId) -> Result<Vec<RecordedTrade>, AppError> {
        self.trades
            .list_for_user(user_id)
            .await
            .map_err(AppError::from_repository)
    }

    async fn owned(
        &self,
        caller: &Caller,
        portfolio_id: PortfolioId,
    ) -> Result<Portfolio, AppError> {
        let portfolio = self
            .portfolios
            .find_by_id(portfolio_id)
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("portfolio"))?;
        caller.require_owner(portfolio.user_id())?;
        Ok(portfolio)
    }
}
