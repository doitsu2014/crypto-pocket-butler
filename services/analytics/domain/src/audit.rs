//! The C10 actions Analytics emits. Actor and target are supplied by the caller;
//! no action string ever carries a property value, user reference, or subject.

/// A flag was created.
pub const FLAG_CREATED: &str = "analytics.flag.created";
/// A flag was updated.
pub const FLAG_UPDATED: &str = "analytics.flag.updated";
/// A flag was deleted.
pub const FLAG_DELETED: &str = "analytics.flag.deleted";
/// An ingest was refused.
pub const INGEST_REFUSED: &str = "analytics.ingest.refused";
/// A user's events were erased.
pub const PRIVACY_ERASED: &str = "analytics.privacy.erased";
/// A retention sweep ran.
pub const RETENTION_RUN: &str = "analytics.retention.run";
/// Nightly aggregation ran.
pub const AGGREGATE_RUN: &str = "analytics.aggregate.run";

/// The refusal categories recorded on `analytics.ingest.refused`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusalKind {
    /// Consent was absent or withdrawn.
    ConsentNotGranted,
    /// The event name was unknown or a value was PII-shaped.
    EventRejected,
    /// The consent service could not answer.
    ConsentUnavailable,
    /// The salt could not be read.
    SaltUnavailable,
}

impl RefusalKind {
    /// The context value recorded in the audit event.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ConsentNotGranted => "consent-not-granted",
            Self::EventRejected => "event-rejected",
            Self::ConsentUnavailable => "consent-unavailable",
            Self::SaltUnavailable => "salt-unavailable",
        }
    }
}
