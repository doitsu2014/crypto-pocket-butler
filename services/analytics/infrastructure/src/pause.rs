//! A tokio pause between retention batches.

use analytics_application::ports::Pause;
use async_trait::async_trait;

/// Sleeps between batches. Tests use a no-op instead.
#[derive(Debug, Clone, Copy)]
pub struct TokioPause {
    millis: u64,
}

impl TokioPause {
    /// Creates a pause of `millis` milliseconds.
    #[must_use]
    pub fn new(millis: u64) -> Self {
        Self { millis }
    }
}

#[async_trait]
impl Pause for TokioPause {
    async fn pause(&self) {
        if self.millis > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(self.millis)).await;
        }
    }
}
