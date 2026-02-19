# Balance Normalization Guide

This document explains how token balance normalization works in the Crypto Pocket Butler API and provides examples of usage.

## Overview

Blockchain tokens store balances as integers to avoid floating-point precision issues. The actual human-readable value is calculated as:

```
normalized_balance = raw_balance / 10^decimals
```

### Common Token Decimals

- **ETH** and most ERC-20 tokens: 18 decimals
- **USDC**, **USDT**: 6 decimals
- **BTC**: 8 decimals (satoshis)
- **WBTC**: 8 decimals

## Canonical Storage Rule

> **All `quantity` values stored in account holdings in the database are normalized (human-readable) decimal strings.**

The EVM connector converts raw on-chain integers to human-readable decimals using `normalize_token_balance` _before_ returning the `Balance` struct. OKX already returns human-readable values. This means:

- `"1.5"` is stored for 1.5 ETH — **not** `"1500000000000000000"`.
- All downstream calculations (portfolio construction, value estimation) operate on normalized quantities and must **not** apply `normalize_token_balance` again.
- The optional `decimals` field in holdings is retained as metadata only and should **not** be used to re-normalize the `quantity`.

## Helper Functions

The balance normalization helper is located at `api/src/helpers/balance_normalization.rs`.

### `normalize_token_balance(raw_balance: &str, decimals: u8)`

Converts a raw integer balance to a human-readable decimal string. Called once in the EVM connector before storing; **do not call again on already-normalized quantities**.

**Example:**
```rust
use crypto_pocket_butler_backend::helpers::balance_normalization::normalize_token_balance;

// ETH with 18 decimals
let eth_balance = normalize_token_balance("1500000000000000000", 18).unwrap();
assert_eq!(eth_balance, "1.50"); // 1.5 ETH

// USDC with 6 decimals  
let usdc_balance = normalize_token_balance("1500000", 6).unwrap();
assert_eq!(usdc_balance, "1.50"); // 1.5 USDC

// Very small balance from the issue example
let tiny_eth = normalize_token_balance("291725391649", 18).unwrap();
assert_eq!(tiny_eth, "0.000000291725391649"); // 0.000000291... ETH
```

### `normalize_and_format_balance(raw_balance: &str, decimals: u8, display_decimals: u32)`

Normalizes and formats a raw balance for display with a specified number of decimal places.
Use this only for display formatting of raw values fetched directly from the chain, not for values already stored in the database.

**Example:**
```rust
use crypto_pocket_butler_backend::helpers::balance_normalization::normalize_and_format_balance;

// Show ETH with 4 decimal places
let eth = normalize_and_format_balance("1234567890123456789", 18, 4).unwrap();
assert_eq!(eth, "1.2345");

// Show USDC with 2 decimal places
let usdc = normalize_and_format_balance("1234567", 6, 2).unwrap();
assert_eq!(usdc, "1.23");
```

## API Integration

### Balance Struct

The `Balance` struct (in `api/src/connectors/mod.rs`) stores the **normalized** quantity:

```rust
pub struct Balance {
    pub asset: String,
    /// Normalized (human-readable) decimal quantity, e.g. "1.5" for 1.5 ETH
    pub quantity: String,
    pub available: String,
    pub frozen: String,
    /// Kept as metadata; the quantity field is already normalized.
    pub decimals: Option<u8>,
}
```

### Portfolio Holdings Response

The portfolio holdings API endpoint (`GET /api/v1/portfolios/{id}/holdings`) returns normalized balances. `total_quantity` and `normalized_quantity` are identical — both represent the human-readable value:

```json
{
  "portfolio_id": "...",
  "total_value_usd": 3000.0,
  "holdings": [
    {
      "asset": "ETH",
      "total_quantity": "1.50",
      "decimals": 18,
      "normalized_quantity": "1.50",
      "price_usd": 2000.0,
      "value_usd": 3000.0,
      "accounts": [
        {
          "account_id": "...",
          "account_name": "My Wallet",
          "quantity": "1.50",
          "decimals": 18,
          "normalized_quantity": "1.50"
        }
      ]
    }
  ]
}
```

### EVM Connector

The EVM connector (`api/src/connectors/evm.rs`) normalizes balances immediately after fetching:

1. **Native tokens** (ETH, BNB) use 18 decimals
2. **ERC-20 tokens** fetch decimals by calling the `decimals()` method on the contract, then normalize

```rust
// After fetching raw balance from chain, normalize before returning:
let normalized = normalize_token_balance(&raw_balance_str, token_decimals)
    .unwrap_or_else(|_| raw_balance_str.clone());
```

## Data Migration

Migration `m20260219_000002_normalize_holdings` converts any existing raw on-chain integers
in account holdings to normalized decimal strings. It only touches holdings where:
- `decimals` is set in the holding JSON, **and**
- `quantity` has no decimal point (i.e., is still a raw integer)

