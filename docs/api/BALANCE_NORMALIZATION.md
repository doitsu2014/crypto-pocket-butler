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

## Helper Functions

The balance normalization helper is located at `api/src/helpers/balance_normalization.rs`.

### `normalize_token_balance(raw_balance: &str, decimals: u8)`

Converts a raw integer balance to a human-readable decimal string.

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

Normalizes and formats balance for display with a specified number of decimal places.

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

The `Balance` struct (in `api/src/connectors/mod.rs`) now includes an optional `decimals` field:

```rust
pub struct Balance {
    pub asset: String,
    pub quantity: String,      // Raw balance (e.g., "291725391649")
    pub available: String,
    pub frozen: String,
    pub decimals: Option<u8>,  // Number of decimals (e.g., Some(18))
}
```

### Portfolio Holdings Response

The portfolio holdings API endpoint (`GET /api/v1/portfolios/{id}/holdings`) returns normalized balances:

```json
{
  "portfolio_id": "...",
  "total_value_usd": 1234.56,
  "holdings": [
    {
      "asset": "ETH",
      "total_quantity": "1500000000000000000",  // Raw balance
      "decimals": 18,
      "normalized_quantity": "1.50",             // Human-readable balance
      "price_usd": 2000.0,
      "value_usd": 3000.0,
      "accounts": [
        {
          "account_id": "...",
          "account_name": "My Wallet",
          "quantity": "1500000000000000000",
          "decimals": 18,
          "normalized_quantity": "1.50"
        }
      ]
    }
  ]
}
```

### EVM Connector

The EVM connector (`api/src/connectors/evm.rs`) automatically fetches decimals for tokens:

1. **Native tokens** (ETH, BNB) are hardcoded to 18 decimals
2. **ERC-20 tokens** fetch decimals by calling the `decimals()` method on the contract

```rust
// Fetching decimals from ERC20 contract
let contract = ERC20::new(contract_address, provider.clone());
let decimals = contract.decimals().call().await?;
```

## Determining Decimals for Tokens

### On-Chain Method (ERC-20)

For ERC-20 tokens, decimals are determined by calling the `decimals()` function on the smart contract:

```solidity
function decimals() public view returns (uint8);
```

This is the most reliable method and is what the EVM connector uses.

### Off-Chain Method (API/Database)

For centralized exchanges (like OKX), decimals information may need to be:
- Looked up in a reference database
- Hardcoded for common tokens
- Retrieved from external APIs (CoinGecko, CoinMarketCap)

**Note:** The OKX connector currently does not provide decimals as this information is not available from their API.

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

Example using event logs (not implemented):
```solidity
// ERC-20 Transfer event
event Transfer(address indexed from, address indexed to, uint256 value);

// Query all Transfer events where `to` or `from` is the wallet address
// Then check balance for each unique token contract found
```

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

1. **Always store raw balances**: Store the original raw balance string from the blockchain
2. **Display normalized values**: Show normalized values to users but keep raw values for calculations
3. **Handle missing decimals gracefully**: Not all connectors can provide decimals (e.g., OKX)
4. **Use Decimal for arithmetic**: Use `rust_decimal::Decimal` for precise calculations, not `f64`
5. **Validate input**: Check that balance strings are valid integers before normalization

## Examples

### Example 1: Display ETH Balance from Issue

```rust
// From the issue: ETH = 291725391649 wei
let raw_balance = "291725391649";
let decimals = 18;

let normalized = normalize_token_balance(raw_balance, decimals).unwrap();
// Result: "0.000000291725391649"

// For display with 6 decimals
let display = normalize_and_format_balance(raw_balance, decimals, 6).unwrap();
// Result: "0.000000"
```

### Example 2: Display USDC Balance from Issue

```rust
// From the issue: USDC = 706000
let raw_balance = "706000";
let decimals = 6;

let normalized = normalize_token_balance(raw_balance, decimals).unwrap();
// Result: "0.706"

// For display with 2 decimals
let display = normalize_and_format_balance(raw_balance, decimals, 2).unwrap();
// Result: "0.71"
```

### Example 3: Fetching and Normalizing EVM Token Balance

```rust
use alloy::primitives::Address;
use alloy::providers::ProviderBuilder;

// Connect to Ethereum
let provider = ProviderBuilder::new()
    .connect_http("https://eth.llamarpc.com".parse()?);

let wallet: Address = "0xYourWalletAddress".parse()?;
let token: Address = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse()?; // USDC

// Get token contract
let contract = ERC20::new(token, provider.clone());

// Fetch balance and decimals
let balance = contract.balanceOf(wallet).call().await?;
let decimals = contract.decimals().call().await?;

// Normalize
let normalized = normalize_token_balance(&balance.to_string(), decimals)?;
println!("USDC Balance: {}", normalized);
```

## See Also

- [EVM Connector Documentation](../../api/src/connectors/evm.rs)
- [Balance Normalization Helper](../../api/src/helpers/balance_normalization.rs)
- [Portfolio Handlers](../../api/src/handlers/portfolios.rs)
