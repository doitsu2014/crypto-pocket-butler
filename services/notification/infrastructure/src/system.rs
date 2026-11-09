//! The system clock and identifier adapters.

use chrono::{DateTime, Utc};
use notification_application::ports::{Clock, IdGenerator};
use uuid::Uuid;

/// The production clock.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// The production identifier source.
#[derive(Debug, Default, Clone, Copy)]
pub struct UuidGenerator;

impl IdGenerator for UuidGenerator {
    fn next(&self) -> Uuid {
        Uuid::new_v4()
    }
}
