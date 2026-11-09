//! WF2 and WF4: backend valuation with provenance, degraded states, and
//! immutable snapshots (BR2.1–BR2.3, BR3.1, BR3.3, BR4.1–BR4.2).

use std::collections::BTreeSet;
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use portfolio_domain::{
    apply_overlays, compute_valuation, hour_bucket, AccountId, Holding, Operation, Portfolio,
    PortfolioId, Quote, TargetAllocation, UserId, Valuation, ValuationSnapshot,
    ValuationSnapshotId,
};

use crate::common::{AppError, Caller, ServiceCaller};
use crate::ports::{
    Clock, HoldingsClient, IdGenerator, MarketDataClient, MembershipRepository, OverlayRepository,
    PortfolioRepository, SnapshotRepository, TargetRepository,
};
use crate::snapshot::{freeze_inputs, lines_json, verify_snapshot, FrozenInputs, StoredValuation};

/// How long a stored snapshot may be served directly before a fresh compute.
pub const SNAPSHOT_SERVED_SECONDS: i64 = 60;

/// The default snapshot-history page size.
pub const DEFAULT_HISTORY_PAGE: i64 = 100;

/// The maximum snapshot-history page size.
pub const MAX_HISTORY_PAGE: i64 = 500;

/// The prompt returned when a portfolio has no targets (BR3.3).
pub const SET_TARGETS_PROMPT: &str = "Set asset targets to see drift.";

/// A valuation computed from live C5 and C6 data.
#[derive(Debug, Clone)]
pub struct FreshValuation {
    /// The computed valuation.
    pub valuation: Valuation,
    /// The frozen inputs used.
    pub inputs: FrozenInputs,
    /// Whether C5 holdings were available.
    pub holdings_available: bool,
}

/// One portfolio valuation read.
#[derive(Debug, Clone)]
pub struct ValuationRead {
    /// The portfolio.
    pub portfolio: Portfolio,
    /// The valuation.
    pub valuation: Valuation,
    /// When the valuation is as-of.
    pub as_of: DateTime<Utc>,
    /// Whether a stored snapshot was served.
    pub served_from_snapshot: bool,
    /// Whether a stored snapshot failed digest verification and was recomputed.
    pub digest_mismatch: bool,
    /// Whether the portfolio has targets configured.
    pub targets_configured: bool,
    /// The set-targets prompt, when drift is unavailable.
    pub prompt: Option<String>,
}

/// One snapshot returned by a history read. Reproduction is not performed here;
/// it happens on demand for a single snapshot.
#[derive(Debug, Clone)]
pub struct SnapshotView {
    /// The snapshot.
    pub snapshot: ValuationSnapshot,
}

/// One bounded page of snapshot history.
#[derive(Debug, Clone)]
pub struct SnapshotPage {
    /// The snapshots in the page, newest first.
    pub snapshots: Vec<SnapshotView>,
    /// The `as_of` cursor for the next page, when one exists.
    pub next_cursor: Option<DateTime<Utc>>,
    /// Whether another page exists.
    pub has_more: bool,
}

/// One snapshot reproduced on demand.
#[derive(Debug, Clone)]
pub struct SnapshotDetail {
    /// The snapshot.
    pub snapshot: ValuationSnapshot,
    /// Whether the digest and the reproduced lines verified.
    pub verified: bool,
}

/// Computes valuations and serves immutable snapshots.
pub struct ValuationService {
    portfolios: Arc<dyn PortfolioRepository>,
    memberships: Arc<dyn MembershipRepository>,
    targets: Arc<dyn TargetRepository>,
    snapshots: Arc<dyn SnapshotRepository>,
    overlays: Arc<dyn OverlayRepository>,
    holdings: Arc<dyn HoldingsClient>,
    market: Arc<dyn MarketDataClient>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
}

