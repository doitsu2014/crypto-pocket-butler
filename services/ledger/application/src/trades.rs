//! WF6: record an executed trade, labelled `priceSource = user` when the user
//! supplies the price (BR5.1, BR5.2, NFR1.7).

use std::sync::Arc;

use chrono::{DateTime, Utc};
use ledger_domain::{
    Kind, NewTransaction, PnlReport, PriceSource, Source, Transaction, TransactionId,
};
use rust_decimal::Decimal;

use crate::common::AppError;
use crate::ingest::IngestService;
use crate::ports::{AssetResolver, Clock, IdGenerator, RepositoryError, TransactionRepository};
use crate::rebuild::Rebuilder;
use crate::trail::Trail;

/// A recorded trade submitted by Portfolio.
#[derive(Debug, Clone, PartialEq)]
pub struct TradeCommand {
    /// The asset.
    pub asset_id: String,
    /// `buy` or `sell`.
    pub kind: Kind,
    /// The moved quantity.
    pub quantity: Decimal,
    /// The user-supplied unit price; absent means the user has not supplied it.
    pub price: Option<Decimal>,
    /// When the trade executed.
    pub executed_at: DateTime<Utc>,
    /// The ISO currency the price is quoted in; absent means the base currency.
    pub currency: Option<String>,
}

/// A stored recorded trade and the state of its follow-ups.
#[derive(Debug, Clone)]
pub struct RecordedTrade {
    /// The append-only transaction.
    pub transaction: Transaction,
    /// The recomputed P&L, when the rebuild succeeded.
    pub report: Option<PnlReport>,
    /// Whether the C10 audit retries were exhausted.
    pub audit_failed: bool,
    /// Whether the derived state could not be rebuilt.
    pub rebuild_failed: bool,
}

/// Stores a user-recorded trade.
pub struct TradesService {
    transactions: Arc<dyn TransactionRepository>,
    ingest: Arc<IngestService>,
    assets: Arc<dyn AssetResolver>,
    rebuilder: Arc<Rebuilder>,
    pnl: Arc<crate::reads::PnlService>,
    trail: Arc<Trail>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
    base_currency: String,
}

impl TradesService {
    /// Creates the use case.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        transactions: Arc<dyn TransactionRepository>,
        ingest: Arc<IngestService>,
        assets: Arc<dyn AssetResolver>,
        rebuilder: Arc<Rebuilder>,
        pnl: Arc<crate::reads::PnlService>,
        trail: Arc<Trail>,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
        base_currency: impl Into<String>,
    ) -> Self {
        Self {
            transactions,
            ingest,
            assets,
            rebuilder,
            pnl,
            trail,
            clock,
            ids,
            base_currency: ledger_domain::normalise_currency(&base_currency.into()),
        }
    }

    /// Records a trade. A user-recorded sell beyond known holdings is refused;
    /// an ingested sell is not, because it carries a coverage gap instead.
    ///
    /// # Errors
    /// Returns [`AppError::Rejected`] for a missing price, a non-trade kind, an
    /// unknown asset, or a sell beyond holdings.
    pub async fn record(
        &self,
        user_id: &str,
        command: TradeCommand,
    ) -> Result<RecordedTrade, AppError> {
        if !matches!(command.kind, Kind::Buy | Kind::Sell) {
            return Err(AppError::field(
                "kind",
                "a recorded trade must be a buy or a sell",
            ));
        }
        if command.quantity <= Decimal::ZERO {
            return Err(AppError::field("quantity", "must be greater than zero"));
        }
        let price = command.price.ok_or_else(|| {
            AppError::field(
                "price",
                "is required; a recorded trade without a price must have one supplied by the user",
            )
        })?;
        if price < Decimal::ZERO {
            return Err(AppError::field("price", "must be zero or greater"));
        }
        let exists = self
            .assets
            .exists(&command.asset_id)
            .await
            .map_err(AppError::External)?;
        if !exists {
            return Err(AppError::field("assetId", "is not registered"));
        }
        let subject = self.ingest.ensure_subject(user_id).await?;
        if command.kind == Kind::Sell {
            let held = self
                .rebuilder
                .held_quantity(subject.id(), &command.asset_id)
                .await?;
            if command.quantity > held {
                return Err(AppError::field(
                    "quantity",
                    "exceeds the holdings known to the ledger",
                ));
            }
        }
        let transaction = Transaction::record(
            TransactionId::new(self.ids.next()),
            NewTransaction {
                subject_id: subject.id(),
                user_id: user_id.to_string(),
                account_id: None,
                asset_id: command.asset_id,
                kind: command.kind,
                quantity: command.quantity,
                price: Some(price),
                price_source: Some(PriceSource::User),
                currency: command
                    .currency
                    .clone()
                    .unwrap_or_else(|| self.base_currency.clone()),
                source: Source::RecordedTrade,
                external_ref: None,
                executed_at: command.executed_at,
                recorded_at: self.clock.now(),
            },
        )?;
        match self.transactions.insert(&transaction).await {
            Ok(()) => {}
            Err(RepositoryError::Conflict(_)) => {
                return Err(AppError::field(
                    "transaction",
                    "this trade is already recorded",
                ));
            }
            Err(error) => return Err(AppError::Repository(error)),
        }
        let audit_failed = !self
            .trail
            .trade_recorded(user_id, &transaction.id().to_string())
            .await;
        let (report, rebuild_failed) = match self.pnl.report(user_id, None).await {
            Ok(report) => (Some(report), false),
            Err(_) => {
                tracing::error!("the P&L refresh failed after recording a trade");
                (None, true)
            }
        };
        Ok(RecordedTrade {
            transaction,
            report,
            audit_failed,
            rebuild_failed,
        })
    }
}
