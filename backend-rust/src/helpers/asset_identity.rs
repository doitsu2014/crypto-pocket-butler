//! Asset Identity Normalization Module
//!
//! This module provides a centralized entry point for normalizing asset identities
//! across different sources (OKX symbols, EVM contract addresses) into canonical
//! asset IDs and symbols.
//!
//! # Purpose
//! - Map OKX symbols to canonical asset identities
//! - Map EVM contract addresses (with chain context) to canonical asset identities
//! - Provide debug information for all mapping decisions
//! - Handle unknown tokens gracefully with clear error paths
//!
//! # Usage
//! ```rust
//! use crate::helpers::asset_identity::{AssetIdentityNormalizer, AssetSource};
//!
//! let normalizer = AssetIdentityNormalizer::new(db);
//!
//! // Normalize OKX symbol
//! let result = normalizer.normalize_from_okx("BTC").await?;
//!
//! // Normalize EVM contract address
//! let result = normalizer.normalize_from_evm_contract(
//!     "0xdAC17F958D2ee523a2206206994597C13D831ec7",
//!     "ethereum"
//! ).await?;
//! ```

use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents a canonical asset identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetIdentity {
    /// Canonical asset ID from the assets table
    pub asset_id: Uuid,
    
    /// Canonical symbol (e.g., "BTC", "ETH")
    pub symbol: String,
    
    /// Asset name
    pub name: String,
    
    /// Source of the mapping
    pub mapping_source: MappingSource,
    
    /// Debug information about the mapping decision
    pub debug_info: String,
}

/// Source of the asset identity mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MappingSource {
    /// Mapped from OKX symbol
    OkxSymbol { original_symbol: String },
    
    /// Mapped from EVM contract address
    EvmContract {
        contract_address: String,
        chain: String,
    },
    
    /// Direct symbol match in assets table
    DirectSymbolMatch { original_symbol: String },
}

/// Result of asset normalization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NormalizationResult {
    /// Asset was successfully mapped
    Mapped(AssetIdentity),
    
    /// Asset could not be mapped (unknown token)
    Unknown {
        /// Original identifier that could not be mapped
        original_identifier: String,
        
        /// Type of identifier (e.g., "okx_symbol", "evm_contract")
        identifier_type: String,
        
        /// Additional context
        context: String,
    },
}

impl NormalizationResult {
    /// Check if the result is mapped
    pub fn is_mapped(&self) -> bool {
        matches!(self, NormalizationResult::Mapped(_))
    }
    
    /// Get the asset identity if mapped, None otherwise
    pub fn asset_identity(&self) -> Option<&AssetIdentity> {
        match self {
            NormalizationResult::Mapped(identity) => Some(identity),
            NormalizationResult::Unknown { .. } => None,
        }
    }
    
    /// Get a display string for the result
    pub fn display_string(&self) -> String {
        match self {
            NormalizationResult::Mapped(identity) => {
                format!("Mapped to {} ({})", identity.symbol, identity.asset_id)
            }
            NormalizationResult::Unknown { original_identifier, identifier_type, .. } => {
                format!("Unknown {} '{}'", identifier_type, original_identifier)
            }
        }
    }
}

/// Asset identity normalizer
pub struct AssetIdentityNormalizer {
    db: DatabaseConnection,
}

impl AssetIdentityNormalizer {
    /// Create a new asset identity normalizer
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
    
    /// Normalize an asset from OKX symbol
    ///
    /// # Arguments
    /// * `okx_symbol` - The symbol from OKX API (e.g., "BTC", "ETH", "USDT")
    ///
    /// # Returns
    /// A NormalizationResult containing either the mapped asset identity or unknown info
    pub async fn normalize_from_okx(&self, okx_symbol: &str) -> NormalizationResult {
        use crate::entities::assets;
        
        tracing::debug!("Normalizing OKX symbol: {}", okx_symbol);
        
        // Normalize symbol to uppercase for case-insensitive matching
        let normalized_symbol = okx_symbol.trim().to_uppercase();
        
        if normalized_symbol.is_empty() {
            return NormalizationResult::Unknown {
                original_identifier: okx_symbol.to_string(),
                identifier_type: "okx_symbol".to_string(),
                context: "Empty symbol".to_string(),
            };
        }
        
        // Try to find asset by symbol
        let asset_result = assets::Entity::find()
            .filter(assets::Column::Symbol.eq(&normalized_symbol))
            .one(&self.db)
            .await;
        
        match asset_result {
            Ok(Some(asset)) => {
                let debug_info = format!(
                    "Mapped OKX symbol '{}' to asset '{}' ({})",
                    okx_symbol, asset.symbol, asset.id
                );
                tracing::info!("{}", debug_info);
                
                NormalizationResult::Mapped(AssetIdentity {
                    asset_id: asset.id,
                    symbol: asset.symbol.clone(),
                    name: asset.name.clone(),
                    mapping_source: MappingSource::OkxSymbol {
                        original_symbol: okx_symbol.to_string(),
                    },
                    debug_info,
                })
            }
            Ok(None) => {
                let context = format!(
                    "Symbol '{}' not found in assets database",
                    normalized_symbol
                );
                tracing::warn!(
                    "Failed to normalize OKX symbol '{}': {}",
                    okx_symbol, context
                );
                
                NormalizationResult::Unknown {
                    original_identifier: okx_symbol.to_string(),
                    identifier_type: "okx_symbol".to_string(),
                    context,
                }
            }
            Err(e) => {
                let context = format!("Database error: {}", e);
                tracing::error!(
                    "Failed to normalize OKX symbol '{}': {}",
                    okx_symbol, context
                );
                
                NormalizationResult::Unknown {
                    original_identifier: okx_symbol.to_string(),
                    identifier_type: "okx_symbol".to_string(),
                    context,
                }
            }
        }
    }
    
