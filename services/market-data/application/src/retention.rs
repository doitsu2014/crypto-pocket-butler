//! WF retention (BR5.3): observations and FX readings are pruned after 90 days,
//! keeping the newest row per asset/source and per pair so stale prices and rates
//! stay servable.

use std::sync::Arc;

use chrono::Duration;

use crate::common::AppError;
use crate::ports::{Clock, FxRepository, ObservationRepository};
use market_data_domain::RETENTION_DAYS;

/// What one retention sweep removed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RetentionReport {
    /// Observation rows removed.
    pub observations: u64,
    /// FX reading rows removed.
    pub fx_rates: u64,
}

/// Prunes history on the retention policy.
pub struct RetentionService {
    observations: Arc<dyn ObservationRepository>,
    fx: Arc<dyn FxRepository>,
    clock: Arc<dyn Clock>,
}

impl RetentionService {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        observations: Arc<dyn ObservationRepository>,
        fx: Arc<dyn FxRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            observations,
            fx,
            clock,
        }
    }

    /// Runs one sweep.
    ///
    /// # Errors
    /// Returns a repository refusal when a prune cannot run.
    pub async fn sweep(&self) -> Result<RetentionReport, AppError> {
        let cutoff = self.clock.now() - Duration::days(RETENTION_DAYS);
        let observations = self
            .observations
            .prune_before(cutoff)
            .await
            .map_err(AppError::from_repository)?;
        let fx_rates = self
            .fx
            .prune_before(cutoff)
            .await
            .map_err(AppError::from_repository)?;
        Ok(RetentionReport {
            observations,
            fx_rates,
        })
    }
}
