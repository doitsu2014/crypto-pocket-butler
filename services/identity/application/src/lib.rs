//! Identity application layer: use cases and outward-implemented ports.
//!
//! Dependencies point inward: this crate depends on `identity-domain` only.

pub mod account_lifecycle;
pub mod authorisation;
pub mod common;
pub mod consent;
pub mod deletion;
pub mod due_users;
pub mod export;
pub mod jurisdiction;
pub mod ports;
pub mod preferences;
pub mod principal;
pub mod profile;
pub mod sessions;
pub mod signup_gate;
pub mod support;
pub mod trail;
pub mod user_registry;
pub mod worker;

pub use account_lifecycle::{
    AccountLifecycleService, AccountObservation, AccountStatusView, NoAccountObservation,
};
pub use authorisation::catalogue::OperationCatalogue;
pub use authorisation::decision::{AuthorisationDecision, DecisionError};
pub use authorisation::role_change::{RoleChange, RoleChangeView};
pub use common::{AppError, Caller, FieldError, PrincipalView};
pub use consent::ConsentService;
pub use deletion::{DeletionService, DeletionWorker};
pub use due_users::{DueUserView, DueUsersService};
pub use export::{ExportService, ExportStatusView, ExportWorker};
pub use jurisdiction::{CheckJurisdiction, CheckResult, PublishList, PublishedList};
pub use preferences::PreferencesService;
pub use principal::{ContactView, PrincipalService};
pub use profile::{ProfileService, ProfileView};
pub use sessions::SessionService;
pub use signup_gate::{GateResponse, SignupGateService};
pub use support::SupportService;
pub use trail::Trail;
pub use user_registry::{subject_hash, AdminLookup, UserRecordView};
pub use worker::{PrivacyWorker, WorkerReport};
