//! WF5: the six-month event and ninety-day consent-check sweep (BR5.1).

use std::sync::Arc;

use analytics_domain::{
    cutoff_consent_checks_before, cutoff_six_months_before, RETENTION_BATCH_SIZE,
    RETENTION_MAX_BATCHES,
};

use crate::ports::{Clock, ConsentRepository, EventRepository, Pause, RepositoryError};

/// What one sweep removed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RetentionReport {
    /// Raw events removed.
    pub events_deleted: u64,
    /// Consent-check rows removed.
    pub consent_checks_deleted: u64,
}

/// Why a sweep failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RetentionError {
    /// A storage failure.
    #[error("retention storage failure: {0}")]
    Repository(#[from] RepositoryError),
}

/// Runs the retention sweep in bounded batches.
pub struct RetentionService {
    events: Arc<dyn EventRepository>,
    consents: Arc<dyn ConsentRepository>,
    clock: Arc<dyn Clock>,
    pause: Arc<dyn Pause>,
    batch_size: u32,
    max_batches: u32,
}

impl RetentionService {
    /// Creates the use case with the production batch defaults.
    #[must_use]
    pub fn new(
        events: Arc<dyn EventRepository>,
        consents: Arc<dyn ConsentRepository>,
        clock: Arc<dyn Clock>,
        pause: Arc<dyn Pause>,
    ) -> Self {
        Self {
            events,
            consents,
            clock,
            pause,
            batch_size: RETENTION_BATCH_SIZE,
            max_batches: RETENTION_MAX_BATCHES,
        }
    }

    /// Overrides the batch bounds, used by tests.
    #[must_use]
    pub fn with_bounds(mut self, batch_size: u32, max_batches: u32) -> Self {
        self.batch_size = batch_size.max(1);
        self.max_batches = max_batches.max(1);
        self
    }

    /// Runs one sweep: raw events older than six months and consent checks
    /// older than ninety days. Aggregates and flags are untouched.
    ///
    /// # Errors
    /// Returns [`RetentionError::Repository`] when a batch cannot be deleted.
    pub async fn run(&self) -> Result<RetentionReport, RetentionError> {
        let now = self.clock.now();
        let event_cutoff = cutoff_six_months_before(now);
        let consent_cutoff = cutoff_consent_checks_before(now);
        let mut report = RetentionReport::default();

        for _ in 0..self.max_batches {
            let deleted = self
                .events
                .delete_before(event_cutoff, self.batch_size)
                .await?;
            report.events_deleted += deleted;
            if deleted < u64::from(self.batch_size) {
                break;
            }
            self.pause.pause().await;
        }

        for _ in 0..self.max_batches {
            let deleted = self
                .consents
                .delete_before(consent_cutoff, self.batch_size)
                .await?;
            report.consent_checks_deleted += deleted;
            if deleted < u64::from(self.batch_size) {
                break;
            }
            self.pause.pause().await;
        }

        Ok(report)
    }
}
