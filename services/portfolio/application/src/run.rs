//! C11: the scheduled valuation run (BR4.1, BR6.1, BR7.1). One leased run takes
//! at most one snapshot per portfolio per hour, reconciles trade overlays that
//! C5 now reports, and evaluates guardrails.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use futures::stream::{FuturesUnordered, StreamExt};
use portfolio_domain::{Operation, Portfolio};
use uuid::Uuid;

use crate::common::{AppError, ServiceCaller};
use crate::ports::{
    Clock, HoldingsClient, IdGenerator, MembershipRepository, OverlayRepository,
    PortfolioRepository, RunRepository, RunStatus, TradeRepository, ValuationRun,
};
use crate::valuation::ValuationService;

/// The C11 job name.
pub const RUN_JOB: &str = "portfolio-valuation";
/// The lease length, matching the 15-minute whole-run budget.
pub const LEASE_SECONDS: i64 = 900;
/// How often the lease is renewed.
pub const RENEW_SECONDS: u64 = 60;
/// The portfolios read per page.
pub const BATCH_SIZE: i64 = 500;
/// The portfolio batches processed in parallel.
pub const BATCHES_IN_FLIGHT: usize = 4;
/// The portfolios processed in flight inside one batch.
pub const BATCH_IN_FLIGHT: usize = 4;
/// How old an unmatched overlay may be before it is expired.
pub const OVERLAY_EXPIRY_DAYS: i64 = 7;

/// The outcome of one scheduled run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RunReport {
    /// Portfolios processed.
    pub portfolios: u32,
    /// Snapshots written.
    pub snapshots: u32,
    /// Portfolios skipped because the hour bucket already had a snapshot.
    pub skipped: u32,
    /// Portfolios that could not be valued.
    pub failed: u32,
    /// Guardrail alerts that failed and will be retried.
    pub guardrail_alerts_failed: u32,
    /// Unmatched overlays expired after seven days.
    pub overlays_expired: u64,
    /// When the run started.
    pub started_at: Option<DateTime<Utc>>,
    /// How long the run took.
    pub duration_seconds: u64,
}

/// Runs the scheduled valuation.
pub struct ValuationRunService {
    portfolios: Arc<dyn PortfolioRepository>,
    memberships: Arc<dyn MembershipRepository>,
    overlays: Arc<dyn OverlayRepository>,
    holdings: Arc<dyn HoldingsClient>,
    trades: Arc<dyn TradeRepository>,
    runs: Arc<dyn RunRepository>,
    valuation: Arc<ValuationService>,
    guardrails: Arc<crate::guardrails::GuardrailService>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
}

