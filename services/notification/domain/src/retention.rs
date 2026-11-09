//! The twelve-month retention cutoff (NFR20.3, NFR20.6).

use chrono::{DateTime, Duration, Months, Utc};

use crate::constants::RETENTION_MONTHS;

/// The cutoff for alerts and delivery attempts: twelve calendar months before
/// `now`.
#[must_use]
pub fn cutoff_twelve_months_before(now: DateTime<Utc>) -> DateTime<Utc> {
    now.checked_sub_months(Months::new(u32::try_from(RETENTION_MONTHS).unwrap_or(12)))
        .unwrap_or_else(|| now - Duration::days(365))
}
