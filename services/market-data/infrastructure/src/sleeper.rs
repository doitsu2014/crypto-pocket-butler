//! The production retry sleeper.

use async_trait::async_trait;
use chrono::Duration;
use market_data_application::ports::Sleeper;

/// Sleeps with the Tokio timer.
#[derive(Debug, Default, Clone, Copy)]
pub struct TokioSleeper;

#[async_trait]
impl Sleeper for TokioSleeper {
    async fn sleep(&self, duration: Duration) {
        if let Ok(millis) = u64::try_from(duration.num_milliseconds()) {
            tokio::time::sleep(std::time::Duration::from_millis(millis)).await;
        }
    }
}
