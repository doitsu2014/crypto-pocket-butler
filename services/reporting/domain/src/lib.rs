//! Reporting domain: briefs, their period and data status, the composition rules
//! that turn valuation and performance reads into brief sections, and the
//! internal service identities.
//!
//! This crate holds entities, value types, and business rules only. It imports
//! no framework and no sibling crate.

pub mod audit;
pub mod brief;
pub mod compose;
pub mod error;
pub mod ids;
pub mod service;

pub use audit::{AuditOutcome, AuditRecord};
pub use brief::{
    Brief, BriefDelivery, BriefStatus, DataStatus, DeliveryChannel, DeliveryStatus, Period,
    MAX_FAILURE_REASON_CHARS,
};
pub use compose::{compose_sections, PnlRead, ValuationLineRead, ValuationRead};
pub use error::DomainError;
pub use ids::{BriefDeliveryId, BriefId, UserId};
pub use service::{service_allowed, Operation};
