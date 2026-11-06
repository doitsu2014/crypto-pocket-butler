//! A fixed-window, in-memory rate limiter.
//!
//! Per-user, per-IP, and per-region limits are the target; the walking skeleton
//! keys on the forwarded client IP and whether a bearer token is present, with a
//! stricter anonymous budget. The edge/CDN enforces the regional dimension.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const WINDOW: Duration = Duration::from_secs(60);
const MAX_TRACKED_KEYS: usize = 10_000;

#[derive(Debug)]
struct Window {
    started: Instant,
    count: u32,
}

/// Fixed-window request limiter.
#[derive(Debug)]
pub struct RateLimiter {
    authenticated_limit: u32,
    anonymous_limit: u32,
    windows: Mutex<HashMap<String, Window>>,
}

impl RateLimiter {
    /// Creates a limiter. A limit of zero disables that class.
    #[must_use]
    pub fn new(authenticated_limit: u32, anonymous_limit: u32) -> Self {
        Self {
            authenticated_limit,
            anonymous_limit,
            windows: Mutex::new(HashMap::new()),
        }
    }

    /// Records one request and returns the `Retry-After` seconds when the budget
    /// is exhausted.
    ///
    /// # Errors
    /// Returns the number of seconds to wait before retrying.
    pub fn check(&self, key: &str, authenticated: bool) -> Result<(), u64> {
        let limit = if authenticated {
            self.authenticated_limit
        } else {
            self.anonymous_limit
        };
        if limit == 0 {
            return Ok(());
        }

        let now = Instant::now();
        let mut windows = self
            .windows
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        if windows.len() > MAX_TRACKED_KEYS {
            windows.retain(|_, window| now.duration_since(window.started) < WINDOW);
            if windows.len() > MAX_TRACKED_KEYS {
                windows.clear();
            }
        }

        let window = windows.entry(key.to_string()).or_insert(Window {
            started: now,
            count: 0,
        });
        if now.duration_since(window.started) >= WINDOW {
            window.started = now;
            window.count = 0;
        }
        if window.count >= limit {
            let remaining = WINDOW
                .as_secs()
                .saturating_sub(now.duration_since(window.started).as_secs())
                .max(1);
            return Err(remaining);
        }
        window.count += 1;
        Ok(())
    }
}
