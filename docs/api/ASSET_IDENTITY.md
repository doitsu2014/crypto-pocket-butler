# Asset Identity Normalization Module

## Overview

The Asset Identity Normalization module provides a centralized entry point for normalizing asset identities from various sources (OKX symbols, EVM contract addresses) into canonical asset IDs and symbols.

## Purpose

- **Centralized Mapping**: Single source of truth for asset identity normalization across the application
- **OKX Symbol Support**: Maps OKX exchange symbols (e.g., "BTC", "ETH", "USDT") to canonical assets
- **EVM Contract Support**: Maps EVM contract addresses with chain context (e.g., "0xdAC17F958D2ee523a2206206994597C13D831ec7" on "ethereum") to canonical assets
- **Debug Information**: Provides detailed logging for all mapping decisions
- **Unknown Token Handling**: Gracefully handles unknown tokens with clear error information

## Core Types

### `AssetIdentity`

Represents a canonical asset identity after successful normalization.

```rust
pub struct AssetIdentity {
    pub asset_id: Uuid,           // Canonical asset ID from database
    pub symbol: String,            // Canonical symbol (e.g., "BTC", "ETH")
    pub name: String,              // Asset name (e.g., "Bitcoin")
    pub mapping_source: MappingSource, // How the asset was mapped
    pub debug_info: String,        // Debug information about the mapping
}
```

### `MappingSource`

Indicates the source of the asset identity mapping.

```rust
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
```

### `NormalizationResult`

The result of an asset normalization attempt.

```rust
pub enum NormalizationResult {
    /// Asset was successfully mapped
    Mapped(AssetIdentity),
    
    /// Asset could not be mapped (unknown token)
    Unknown {
        original_identifier: String,
        identifier_type: String,
        context: String,
    },
}
```

**Helper Methods:**
- `is_mapped() -> bool`: Check if the result is mapped
- `asset_identity() -> Option<&AssetIdentity>`: Get the asset identity if mapped
- `display_string() -> String`: Get a human-readable display string

## Usage

### Basic Usage

```rust
use crate::helpers::asset_identity::{AssetIdentityNormalizer, NormalizationResult};

// Create normalizer with database connection
let normalizer = AssetIdentityNormalizer::new(db.clone());

// Normalize an OKX symbol
let result = normalizer.normalize_from_okx("BTC").await;
match result {
    NormalizationResult::Mapped(identity) => {
        println!("Mapped to: {} ({})", identity.symbol, identity.asset_id);
    }
    NormalizationResult::Unknown { original_identifier, context, .. } => {
        println!("Unknown token: {} - {}", original_identifier, context);
    }
}
```

### Normalize from OKX Symbol

```rust
// Normalize OKX symbol (case-insensitive)
let result = normalizer.normalize_from_okx("btc").await;
let result = normalizer.normalize_from_okx("ETH").await;
let result = normalizer.normalize_from_okx("USDT").await;
```

**Behavior:**
- Symbol is normalized to uppercase for matching
- Matches against `assets` table by symbol
- Returns `Unknown` if symbol not found in database
- Logs all mapping decisions at INFO level for successful mappings, WARN for failures

### Normalize from EVM Contract Address

```rust
// Normalize EVM contract with chain context
let result = normalizer.normalize_from_evm_contract(
    "0xdAC17F958D2ee523a2206206994597C13D831ec7",
    "ethereum"
).await;

// Works with any supported chain
let result = normalizer.normalize_from_evm_contract(
    "0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9",
    "arbitrum"
).await;
```

**Behavior:**
- Contract address is normalized to lowercase
- Looks up contract in `asset_contracts` table by address and chain
- Returns the associated asset from `assets` table
- Returns `Unknown` if contract not found in database
- Logs all mapping decisions

**Supported Chains:**
- `ethereum`
- `arbitrum`
- `optimism`
- `base`
- `bsc` (Binance Smart Chain)

### Normalize from Generic Symbol

```rust
// Normalize a generic symbol (works for any source)
let result = normalizer.normalize_from_symbol("BTC").await;
```

**Special Handling for Chain-Specific Symbols:**

The `normalize_from_symbol` method has special handling for chain-specific symbols in the format `SYMBOL-CHAIN`:

```rust
// These are automatically handled
normalizer.normalize_from_symbol("ETH-ethereum").await;
normalizer.normalize_from_symbol("USDT-arbitrum").await;
normalizer.normalize_from_symbol("USDC-optimism").await;
```

**Behavior:**
- If symbol contains a `-` and the suffix matches a known chain, extracts the base symbol
- Attempts to match the base symbol directly (for native tokens like ETH)
- Falls back to standard symbol matching if no chain-specific handling applies
- Returns `Unknown` if symbol not found

## Integration with Portfolio Construction