impl ValuationService {
    /// Creates the use case.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        portfolios: Arc<dyn PortfolioRepository>,
        memberships: Arc<dyn MembershipRepository>,
        targets: Arc<dyn TargetRepository>,
        snapshots: Arc<dyn SnapshotRepository>,
        overlays: Arc<dyn OverlayRepository>,
        holdings: Arc<dyn HoldingsClient>,
        market: Arc<dyn MarketDataClient>,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            portfolios,
            memberships,
            targets,
            snapshots,
            overlays,
            holdings,
            market,
            clock,
            ids,
        }
    }

    /// Returns the user's portfolio entity when the caller owns it (BR1.1).
    ///
    /// # Errors
    /// Returns [`AppError::NotFound`] for an unknown portfolio and
    /// [`AppError::Forbidden`] for another user's portfolio.
    pub async fn owned(
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

    /// Reads a portfolio's valuation, serving a fresh-enough snapshot or
    /// computing from C5 and C6 (WF2, WF4).
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for another user's portfolio, a
    /// repository refusal when Postgres fails, and never a fabricated line when
    /// C5 or C6 is unavailable.
    pub async fn read(
        &self,
        caller: &Caller,
        portfolio_id: PortfolioId,
    ) -> Result<ValuationRead, AppError> {
        let portfolio = self.owned(caller, portfolio_id).await?;
        self.read_for(portfolio).await
    }

    /// C8: reads a portfolio's valuation for the Reporting service. The caller
    /// holds a service token, not a user identity, so the allow-listed operation
    /// is the authorisation and no owner check applies.
    ///
    /// # Errors
    /// Returns [`AppError::NotFound`] for an unknown portfolio and a repository
    /// refusal when Postgres fails.
    pub async fn read_internal(
        &self,
        portfolio_id: PortfolioId,
    ) -> Result<ValuationRead, AppError> {
        let portfolio = self
            .portfolios
            .find_by_id(portfolio_id)
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("portfolio"))?;
        self.read_for(portfolio).await
    }

    async fn read_for(&self, portfolio: Portfolio) -> Result<ValuationRead, AppError> {
        let now = self.clock.now();
        let mut digest_mismatch = false;
        if let Some(snapshot) = self
            .snapshots
            .latest(portfolio.id())
            .await
            .map_err(AppError::from_repository)?
        {
            if now - snapshot.as_of() <= Duration::seconds(SNAPSHOT_SERVED_SECONDS) {
                match verify_snapshot(
                    snapshot.inputs_json(),
                    snapshot.inputs_digest(),
                    snapshot.lines_json(),
                ) {
                    Ok(_) => {
                        let inputs = FrozenInputs::from_json(snapshot.inputs_json())
                            .map_err(AppError::Internal)?;
                        let targets_configured = !inputs.targets.is_empty();
                        // A recorded trade applied after the snapshot must be
                        // visible on reload (AC5.4.1). The deltas are applied to
                        // the frozen holdings and reproduced in memory; C5 and C6
                        // are not called.
                        let overlays = self
                            .overlays
                            .list_for_portfolio(portfolio.id())
                            .await
                            .map_err(AppError::from_repository)?;
                        let after: Vec<&portfolio_domain::TradeOverlay> = overlays
                            .iter()
                            .filter(|overlay| overlay.applied_at() > snapshot.as_of())
                            .collect();
                        let (valuation, as_of) = if after.is_empty() {
                            let valuation = inputs.reproduce().map_err(AppError::Internal)?;
                            (valuation, snapshot.as_of())
                        } else {
                            let augmented = crate::snapshot::apply_overlay_deltas(&inputs, &after);
                            let valuation = augmented.reproduce().map_err(AppError::Internal)?;
                            let as_of = after
                                .iter()
                                .map(|overlay| overlay.applied_at())
                                .max()
                                .unwrap_or_else(|| snapshot.as_of());
                            (valuation, as_of)
                        };
                        return Ok(self.read_from(
                            portfolio,
                            valuation,
                            as_of,
                            true,
                            false,
                            targets_configured,
                        ));
                    }
                    Err(reason) => {
                        tracing::warn!(%reason, snapshot = %snapshot.id(), "snapshot digest mismatch; recomputing");
                        digest_mismatch = true;
                    }
                }
            }
        }
        let fresh = self.fresh(&portfolio).await?;
        let targets_configured = !fresh.inputs.targets.is_empty();
        Ok(self.read_from(
            portfolio,
            fresh.valuation,
            now,
            false,
            digest_mismatch,
            targets_configured,
        ))
    }

    fn read_from(
        &self,
        portfolio: Portfolio,
        valuation: Valuation,
        as_of: DateTime<Utc>,
        served_from_snapshot: bool,
        digest_mismatch: bool,
        targets_configured: bool,
    ) -> ValuationRead {
        ValuationRead {
            portfolio,
            valuation,
            as_of,
            served_from_snapshot,
            digest_mismatch,
            targets_configured,
            prompt: if targets_configured {
                None
            } else {
                Some(SET_TARGETS_PROMPT.to_string())
            },
        }
    }

    /// Computes a valuation from live C5 holdings (with overlays) and C6
    /// prices/FX. A C5 failure degrades to `partial` with no fabricated lines; a
    /// C6 failure leaves the affected lines unpriced.
    ///
    /// # Errors
    /// Returns a repository refusal when the service's own Postgres reads fail.
    pub async fn fresh(&self, portfolio: &Portfolio) -> Result<FreshValuation, AppError> {
        let memberships = self
            .memberships
            .list_for_portfolio(portfolio.id())
            .await
            .map_err(AppError::from_repository)?;
        let account_ids: BTreeSet<AccountId> = memberships
            .iter()
            .map(portfolio_domain::PortfolioAccount::account_id)
            .collect();

        let (observations, holdings_available) = match self
            .holdings
            .holdings_for_user(portfolio.user_id())
            .await
        {
            Ok(observations) => (
                observations
                    .into_iter()
                    .filter(|holding| account_ids.contains(&holding.account_id))
                    .collect::<Vec<_>>(),
                true,
            ),
            Err(error) => {
                tracing::warn!(%error, "C5 holdings unavailable; returning a labelled partial valuation");
                (Vec::new(), false)
            }
        };

        let overlays = self
            .overlays
            .list_for_portfolio(portfolio.id())
            .await
            .map_err(AppError::from_repository)?;
        let targets = self
            .targets
            .list_targets(portfolio.id())
            .await
            .map_err(AppError::from_repository)?;

        let now = self.clock.now();
        let holdings = if holdings_available {
            let base: Vec<Holding> = observations
                .iter()
                .filter_map(|holding| {
                    Holding::new(
                        Some(holding.account_id),
                        &holding.asset_id,
                        holding.quantity,
                        holding.observed_at,
                    )
                    .ok()
                })
                .collect();
            apply_overlays(&base, &overlays, now)
        } else {
            Vec::new()
        };

        let asset_ids: Vec<String> = holdings
            .iter()
            .map(|holding| holding.asset_id().to_string())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let quotes = if holdings_available && !asset_ids.is_empty() {
            match self.market.prices(&asset_ids).await {
                Ok(quotes) => quotes,
                Err(error) => {
                    tracing::warn!(%error, "C6 prices unavailable; lines are labelled unpriced");
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };

        let mut fx: Vec<portfolio_domain::FxRate> = Vec::new();
        if holdings_available {
            let base_currency = portfolio.base_currency().to_string();
            let wanted: BTreeSet<String> = quotes
                .iter()
                .filter(|quote| quote.price().is_some())
                .filter_map(|quote| quote.currency().map(ToString::to_string))
                .filter(|currency| currency != &base_currency)
                .collect();
            for currency in wanted {
                match self.market.fx(&currency, &base_currency).await {
                    Ok(rate) => fx.push(rate),
                    Err(error) => {
                        tracing::warn!(%error, currency, "C6 FX unavailable; the line is labelled unpriced");
                    }
                }
            }
        }

        let valuation = compute_valuation(
            portfolio.base_currency(),
            &holdings,
            &quotes,
            &fx,
            &targets,
            holdings_available,
        );
        let inputs = freeze_inputs(portfolio.base_currency(), &holdings, &quotes, &fx, &targets);
        Ok(FreshValuation {
            valuation,
            inputs,
            holdings_available,
        })
    }

    /// Stores an immutable snapshot for the hour bucket, or returns `None` when
    /// one already exists (BR4.1).
    ///
    /// # Errors
    /// Returns a repository refusal when the write fails for another reason.
    pub async fn store_snapshot(
        &self,
        portfolio: &Portfolio,
        valuation: &Valuation,
        inputs: &FrozenInputs,
        as_of: DateTime<Utc>,
    ) -> Result<Option<ValuationSnapshot>, AppError> {
        let Some(total_value) = valuation.total_value() else {
            return Ok(None);
        };
        let bucket = hour_bucket(as_of);
        if self
            .snapshots
            .exists_for_hour(portfolio.id(), bucket)
            .await
            .map_err(AppError::from_repository)?
        {
            return Ok(None);
        }
        let snapshot = ValuationSnapshot::new(
            ValuationSnapshotId::new(self.ids.next()),
            portfolio.id(),
            as_of,
            total_value,
            valuation.currency(),
            inputs.digest(),
            inputs.to_json(),
            lines_json(valuation),
        )
        .map_err(AppError::from_domain)?;
        match self.snapshots.insert(&snapshot).await {
            Ok(()) => Ok(Some(snapshot)),
            Err(crate::ports::RepositoryError::Conflict(_)) => Ok(None),
            Err(error) => Err(AppError::from_repository(error)),
        }
    }

    /// Returns a bounded page of a portfolio's snapshot history (C8). Review of
    /// the digest is deliberately not performed for every row; reproduction is
    /// on demand through [`Self::snapshot_detail`].
    ///
    /// # Errors
    /// Returns a repository refusal when the read fails.
    pub async fn history(
        &self,
        portfolio_id: PortfolioId,
        from: Option<DateTime<Utc>>,
        limit: i64,
        cursor: Option<DateTime<Utc>>,
    ) -> Result<SnapshotPage, AppError> {
        let limit = limit.clamp(1, MAX_HISTORY_PAGE);
        // Fetch one extra row to learn whether another page exists.
        let mut snapshots = self
            .snapshots
            .list(portfolio_id, from, limit + 1, cursor)
            .await
            .map_err(AppError::from_repository)?;
        let has_more = snapshots.len() as i64 > limit;
        snapshots.truncate(limit as usize);
        let next_cursor = if has_more {
            snapshots.last().map(ValuationSnapshot::as_of)
        } else {
            None
        };
        Ok(SnapshotPage {
            snapshots: snapshots
                .into_iter()
                .map(|snapshot| SnapshotView { snapshot })
                .collect(),
            next_cursor,
            has_more,
        })
    }

    /// Reproduces one snapshot on demand and reports whether it verified.
    ///
    /// # Errors
    /// Returns a repository refusal when the read fails, and `None` when the
    /// snapshot does not belong to the portfolio.
    pub async fn snapshot_detail(
        &self,
        portfolio_id: PortfolioId,
        snapshot_id: ValuationSnapshotId,
    ) -> Result<Option<SnapshotDetail>, AppError> {
        let Some(snapshot) = self
            .snapshots
            .find_by_id(snapshot_id)
            .await
            .map_err(AppError::from_repository)?
        else {
            return Ok(None);
        };
        if snapshot.portfolio_id() != portfolio_id {
            return Ok(None);
        }
        let verified = match verify_snapshot(
            snapshot.inputs_json(),
            snapshot.inputs_digest(),
            snapshot.lines_json(),
        ) {
            Ok(_) => true,
            Err(reason) => {
                tracing::warn!(%reason, snapshot = %snapshot.id(), "snapshot digest mismatch");
                false
            }
        };
        Ok(Some(SnapshotDetail { snapshot, verified }))
    }

    /// The latest snapshot's reproduced valuation, without a C5 or C6 call. Used
    /// by the in-memory intent/target preview path (NFR2.2).
    ///
    /// # Errors
    /// Returns a repository refusal when the read fails.
    pub async fn latest_valuation(
        &self,
        portfolio: &Portfolio,
    ) -> Result<Option<(Valuation, DateTime<Utc>)>, AppError> {
        let Some(snapshot) = self
            .snapshots
            .latest(portfolio.id())
            .await
            .map_err(AppError::from_repository)?
        else {
            return Ok(None);
        };
        match verify_snapshot(
            snapshot.inputs_json(),
            snapshot.inputs_digest(),
            snapshot.lines_json(),
        ) {
            Ok(valuation) => Ok(Some((valuation, snapshot.as_of()))),
            Err(reason) => {
                tracing::warn!(%reason, snapshot = %snapshot.id(), "snapshot digest mismatch; the preview falls back to a fresh compute");
                Ok(None)
            }
        }
    }

    /// Returns the history for an internal C8 caller (Reporting).
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for a caller without the snapshots
    /// operation, and a repository refusal when the read fails.
    pub async fn history_for_service(
        &self,
        caller: &ServiceCaller,
        portfolio_id: PortfolioId,
        from: Option<DateTime<Utc>>,
        limit: i64,
        cursor: Option<DateTime<Utc>>,
    ) -> Result<SnapshotPage, AppError> {
        caller.require_operation(Operation::SnapshotsRead)?;
        self.history(portfolio_id, from, limit, cursor).await
    }

    /// Returns the newest snapshot's stored valuation for a portfolio, if one
    /// exists, without verifying it. Used by the read path's tests and the run.
    ///
    /// # Errors
    /// Returns a repository refusal when the read fails.
    pub async fn latest_stored(
        &self,
        portfolio_id: PortfolioId,
    ) -> Result<Option<ValuationSnapshot>, AppError> {
        self.snapshots
            .latest(portfolio_id)
            .await
            .map_err(AppError::from_repository)
    }

    /// Lists every snapshot of the user's portfolios for the privacy export.
    ///
    /// # Errors
    /// Returns a repository refusal when the read fails.
    pub async fn snapshots_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<ValuationSnapshot>, AppError> {
        self.snapshots
            .list_for_user(user_id)
            .await
            .map_err(AppError::from_repository)
    }

    /// Exposes the helper `freeze_inputs` to the run's tests.
    #[must_use]
    pub fn frozen(
        base_currency: &str,
        holdings: &[Holding],
        quotes: &[Quote],
        fx: &[portfolio_domain::FxRate],
        targets: &[TargetAllocation],
    ) -> FrozenInputs {
        freeze_inputs(base_currency, holdings, quotes, fx, targets)
    }

    /// Exposes the stored-lines parser to the run's tests.
    ///
    /// # Errors
    /// Returns a message when the stored lines are malformed.
    pub fn parse_stored_lines(raw: &str) -> Result<StoredValuation, String> {
        crate::snapshot::parse_lines(raw)
    }

    /// Re-export used by the handler boundary for its DTO mapping.
    #[must_use]
    pub fn stored_view(valuation: &Valuation) -> StoredValuation {
        crate::snapshot::stored_valuation(valuation)
    }
}
