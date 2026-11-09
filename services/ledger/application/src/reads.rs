//! C7 reads: the transaction list and the P&L with coverage.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use ledger_domain::{
    compute_pnl, normalise_currency, AssetPnl, CoverageLabel, FxRates, HeldAsset, PnlReport,
    PnlSnapshot, PnlSnapshotId, PriceFreshness, PriceMark, ReplayOutcome, Transaction,
    TransactionId, LIST_PAGE_DEFAULT, LIST_PAGE_MAX,
};
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::common::AppError;
use crate::ports::{
    AccountClient, Clock, IdGenerator, MarketDataClient, SubjectRepository, TransactionCursor,
    TransactionRepository,
};
use crate::rebuild::Rebuilder;

/// One keyset page and the cursor that follows it.
#[derive(Debug, Clone)]
pub struct TransactionPage {
    /// The transactions, newest first.
    pub transactions: Vec<Transaction>,
    /// The cursor to request the next page, when this page was full.
    pub next_cursor: Option<String>,
}

/// Reads the ledger list.
pub struct ReadService {
    transactions: Arc<dyn TransactionRepository>,
    subjects: Arc<dyn SubjectRepository>,
}

impl ReadService {
    /// Creates the read use case.
    #[must_use]
    pub fn new(
        transactions: Arc<dyn TransactionRepository>,
        subjects: Arc<dyn SubjectRepository>,
    ) -> Self {
        Self {
            transactions,
            subjects,
        }
    }

    /// One page of a user's transactions.
    ///
    /// # Errors
    /// Returns [`AppError::Rejected`] for a malformed cursor and
    /// [`AppError::Repository`] when the read fails.
    pub async fn transactions(
        &self,
        user_id: &str,
        after: Option<&str>,
        limit: Option<u32>,
    ) -> Result<TransactionPage, AppError> {
        let limit = limit.unwrap_or(LIST_PAGE_DEFAULT).clamp(1, LIST_PAGE_MAX);
        let cursor = after.map(decode_cursor).transpose()?;
        let subject = self.subjects.find_by_user(user_id).await?;
        let Some(subject) = subject else {
            return Ok(TransactionPage {
                transactions: Vec::new(),
                next_cursor: None,
            });
        };
        let transactions = self
            .transactions
            .list_page(subject.id(), cursor, limit)
            .await?;
        let next_cursor = transactions
            .last()
            .filter(|_| transactions.len() as u32 == limit)
            .map(|transaction| encode_cursor(transaction.executed_at(), transaction.id()));
        Ok(TransactionPage {
            transactions,
            next_cursor,
        })
    }
}

/// Computes a user's P&L from C5 holdings, C6 marks, and C6 FX.
pub struct PnlService {
    subjects: Arc<dyn SubjectRepository>,
    rebuilder: Arc<Rebuilder>,
    account: Arc<dyn AccountClient>,
    market: Arc<dyn MarketDataClient>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
    base_currency: String,
}

