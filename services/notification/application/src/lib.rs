//! Notification application layer: use cases and outward-implemented ports.
//!
//! Dependencies point inward: this crate depends on `notification-domain` only.

pub mod common;
pub mod delivery;
pub mod erase;
pub mod feed;
pub mod hashing;
pub mod ports;
pub mod preferences;
pub mod retention;
pub mod submit;
pub mod trail;

pub use common::FieldError;
pub use delivery::{AlertOutcome, AttemptOutcome, DeliveryError, DeliveryReport, DeliveryService};
pub use erase::{EraseError, EraseReport, EraseService};
pub use feed::{FeedCursor, FeedError, FeedPage, FeedService};
pub use hashing::sha256_hex;
pub use ports::{
    AlertRepository, AnonymisedRows, AttemptRepository, AuditClient, AuditRecord, AuthError, Clock,
    ContactDirectory, DeliveryControl, EmailMessage, EmailSender, ExternalError, IdGenerator,
    InsertOutcome, Pause, PreferenceRepository, Principal, ReadinessProbe, RepositoryError, Role,
    ServiceCaller, ServiceTokenRegistry, TokenValidator,
};
pub use preferences::{PreferenceChange, PreferenceError, PreferenceService, SavedPreferences};
pub use retention::{RetentionError, RetentionReport, RetentionService};
pub use submit::{SubmitAlert, SubmitCommand, SubmitError, SubmitResult, SubmitStatus};
pub use trail::Trail;
