//! Asset Registry domain: canonical asset identity, chains, contracts, price sources,
//! and the quarantine for unmapped observations (BR1.1–BR1.3, BR2.1–BR2.2).
//!
//! This crate holds entities, value types, and business rules only. It imports no
//! framework and no sibling crate.

pub mod asset;
pub mod chain;
pub mod contract;
pub mod error;
pub mod ids;
pub mod observation;
pub mod price;
pub mod price_source;
pub mod symbol;

pub use asset::{Asset, MAX_DECIMALS};
pub use chain::{Chain, ChainFamily};
pub use contract::{ensure_decimals_match, ContractAddress, TokenContract};
pub use error::DomainError;
pub use ids::{AssetId, ChainId, ObservationId, PriceSourceId, TokenContractId};
pub use observation::{ObservationStatus, UnmappedObservation};
pub use price::Price;
pub use price_source::{ensure_unique_enabled_priorities, PriceSource};
pub use symbol::normalize_symbol;
