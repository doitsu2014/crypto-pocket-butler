//! The system clock adapter.

use asset_registry_application::ports::Clock;
use chrono::{DateTime, Utc};

/// The production clock.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}
