//! Retry classification shared by the HTTP adapters.

use std::time::Duration;

/// Whether a status is worth retrying.
#[must_use]
pub fn is_transient(status: u16) -> bool {
    status == 408 || status == 429 || (500..=599).contains(&status)
}

/// The bounded backoff between attempts: 1 s, then 2 s.
pub async fn pause_before_retry(attempt: u32) {
    let seconds = 1_u64 << attempt.min(2);
    tokio::time::sleep(Duration::from_secs(seconds)).await;
}
