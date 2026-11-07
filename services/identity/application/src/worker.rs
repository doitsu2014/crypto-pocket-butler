//! The privacy supervisor: one pass of export assembly plus one pass of deletion
//! execution. The API's wake endpoint and the 60-second sweep both call it.

use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::deletion::DeletionWorker;
use crate::export::ExportWorker;

/// What one supervisor pass processed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WorkerReport {
    /// Export requests processed.
    pub exports_processed: usize,
    /// Deletion requests processed.
    pub deletions_processed: usize,
}

/// Runs the export and deletion workers together.
pub struct PrivacyWorker {
    exports: Arc<ExportWorker>,
    deletions: Arc<DeletionWorker>,
}

impl PrivacyWorker {
    /// Creates the supervisor from the two workers.
    #[must_use]
    pub fn new(exports: Arc<ExportWorker>, deletions: Arc<DeletionWorker>) -> Self {
        Self { exports, deletions }
    }

    /// Runs one pass. A failure in one worker is logged and does not stop the
    /// other: the records are status-driven and the next sweep resumes them.
    pub async fn run_once(&self, worker: &str, now: DateTime<Utc>) -> WorkerReport {
        let exports_processed = match self.exports.run_once(worker, now).await {
            Ok(count) => count,
            Err(error) => {
                tracing::error!(%error, "export worker pass failed");
                0
            }
        };
        let deletions_processed = match self.deletions.run_once(worker, now).await {
            Ok(count) => count,
            Err(error) => {
                tracing::error!(%error, "deletion worker pass failed");
                0
            }
        };
        WorkerReport {
            exports_processed,
            deletions_processed,
        }
    }
}
