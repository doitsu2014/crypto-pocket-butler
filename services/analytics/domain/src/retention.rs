//! BR5.1: the two retention cutoffs.

use chrono::{DateTime, Duration, Months, Utc};

use crate::constants::{CONSENT_CHECK_RETENTION_DAYS, EVENT_RETENTION_MONTHS};

/// The cutoff for raw events: six calendar months before `now`.
#[must_use]
pub fn cutoff_six_months_before(now: DateTime<Utc>) -> DateTime<Utc> {
    now.checked_sub_months(Months::new(
        u32::try_from(EVENT_RETENTION_MONTHS).unwrap_or(6),
    ))
    .unwrap_or_else(|| now - Duration::days(183))
}

/// The cutoff for consent-check rows: ninety days before `now`.
#[must_use]
pub fn cutoff_consent_checks_before(now: DateTime<Utc>) -> DateTime<Utc> {
    now - Duration::days(CONSENT_CHECK_RETENTION_DAYS)
}
