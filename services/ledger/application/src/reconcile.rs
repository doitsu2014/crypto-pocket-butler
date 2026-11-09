//! C11 reconcile: rebuild the derived lots and P&L once, in batches bounded by
//! transaction volume, under a lease (NFR1.5, NFR20.4).

use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use ledger_domain::{LedgerSubjectId, RECONCILE_BATCH_TRANSACTIONS, RECONCILE_LEASE_SECONDS};

use crate::common::AppError;
use crate::ports::{Clock, ReconcileLease, SubjectRepository, TransactionRepository};
use crate::reads::PnlService;
use crate::rebuild::Rebuilder;

/// Whether the run executed or was already leased.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconcileStatus {
    /// The run executed.
    Completed,
    /// Another run held the lease.
    Skipped,
}

impl ReconcileStatus {
    /// The wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Skipped => "skipped",
        }
    }
}

/// The outcome of one reconcile run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileReport {
    /// Whether the run executed.
    pub status: ReconcileStatus,
    /// How many subjects were rebuilt.
    pub subjects: u64,
    /// How many transaction-volume batches were used.
    pub batches: u64,
    /// How many transactions were replayed.
    pub transactions: u64,
}

/// Rebuilds every subject's derived state under a lease.
pub struct ReconcileService {
    transactions: Arc<dyn TransactionRepository>,
    subjects: Arc<dyn SubjectRepository>,
    lease: Arc<dyn ReconcileLease>,
    rebuilder: Arc<Rebuilder>,
    pnl: Arc<PnlService>,
    clock: Arc<dyn Clock>,
}

impl ReconcileService {
    /// Creates the reconcile use case.
    #[must_use]
    pub fn new(
        transactions: Arc<dyn TransactionRepository>,
        subjects: Arc<dyn SubjectRepository>,
        lease: Arc<dyn ReconcileLease>,
        rebuilder: Arc<Rebuilder>,
        pnl: Arc<PnlService>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            transactions,
            subjects,
            lease,
            rebuilder,
            pnl,
            clock,
        }
    }

    /// Runs one reconcile. A busy lease returns [`ReconcileStatus::Skipped`].
    ///
    /// # Errors
    /// Returns [`AppError`] when the lease or a rebuild fails; the lease is
    /// released before the error is returned.
    pub async fn run(&self) -> Result<ReconcileReport, AppError> {
        let now = self.clock.now();
        let until = now + Duration::seconds(RECONCILE_LEASE_SECONDS);
        if !self.lease.acquire(now, until).await? {
            return Ok(ReconcileReport {
                status: ReconcileStatus::Skipped,
                subjects: 0,
                batches: 0,
                transactions: 0,
            });
        }
        match self.rebuild_all().await {
            Ok(report) => {
                self.lease.release(self.clock.now()).await?;
                Ok(report)
            }
            Err(error) => {
                if self.lease.release(self.clock.now()).await.is_err() {
                    tracing::error!("the reconcile lease could not be released");
                }
                Err(error)
            }
        }
    }

    async fn rebuild_all(&self) -> Result<ReconcileReport, AppError> {
        let subjects = self.transactions.list_subjects().await?;
        let mut rebuilt = 0u64;
        let mut batches = 0u64;
        let mut transactions = 0u64;
        let mut batch_transactions = 0u64;
        for subject in subjects {
            let count = self.transactions.count_for_subject(subject).await?;
            if batch_transactions > 0 && batch_transactions + count > RECONCILE_BATCH_TRANSACTIONS {
                batches += 1;
                batch_transactions = 0;
            }
            batch_transactions += count;
            self.rebuild_subject(subject).await?;
            rebuilt += 1;
            transactions += count;
        }
        if batch_transactions > 0 {
            batches += 1;
        }
        Ok(ReconcileReport {
            status: ReconcileStatus::Completed,
            subjects: rebuilt,
            batches,
            transactions,
        })
    }

    async fn rebuild_subject(&self, subject: LedgerSubjectId) -> Result<(), AppError> {
        self.rebuilder.rebuild_lots(subject).await?;
        if let Some(row) = self.subjects.find_by_id(subject).await? {
            if let Some(user_id) = row.user_id() {
                // An anonymised subject has no user id and therefore no C5
                // holdings; its lots are rebuilt and no snapshot is written.
                let _ = self.pnl.report(user_id, None).await?;
            }
        }
        Ok(())
    }

    /// When the last completed run finished.
    ///
    /// # Errors
    /// Returns [`AppError`] when the read fails.
    pub async fn last_completed(&self) -> Result<Option<DateTime<Utc>>, AppError> {
        Ok(self.lease.last_completed().await?)
    }
}
