//! The bounded retry used by every outbound HTTP adapter (reliability design):
//! 3 attempts, exponential backoff with jitter, transient failures only.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Whether a response status is worth retrying. A 4xx is not.
#[must_use]
pub fn is_transient(status: u16) -> bool {
    status == 429 || status >= 500
}

/// Waits before the next attempt: 100 ms, 200 ms, 400 ms plus jitter.
pub async fn pause_before_retry(attempt: u32) {
    let base = 100u64 << attempt.min(3);
    let jitter = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| u64::from(elapsed.subsec_millis()) % 50)
        .unwrap_or(0);
    tokio::time::sleep(Duration::from_millis(base + jitter)).await;
}