impl PnlService {
    /// Creates the P&L use case.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        subjects: Arc<dyn SubjectRepository>,
        rebuilder: Arc<Rebuilder>,
        account: Arc<dyn AccountClient>,
        market: Arc<dyn MarketDataClient>,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
        base_currency: impl Into<String>,
    ) -> Self {
        Self {
            subjects,
            rebuilder,
            account,
            market,
            clock,
            ids,
            base_currency: normalise_currency(&base_currency.into()),
        }
    }

    /// The P&L with its coverage. Coverage is measured against Account's C5
    /// holdings and valued with C6 marks in the user's base currency.
    ///
    /// A C5 or C6 outage never becomes a 503: the affected holdings are
    /// labelled unpriced, coverage is labelled partial with the gap naming
    /// them, and the warning flags are set. Only a ledger-store (Postgres)
    /// failure returns an error.
    ///
    /// # Errors
    /// Returns [`AppError`] when the ledger store (Postgres) fails.
    pub async fn report(
        &self,
        user_id: &str,
        portfolio_id: Option<Uuid>,
    ) -> Result<PnlReport, AppError> {
        let (holdings, holdings_unavailable) = self.holdings(user_id).await;
        let subject = self.subjects.find_by_user(user_id).await?;
        let mut asset_ids: BTreeSet<String> = holdings
            .iter()
            .map(|holding| holding.asset_id.clone())
            .collect();
        if let Some(subject) = &subject {
            asset_ids.extend(self.rebuilder.asset_ids(subject.id()).await?);
        }
        let asset_ids: Vec<String> = asset_ids.into_iter().collect();
        let (prices, marks_unavailable) = self.price_marks(&asset_ids).await;
        if holdings_unavailable || marks_unavailable {
            return Ok(self.degraded_report(
                &holdings,
                &asset_ids,
                holdings_unavailable,
                marks_unavailable,
                false,
            ));
        }
        let Some(subject) = subject else {
            let fx = FxRates::base_only(&self.base_currency);
            return compute_pnl(self.clock.now(), &empty_outcome(), &holdings, &prices, &fx)
                .map_err(AppError::from);
        };

        let last_change = self.last_change(subject.id()).await?;
        if let Some(snapshot) = self
            .rebuilder
            .latest_snapshot(subject.id(), portfolio_id)
            .await?
        {
            if last_change.is_none_or(|changed| snapshot.computed_at() > changed) {
                let lots = self.rebuilder.open_lots(subject.id()).await?;
                let currencies: BTreeSet<String> =
                    lots.iter().map(|lot| lot.currency().to_string()).collect();
                let (fx, fx_unavailable) = self.fx_table(&currencies).await;
                if fx_unavailable {
                    return Ok(self.degraded_report(&holdings, &asset_ids, false, false, true));
                }
                let mut outcome = empty_outcome();
                outcome.lots = lots;
                let mut report = compute_pnl(self.clock.now(), &outcome, &holdings, &prices, &fx)
                    .map_err(AppError::from)?;
                apply_snapshot(&mut report, &snapshot);
                return Ok(report);
            }
        }

        let currencies = self.rebuilder.currencies(subject.id()).await?;
        let (fx, fx_unavailable) = self.fx_table(&currencies).await;
        if fx_unavailable {
            return Ok(self.degraded_report(&holdings, &asset_ids, false, false, true));
        }
        let outcome = self.rebuilder.replay(subject.id(), &fx).await?;
        self.rebuilder.save_lots(subject.id(), &outcome).await?;
        let report = compute_pnl(self.clock.now(), &outcome, &holdings, &prices, &fx)
            .map_err(AppError::from)?;
        let snapshot = PnlSnapshot::record(
            PnlSnapshotId::new(self.ids.next()),
            subject.id(),
            portfolio_id,
            &report,
            self.clock.now(),
        );
        self.rebuilder.save_snapshot(&snapshot).await?;
        Ok(report)
    }

    /// C5 holdings, and whether C5 was unavailable. A C5 outage is a degraded
    /// read, never a 503.
    async fn holdings(&self, user_id: &str) -> (Vec<HeldAsset>, bool) {
        let rows = match self.account.holdings_for_user(user_id).await {
            Ok(rows) => rows,
            Err(_) => return (Vec::new(), true),
        };
        let mut aggregated: BTreeMap<String, Decimal> = BTreeMap::new();
        for row in rows {
            if let Some(asset) = row.asset_id.filter(|asset| !asset.trim().is_empty()) {
                *aggregated.entry(asset).or_insert(Decimal::ZERO) += row.quantity;
            }
        }
        (
            aggregated
                .into_iter()
                .map(|(asset_id, quantity)| HeldAsset { asset_id, quantity })
                .collect(),
            false,
        )
    }

    /// C6 marks, and whether C6 was unavailable. A C6 outage yields no marks,
    /// which labels every affected holding unpriced, never zero.
    async fn price_marks(&self, asset_ids: &[String]) -> (BTreeMap<String, PriceMark>, bool) {
        if asset_ids.is_empty() {
            return (BTreeMap::new(), false);
        }
        let quotes = match self.market.prices(asset_ids).await {
            Ok(quotes) => quotes,
            Err(_) => return (BTreeMap::new(), true),
        };
        let mut marks = BTreeMap::new();
        for quote in quotes {
            let freshness = PriceFreshness::parse(&quote.freshness);
            let price = if freshness == PriceFreshness::Unpriced {
                None
            } else {
                quote.price
            };
            marks.insert(
                quote.asset_id,
                PriceMark {
                    price,
                    currency: quote.currency,
                    as_of: quote.as_of,
                    freshness,
                },
            );
        }
        (marks, false)
    }

    /// The C6 FX table, and whether C6 FX was unavailable. A missing rate is a
    /// degraded read rather than a mixed-currency total.
    async fn fx_table(&self, currencies: &BTreeSet<String>) -> (FxRates, bool) {
        let base = self.base_currency.clone();
        let mut rates = BTreeMap::new();
        let mut as_of: Option<DateTime<Utc>> = None;
        for currency in currencies {
            let currency = normalise_currency(currency);
            if currency == base {
                continue;
            }
            let quote = match self.market.fx(&base, &currency).await {
                Ok(quote) => quote,
                Err(_) => return (FxRates::base_only(base), true),
            };
            rates.insert(currency, quote.rate);
            as_of = max_time(as_of, quote.as_of);
        }
        if rates.is_empty() {
            (FxRates::base_only(base), false)
        } else {
            (FxRates::new(base, rates, as_of), false)
        }
    }

    /// A labelled degraded answer: no figures, the affected assets named in the
    /// gap, and the availability flags set.
    fn degraded_report(
        &self,
        holdings: &[HeldAsset],
        asset_ids: &[String],
        holdings_unavailable: bool,
        marks_unavailable: bool,
        fx_unavailable: bool,
    ) -> PnlReport {
        let mut gap: BTreeSet<String> = holdings
            .iter()
            .map(|holding| holding.asset_id.clone())
            .collect();
        gap.extend(asset_ids.iter().cloned());
        let assets = gap
            .iter()
            .map(|asset_id| AssetPnl {
                asset_id: asset_id.clone(),
                net_invested: None,
                realized: None,
                unrealized: None,
                total_return: None,
                mark_price: None,
                price_as_of: None,
                price_freshness: PriceFreshness::Unpriced.as_str().to_string(),
                covered: false,
            })
            .collect();
        PnlReport {
            as_of: self.clock.now(),
            currency: self.base_currency.clone(),
            fx_as_of: None,
            fx_estimated: false,
            net_invested: None,
            realized: None,
            unrealized: None,
            total_return: None,
            total_acquisition_cost: None,
            marks_unavailable,
            holdings_unavailable,
            fx_unavailable,
            coverage: Decimal::ZERO,
            label: if gap.is_empty() {
                CoverageLabel::Complete
            } else {
                CoverageLabel::Partial
            },
            coverage_gap: gap.into_iter().collect(),
            assets,
        }
    }

    async fn last_change(
        &self,
        subject: ledger_domain::LedgerSubjectId,
    ) -> Result<Option<DateTime<Utc>>, AppError> {
        let recorded = self.rebuilder.last_recorded_at(subject).await?;
        let corrected = self.rebuilder.last_corrected_at(subject).await?;
        Ok(max_time(recorded, corrected))
    }
}

