//! WF5: advisory order intents (BR5.1–BR5.3). A preview projects allocation,
//! drift, and any shortfall; recording stores a pending intent and makes no
//! external order call.

use std::sync::Arc;

use portfolio_domain::{
    project_intent, AuditOutcome, IntentProjection, OrderIntent, OrderIntentId, OrderSide,
    Portfolio, PortfolioId, Valuation,
};
use rust_decimal::Decimal;

use crate::common::{AppError, Caller};
use crate::ports::{Clock, IdGenerator, IntentRepository, MarketDataClient, PortfolioRepository};
use crate::trail::Trail;
use crate::valuation::ValuationService;

/// A requested buy or sell intent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentRequest {
    /// The asset.
    pub asset_id: String,
    /// The side.
    pub side: OrderSide,
    /// The quantity.
    pub quantity: Decimal,
}

/// Previews, records, cancels, and lists order intents.
pub struct IntentService {
    portfolios: Arc<dyn PortfolioRepository>,
    intents: Arc<dyn IntentRepository>,
    market: Arc<dyn MarketDataClient>,
    valuation: Arc<ValuationService>,
    trail: Arc<Trail>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
}

impl IntentService {
    /// Creates the use case.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        portfolios: Arc<dyn PortfolioRepository>,
        intents: Arc<dyn IntentRepository>,
        market: Arc<dyn MarketDataClient>,
        valuation: Arc<ValuationService>,
        trail: Arc<Trail>,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            portfolios,
            intents,
            market,
            valuation,
            trail,
            clock,
            ids,
        }
    }

    /// Projects the post-trade allocation, drift, and shortfall without
    /// recording (BR5.2).
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for another user's portfolio and a
    /// validation refusal when the portfolio or asset has no usable price.
    pub async fn preview(
        &self,
        caller: &Caller,
        portfolio_id: PortfolioId,
        request: &IntentRequest,
    ) -> Result<IntentProjection, AppError> {
        let portfolio = self.owned(caller, portfolio_id).await?;
        self.project(&portfolio, request).await
    }

    /// Records a pending intent. No external order call is made (BR5.1).
    ///
    /// # Errors
    /// Returns the preview's refusals and a repository refusal when the write
    /// fails.
    pub async fn create(
        &self,
        caller: &Caller,
        portfolio_id: PortfolioId,
        request: &IntentRequest,
    ) -> Result<OrderIntent, AppError> {
        let portfolio = self.owned(caller, portfolio_id).await?;
        let projection = self.project(&portfolio, request).await?;
        let now = self.clock.now();
        let intent = OrderIntent::new(
            OrderIntentId::new(self.ids.next()),
            portfolio.id(),
            &request.asset_id,
            request.side,
            request.quantity,
            &projection,
            now,
        )
        .map_err(AppError::from_domain)?;
        self.intents
            .insert(&intent)
            .await
            .map_err(AppError::from_repository)?;
        Ok(intent)
    }

    /// Cancels a pending intent; a cancelled intent stays visible (BR5.3).
    ///
    /// # Errors
    /// Returns [`AppError::NotFound`] for an unknown intent,
    /// [`AppError::Forbidden`] for another user's portfolio, and a conflict when
    /// it is already cancelled.
    pub async fn cancel(
        &self,
        caller: &Caller,
        portfolio_id: PortfolioId,
        intent_id: OrderIntentId,
    ) -> Result<OrderIntent, AppError> {
        let portfolio = self.owned(caller, portfolio_id).await?;
        let mut intent = self
            .intents
            .find_by_id(intent_id)
            .await
            .map_err(AppError::from_repository)?
            .filter(|intent| intent.portfolio_id() == portfolio.id())
            .ok_or(AppError::NotFound("order intent"))?;
        intent
            .cancel(self.clock.now())
            .map_err(AppError::from_domain)?;
        self.intents
            .update(&intent)
            .await
            .map_err(AppError::from_repository)?;
        self.trail
            .record_action(
                Some(caller.user_id),
                "portfolio.intent.cancelled",
                intent.id().to_string(),
                AuditOutcome::Success,
                &caller.correlation_id,
                "BR5.3",
            )
            .await?;
        Ok(intent)
    }

    /// Lists the portfolio's intents, newest first.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for another user's portfolio and a
    /// repository refusal when the read fails.
    pub async fn list(
        &self,
        caller: &Caller,
        portfolio_id: PortfolioId,
    ) -> Result<Vec<OrderIntent>, AppError> {
        let portfolio = self.owned(caller, portfolio_id).await?;
        self.intents
            .list_for_portfolio(portfolio.id())
            .await
            .map_err(AppError::from_repository)
    }

    async fn project(
        &self,
        portfolio: &Portfolio,
        request: &IntentRequest,
    ) -> Result<IntentProjection, AppError> {
        // NFR2.2: the projection runs over the latest snapshot's lines, so a stale
        // snapshot is still the 300 ms path and never triggers a C5+C6 compute.
        // A portfolio with no snapshot yet falls back to the fresh compute once.
        let valuation = match self.valuation.latest_valuation(portfolio).await? {
            Some((valuation, _as_of)) => valuation,
            None => self.valuation.fresh(portfolio).await?.valuation,
        };
        let unit_price = self
            .unit_price(portfolio, &valuation, &request.asset_id)
            .await?;
        project_intent(
            &valuation,
            request.asset_id.trim(),
            request.side,
            request.quantity,
            unit_price,
        )
        .map_err(AppError::from_domain)
    }

    async fn unit_price(
        &self,
        portfolio: &Portfolio,
        valuation: &Valuation,
        asset_id: &str,
    ) -> Result<Option<Decimal>, AppError> {
        if let Some(line) = valuation.line(asset_id.trim()) {
            if let (Some(value), quantity) = (line.value(), line.quantity()) {
                if quantity > Decimal::ZERO {
                    return Ok(Some(value / quantity));
                }
            }
        }
        let quotes = self
            .market
            .prices(&[asset_id.trim().to_string()])
            .await
            .map_err(AppError::from_external)?;
        let Some(quote) = quotes.first() else {
            return Ok(None);
        };
        let Some(price) = quote.price() else {
            return Ok(None);
        };
        match quote.currency() {
            Some(currency) if currency != portfolio.base_currency() => {
                let rate = self
                    .market
                    .fx(currency, portfolio.base_currency())
                    .await
                    .map_err(AppError::from_external)?;
                Ok(Some(price * rate.rate()))
            }
            _ => Ok(Some(price)),
        }
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
