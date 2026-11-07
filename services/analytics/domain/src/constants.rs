//! Constants fixed by `functional-design/rules.md`.

/// A salt is 32 random bytes held in Supabase Vault.
pub const SALT_BYTES: usize = 32;
/// Every salt version stays in Vault for twelve months, longer than the event
/// retention window, so pseudonyms, cohorts, rollout, and erasure stay stable.
pub const SALT_RETENTION_MONTHS: i64 = 12;
/// A cohort below ten distinct users is suppressed (BR3.2).
pub const SUPPRESSION_MIN_USERS: u64 = 10;
/// Raw events are kept six months (BR5.1).
pub const EVENT_RETENTION_MONTHS: i64 = 6;
/// Consent-check rows are kept ninety days (BR5.1).
pub const CONSENT_CHECK_RETENTION_DAYS: i64 = 90;
/// Feature flags are cached in process for thirty seconds.
pub const FLAG_CACHE_SECONDS: i64 = 30;
/// The longest event name the catalogue permits.
pub const MAX_EVENT_NAME_LEN: usize = 64;
/// The longest property value that may be stored.
pub const MAX_PROPERTY_VALUE_LEN: usize = 256;
/// The longest flag key that may be stored.
pub const MAX_FLAG_KEY_LEN: usize = 80;
/// The longest owner label that may be stored.
pub const MAX_OWNER_LEN: usize = 120;
/// The most events one daily aggregation window may read. The window is nine
/// days (closed day, seven-day weekly-active look-back, and one-day activation
/// look-ahead); at the launch model that is roughly 300 000 events. Exceeding
/// the cap fails the run instead of truncating the closed day.
pub const MAX_DAILY_WINDOW_EVENTS: u32 = 500_000;
/// The most events one single-day retention cohort window may read.
pub const MAX_COHORT_WINDOW_EVENTS: u32 = 200_000;
/// The default retention batch size.
pub const RETENTION_BATCH_SIZE: u32 = 10_000;
/// The default maximum batches one sweep runs.
pub const RETENTION_MAX_BATCHES: u32 = 1_000;
