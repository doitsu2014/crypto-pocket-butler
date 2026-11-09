//! The retention worker's pause adapter, so batch pacing is configuration rather
//! than a constant buried in the sweep.

use std::time::Duration;

use async_trait::async_trait;
use audit_application::ports::Pause;

/// Sleeps between retention batches.
#[derive(Debug, Clone, Copy)]
pub struct TokioPause {
    millis: u64,
}

impl TokioPause {
    /// Creates a pause adapter.
    #[must_use]
    pub fn new(millis: u64) -> Self {
        Self { millis }
    }
}

#[async_trait]
impl Pause for TokioPause {
    async fn pause(&self) {
        if self.millis > 0 {
            tokio::time::sleep(Duration::from_millis(self.millis)).await;
        }
    }
}
