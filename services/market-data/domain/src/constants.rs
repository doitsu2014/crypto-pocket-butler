//! The constants the business rules use. They are quoted in `rules.md`; changing
//! one is a requirements change, not a tuning knob.

/// The freshness bound of a `top100` asset.
pub const TOP100_BOUND_SECONDS: i64 = 300;
/// The freshness bound of a `tail` asset.
pub const TAIL_BOUND_SECONDS: i64 = 900;
/// Beyond this age an observation is never served.
pub const HARD_BOUND_SECONDS: i64 = 86_400;
/// How far ahead of `receivedAt` a source timestamp may be.
pub const CLOCK_TOLERANCE_SECONDS: i64 = 300;
/// The freshness bound of an FX reading.
pub const FX_BOUND_SECONDS: i64 = 3_600;
/// The single-flight lease a refresh run holds.
pub const RUN_LEASE_SECONDS: i64 = 300;
/// The interval at which a run renews its lease.
pub const RENEW_INTERVAL_SECONDS: i64 = 60;
/// The interval at which the worker sweeps expired leases.
pub const SWEEP_INTERVAL_SECONDS: i64 = 60;
/// How long observations and FX readings are retained.
pub const RETENTION_DAYS: i64 = 90;
/// The largest batch `getPrices` accepts.
pub const MAX_ASSET_IDS: usize = 1_000;
/// The longest canonical asset id accepted.
pub const MAX_ASSET_ID_CHARS: usize = 200;
/// The longest non-secret failure summary a run stores.
pub const MAX_ERROR_MESSAGE_CHARS: usize = 500;
/// The longest rejection reason an observation stores.
pub const MAX_REJECTION_REASON_CHARS: usize = 300;
