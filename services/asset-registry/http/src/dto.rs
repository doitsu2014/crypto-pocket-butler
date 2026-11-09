//! Request and response DTOs. They live only at the HTTP boundary and map to and
//! from domain types here; no DTO type reaches application or domain.

use asset_registry_application::resolve_asset::ResolveAssetQuery;
use asset_registry_application::resolve_price::{PriceAttempt, PriceOutcome};
use asset_registry_domain::{
    Asset, ObservationStatus, PriceSource, TokenContract, UnmappedObservation,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Query for `GET /v1/assets/resolve`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveAssetQueryDto {
    /// The chain identifier.
    pub chain: String,
    /// The observed contract address.
    #[serde(default)]
    pub contract_address: Option<String>,
    /// The observed symbol.
    #[serde(default)]
    pub symbol: Option<String>,
}

impl From<ResolveAssetQueryDto> for ResolveAssetQuery {
    fn from(dto: ResolveAssetQueryDto) -> Self {
        Self {
            chain: dto.chain,
            contract_address: dto.contract_address,
            symbol: dto.symbol,
        }
    }
}

/// A canonical asset.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetDto {
    /// Stable identifier.
    pub id: String,
    /// Normalized ticker.
    pub symbol: String,
    /// Display name.
    pub name: String,
    /// Canonical precision.
    pub decimals: u8,
    /// Whether this is a real registry entry.
    pub canonical: bool,
}

impl From<&Asset> for AssetDto {
    fn from(asset: &Asset) -> Self {
        Self {
            id: asset.id().to_string(),
            symbol: asset.symbol().to_string(),
            name: asset.name().to_string(),
            decimals: asset.decimals(),
            canonical: asset.canonical(),
        }
    }
}

/// One source that did not produce a usable price.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceAttemptDto {
    /// The source name.
    pub source: String,
    /// Why it did not answer.
    pub reason: String,
}

impl From<PriceAttempt> for PriceAttemptDto {
    fn from(attempt: PriceAttempt) -> Self {
        Self {
            source: attempt.source,
            reason: attempt.reason,
        }
    }
}

/// A price resolution. An unpriced asset never carries a price field (BR2.2).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum PriceResponseDto {
    /// A usable price.
    Priced {
        /// Decimal string; never a float.
        price: String,
        /// The source identifier.
        #[serde(rename = "sourceId")]
        source_id: String,
        /// The source name.
        #[serde(rename = "sourceName")]
        source_name: String,
        /// When the price was recorded.
        #[serde(rename = "observedAt")]
        observed_at: DateTime<Utc>,
    },
    /// No usable price.
    Unpriced {
        /// Every source that was asked, in priority order.
        attempted: Vec<PriceAttemptDto>,
        /// Why the asset is unpriced.
        reason: String,
    },
}

impl From<PriceOutcome> for PriceResponseDto {
    fn from(outcome: PriceOutcome) -> Self {
        match outcome {
            PriceOutcome::Priced {
                price,
                source,
                observed_at,
            } => Self::Priced {
                price: price.value().to_string(),
                source_id: source.id().to_string(),
                source_name: source.name().to_string(),
                observed_at,
            },
            PriceOutcome::Unpriced { attempted, reason } => Self::Unpriced {
                attempted: attempted.into_iter().map(PriceAttemptDto::from).collect(),
                reason,
            },
        }
    }
}

/// Add an asset.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddAssetRequest {
    /// Stable identifier.
    pub id: String,
    /// Ticker.
    pub symbol: String,
    /// Display name.
    pub name: String,
    /// Canonical precision.
    pub decimals: u8,
    /// When set, the asset is native to this chain.
    #[serde(default)]
    pub native_chain: Option<String>,
}

/// Add a chain-plus-contract mapping.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddTokenContractRequest {
    /// Stable identifier.
    pub id: String,
    /// Chain identifier.
    pub chain_id: String,
    /// Contract address as observed.
    pub contract_address: String,
    /// Canonical asset identifier.
    pub asset_id: String,
    /// Contract precision.
    pub decimals: u8,
}

/// Create or update a price source.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertPriceSourceRequest {
    /// Stable identifier.
    pub id: String,
    /// Unique name.
    pub name: String,
    /// Lower is tried first.
    pub priority: u32,
    /// Licence reference.
    #[serde(default)]
    pub licence_ref: Option<String>,
    /// Whether the source is enabled.
    #[serde(default)]
    pub enabled: bool,
}

/// Enable or disable a price source.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetEnabledRequest {
    /// The desired state.
    pub enabled: bool,
}

/// Resolve a quarantined observation.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveObservationRequest {
    /// The canonical asset to link.
    pub asset_id: String,
}

/// Ignore a quarantined observation.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IgnoreObservationRequest {
    /// Why the observation is ignored.
    pub reason: String,
}

/// A token contract.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenContractDto {
    /// Stable identifier.
    pub id: String,
    /// Chain identifier.
    pub chain_id: String,
    /// Contract address as observed.
    pub contract_address: String,
    /// Canonical asset identifier.
    pub asset_id: String,
    /// Contract precision.
    pub decimals: u8,
}

impl From<&TokenContract> for TokenContractDto {
    fn from(contract: &TokenContract) -> Self {
        Self {
            id: contract.id().to_string(),
            chain_id: contract.chain_id().to_string(),
            contract_address: contract.contract_address().to_string(),
            asset_id: contract.asset_id().to_string(),
            decimals: contract.decimals(),
        }
    }
}

/// A price source.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceSourceDto {
    /// Stable identifier.
    pub id: String,
    /// Unique name.
    pub name: String,
    /// Lower is tried first.
    pub priority: u32,
    /// Licence reference.
    pub licence_ref: Option<String>,
    /// Whether it is enabled.
    pub enabled: bool,
}

impl From<&PriceSource> for PriceSourceDto {
    fn from(source: &PriceSource) -> Self {
        Self {
            id: source.id().to_string(),
            name: source.name().to_string(),
            priority: source.priority(),
            licence_ref: source.licence_ref().map(ToString::to_string),
            enabled: source.enabled(),
        }
    }
}

/// A quarantined observation.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationDto {
    /// Stable identifier.
    pub id: String,
    /// Observed symbol.
    pub observed_symbol: String,
    /// Chain identifier.
    pub chain_id: String,
    /// Observed contract address.
    pub contract_address: Option<String>,
    /// When it was first seen.
    pub first_seen_at: DateTime<Utc>,
    /// `quarantined`, `resolved`, or `ignored`.
    pub status: String,
    /// The linked asset when resolved.
    pub resolved_asset_id: Option<String>,
    /// Why it was ignored.
    pub ignore_reason: Option<String>,
}

impl From<&UnmappedObservation> for ObservationDto {
    fn from(observation: &UnmappedObservation) -> Self {
        let (status, resolved_asset_id, ignore_reason) = match observation.status() {
            ObservationStatus::Quarantined => ("quarantined".to_string(), None, None),
            ObservationStatus::Resolved { asset_id } => {
                ("resolved".to_string(), Some(asset_id.to_string()), None)
            }
            ObservationStatus::Ignored { reason } => {
                ("ignored".to_string(), None, Some(reason.clone()))
            }
        };
        Self {
            id: observation.id().to_string(),
            observed_symbol: observation.observed_symbol().to_string(),
            chain_id: observation.chain_id().to_string(),
            contract_address: observation.contract_address().map(ToString::to_string),
            first_seen_at: observation.first_seen_at(),
            status,
            resolved_asset_id,
            ignore_reason,
        }
    }
}
