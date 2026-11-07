//! Market Data domain: freshness tiers, price observations, served quotes, FX
//! rates, and refresh runs with their leases.
//!
//! This crate holds entities, value types, and business rules only. It imports
//! no framework and no sibling crate.

pub mod audit;
pub mod constants;
pub mod error;
pub mod freshness;
pub mod fx;
pub mod ids;
pub mod price;
pub mod rate;
pub mod run;
pub mod secrets;
pub mod service;
pub mod tier;

pub use audit::{AuditOutcome, AuditRecord};
pub use constants::{
    CLOCK_TOLERANCE_SECONDS, FX_BOUND_SECONDS, HARD_BOUND_SECONDS, MAX_ASSET_IDS,
    MAX_ASSET_ID_CHARS, MAX_ERROR_MESSAGE_CHARS, MAX_REJECTION_REASON_CHARS,
    RENEW_INTERVAL_SECONDS, RETENTION_DAYS, RUN_LEASE_SECONDS, SWEEP_INTERVAL_SECONDS,
    TAIL_BOUND_SECONDS, TOP100_BOUND_SECONDS,
};
pub use error::DomainError;
pub use freshness::{classify_freshness, Freshness};
pub use fx::{normalise_currency, FxRate, ServedFxRate};
pub use ids::{FxRateId, ObservationId, RefreshRunId};
pub use price::{is_clock_skewed, parse_price, ObservationStatus, PriceObservation, PriceQuote};
pub use rate::{bucket_try_take, refill_tokens};
pub use run::{ErrorCategory, RefreshJob, RefreshRun, RunStatus, Trigger};
pub use secrets::{looks_like_secret, sanitise_message};
pub use service::Operation;
pub use tier::{bound_for_rank, validate_asset_id, AssetPriceTier, PriceTier};