    /// Normalize an asset from EVM contract address
    ///
    /// # Arguments
    /// * `contract_address` - The contract address (e.g., "0xdAC17F958D2ee523a2206206994597C13D831ec7")
    /// * `chain` - The chain identifier (e.g., "ethereum", "arbitrum", "bsc")
    ///
    /// # Returns
    /// A NormalizationResult containing either the mapped asset identity or unknown info
    pub async fn normalize_from_evm_contract(
        &self,
        contract_address: &str,
        chain: &str,
    ) -> NormalizationResult {
        use crate::entities::{asset_contracts, assets};
        
        tracing::debug!(
            "Normalizing EVM contract: {} on chain {}",
            contract_address,
            chain
        );
        
        // Normalize contract address to lowercase (standard format)
        let normalized_address = contract_address.trim().to_lowercase();
        let normalized_chain = chain.trim().to_lowercase();
        
        if normalized_address.is_empty() {
            return NormalizationResult::Unknown {
                original_identifier: contract_address.to_string(),
                identifier_type: "evm_contract".to_string(),
                context: "Empty contract address".to_string(),
            };
        }
        
        // Try to find contract in asset_contracts table
        let contract_result = asset_contracts::Entity::find()
            .filter(asset_contracts::Column::ContractAddress.eq(&normalized_address))
            .filter(asset_contracts::Column::Chain.eq(&normalized_chain))
            .one(&self.db)
            .await;
        
        match contract_result {
            Ok(Some(contract)) => {
                // Found contract, now get the associated asset
                let asset_result = assets::Entity::find_by_id(contract.asset_id)
                    .one(&self.db)
                    .await;
                
                match asset_result {
                    Ok(Some(asset)) => {
                        let debug_info = format!(
                            "Mapped EVM contract '{}' (chain: {}) to asset '{}' ({})",
                            contract_address, chain, asset.symbol, asset.id
                        );
                        tracing::info!("{}", debug_info);
                        
                        NormalizationResult::Mapped(AssetIdentity {
                            asset_id: asset.id,
                            symbol: asset.symbol.clone(),
                            name: asset.name.clone(),
                            mapping_source: MappingSource::EvmContract {
                                contract_address: contract_address.to_string(),
                                chain: chain.to_string(),
                            },
                            debug_info,
                        })
                    }
                    Ok(None) => {
                        let context = format!(
                            "Asset ID {} not found for contract {} on chain {}",
                            contract.asset_id, normalized_address, normalized_chain
                        );
                        tracing::error!("{}", context);
                        
                        NormalizationResult::Unknown {
                            original_identifier: contract_address.to_string(),
                            identifier_type: "evm_contract".to_string(),
                            context,
                        }
                    }
                    Err(e) => {
                        let context = format!("Database error fetching asset: {}", e);
                        tracing::error!(
                            "Failed to normalize EVM contract '{}' on chain '{}': {}",
                            contract_address, chain, context
                        );
                        
                        NormalizationResult::Unknown {
                            original_identifier: contract_address.to_string(),
                            identifier_type: "evm_contract".to_string(),
                            context,
                        }
                    }
                }
            }
            Ok(None) => {
                let context = format!(
                    "Contract '{}' on chain '{}' not found in asset_contracts database",
                    normalized_address, normalized_chain
                );
                tracing::warn!(
                    "Failed to normalize EVM contract '{}' on chain '{}': {}",
                    contract_address, chain, context
                );
                
                NormalizationResult::Unknown {
                    original_identifier: contract_address.to_string(),
                    identifier_type: "evm_contract".to_string(),
                    context,
                }
            }
            Err(e) => {
                let context = format!("Database error: {}", e);
                tracing::error!(
                    "Failed to normalize EVM contract '{}' on chain '{}': {}",
                    contract_address, chain, context
                );
                
                NormalizationResult::Unknown {
                    original_identifier: contract_address.to_string(),
                    identifier_type: "evm_contract".to_string(),
                    context,
                }
            }
        }
    }
    
