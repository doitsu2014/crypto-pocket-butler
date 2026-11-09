//! Reporting application layer: use cases and outward-implemented ports.
//!
//! Dependencies point inward: this crate depends on `reporting-domain` only.

pub mod briefs;
pub mod common;
pub mod generation;
pub mod ports;
pub mod privacy;
pub mod sections;
pub mod window;

pub use briefs::{BriefService, BriefView, DEFAULT_PAGE, MAX_PAGE};
pub use common::{AppError, AssuranceLevel, Caller, FieldError, ServiceCaller, VerifiedClaims};
pub use generation::{
    GenerationService, RunReport, ALERT_TYPE, CONCURRENCY, EMAIL_CHANNEL, LEASE_SECONDS,
    RENEW_SECONDS, RETRY_AFTER_MINUTES, RUN_JOB,
};
pub use ports::{
    AnonymisationRecord, AuditClient, AuthError, BriefRepository, BriefRun, Clock,
    DeliveryRepository, DueUser, DueUsersClient, ExternalError, IdGenerator, NotificationClient,
    PerformanceClient, PrivacyRepository, ReadinessProbe, RegisteredService, RepositoryError,
    RunRepository, RunStatus, ServiceTokenRegistry, StoredBrief, TokenValidator, ValuationClient,
};
pub use privacy::{AnonymiseResult, PrivacyService, RETAINED_CATEGORIES};
pub use sections::{email_payload, sections_json};
pub use window::{window, BOUNDARY_HOUR};
