//! Bounded retry helpers shared by the provider clients.

use std::time::Duration;

/// Whether a response status is worth retrying.
#[must_use]
pub fn is_transient(status: u16) -> bool {
    matches!(status, 408 | 425 | 429 | 500 | 502 | 503 | 504)
}

/// Pauses before a retry with bounded exponential backoff and no jitter source
/// dependency (the attempt index supplies the bound).
pub async fn pause_before_retry(attempt: u32) {
    let millis = 100u64.saturating_mul(1u64 << attempt.min(4));
    tokio::time::sleep(Duration::from_millis(millis)).await;
}
