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

use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, QueryOrder};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Known EVM chain identifiers used for chain-specific symbol parsing
/// 
/// This constant is used when parsing symbols in the format "SYMBOL-CHAIN"
/// to distinguish chain suffixes from legitimate hyphenated asset symbols.
const KNOWN_EVM_CHAINS: &[&str] = &["ethereum", "arbitrum", "optimism", "base", "bsc"];

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
    /// 
    /// # Note
    /// This method only uses symbol for lookup as OKX doesn't provide name.
    /// For full symbol+name lookup, use normalize_from_symbol_and_name instead.
    /// When multiple assets share the same symbol, prefers the one with lower rank (higher market cap).
    pub async fn normalize_from_okx(&self, okx_symbol: &str) -> NormalizationResult {
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
        
        // Try to find asset by symbol only (OKX doesn't provide name)
        // Prefer assets with better (lower) rank when multiple assets share the same symbol
        let asset_result = self.find_asset_by_symbol_with_rank(&normalized_symbol).await;
        
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
    
    /// Normalize an asset from both symbol and name
    ///
    /// This method enforces the new uniqueness constraint by checking both
    /// symbol AND name fields together.
    ///
    /// # Arguments
    /// * `symbol` - The asset symbol (e.g., "BTC", "ETH")
    /// * `name` - The asset name (e.g., "Bitcoin", "Ethereum")
    ///
    /// # Returns
    /// A NormalizationResult containing either the mapped asset identity or unknown info
    pub async fn normalize_from_symbol_and_name(
        &self,
        symbol: &str,
        name: &str,
    ) -> NormalizationResult {
        use crate::entities::assets;
        
        tracing::debug!("Normalizing by symbol '{}' and name '{}'", symbol, name);
        
        // Normalize symbol to uppercase and trim both fields
        let normalized_symbol = symbol.trim().to_uppercase();
        let normalized_name = name.trim();
        
        if normalized_symbol.is_empty() || normalized_name.is_empty() {
            return NormalizationResult::Unknown {
                original_identifier: format!("{}:{}", symbol, name),
                identifier_type: "symbol_and_name".to_string(),
                context: "Empty symbol or name".to_string(),
            };
        }
        
        // Try to find asset by both symbol AND name
        let asset_result = assets::Entity::find()
            .filter(assets::Column::Symbol.eq(&normalized_symbol))
            .filter(assets::Column::Name.eq(normalized_name))
            .one(&self.db)
            .await;
        
        match asset_result {
            Ok(Some(asset)) => {
                let debug_info = format!(
                    "Mapped symbol '{}' and name '{}' to asset '{}' ({})",
                    symbol, name, asset.symbol, asset.id
                );
                tracing::info!("{}", debug_info);
                
                NormalizationResult::Mapped(AssetIdentity {
                    asset_id: asset.id,
                    symbol: asset.symbol.clone(),
                    name: asset.name.clone(),
                    mapping_source: MappingSource::DirectSymbolMatch {
                        original_symbol: format!("{}:{}", symbol, name),
                    },
                    debug_info,
                })
            }
            Ok(None) => {
                let context = format!(
                    "Asset with symbol '{}' and name '{}' not found in assets database",
                    normalized_symbol, normalized_name
                );
                tracing::warn!(
                    "Failed to normalize symbol '{}' and name '{}': {}",
                    symbol, name, context
                );
                
                NormalizationResult::Unknown {
                    original_identifier: format!("{}:{}", symbol, name),
                    identifier_type: "symbol_and_name".to_string(),
                    context,
                }
            }
            Err(e) => {
                let context = format!("Database error: {}", e);
                tracing::error!(
                    "Failed to normalize symbol '{}' and name '{}': {}",
                    symbol, name, context
                );
                
                NormalizationResult::Unknown {
                    original_identifier: format!("{}:{}", symbol, name),
                    identifier_type: "symbol_and_name".to_string(),
                    context,
                }
            }
        }
    }
    
    /// Find the best asset by symbol, preferring assets with lower rank (higher market cap)
    /// when multiple assets share the same symbol.
    ///
    /// This helper method joins with asset_prices to access rank information and selects
    /// the asset with the lowest rank value (e.g., rank 2 before rank 900).
    async fn find_asset_by_symbol_with_rank(&self, normalized_symbol: &str) -> Result<Option<crate::entities::assets::Model>, sea_orm::DbErr> {
        use crate::entities::{assets, asset_prices};
        
        // First, try to find assets with the given symbol that have price data with rank
        // We only consider non-null ranks and order by rank ascending (lower is better)
        let asset_with_rank = assets::Entity::find()
            .filter(assets::Column::Symbol.eq(normalized_symbol))
            .inner_join(asset_prices::Entity)
            .filter(asset_prices::Column::Rank.is_not_null())
            .order_by_asc(asset_prices::Column::Rank)
            .one(&self.db)
            .await?;
        
        if let Some(asset) = asset_with_rank {
            return Ok(Some(asset));
        }
        
        // Fallback: if no assets have price data with rank, just return any matching asset
        assets::Entity::find()
            .filter(assets::Column::Symbol.eq(normalized_symbol))
            .one(&self.db)
            .await
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
        tracing::debug!("Normalizing generic symbol: {}", symbol);
        
        // Check if this is a chain-specific symbol (e.g., "USDT-ethereum")
        // We only treat it as chain-specific if the suffix matches a known EVM chain
        // This avoids false positives with legitimately hyphenated symbols
        if let Some((base_symbol, chain_suffix)) = symbol.rsplit_once('-') {
            // Check if the suffix is a known chain (case-insensitive)
            let chain_lower = chain_suffix.to_lowercase();
            if KNOWN_EVM_CHAINS.contains(&chain_lower.as_str()) {
                tracing::debug!(
                    "Detected chain-specific symbol: {} on chain {}",
                    base_symbol, chain_suffix
                );
                // Try to normalize as a symbol first (for native tokens like ETH)
                let normalized_symbol = base_symbol.trim().to_uppercase();
                
                // Try to find asset by symbol first (preferring best rank)
                let asset_result = self.find_asset_by_symbol_with_rank(&normalized_symbol).await;
                
                if let Ok(Some(asset)) = asset_result {
                    let debug_info = format!(
                        "Mapped chain-specific symbol '{}-{}' to asset '{}' ({}) via symbol match",
                        base_symbol, chain_suffix, asset.symbol, asset.id
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
        
        // Try to find asset by symbol (preferring best rank)
        let asset_result = self.find_asset_by_symbol_with_rank(&normalized_symbol).await;
        
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
    
    #[test]
    fn test_mapping_source_okx() {
        let source = MappingSource::OkxSymbol {
            original_symbol: "BTC".to_string(),
        };
        
        match source {
            MappingSource::OkxSymbol { original_symbol } => {
                assert_eq!(original_symbol, "BTC");
            }
            _ => panic!("Wrong mapping source variant"),
        }
    }
    
    #[test]
    fn test_mapping_source_evm_contract() {
        let source = MappingSource::EvmContract {
            contract_address: "0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string(),
            chain: "ethereum".to_string(),
        };
        
        match source {
            MappingSource::EvmContract { contract_address, chain } => {
                assert_eq!(contract_address, "0xdAC17F958D2ee523a2206206994597C13D831ec7");
                assert_eq!(chain, "ethereum");
            }
            _ => panic!("Wrong mapping source variant"),
        }
    }
    
    #[test]
    fn test_mapping_source_direct_symbol() {
        let source = MappingSource::DirectSymbolMatch {
            original_symbol: "ETH".to_string(),
        };
        
        match source {
            MappingSource::DirectSymbolMatch { original_symbol } => {
                assert_eq!(original_symbol, "ETH");
            }
            _ => panic!("Wrong mapping source variant"),
        }
    }
    
    #[test]
    fn test_asset_identity_fields() {
        let asset_id = Uuid::new_v4();
        let identity = AssetIdentity {
            asset_id,
            symbol: "BTC".to_string(),
            name: "Bitcoin".to_string(),
            mapping_source: MappingSource::OkxSymbol {
                original_symbol: "btc".to_string(),
            },
            debug_info: "Mapped from OKX".to_string(),
        };
        
        assert_eq!(identity.asset_id, asset_id);
        assert_eq!(identity.symbol, "BTC");
        assert_eq!(identity.name, "Bitcoin");
        assert_eq!(identity.debug_info, "Mapped from OKX");
    }
    
    #[test]
    fn test_serialization() {
        let result = NormalizationResult::Unknown {
            original_identifier: "UNKNOWN".to_string(),
            identifier_type: "symbol".to_string(),
            context: "Not found".to_string(),
        };
        
        // Test that the result can be serialized
        let json = serde_json::to_string(&result);
        assert!(json.is_ok());
    }
    
    #[test]
    fn test_normalization_result_variants() {
        // Test Mapped variant
        let mapped = NormalizationResult::Mapped(AssetIdentity {
            asset_id: Uuid::new_v4(),
            symbol: "BTC".to_string(),
            name: "Bitcoin".to_string(),
            mapping_source: MappingSource::OkxSymbol {
                original_symbol: "BTC".to_string(),
            },
            debug_info: "test".to_string(),
        });
        
        assert!(matches!(mapped, NormalizationResult::Mapped(_)));
        
        // Test Unknown variant
        let unknown = NormalizationResult::Unknown {
            original_identifier: "TEST".to_string(),
            identifier_type: "test_type".to_string(),
            context: "Test context".to_string(),
        };
        
        assert!(matches!(unknown, NormalizationResult::Unknown { .. }));
    }
}
