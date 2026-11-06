//! A small per-replica fixed-window limiter.
//!
//! Replicas share no cache and the project forbids adding one, so the effective
//! ceiling is the stated figure times the replica count. The stated figures are a
//! third of the intended per-user ceiling; this is written down in the security
//! design rather than glossed over.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// The largest number of windows held before the map is reset.
const MAX_WINDOWS: usize = 10_000;

/// A per-key fixed-window counter.
#[derive(Debug)]
pub struct RateLimiter {
    windows: Mutex<HashMap<String, Window>>,
}

#[derive(Debug, Clone, Copy)]
struct Window {
    started: Instant,
    count: u32,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self {
            windows: Mutex::new(HashMap::new()),
        }
    }
}

impl RateLimiter {
    /// Counts one request against `key`, returning `false` once the window is full.
    pub fn allow(&self, key: &str, limit: u32, window: Duration) -> bool {
        let mut windows = match self.windows.lock() {
            Ok(windows) => windows,
            Err(poisoned) => poisoned.into_inner(),
        };
        if windows.len() > MAX_WINDOWS {
            windows.clear();
        }
        let now = Instant::now();
        let entry = windows.entry(key.to_string()).or_insert(Window {
            started: now,
            count: 0,
        });
        if now.duration_since(entry.started) >= window {
            entry.started = now;
            entry.count = 0;
        }
        if entry.count >= limit {
            return false;
        }
        entry.count += 1;
        true
    }
}

/// The per-replica limits.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Appends per minute per emitting service.
    pub appends_per_minute: u32,
    /// Admin queries per minute per caller.
    pub queries_per_minute: u32,
    /// Exports per hour per caller.
    pub exports_per_hour: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            appends_per_minute: 1_000,
            queries_per_minute: 30,
            exports_per_hour: 5,
        }
    }
}