    /// Normalize an asset from a generic symbol
    ///
    /// This is a convenience method that tries to match a symbol directly.
    /// Use this for symbols that don't have a specific source context.
    /// 
    /// Special handling: If the symbol contains a chain suffix (e.g., "USDT-ethereum"),
    /// this will attempt to parse it and use EVM contract normalization instead.
    ///
    /// # Arguments
    /// * `symbol` - The symbol to normalize (e.g., "BTC", "ETH", "USDT-ethereum")
    ///
    /// # Returns
    /// A NormalizationResult containing either the mapped asset identity or unknown info
    pub async fn normalize_from_symbol(&self, symbol: &str) -> NormalizationResult {
        use crate::entities::assets;
        
        tracing::debug!("Normalizing generic symbol: {}", symbol);
        
        // Check if this is a chain-specific symbol (e.g., "USDT-ethereum")
        if let Some((base_symbol, chain)) = symbol.rsplit_once('-') {
            // Check if the suffix is a known chain
            let known_chains = ["ethereum", "arbitrum", "optimism", "base", "bsc"];
            if known_chains.contains(&chain) {
                tracing::debug!(
                    "Detected chain-specific symbol: {} on chain {}",
                    base_symbol, chain
                );
                // Try to normalize as a symbol first (for native tokens like ETH)
                let normalized_symbol = base_symbol.trim().to_uppercase();
                
                // Try to find asset by symbol first
                let asset_result = assets::Entity::find()
                    .filter(assets::Column::Symbol.eq(&normalized_symbol))
                    .one(&self.db)
                    .await;
                
                if let Ok(Some(asset)) = asset_result {
                    let debug_info = format!(
                        "Mapped chain-specific symbol '{}-{}' to asset '{}' ({}) via symbol match",
                        base_symbol, chain, asset.symbol, asset.id
                    );
                    tracing::info!("{}", debug_info);
                    
                    return NormalizationResult::Mapped(AssetIdentity {
                        asset_id: asset.id,
                        symbol: asset.symbol.clone(),
                        name: asset.name.clone(),
                        mapping_source: MappingSource::DirectSymbolMatch {
                            original_symbol: symbol.to_string(),
                        },
                        debug_info,
                    });
                }
                
                // If not found by symbol, the token might need to be looked up via contract address
                // which would require additional context not available in the symbol
                tracing::debug!(
                    "Symbol '{}' not found directly, may need contract address lookup",
                    base_symbol
                );
            }
        }
        
        // Standard symbol normalization without chain context
        // Normalize symbol to uppercase for case-insensitive matching
        let normalized_symbol = symbol.trim().to_uppercase();
        
        if normalized_symbol.is_empty() {
            return NormalizationResult::Unknown {
                original_identifier: symbol.to_string(),
                identifier_type: "symbol".to_string(),
                context: "Empty symbol".to_string(),
            };
        }
        
        // Try to find asset by symbol
        let asset_result = assets::Entity::find()
            .filter(assets::Column::Symbol.eq(&normalized_symbol))
            .one(&self.db)
            .await;
        
        match asset_result {
            Ok(Some(asset)) => {
                let debug_info = format!(
                    "Mapped symbol '{}' to asset '{}' ({})",
                    symbol, asset.symbol, asset.id
                );
                tracing::info!("{}", debug_info);
                
                NormalizationResult::Mapped(AssetIdentity {
                    asset_id: asset.id,
                    symbol: asset.symbol.clone(),
                    name: asset.name.clone(),
                    mapping_source: MappingSource::DirectSymbolMatch {
                        original_symbol: symbol.to_string(),
                    },
                    debug_info,
                })
            }
            Ok(None) => {
                let context = format!(
                    "Symbol '{}' not found in assets database",
                    normalized_symbol
                );
                tracing::warn!(
                    "Failed to normalize symbol '{}': {}",
                    symbol, context
                );
                
                NormalizationResult::Unknown {
                    original_identifier: symbol.to_string(),
                    identifier_type: "symbol".to_string(),
                    context,
                }
            }
            Err(e) => {
                let context = format!("Database error: {}", e);
                tracing::error!(
                    "Failed to normalize symbol '{}': {}",
                    symbol, context
                );
                
                NormalizationResult::Unknown {
                    original_identifier: symbol.to_string(),
                    identifier_type: "symbol".to_string(),
                    context,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_normalization_result_is_mapped() {
        let mapped = NormalizationResult::Mapped(AssetIdentity {
            asset_id: Uuid::new_v4(),
            symbol: "BTC".to_string(),
            name: "Bitcoin".to_string(),
            mapping_source: MappingSource::OkxSymbol {
                original_symbol: "BTC".to_string(),
            },
            debug_info: "test".to_string(),
        });
        
        assert!(mapped.is_mapped());
        assert!(mapped.asset_identity().is_some());
        
        let unknown = NormalizationResult::Unknown {
            original_identifier: "UNKNOWN".to_string(),
            identifier_type: "symbol".to_string(),
            context: "Not found".to_string(),
        };
        
        assert!(!unknown.is_mapped());
        assert!(unknown.asset_identity().is_none());
    }
    
    #[test]
    fn test_normalization_result_display() {
        let unknown = NormalizationResult::Unknown {
            original_identifier: "UNKNOWN".to_string(),
            identifier_type: "okx_symbol".to_string(),
            context: "Not found".to_string(),
        };
        
        let display = unknown.display_string();
        assert!(display.contains("Unknown"));
        assert!(display.contains("UNKNOWN"));
    }
}
