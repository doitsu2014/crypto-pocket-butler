//! The `tokio` pause adapter.

use std::time::Duration;

use async_trait::async_trait;
use ledger_application::ports::Pause;

/// Pauses with `tokio::time::sleep`.
#[derive(Debug, Default, Clone, Copy)]
pub struct TokioPause;

impl TokioPause {
    /// Creates the adapter.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Pause for TokioPause {
    async fn pause_for(&self, duration: Duration) {
        tokio::time::sleep(duration).await;
    }
}