fn max_time(left: Option<DateTime<Utc>>, right: Option<DateTime<Utc>>) -> Option<DateTime<Utc>> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn apply_snapshot(report: &mut PnlReport, snapshot: &PnlSnapshot) {
    report.net_invested = snapshot.net_invested();
    report.realized = snapshot.realized();
    report.unrealized = snapshot.unrealized();
    report.total_return = snapshot.total_return();
    report.total_acquisition_cost = snapshot.total_acquisition_cost();
    report.coverage = snapshot.coverage();
    report.coverage_gap = snapshot.coverage_gap().to_vec();
    report.currency = snapshot.currency().to_string();
    report.fx_as_of = snapshot.fx_as_of();
    report.fx_estimated = snapshot.fx_estimated();
    report.marks_unavailable = false;
    report.holdings_unavailable = false;
    report.fx_unavailable = false;
    report.label = if snapshot.coverage_gap().is_empty() && snapshot.coverage() == Decimal::ONE {
        CoverageLabel::Complete
    } else if snapshot.net_invested().is_none() {
        CoverageLabel::Unavailable
    } else {
        CoverageLabel::Partial
    };
    // The stored snapshot carries only the totals; per-asset realised figures
    // are not persisted, so a short-circuited read does not invent them.
    for asset in &mut report.assets {
        asset.realized = None;
        asset.total_return = None;
    }
}

fn empty_outcome() -> ReplayOutcome {
    ReplayOutcome {
        lots: Vec::new(),
        realized_by_asset: BTreeMap::new(),
        acquisition_cost_by_asset: BTreeMap::new(),
        shortfall_by_asset: BTreeMap::new(),
        standalone_fee_cost: Decimal::ZERO,
    }
}

/// Encodes a keyset cursor as `executedAt|id`.
#[must_use]
pub fn encode_cursor(executed_at: DateTime<Utc>, id: TransactionId) -> String {
    format!("{}|{}", executed_at.to_rfc3339(), id)
}

/// Decodes a keyset cursor.
///
/// # Errors
/// Returns [`AppError::Rejected`] when the cursor is not `executedAt|id`.
pub fn decode_cursor(raw: &str) -> Result<TransactionCursor, AppError> {
    let (timestamp, id) = raw
        .split_once('|')
        .ok_or_else(|| AppError::field("cursor", "must be executedAt|id"))?;
    let executed_at = DateTime::parse_from_rfc3339(timestamp)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| AppError::field("cursor", "the timestamp is not RFC 3339"))?;
    let id =
        TransactionId::parse(id).map_err(|_| AppError::field("cursor", "the id is not a UUID"))?;
    Ok(TransactionCursor { executed_at, id })
}
