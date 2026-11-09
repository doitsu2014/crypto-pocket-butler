//! Audit domain: append-only events, support sessions, retention runs, exports,
//! and privacy deletions.
//!
//! This crate holds entities, value types, and business rules only. It imports no
//! framework and no sibling crate.

pub mod context;
pub mod error;
pub mod event;
pub mod export;
pub mod ids;
pub mod privacy;
pub mod retention;
pub mod secrets;
pub mod support;

pub use context::{Context, ALLOWED_CONTEXT_KEYS, MAX_CONTEXT_BYTES, MAX_CONTEXT_KEYS};
pub use error::DomainError;
pub use event::{
    from_stored, ActorKind, AuditEvent, EventClass, NewAuditEvent, Outcome, Sensitivity,
    MAX_ACTION_LEN, MAX_IDEMPOTENCY_KEY_LEN, MAX_LEGAL_BASIS_LEN, MAX_TARGET_LEN,
    SYSTEM_ACTION_ALLOW_LIST,
};
pub use export::{
    AuditExport, ExportFilter, ExportOutcome, ExportState, MAX_ACTION_PREFIXES, MAX_EXPORT_ROWS,
};
pub use ids::{
    AuditEventId, AuditExportId, DeletionRequestId, PrivacyDeletionId, RetentionRunId,
    SupportSessionId, UserId,
};
pub use privacy::{classify, DeletionReport, EventLabel, PrivacyDeletion, PrivacyDeletionState};
pub use retention::{
    cutoff_six_calendar_months_before, partition_action, PartitionAction, PartitionSummary,
    RetentionCategory, RetentionOutcome, RetentionOwner, RetentionRun, RetentionState,
    AUDIT_LOG_RETENTION_MONTHS, AUDIT_RETENTION_ROLE,
};
pub use secrets::{detect_secret, ensure_secret_free, redact, seed_word_count, SecretKind};
pub use support::{
    SupportDenial, SupportScope, SupportSession, SupportSessionOutcome, SupportSessionRequest,
    SupportSessionState, MAX_REASON_LEN, MAX_SESSION_HOURS, MIN_REASON_LEN,
};
