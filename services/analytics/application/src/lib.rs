//! Analytics application layer: use cases and outward-implemented ports.
//!
//! Dependencies point inward: this crate depends on `analytics-domain` only.

pub mod aggregation;
pub mod common;
pub mod erase;
pub mod flags;
pub mod hashing;
pub mod ingest;
pub mod ports;
pub mod retention;
pub mod trail;

pub use aggregation::{AggregateError, AggregateReport, AggregateService, ScannedCounts};
pub use common::{AppError, FieldError};
pub use erase::{EraseError, EraseService};
pub use flags::{FlagCommand, FlagError, FlagEvaluation, FlagPatch, FlagService};
pub use hashing::{flag_bucket, hmac_sha256_hex, pseudonymous_actor_id, sha256_hex};
pub use ingest::{IngestCommand, IngestError, IngestEvent, IngestResult};
pub use ports::{
    AuditClient, AuditRecord, AuthError, Clock, ConsentChecker, ConsentRepository, EventRepository,
    ExternalError, FlagRepository, Pause, Principal, ReadinessProbe, RepositoryError, Role,
    SaltProvider, SaltVersion, ServiceCaller, ServiceTokenRegistry, TokenValidator,
};
pub use retention::{RetentionError, RetentionReport, RetentionService};
pub use trail::{Trail, TARGET_PREFIX_CHARS};