impl ValuationRunService {
    /// Creates the use case.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        portfolios: Arc<dyn PortfolioRepository>,
        memberships: Arc<dyn MembershipRepository>,
        overlays: Arc<dyn OverlayRepository>,
        holdings: Arc<dyn HoldingsClient>,
        trades: Arc<dyn TradeRepository>,
        runs: Arc<dyn RunRepository>,
        valuation: Arc<ValuationService>,
        guardrails: Arc<crate::guardrails::GuardrailService>,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            portfolios,
            memberships,
            overlays,
            holdings,
            trades,
            runs,
            valuation,
            guardrails,
            clock,
            ids,
        }
    }

    /// Runs one valuation pass. Returns [`AppError::Conflict`] when another run
    /// holds the single-flight lease.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for a caller without the run operation,
    /// [`AppError::Conflict`] for a held lease, and a repository refusal when the
    /// run cannot be recorded.
    pub async fn run(&self, caller: &ServiceCaller, owner: &str) -> Result<RunReport, AppError> {
        caller.require_operation(Operation::RunValuation)?;
        self.execute(owner).await
    }

    /// Runs the scheduled pass from the worker's own supervisor loop, which has
    /// no inbound service token.
    ///
    /// # Errors
    /// Returns [`AppError::Conflict`] for a held lease and a repository refusal
    /// when the run cannot be recorded.
    pub async fn run_scheduled(&self, owner: &str) -> Result<RunReport, AppError> {
        self.execute(owner).await
    }

    async fn execute(&self, owner: &str) -> Result<RunReport, AppError> {
        let started_at = self.clock.now();
        if let Err(error) = self.runs.sweep_expired(started_at).await {
            tracing::warn!(%error, "the expired-run sweep failed");
        }
        let mut run = ValuationRun {
            id: self.ids.next(),
            job: RUN_JOB.to_string(),
            owner: owner.to_string(),
            started_at,
            lease_until: started_at + chrono::Duration::seconds(LEASE_SECONDS),
            finished_at: None,
            status: RunStatus::Running,
            detail: None,
        };
        if !self
            .runs
            .claim(&run)
            .await
            .map_err(AppError::from_repository)?
        {
            return Err(AppError::Conflict(
                "another valuation run holds the lease".to_string(),
            ));
        }

        let stop = Arc::new(AtomicBool::new(false));
        let lease_lost = Arc::new(AtomicBool::new(false));
        let renewer = tokio::spawn(renew_loop(
            Arc::clone(&self.runs),
            run.id,
            Arc::clone(&stop),
            Arc::clone(&lease_lost),
            Arc::clone(&self.clock),
        ));

        let mut report = RunReport {
            started_at: Some(started_at),
            ..RunReport::default()
        };
        let mut offset = 0_i64;
        let mut finished = false;
        while !finished {
            if lease_lost.load(Ordering::Relaxed) {
                tracing::error!("the valuation run lost its lease and stops");
                report.failed += 1;
                break;
            }
            // Four batches of 500 run in parallel; inside each batch four
            // portfolios are in flight. That keeps 10k portfolios inside the
            // 15-minute budget.
            let mut batches = FuturesUnordered::new();
            for _ in 0..BATCHES_IN_FLIGHT {
                let page = self
                    .portfolios
                    .list_page(BATCH_SIZE, offset)
                    .await
                    .map_err(AppError::from_repository)?;
                if page.is_empty() {
                    finished = true;
                    break;
                }
                if page.len() < BATCH_SIZE as usize {
                    finished = true;
                }
                offset += BATCH_SIZE;
                batches.push(self.process_batch(page));
            }
            while let Some(batch) = batches.next().await {
                report.portfolios += batch.portfolios;
                report.snapshots += batch.snapshots;
                report.skipped += batch.skipped;
                report.failed += batch.failed;
                report.guardrail_alerts_failed += batch.alerts_failed;
            }
        }

        // An overlay that no venue sync has reported for seven days is expired
        // so it cannot distort the valuation forever.
        let cutoff = started_at - chrono::Duration::days(OVERLAY_EXPIRY_DAYS);
        match self.overlays.expire_before(cutoff).await {
            Ok(expired) => {
                if expired > 0 {
                    tracing::warn!(
                        expired,
                        "expired trade overlays no venue sync reported within seven days"
                    );
                }
                report.overlays_expired = expired;
            }
            Err(error) => tracing::warn!(%error, "the overlay expiry failed"),
        }

        stop.store(true, Ordering::Relaxed);
        renewer.abort();
        run.finished_at = Some(self.clock.now());
        run.status = if report.failed > 0 || lease_lost.load(Ordering::Relaxed) {
            RunStatus::Failed
        } else {
            RunStatus::Succeeded
        };
        run.detail = Some(format!(
            "portfolios={} snapshots={} skipped={} failed={} guardrailAlertsFailed={} overlaysExpired={}",
            report.portfolios,
            report.snapshots,
            report.skipped,
            report.failed,
            report.guardrail_alerts_failed,
            report.overlays_expired
        ));
        self.runs
            .finish(&run)
            .await
            .map_err(AppError::from_repository)?;
        report.duration_seconds = (self.clock.now() - started_at).num_seconds().max(0) as u64;
        Ok(report)
    }

    async fn process_batch(&self, page: Vec<Portfolio>) -> BatchOutcome {
        let mut outcome = BatchOutcome::default();
        for chunk in page.chunks(BATCH_IN_FLIGHT) {
            let mut futures = FuturesUnordered::new();
            for portfolio in chunk {
                futures.push(self.process_one(portfolio));
            }
            while let Some(item) = futures.next().await {
                outcome.portfolios += 1;
                if item.failed {
                    outcome.failed += 1;
                } else if item.snapshot {
                    outcome.snapshots += 1;
                } else {
                    outcome.skipped += 1;
                }
                outcome.alerts_failed += item.alerts_failed;
            }
        }
        outcome
    }

    async fn process_one(&self, portfolio: &Portfolio) -> Outcome {
        self.reconcile_overlays(portfolio).await;
        let fresh = match self.valuation.fresh(portfolio).await {
            Ok(fresh) => fresh,
            Err(error) => {
                tracing::error!(%error, portfolio = %portfolio.id(), "the portfolio could not be valued");
                return Outcome {
                    snapshot: false,
                    failed: true,
                    alerts_failed: 0,
                };
            }
        };
        if !fresh.holdings_available {
            tracing::warn!(portfolio = %portfolio.id(), "C5 holdings unavailable; skipping the snapshot");
            return Outcome {
                snapshot: false,
                failed: true,
                alerts_failed: 0,
            };
        }
        let snapshot = match self
            .valuation
            .store_snapshot(portfolio, &fresh.valuation, &fresh.inputs, self.clock.now())
            .await
        {
            Ok(Some(_)) => true,
            Ok(None) => false,
            Err(error) => {
                tracing::error!(%error, portfolio = %portfolio.id(), "the snapshot could not be written");
                return Outcome {
                    snapshot: false,
                    failed: true,
                    alerts_failed: 0,
                };
            }
        };
        let guardrail_report = match self.guardrails.evaluate(portfolio, &fresh.valuation).await {
            Ok(report) => report,
            Err(error) => {
                tracing::error!(%error, portfolio = %portfolio.id(), "guardrail evaluation failed");
                return Outcome {
                    snapshot,
                    failed: true,
                    alerts_failed: 0,
                };
            }
        };
        Outcome {
            snapshot,
            failed: false,
            alerts_failed: guardrail_report.alerts_failed,
        }
    }

    async fn reconcile_overlays(&self, portfolio: &Portfolio) {
        let overlays = match self.overlays.list_for_portfolio(portfolio.id()).await {
            Ok(overlays) => overlays,
            Err(error) => {
                tracing::warn!(%error, portfolio = %portfolio.id(), "overlay read failed");
                return;
            }
        };
        if overlays.is_empty() {
            return;
        }
        let memberships = match self.memberships.list_for_portfolio(portfolio.id()).await {
            Ok(memberships) => memberships,
            Err(error) => {
                tracing::warn!(%error, portfolio = %portfolio.id(), "membership read failed");
                return;
            }
        };
        for overlay in overlays {
            let Ok(Some(trade)) = self.trades.find_by_id(overlay.recorded_trade_id()).await else {
                continue;
            };
            // Search around the trade's execution instant, never the overlay's
            // apply time, so a late sync still finds the matching venue row.
            let since = trade.executed_at() - chrono::Duration::hours(24);
            let mut matched = false;
            for membership in &memberships {
                match self
                    .holdings
                    .transactions_for_account(membership.account_id(), Some(since))
                    .await
                {
                    Ok(transactions) => {
                        if transactions
                            .iter()
                            .any(|transaction| same_trade(&trade, transaction))
                        {
                            matched = true;
                            break;
                        }
                    }
                    Err(error) => {
                        tracing::warn!(%error, account = %membership.account_id(), "C5 transaction read failed");
                    }
                }
            }
            if matched {
                if let Err(error) = self
                    .overlays
                    .delete_for_recorded_trade(overlay.recorded_trade_id())
                    .await
                {
                    tracing::warn!(%error, "the matched overlay could not be removed");
                }
            }
        }
    }
}

