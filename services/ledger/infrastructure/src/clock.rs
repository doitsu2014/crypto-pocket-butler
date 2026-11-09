//! The system clock adapter.

use chrono::{DateTime, Utc};
use ledger_application::ports::Clock;

/// The production clock.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}
