//! Identity application layer: use cases and outward-implemented ports.
//!
//! Dependencies point inward: this crate depends on `identity-domain` only.

pub mod common;
pub mod consent;
pub mod deletion;
pub mod due_users;
pub mod export;
pub mod ports;
pub mod preferences;
pub mod principal;
pub mod sessions;
pub mod signup_gate;
pub mod support;
pub mod trail;
pub mod worker;

pub use common::{AppError, Caller, FieldError, PrincipalView};
pub use consent::ConsentService;
pub use deletion::{DeletionService, DeletionWorker};
pub use due_users::{DueUserView, DueUsersService};
pub use export::{ExportService, ExportStatusView, ExportWorker};
pub use preferences::PreferencesService;
pub use principal::{ContactView, PrincipalService};
pub use sessions::SessionService;
pub use signup_gate::{GateResponse, SignupGateService};
pub use support::SupportService;
pub use trail::Trail;
pub use worker::{PrivacyWorker, WorkerReport};
