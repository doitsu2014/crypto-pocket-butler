//! NFR20.6: the nightly sweep deleting alerts and delivery attempts older than
//! twelve months.

use std::sync::Arc;

use notification_domain::cutoff_twelve_months_before;

use crate::ports::{AlertRepository, AttemptRepository, Clock, Pause, RepositoryError};

/// Rows removed by one sweep.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RetentionReport {
    /// Alerts removed.
    pub alerts_deleted: u64,
    /// Delivery attempts removed.
    pub attempts_deleted: u64,
}

/// Why a sweep failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RetentionError {
    /// A storage failure.
    #[error("retention storage failure: {0}")]
    Repository(#[from] RepositoryError),
}

/// The delete batch size. Bounded so a sweep never holds a long transaction.
const RETENTION_BATCH_SIZE: u32 = 10_000;
/// The most batches one sweep runs.
const RETENTION_MAX_BATCHES: u32 = 1_000;

/// Runs the retention sweep in bounded batches.
pub struct RetentionService {
    alerts: Arc<dyn AlertRepository>,
    attempts: Arc<dyn AttemptRepository>,
    clock: Arc<dyn Clock>,
    pause: Arc<dyn Pause>,
}

impl RetentionService {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        alerts: Arc<dyn AlertRepository>,
        attempts: Arc<dyn AttemptRepository>,
        clock: Arc<dyn Clock>,
        pause: Arc<dyn Pause>,
    ) -> Self {
        Self {
            alerts,
            attempts,
            clock,
            pause,
        }
    }

    /// Deletes attempts and then alerts older than twelve months.
    ///
    /// # Errors
    /// Returns [`RetentionError::Repository`] when a batch cannot be deleted.
    pub async fn run(&self) -> Result<RetentionReport, RetentionError> {
        let cutoff = cutoff_twelve_months_before(self.clock.now());
        let mut report = RetentionReport::default();

        // Attempts first: an attempt row references an alert, and the alert
        // delete would otherwise cascade the same rows in an unbounded sweep.
        for _ in 0..RETENTION_MAX_BATCHES {
            let deleted = self
                .attempts
                .delete_before(cutoff, RETENTION_BATCH_SIZE)
                .await?;
            report.attempts_deleted += deleted;
            if deleted < u64::from(RETENTION_BATCH_SIZE) {
                break;
            }
            self.pause
                .pause_for(std::time::Duration::from_millis(200))
                .await;
        }

        for _ in 0..RETENTION_MAX_BATCHES {
            let deleted = self
                .alerts
                .delete_before(cutoff, RETENTION_BATCH_SIZE)
                .await?;
            report.alerts_deleted += deleted;
            if deleted < u64::from(RETENTION_BATCH_SIZE) {
                break;
            }
            self.pause
                .pause_for(std::time::Duration::from_millis(200))
                .await;
        }

        Ok(report)
    }
}
