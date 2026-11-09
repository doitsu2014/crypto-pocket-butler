//! Rebuilding the derived lots from the append-only log, and the shared view
//! used by the P&L calculator.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use ledger_domain::{
    effective_transaction, replay, Correction, CostBasisLot, EffectiveTransaction, FxRates,
    LedgerSubjectId, PnlSnapshot, ReplayOutcome, Transaction, TransactionId,
};
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::common::AppError;
use crate::ports::{
    Clock, CorrectionRepository, LotRepository, SnapshotRepository, TransactionRepository,
    TransferPairRepository,
};

/// Reads the append-only log and replays it into lots and realised P&L.
pub struct Rebuilder {
    transactions: Arc<dyn TransactionRepository>,
    corrections: Arc<dyn CorrectionRepository>,
    lots: Arc<dyn LotRepository>,
    snapshots: Arc<dyn SnapshotRepository>,
    pairs: Arc<dyn TransferPairRepository>,
    clock: Arc<dyn Clock>,
}

impl Rebuilder {
    /// Creates the rebuilder.
    #[must_use]
    pub fn new(
        transactions: Arc<dyn TransactionRepository>,
        corrections: Arc<dyn CorrectionRepository>,
        lots: Arc<dyn LotRepository>,
        snapshots: Arc<dyn SnapshotRepository>,
        pairs: Arc<dyn TransferPairRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            transactions,
            corrections,
            lots,
            snapshots,
            pairs,
            clock,
        }
    }

    /// Replays a subject into lots and realised P&L using the provided FX table.
    ///
    /// # Errors
    /// Returns [`AppError`] when a read fails, a stored correction is invalid,
    /// or an FX rate is missing.
    pub async fn replay(
        &self,
        subject: LedgerSubjectId,
        fx: &FxRates,
    ) -> Result<ReplayOutcome, AppError> {
        let transactions = self.transactions.list_all(subject).await?;
        let corrections = self.corrections.list_for_subject(subject).await?;
        let pairs = self.pairs.list_for_subject(subject).await?;
        let effective = build_effective(&transactions, &corrections)?;
        Ok(replay(subject, &effective, &pairs, fx)?)
    }

    /// Persists the rebuilt open lots.
    ///
    /// # Errors
    /// Returns [`AppError`] when the write fails.
    pub async fn save_lots(
        &self,
        subject: LedgerSubjectId,
        outcome: &ReplayOutcome,
    ) -> Result<(), AppError> {
        self.lots
            .replace_for_subject(subject, &outcome.lots)
            .await?;
        Ok(())
    }

    /// Rebuilds only the lots, with no currency conversion (quantities and
    /// acquisition costs are preserved in their own currency).
    ///
    /// # Errors
    /// Returns [`AppError`] when a read or write fails.
    pub async fn rebuild_lots(&self, subject: LedgerSubjectId) -> Result<(), AppError> {
        let outcome = self.replay(subject, &FxRates::unconverted()).await?;
        self.save_lots(subject, &outcome).await
    }

    /// The quantity a subject currently holds for one asset, used to refuse a
    /// user-recorded sell beyond known holdings (NFR1.7).
    ///
    /// # Errors
    /// Returns [`AppError`] when a read fails or a stored correction is invalid.
    pub async fn held_quantity(
        &self,
        subject: LedgerSubjectId,
        asset_id: &str,
    ) -> Result<Decimal, AppError> {
        let outcome = self.replay(subject, &FxRates::unconverted()).await?;
        Ok(outcome.held_quantity(asset_id))
    }

    /// The latest transaction record time for a subject.
    ///
    /// # Errors
    /// Returns [`AppError`] when the read fails.
    pub async fn last_recorded_at(
        &self,
        subject: LedgerSubjectId,
    ) -> Result<Option<DateTime<Utc>>, AppError> {
        Ok(self.transactions.last_recorded_at(subject).await?)
    }

    /// The latest correction time for a subject.
    ///
    /// # Errors
    /// Returns [`AppError`] when the read fails.
    pub async fn last_corrected_at(
        &self,
        subject: LedgerSubjectId,
    ) -> Result<Option<DateTime<Utc>>, AppError> {
        Ok(self.corrections.last_corrected_at(subject).await?)
    }

    /// The latest snapshot for a subject and portfolio scope.
    ///
    /// # Errors
    /// Returns [`AppError`] when the read fails.
    pub async fn latest_snapshot(
        &self,
        subject: LedgerSubjectId,
        portfolio_id: Option<Uuid>,
    ) -> Result<Option<PnlSnapshot>, AppError> {
        Ok(self.snapshots.find_latest(subject, portfolio_id).await?)
    }

    /// The stored open lots for a subject.
    ///
    /// # Errors
    /// Returns [`AppError`] when the read fails.
    pub async fn open_lots(&self, subject: LedgerSubjectId) -> Result<Vec<CostBasisLot>, AppError> {
        Ok(self.lots.list_open(subject).await?)
    }

    /// Every asset the subject's ledger mentions.
    ///
    /// # Errors
    /// Returns [`AppError`] when the read fails.
    pub async fn asset_ids(&self, subject: LedgerSubjectId) -> Result<BTreeSet<String>, AppError> {
        let transactions = self.transactions.list_all(subject).await?;
        Ok(transactions
            .iter()
            .map(|transaction| transaction.asset_id().to_string())
            .collect())
    }

    /// Every price currency the subject's ledger mentions.
    ///
    /// # Errors
    /// Returns [`AppError`] when the read fails.
    pub async fn currencies(&self, subject: LedgerSubjectId) -> Result<BTreeSet<String>, AppError> {
        let transactions = self.transactions.list_all(subject).await?;
        Ok(transactions
            .iter()
            .map(|transaction| transaction.currency().to_string())
            .collect())
    }

    /// Persists a P&L snapshot.
    ///
    /// # Errors
    /// Returns [`AppError`] when the write fails.
    pub async fn save_snapshot(&self, snapshot: &PnlSnapshot) -> Result<(), AppError> {
        self.snapshots.upsert(snapshot).await?;
        Ok(())
    }

    /// The current time.
    #[must_use]
    pub fn now(&self) -> DateTime<Utc> {
        self.clock.now()
    }
}

/// Applies corrections to build the view the lot engine replays (BR2.3).
///
/// # Errors
/// Returns [`AppError`] when a stored correction's value is malformed.
pub fn build_effective(
    transactions: &[Transaction],
    corrections: &[Correction],
) -> Result<Vec<EffectiveTransaction>, AppError> {
    let mut by_transaction: BTreeMap<TransactionId, Vec<Correction>> = BTreeMap::new();
    for correction in corrections {
        by_transaction
            .entry(correction.transaction_id())
            .or_default()
            .push(correction.clone());
    }
    transactions
        .iter()
        .map(|transaction| {
            let applied = by_transaction
                .get(&transaction.id())
                .map_or(&[][..], Vec::as_slice);
            effective_transaction(transaction, applied).map_err(AppError::from)
        })
        .collect()
}
