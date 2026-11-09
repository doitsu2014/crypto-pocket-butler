//! C8 privacy: export the portfolio data Identity assembles and anonymise the
//! identifying names while keeping the financial-retention rows.

use std::sync::Arc;

use portfolio_domain::{
    AuditOutcome, Guardrail, Operation, OrderIntent, Portfolio, PortfolioAccount, RecordedTrade,
    TargetAllocation, TradeOverlay, UserId, ValuationSnapshot,
};

use crate::common::{AppError, FieldError, ServiceCaller};
use crate::ports::{
    AnonymisationRecord, Clock, IdGenerator, IntentRepository, MembershipRepository,
    OverlayRepository, PortfolioRepository, PrivacyRepository, SnapshotRepository,
    TargetRepository, TradeRepository,
};
use crate::trail::{context, Trail};

/// The portfolio data Identity assembles into the user's export.
#[derive(Debug, Clone)]
pub struct PrivacyExport {
    /// The user.
    pub user_id: UserId,
    /// The portfolios.
    pub portfolios: Vec<Portfolio>,
    /// The account membership rows.
    pub memberships: Vec<PortfolioAccount>,
    /// The targets.
    pub targets: Vec<TargetAllocation>,
    /// The guardrails.
    pub guardrails: Vec<Guardrail>,
    /// The immutable snapshots.
    pub snapshots: Vec<ValuationSnapshot>,
    /// The live trade overlays.
    pub overlays: Vec<TradeOverlay>,
    /// The order intents.
    pub intents: Vec<OrderIntent>,
    /// The recorded trades.
    pub trades: Vec<RecordedTrade>,
}

/// The result of one anonymisation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnonymiseResult {
    /// The user.
    pub user_id: UserId,
    /// The categories kept under a legal basis.
    pub retained_categories: Vec<String>,
    /// Whether an earlier request had already applied it.
    pub already_applied: bool,
}

/// Exports and anonymises a user's portfolio data for Identity.
pub struct PrivacyService {
    portfolios: Arc<dyn PortfolioRepository>,
    memberships: Arc<dyn MembershipRepository>,
    targets: Arc<dyn TargetRepository>,
    snapshots: Arc<dyn SnapshotRepository>,
    overlays: Arc<dyn OverlayRepository>,
    intents: Arc<dyn IntentRepository>,
    trades: Arc<dyn TradeRepository>,
    privacy: Arc<dyn PrivacyRepository>,
    trail: Arc<Trail>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
}

impl PrivacyService {
    /// Creates the use case.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        portfolios: Arc<dyn PortfolioRepository>,
        memberships: Arc<dyn MembershipRepository>,
        targets: Arc<dyn TargetRepository>,
        snapshots: Arc<dyn SnapshotRepository>,
        overlays: Arc<dyn OverlayRepository>,
        intents: Arc<dyn IntentRepository>,
        trades: Arc<dyn TradeRepository>,
        privacy: Arc<dyn PrivacyRepository>,
        trail: Arc<Trail>,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            portfolios,
            memberships,
            targets,
            snapshots,
            overlays,
            intents,
            trades,
            privacy,
            trail,
            clock,
            ids,
        }
    }

    /// Returns the portfolios, membership, targets, guardrails, snapshots,
    /// overlays, intents, and recorded trades for the user.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for a caller without the export
    /// operation, and a repository refusal when the read fails.
    pub async fn export(
        &self,
        caller: &ServiceCaller,
        user_id: UserId,
    ) -> Result<PrivacyExport, AppError> {
        caller.require_operation(Operation::PrivacyExport)?;
        let portfolios = self
            .portfolios
            .find_for_user(user_id)
            .await
            .map_err(AppError::from_repository)?;
        let mut targets = Vec::new();
        let mut guardrails = Vec::new();
        for portfolio in &portfolios {
            targets.extend(
                self.targets
                    .list_targets(portfolio.id())
                    .await
                    .map_err(AppError::from_repository)?,
            );
            guardrails.extend(
                self.targets
                    .list_guardrails(portfolio.id())
                    .await
                    .map_err(AppError::from_repository)?,
            );
        }
        Ok(PrivacyExport {
            user_id,
            portfolios,
            memberships: self
                .memberships
                .list_for_user(user_id)
                .await
                .map_err(AppError::from_repository)?,
            targets,
            guardrails,
            snapshots: self
                .snapshots
                .list_for_user(user_id)
                .await
                .map_err(AppError::from_repository)?,
            overlays: self
                .overlays
                .list_for_user(user_id)
                .await
                .map_err(AppError::from_repository)?,
            intents: self
                .intents
                .list_for_user(user_id)
                .await
                .map_err(AppError::from_repository)?,
            trades: self
                .trades
                .list_for_user(user_id)
                .await
                .map_err(AppError::from_repository)?,
        })
    }

    /// Replaces the subject on portfolios and recorded trades with a random
    /// opaque reference, keeps the financial rows under their legal basis, and
    /// records the retained categories. A post-erasure export by the original id
    /// therefore returns nothing. A retry with the same idempotency key applies
    /// once.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for a caller without the anonymise
    /// operation, a validation refusal for a blank idempotency key, and a
    /// repository refusal when a step fails so the request stays retryable.
    pub async fn anonymise(
        &self,
        caller: &ServiceCaller,
        user_id: UserId,
        idempotency_key: &str,
    ) -> Result<AnonymiseResult, AppError> {
        caller.require_operation(Operation::PrivacyAnonymise)?;
        let idempotency_key = idempotency_key.trim();
        if idempotency_key.is_empty() {
            return Err(AppError::Validation(vec![FieldError::new(
                "idempotencyKey",
                "is required",
            )]));
        }
        if let Some(existing) = self
            .privacy
            .find_anonymisation(idempotency_key)
            .await
            .map_err(AppError::from_repository)?
        {
            return Ok(AnonymiseResult {
                user_id,
                retained_categories: existing.retained_categories,
                already_applied: true,
            });
        }
        let now = self.clock.now();
        let anonymised_ref = UserId::new(self.ids.next());
        self.portfolios
            .anonymise_for_user(user_id, anonymised_ref, now)
            .await
            .map_err(AppError::from_repository)?;
        self.trades
            .anonymise_for_user(user_id, anonymised_ref)
            .await
            .map_err(AppError::from_repository)?;
        let retained_categories = vec![
            "portfolios".to_string(),
            "portfolio_accounts".to_string(),
            "target_allocations".to_string(),
            "guardrails".to_string(),
            "valuation_snapshots".to_string(),
            "trade_overlays".to_string(),
            "order_intents".to_string(),
            "recorded_trades".to_string(),
        ];
        let record = AnonymisationRecord {
            anonymised_ref,
            idempotency_key: idempotency_key.to_string(),
            retained_categories: retained_categories.clone(),
            completed_at: now,
        };
        self.privacy
            .record_anonymisation(&record)
            .await
            .map_err(AppError::from_repository)?;
        // The erasure is complete by this point. The audit action is on the
        // system allow-list and the context carries only allow-listed keys; a
        // failed append is logged and never fails the completed erasure.
        if let Err(error) = self
            .trail
            .record(
                None,
                "system.privacy.deletion",
                anonymised_ref.to_string(),
                AuditOutcome::Success,
                &caller.correlation_id,
                context(&[
                    ("sourceService", "portfolio"),
                    ("ruleId", "privacy.erasure"),
                ]),
            )
            .await
        {
            tracing::error!(%error, "the erasure audit was abandoned; the erasure is complete");
        }
        Ok(AnonymiseResult {
            user_id,
            retained_categories,
            already_applied: false,
        })
    }
}
