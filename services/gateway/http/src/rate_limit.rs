//! A fixed-window, in-memory rate limiter.
//!
//! Per-user, per-IP, and per-region limits are the target; the walking skeleton
//! keys on the forwarded client IP and whether a bearer token is present, with a
//! stricter anonymous budget. The edge/CDN enforces the regional dimension.
//! A second bounded map holds declared per-route budgets keyed on the bearer
//! token's SHA-256 (NFR11.1); the token itself is never stored as a key.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

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
    route_windows: Mutex<HashMap<String, Window>>,
}

impl RateLimiter {
    /// Creates a limiter. A limit of zero disables that class.
    #[must_use]
    pub fn new(authenticated_limit: u32, anonymous_limit: u32) -> Self {
        Self {
            authenticated_limit,
            anonymous_limit,
            windows: Mutex::new(HashMap::new()),
            route_windows: Mutex::new(HashMap::new()),
        }
    }

    /// Records one request in the shared per-caller-address window and returns
    /// the `Retry-After` seconds when the budget is exhausted.
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
        Self::record(&self.windows, key, limit)
    }

    /// Records one request in a declared route's own budget (NFR11.1).
    ///
    /// The bucket is keyed on the route label plus a hash of the bearer token,
    /// never on the `sub` claim and never on the address: two tokens for the
    /// same person are separate budgets, and a token flood cannot evade the IP
    /// window because [`RateLimiter::check`] still runs. The map is separate
    /// from the address windows so neither evicts the other.
    ///
    /// # Errors
    /// Returns the number of seconds to wait before retrying.
    pub fn check_route(&self, route: &str, token: &str, limit: u32) -> Result<(), u64> {
        if limit == 0 {
            return Ok(());
        }
        let digest = Sha256::digest(token.as_bytes());
        let key = format!("{route}:{digest:x}");
        Self::record(&self.route_windows, &key, limit)
    }

    fn record(windows: &Mutex<HashMap<String, Window>>, key: &str, limit: u32) -> Result<(), u64> {
        let now = Instant::now();
        let mut windows = windows
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