## Determining Decimals for Tokens

### On-Chain Method (ERC-20)

For ERC-20 tokens, decimals are determined by calling the `decimals()` function on the smart contract:

```solidity
function decimals() public view returns (uint8);
```

This is the most reliable method and is what the EVM connector uses.

### Off-Chain Method (API/Database)

For centralized exchanges (like OKX), decimals information is not required because the exchange already returns human-readable values.

**Note:** The OKX connector does not set `decimals` since the balances it returns are already in human-readable form.

## Wallet Token Discovery

### Current Implementation

The EVM connector checks a predefined list of common tokens per chain:

- **Ethereum**: USDT, USDC, DAI, WETH
- **Arbitrum**: USDT, USDC, DAI, WETH
- **Optimism**: USDT, USDC, DAI, WETH
- **Base**: USDC, DAI, WETH
- **BSC**: USDT, USDC, DAI, WBNB

This list is defined in `api/src/connectors/evm.rs` in the `get_common_tokens()` function.

### Limitations

**Full on-chain token discovery is not currently implemented** due to the following challenges:

1. **No Standard Registry**: There is no complete on-chain registry of all ERC-20 tokens
2. **Event Log Scanning**: Would require scanning all `Transfer` events to/from the wallet address
3. **Performance**: Event log queries can be slow and expensive on public RPC nodes
4. **Rate Limits**: Public RPC endpoints have strict rate limits

### Future Enhancements

To implement full token discovery, consider:

1. **Indexing Services**: Use services like The Graph, Alchemy, or Moralis that index blockchain data
2. **Token Lists**: Use community-maintained token lists (e.g., Uniswap token list)
3. **Hybrid Approach**: Combine common token checks with event log scanning for recent transactions

## Error Handling

The normalization functions return `Result<String, NormalizationError>`:

```rust
pub enum NormalizationError {
    InvalidBalance(String),     // Invalid balance string (not a number)
    ArithmeticOverflow,         // Calculation overflow
}
```

**Example error handling:**
```rust
match normalize_token_balance(raw_balance, decimals) {
    Ok(normalized) => println!("Balance: {}", normalized),
    Err(NormalizationError::InvalidBalance(msg)) => {
        eprintln!("Invalid balance: {}", msg);
    }
    Err(NormalizationError::ArithmeticOverflow) => {
        eprintln!("Balance too large");
    }
}
```

## Testing

Comprehensive unit tests are available in `api/src/helpers/balance_normalization.rs`:

```bash
cd api
cargo test helpers::balance_normalization::tests
```

Test coverage includes:
- ETH normalization (18 decimals)
- USDC normalization (6 decimals)
- BTC normalization (8 decimals)
- Zero balances
- Very small balances (1 wei)
- Very large balances
- Tokens with 0 decimals
- Invalid input handling
- Formatting with different decimal places

## Best Practices

1. **Store normalized quantities**: The DB canonical value is the human-readable decimal, not the raw integer.
2. **Normalize at ingestion**: Call `normalize_token_balance` once in the connector, never in downstream calculation code.
3. **Do not double-normalize**: Reading a `quantity` from holdings and calling `normalize_token_balance` again will produce incorrect results.
4. **Handle missing decimals gracefully**: Not all connectors provide decimals (e.g., OKX). For OKX the quantity is already human-readable.
5. **Use Decimal for arithmetic**: Use `rust_decimal::Decimal` for precise calculations, not `f64`.

## Examples

### Example 1: Normalized ETH Balance Stored in DB

```rust
// EVM connector normalizes before returning:
let raw_wei = "291725391649";  // from on-chain
let normalized = normalize_token_balance(raw_wei, 18).unwrap();
// normalized == "0.000000291725391649"
// This is what gets stored in account holdings and used for all calculations.
```

### Example 2: USDC Balance

```rust
// EVM connector normalizes before returning:
let raw = "706000";  // from on-chain
let normalized = normalize_token_balance(raw, 6).unwrap();
// normalized == "0.706"
// Stored as "0.706" in holdings, used directly for value calculation.
```

### Example 3: Value Calculation (Correct)

```rust
// quantity from DB holdings is already normalized:
let quantity: Decimal = "1.5".parse().unwrap();  // 1.5 ETH
let price_usd: f64 = 2000.0;
let value_usd = quantity.to_f64().unwrap() * price_usd;
// value_usd == 3000.0  ✓
```

## See Also

- [EVM Connector Documentation](../../api/src/connectors/evm.rs)
- [Balance Normalization Helper](../../api/src/helpers/balance_normalization.rs)
- [Portfolio Handlers](../../api/src/handlers/portfolios.rs)

