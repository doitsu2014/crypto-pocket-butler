//! Constants fixed by `functional-design/rules.md` and the NFR design.

/// A transfer pairs only with a counter-entry of the same asset whose quantity
/// is within one percent (BR6.1).
pub const TRANSFER_QUANTITY_TOLERANCE_BPS: i64 = 100;
/// A transfer pairs only with a counter-entry within twenty-four hours (BR6.1).
pub const TRANSFER_WINDOW_HOURS: i64 = 24;
/// An ingested timestamp may be up to five minutes in the future before it is
/// clamped to the record time (NFR1.7).
pub const FUTURE_SKEW_SECONDS: i64 = 300;
/// Default transaction page size.
pub const LIST_PAGE_DEFAULT: u32 = 50;
/// Largest transaction page a caller may request.
pub const LIST_PAGE_MAX: u32 = 200;
/// Reconcile processes at most five thousand transactions per batch.
pub const RECONCILE_BATCH_TRANSACTIONS: u64 = 5_000;
/// The reconcile lease is held for five minutes.
pub const RECONCILE_LEASE_SECONDS: i64 = 300;
/// C10 audit retry delays in seconds: one, two, then five.
pub const AUDIT_RETRY_SECONDS: [u64; 3] = [1, 2, 5];
/// How long an official range stays whole. Placeholder for retention policy.
pub const RETENTION_YEARS: i64 = 2;
