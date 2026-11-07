//! Identity domain: users, consent, preferences, support grants, device
//! sessions, the jurisdiction gate, and the export and deletion state machines.
//!
//! This crate holds entities, value types, and business rules only. It imports
//! no framework and no sibling crate.

pub mod audit;
pub mod claims;
pub mod consent;
pub mod deletion;
pub mod error;
pub mod export;
pub mod ids;
pub mod jurisdiction;
pub mod preferences;
pub mod secrets;
pub mod session;
pub mod shredding;
pub mod support;
pub mod user;

pub use audit::{AuditOutcome, AuditRecord};
pub use claims::{AssuranceLevel, NextStep, Role, VerifiedClaims};
pub use consent::{ConsentPurpose, ConsentRecord, ConsentSource, ConsentView};
pub use deletion::{DeletionProgress, DeletionRequest, DeletionStatus, RetainedCategory};
pub use error::DomainError;
pub use export::{
    pending_contributions, ContributionState, Contributions, Contributor, ExportRequest,
    ExportStatus, DEFAULT_ASSEMBLY_MINUTES, DOWNLOAD_WINDOW_HOURS, LEASE_SECONDS,
};
pub use ids::{
    ConsentRecordId, DeletionRequestId, DeviceSessionId, ExportRequestId, ShreddingKeyId,
    SupportGrantId, UserId,
};
pub use jurisdiction::{
    evaluate, CountryCode, GateOutcome, JurisdictionRule, SignupDeclaration, GATE_TOKEN_TTL_MINUTES,
};
pub use preferences::{BaseCurrency, Locale, Preferences, TimeZone};
pub use secrets::find_secret_field;
pub use session::DeviceSession;
pub use shredding::{ShreddingKey, ShreddingKeyStatus};
pub use support::{
    SupportAccessGrant, SupportGrantStatus, DEFAULT_GRANT_MINUTES, MAX_GRANT_MINUTES,
};
pub use user::{User, UserStatus};
