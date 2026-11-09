//! Audit application layer: use cases and outward-implemented ports.
//!
//! Dependencies point inward: this crate depends on `audit-domain` only.

pub mod append;
pub mod common;
pub mod export;
pub mod hashing;
pub mod ports;
pub mod privacy;
pub mod query;
pub mod retention;
pub mod support;
pub mod trail;

pub use append::{AppendCommand, AppendError, AppendEvent, AppendResult, Emitter};
pub use audit_domain::Context;
pub use common::FieldError;
pub use export::{ExportAudit, ExportError, ExportSession, EXPORT_BATCH_SIZE, EXPORT_ROW_CAP};
pub use ports::{Clock, Principal, ReadinessProbe, Role, ServiceTokenRegistry, TokenValidator};
pub use privacy::{PrivacyDeletions, PrivacyError, RequestCommand};
pub use query::{QueryCommand, QueryError, QueryEvents, QueryResult};
pub use retention::{
    ReportCommand, RetentionError, RetentionService, SweepConfig, DEFAULT_BATCH_SIZE,
    DEFAULT_MAX_BATCHES,
};
pub use support::{CheckDecision, StartSessionCommand, SupportError, SupportSessions};
pub use trail::Trail;
