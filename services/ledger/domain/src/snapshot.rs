//! The stored P&L snapshot: only the total is persisted (BR2.2, BR3.1).

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::ids::{LedgerSubjectId, PnlSnapshotId};
use crate::pnl::PnlReport;

/// A computed total P&L figure with its coverage.
#[derive(Debug, Clone, PartialEq)]
pub struct PnlSnapshot {
    id: PnlSnapshotId,
    subject_id: LedgerSubjectId,
    portfolio_id: Option<Uuid>,
    as_of: DateTime<Utc>,
    currency: String,
    fx_as_of: Option<DateTime<Utc>>,
    fx_estimated: bool,
    net_invested: Option<Decimal>,
    realized: Option<Decimal>,
    unrealized: Option<Decimal>,
    total_return: Option<Decimal>,
    total_acquisition_cost: Option<Decimal>,
    coverage: Decimal,
    coverage_gap: Vec<String>,
    computed_at: DateTime<Utc>,
}

impl PnlSnapshot {
    /// Records the total figures from a report.
    #[must_use]
    pub fn record(
        id: PnlSnapshotId,
        subject_id: LedgerSubjectId,
        portfolio_id: Option<Uuid>,
        report: &PnlReport,
        computed_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            subject_id,
            portfolio_id,
            as_of: report.as_of,
            currency: report.currency.clone(),
            fx_as_of: report.fx_as_of,
            fx_estimated: report.fx_estimated,
            net_invested: report.net_invested,
            realized: report.realized,
            unrealized: report.unrealized,
            total_return: report.total_return,
            total_acquisition_cost: report.total_acquisition_cost,
            coverage: report.coverage,
            coverage_gap: report.coverage_gap.clone(),
            computed_at,
        }
    }

    /// Rehydrates a stored snapshot.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: PnlSnapshotId,
        subject_id: LedgerSubjectId,
        portfolio_id: Option<Uuid>,
        as_of: DateTime<Utc>,
        currency: String,
        fx_as_of: Option<DateTime<Utc>>,
        fx_estimated: bool,
        net_invested: Option<Decimal>,
        realized: Option<Decimal>,
        unrealized: Option<Decimal>,
        total_return: Option<Decimal>,
        total_acquisition_cost: Option<Decimal>,
        coverage: Decimal,
        coverage_gap: Vec<String>,
        computed_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            subject_id,
            portfolio_id,
            as_of,
            currency,
            fx_as_of,
            fx_estimated,
            net_invested,
            realized,
            unrealized,
            total_return,
            total_acquisition_cost,
            coverage,
            coverage_gap,
            computed_at,
        }
    }

    /// The identifier.
    #[must_use]
    pub fn id(&self) -> PnlSnapshotId {
        self.id
    }

    /// The owning subject.
    #[must_use]
    pub fn subject_id(&self) -> LedgerSubjectId {
        self.subject_id
    }

    /// The optional portfolio scope.
    #[must_use]
    pub fn portfolio_id(&self) -> Option<Uuid> {
        self.portfolio_id
    }

    /// When the figures were computed.
    #[must_use]
    pub fn as_of(&self) -> DateTime<Utc> {
        self.as_of
    }

    /// The base currency the figures are in.
    #[must_use]
    pub fn currency(&self) -> &str {
        &self.currency
    }

    /// When the FX rate was observed.
    #[must_use]
    pub fn fx_as_of(&self) -> Option<DateTime<Utc>> {
        self.fx_as_of
    }

    /// Whether any non-base amount was converted.
    #[must_use]
    pub fn fx_estimated(&self) -> bool {
        self.fx_estimated
    }

    /// Cost basis of the open lots.
    #[must_use]
    pub fn net_invested(&self) -> Option<Decimal> {
        self.net_invested
    }

    /// Realised P&L.
    #[must_use]
    pub fn realized(&self) -> Option<Decimal> {
        self.realized
    }

    /// Unrealised P&L.
    #[must_use]
    pub fn unrealized(&self) -> Option<Decimal> {
        self.unrealized
    }

    /// Realised plus unrealised.
    #[must_use]
    pub fn total_return(&self) -> Option<Decimal> {
        self.total_return
    }

    /// Consumed plus remaining acquisition cost.
    #[must_use]
    pub fn total_acquisition_cost(&self) -> Option<Decimal> {
        self.total_acquisition_cost
    }

    /// The coverage share.
    #[must_use]
    pub fn coverage(&self) -> Decimal {
        self.coverage
    }

    /// The uncovered assets.
    #[must_use]
    pub fn coverage_gap(&self) -> &[String] {
        &self.coverage_gap
    }

    /// When the snapshot row was written.
    #[must_use]
    pub fn computed_at(&self) -> DateTime<Utc> {
        self.computed_at
    }
}
