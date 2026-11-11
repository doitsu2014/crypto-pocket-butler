//! Identity domain: users, consent, preferences, support grants, device
//! sessions, the jurisdiction gate, and the export and deletion state machines.
//!
//! This crate holds entities, value types, and business rules only. It imports
//! no framework and no sibling crate.

pub mod audit;
pub mod authorisation;
pub mod claims;
pub mod consent;
pub mod deletion;
pub mod error;
pub mod export;
pub mod ids;
pub mod jurisdiction;
pub mod lifecycle;
pub mod preferences;
pub mod secrets;
pub mod session;
pub mod shredding;
pub mod support;
pub mod user;

pub use audit::{AuditOutcome, AuditRecord};
pub use authorisation::{
    CallerIdentity, Decision as AuthorisationDecision, DenialReason, Operation, OperationClass,
    ResourceOwnership, ResourceScope,
};
pub use claims::{AssuranceLevel, NextStep, Role, VerifiedClaims};
pub use consent::{
    parse_iana_timezone, BaseCurrencyCode, ConsentAction, ConsentEvent, ConsentEventDraft,
    ConsentKind, ConsentPolicy, ConsentPurpose, ConsentRecord, ConsentSource, ConsentState,
    ConsentStateView, ConsentView, EffectivePreferences, PreferenceSource, PreferenceUpdate,
    PreferenceValue, ProviderProfile, StoredPreferences,
};
pub use deletion::{
    DeletionProgress, DeletionRecord, DeletionRequest, DeletionStatus, RetainedCategory,
};
pub use error::DomainError;
pub use export::{
    pending_contributions, ContributionState, Contributions, Contributor, ExportRequest,
    ExportStatus, DEFAULT_ASSEMBLY_MINUTES, DOWNLOAD_WINDOW_HOURS, LEASE_SECONDS,
};
pub use ids::{
    ConsentRecordId, DeletionRequestId, DeviceSessionId, ExportRequestId, ShreddingKeyId,
    StatusTransitionId, SupportGrantId, UserId,
};
pub use jurisdiction::{
    decide, evaluate, CountryCode, Decision, GateOutcome, JurisdictionListEntry,
    JurisdictionListVersion, JurisdictionRule, JurisdictionStatus, RestrictionSource,
    SignupDeclaration, GATE_TOKEN_TTL_MINUTES,
};
pub use lifecycle::{
    backoff_minutes, is_permitted, optional_reason, recoverable_until, require_edge,
    validate_reason, StatusTransition, MAX_ACTOR_LEN, MAX_ERASURE_ATTEMPTS, MAX_REASON_LEN,
    RECOVERY_WINDOW_DAYS, SYSTEM_ACTOR,
};
pub use preferences::{BaseCurrency, Locale, Preferences, TimeZone};
pub use secrets::find_secret_field;
pub use session::DeviceSession;
pub use shredding::{ShreddingKey, ShreddingKeyStatus};
pub use support::{
    SupportAccessGrant, SupportGrantStatus, DEFAULT_GRANT_MINUTES, MAX_GRANT_MINUTES,
};
pub use user::{is_email_shaped, User, UserStatus, MAX_DISPLAY_NAME_LEN, MAX_EMAIL_LEN};
