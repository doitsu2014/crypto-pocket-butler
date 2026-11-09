//! Notification domain: the closed type set, per-type payload allow-lists, the
//! content guard, preferences and their defaults, the stored alert and delivery
//! attempt, retry bounds, and the retention cutoff.
//!
//! This crate holds entities, value types, and business rules only. It imports
//! no framework and no sibling crate.

pub mod alert;
pub mod attempt;
pub mod audit;
pub mod constants;
pub mod content;
pub mod error;
pub mod ids;
pub mod preference;
pub mod retention;

pub use alert::{
    sanitise_payload, Alert, AlertStatus, NotificationType, RequestedChannel, SanitisedPayload,
};
pub use attempt::{
    alert_status, next_attempt_at, summarise, AttemptStatus, ChannelOutcome, DeliveryAttempt,
    ErrorCategory,
};
pub use constants::{
    AUDIT_RETRY_SECONDS, DEFAULT_FEED_LIMIT, DELIVERY_BATCH_SIZE, LEASE_SECONDS, MAX_ATTEMPTS,
    MAX_EMAILS_PER_RUN, MAX_FEED_LIMIT, MAX_IDEMPOTENCY_KEY_LEN, MAX_PAYLOAD_VALUE_LEN,
    MAX_RUN_BATCHES, PROVIDER_CONCURRENCY, RETENTION_MONTHS, RETRY_DELAY_SECONDS,
    WAKE_POLL_SECONDS,
};
pub use content::{is_canonical_uuid, is_sensitive};
pub use error::DomainError;
pub use ids::{AlertId, DeliveryAttemptId, PreferenceId, UserId};
pub use preference::{
    default_enabled, effective_enabled, honoured, is_mandatory, resolve_all, validate_change,
    Channel, NotificationPreference, ResolvedPreference,
};
pub use retention::cutoff_twelve_months_before;
