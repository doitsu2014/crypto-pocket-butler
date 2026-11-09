//! Portfolio application layer: use cases and outward-implemented ports.
//!
//! Dependencies point inward: this crate depends on `portfolio-domain` only.

pub mod common;
pub mod guardrails;
pub mod intents;
pub mod portfolios;
pub mod ports;
pub mod privacy;
pub mod run;
pub mod snapshot;
pub mod targets;
pub mod trades;
pub mod trail;
pub mod valuation;

pub use common::{AppError, AssuranceLevel, Caller, FieldError, ServiceCaller, VerifiedClaims};
pub use guardrails::{GuardrailReport, GuardrailService};
pub use intents::{IntentRequest, IntentService};
pub use portfolios::PortfolioService;
pub use privacy::{AnonymiseResult, PrivacyExport, PrivacyService};
pub use run::{
    RunReport, ValuationRunService, BATCHES_IN_FLIGHT, BATCH_IN_FLIGHT, BATCH_SIZE, LEASE_SECONDS,
    OVERLAY_EXPIRY_DAYS, RUN_JOB,
};
pub use snapshot::{FrozenInputs, StoredLine, StoredValuation};
pub use targets::{TargetInput, TargetSave, TargetService};
pub use trades::{TradeRequest, TradeService};
pub use trail::Trail;
pub use valuation::{
    FreshValuation, SnapshotDetail, SnapshotPage, SnapshotView, ValuationRead, ValuationService,
    DEFAULT_HISTORY_PAGE, MAX_HISTORY_PAGE,
};
