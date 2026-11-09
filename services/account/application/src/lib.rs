//! Account application layer: use cases and outward-implemented ports.
//!
//! Dependencies point inward: this crate depends on `account-domain` only.

pub mod accounts;
pub mod common;
pub mod holdings;
pub mod imports;
pub mod ports;
pub mod privacy;
pub mod sync;
pub mod trail;
pub mod verification;

pub use accounts::{AccountService, ExchangeKey, ResyncOutcome};
pub use common::{AppError, AssuranceLevel, Caller, FieldError, ServiceCaller, VerifiedClaims};
pub use holdings::{AccountHoldings, HoldingService};
pub use imports::ImportService;
pub use privacy::{AnonymiseResult, PrivacyExport, PrivacyService};
pub use sync::{RunOutcome, RunReport, SyncService};
pub use trail::Trail;
pub use verification::{VerificationReport, VerificationService};
