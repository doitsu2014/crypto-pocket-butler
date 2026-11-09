//! Analytics domain: the closed event catalogue, per-event allow-lists, the PII
//! detector, consent, feature flags, aggregate formulas, and retention cutoffs.
//!
//! This crate holds entities, value types, and business rules only. It imports
//! no framework and no sibling crate.

pub mod aggregate;
pub mod aggregation;
pub mod audit;
pub mod consent;
pub mod constants;
pub mod error;
pub mod event;
pub mod flag;
pub mod ids;
pub mod pii;
pub mod retention;

pub use aggregate::{Metric, MetricAggregate, MetricCandidate, Period};
pub use aggregation::{compute_metrics, AggregateInputs, EventFact};
pub use audit::RefusalKind;
pub use consent::{ConsentCheck, ConsentPurpose};
pub use constants::{
    CONSENT_CHECK_RETENTION_DAYS, EVENT_RETENTION_MONTHS, FLAG_CACHE_SECONDS,
    MAX_COHORT_WINDOW_EVENTS, MAX_DAILY_WINDOW_EVENTS, MAX_EVENT_NAME_LEN, MAX_FLAG_KEY_LEN,
    MAX_OWNER_LEN, MAX_PROPERTY_VALUE_LEN, RETENTION_BATCH_SIZE, RETENTION_MAX_BATCHES, SALT_BYTES,
    SALT_RETENTION_MONTHS, SUPPRESSION_MIN_USERS,
};
pub use error::DomainError;
pub use event::{
    definition, event_class, sanitize_event, AnalyticsEvent, EventClass, EventDefinition,
    SanitizedEvent, CATALOGUE,
};
pub use flag::{FeatureFlag, FlagStatus, FlagUpdate, NewFeatureFlag};
pub use ids::{
    AnalyticsEventId, AnalyticsUserId, ConsentCheckId, FeatureFlagId, MetricAggregateId,
};
pub use pii::{detect_pii, is_pii, PiiKind};
pub use retention::{cutoff_consent_checks_before, cutoff_six_months_before};