The asset identity normalizer is integrated into the portfolio construction flow in `handlers/portfolios.rs`:

```rust
// In construct_portfolio_allocation handler
use crate::helpers::asset_identity::{AssetIdentityNormalizer, NormalizationResult};

let normalizer = AssetIdentityNormalizer::new(db.clone());

for (symbol, quantity) in holdings_map.iter() {
    // Normalize the asset symbol to get canonical asset identity
    let normalization_result = normalizer.normalize_from_symbol(symbol).await;
    
    match normalization_result {
        NormalizationResult::Mapped(asset_identity) => {
            // Use canonical asset_id to fetch prices
            let latest_price = asset_prices::Entity::find()
                .filter(asset_prices::Column::AssetId.eq(asset_identity.asset_id))
                .order_by_desc(asset_prices::Column::Timestamp)
                .one(&db)
                .await?;
            
            // Use canonical symbol in allocation
            // ...
        }
        NormalizationResult::Unknown { original_identifier, context, .. } => {
            // Mark as unpriced, use original symbol
            tracing::warn!("Could not normalize asset '{}': {}", original_identifier, context);
            // ...
        }
    }
}
```

## Logging and Debug Information

The module provides comprehensive logging at different levels:

### INFO Level Logs
- Successful mappings with full details

```
INFO: Mapped OKX symbol 'BTC' to asset 'BTC' (123e4567-e89b-12d3-a456-426614174000)
INFO: Mapped EVM contract '0xdAC17...' (chain: ethereum) to asset 'USDT' (...)
```

### WARN Level Logs
- Failed normalizations (unknown tokens)

```
WARN: Failed to normalize OKX symbol 'UNKNOWN': Symbol 'UNKNOWN' not found in assets database
WARN: Failed to normalize EVM contract '0x123...' on chain 'ethereum': Contract not found
```

### DEBUG Level Logs
- Detailed processing information

```
DEBUG: Normalizing OKX symbol: BTC
DEBUG: Normalizing EVM contract: 0xdAC17... on chain ethereum
DEBUG: Detected chain-specific symbol: USDT on chain ethereum
```

## Error Handling

The module handles errors gracefully:

1. **Empty Input**: Returns `Unknown` with clear context
2. **Database Errors**: Returns `Unknown` with error details logged
3. **Not Found**: Returns `Unknown` with helpful context
4. **Invalid Format**: Returns `Unknown` with validation error

All errors are logged appropriately and returned as `NormalizationResult::Unknown` variants, allowing calling code to decide how to handle unknown tokens.

## Testing

The module includes comprehensive unit tests:

```bash
# Run all asset identity tests
cargo test helpers::asset_identity::tests

# Run with output
cargo test helpers::asset_identity::tests -- --nocapture
```

**Test Coverage:**
- `test_normalization_result_is_mapped`: Verify result type checking
- `test_normalization_result_display`: Test display string generation
- `test_mapping_source_okx`: Verify OKX mapping source
- `test_mapping_source_evm_contract`: Verify EVM contract mapping source
- `test_mapping_source_direct_symbol`: Verify direct symbol mapping
- `test_asset_identity_fields`: Verify asset identity structure
- `test_serialization`: Verify JSON serialization
- `test_normalization_result_variants`: Verify enum variant matching

## Future Enhancements

Potential areas for future improvement:

1. **Caching**: Add in-memory caching for frequently accessed mappings
2. **Batch Operations**: Support batch normalization for multiple assets
3. **Fuzzy Matching**: Implement fuzzy matching for similar symbols
4. **External APIs**: Support fallback to external APIs (CoinGecko, CoinMarketCap) for unknown tokens
5. **Symbol Aliases**: Support common symbol aliases (e.g., "WETH" -> "ETH")
6. **Chain Detection**: Automatically detect chain from contract address format

## Best Practices

1. **Always use the normalizer**: Don't bypass the normalizer and query assets directly
2. **Handle Unknown results**: Always handle the `Unknown` variant appropriately
3. **Log mapping decisions**: The module logs automatically, but consider adding application-level logging for business decisions
4. **Keep database updated**: Ensure `assets` and `asset_contracts` tables are kept up-to-date
5. **Use canonical symbols**: Always use the canonical symbol from `AssetIdentity` in output

## Related Files

- **Implementation**: `api/src/helpers/asset_identity.rs`
- **Integration**: `api/src/handlers/portfolios.rs` (construct_portfolio_allocation)
- **Database Models**:
  - `api/src/entities/assets.rs`
  - `api/src/entities/asset_contracts.rs`
  - `api/src/entities/asset_prices.rs`
- **Connectors**:
  - `api/src/connectors/okx.rs`
  - `api/src/connectors/evm.rs`

## Contact

For questions or issues related to the asset identity normalization module, please refer to the repository's issue tracker or contact the maintainers.