#[derive(Default)]
struct BatchOutcome {
    portfolios: u32,
    snapshots: u32,
    skipped: u32,
    failed: u32,
    alerts_failed: u32,
}

struct Outcome {
    snapshot: bool,
    failed: bool,
    alerts_failed: u32,
}

/// Whether a venue transaction matches a recorded trade. The quantity may differ
/// by at most 0.1% (venues round), the execution instant by at most 24 hours, and
/// the portfolio is implied by the overlay's membership. The asset and side must
/// match exactly.
fn same_trade(
    trade: &portfolio_domain::RecordedTrade,
    transaction: &crate::ports::VenueTradeObservation,
) -> bool {
    trade.asset_id() == transaction.asset_id
        && trade.side() == transaction.side
        && quantity_within_tolerance(trade.quantity(), transaction.quantity)
        && (trade.executed_at() - transaction.executed_at).abs() <= chrono::Duration::hours(24)
}

fn quantity_within_tolerance(left: rust_decimal::Decimal, right: rust_decimal::Decimal) -> bool {
    let difference = (left - right).abs();
    let scale = left.abs().max(right.abs());
    difference <= scale * rust_decimal::Decimal::new(1, 3)
}

async fn renew_loop(
    runs: Arc<dyn RunRepository>,
    id: Uuid,
    stop: Arc<AtomicBool>,
    lease_lost: Arc<AtomicBool>,
    clock: Arc<dyn Clock>,
) {
    loop {
        tokio::time::sleep(Duration::from_secs(RENEW_SECONDS)).await;
        if stop.load(Ordering::Relaxed) {
            break;
        }
        let now = clock.now();
        match runs
            .renew(id, now + chrono::Duration::seconds(LEASE_SECONDS), now)
            .await
        {
            Ok(true) => {}
            Ok(false) => {
                lease_lost.store(true, Ordering::Relaxed);
                break;
            }
            Err(error) => {
                tracing::error!(%error, "the valuation lease could not be renewed");
                lease_lost.store(true, Ordering::Relaxed);
                break;
            }
        }
    }
}

/// Re-export for the worker loop's type ascription.
pub type RunResult = RunReport;
