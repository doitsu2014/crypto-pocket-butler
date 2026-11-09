//! A tokio pause between audit retries and retention batches.

use std::time::Duration;

use async_trait::async_trait;
use notification_application::ports::Pause;

/// Sleeps for the requested duration. Tests use a recording no-op instead.
#[derive(Debug, Default, Clone, Copy)]
pub struct TokioPause;

#[async_trait]
impl Pause for TokioPause {
    async fn pause_for(&self, duration: Duration) {
        if !duration.is_zero() {
            tokio::time::sleep(duration).await;
        }
    }
}
