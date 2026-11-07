//! The shared Postgres token bucket's arithmetic (BR4.3).
//!
//! The bucket holds at most `capacity` tokens (the source's rate limit per
//! minute) and refills continuously at `capacity / 60` tokens per second. The
//! persistence adapter performs the refill and the take in one atomic statement;
//! these pure helpers are the rule it implements.

/// Refills a bucket for `elapsed_seconds` of wall-clock time, capped at
/// `capacity`.
#[must_use]
pub fn refill_tokens(tokens: i64, capacity: i64, elapsed_seconds: i64) -> i64 {
    let capacity = capacity.max(0);
    let elapsed = elapsed_seconds.max(0);
    let refill = (elapsed.saturating_mul(capacity)) / 60;
    (tokens.max(0) + refill).min(capacity)
}

/// Refills then tries to take one token. Returns whether a token was taken and
/// the bucket's remaining tokens.
#[must_use]
pub fn bucket_try_take(tokens: i64, capacity: i64, elapsed_seconds: i64) -> (bool, i64) {
    let available = refill_tokens(tokens, capacity, elapsed_seconds);
    if available >= 1 {
        (true, available - 1)
    } else {
        (false, available)
    }
}
