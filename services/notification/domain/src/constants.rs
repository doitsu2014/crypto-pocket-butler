//! Constants fixed by `functional-design/rules.md` and `nfr-requirements/`.

/// The longest idempotency key an alert may carry (BR1.1).
pub const MAX_IDEMPOTENCY_KEY_LEN: usize = 128;
/// The longest allow-listed payload value that may be stored (BR5.1).
pub const MAX_PAYLOAD_VALUE_LEN: usize = 256;
/// The most delivery attempts per alert and channel: the initial one plus two
/// retries (BR4.1).
pub const MAX_ATTEMPTS: u8 = 3;
/// The retry delays after the first and second failed attempt (BR4.1).
pub const RETRY_DELAY_SECONDS: [i64; 2] = [60, 300];
/// Alerts and delivery attempts are kept twelve months (NFR20.3, NFR20.6).
pub const RETENTION_MONTHS: i64 = 12;
/// The delivery batch size one run processes at a time (NFR20.4).
pub const DELIVERY_BATCH_SIZE: u32 = 200;
/// The provider concurrency ceiling (performance design).
pub const PROVIDER_CONCURRENCY: usize = 20;
/// The most email calls one run may make: 1,200 calls at 240/min is five
/// minutes, the run budget (NFR2.3).
pub const MAX_EMAILS_PER_RUN: u32 = 1_200;
/// The most batches one run drains before leaving the rest to the next run.
pub const MAX_RUN_BATCHES: u32 = 10;
/// The delivery lease lifetime. It outlives the five-minute run budget so a
/// second replica can never take over mid-run.
pub const LEASE_SECONDS: i64 = 600;
/// The worker polls the durable wake flag and the queue every five seconds
/// (NFR2.5).
pub const WAKE_POLL_SECONDS: u64 = 5;
/// C10 takes three attempts with 1 s, 2 s, then 5 s between them (NFR1.5).
pub const AUDIT_RETRY_SECONDS: [u64; 3] = [1, 2, 5];
/// The default in-app feed page size.
pub const DEFAULT_FEED_LIMIT: u32 = 50;
/// The largest in-app feed page.
pub const MAX_FEED_LIMIT: u32 = 200;
