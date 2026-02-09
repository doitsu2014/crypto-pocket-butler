# Exchange and Wallet Connectors

This module implements connectors for fetching balances from various exchanges and blockchain wallets.

## Available Connectors

### OKX Connector

A read-only connector for OKX exchange to fetch spot account balances.

## Features

- **Read-Only Access**: Uses OKX API keys with read-only permissions
- **HMAC-SHA256 Signature**: Implements OKX API authentication with proper signature generation
- **Spot Balance Fetching**: Retrieves all spot balances from OKX trading account
- **Error Handling**: Comprehensive error handling for API calls and network issues
- **Async/Await**: Non-blocking I/O using Tokio runtime

## API Authentication

The connector uses OKX's API authentication mechanism:

1. **API Key**: Public identifier for your API credentials
2. **API Secret**: Used to sign requests with HMAC-SHA256
3. **Passphrase**: Additional security layer required by OKX

### Signature Generation

The signature is generated using:
```
signature = Base64(HMAC-SHA256(timestamp + method + requestPath, secretKey))
```

## Usage

### Creating a Connector

```rust
use crypto_pocket_butler_backend::connectors::okx::OkxConnector;

let connector = OkxConnector::new(
    "your-api-key".to_string(),
    "your-api-secret".to_string(),
    "your-passphrase".to_string(),
);
```

### Fetching Balances

```rust
use crypto_pocket_butler_backend::connectors::ExchangeConnector;

let balances = connector.fetch_spot_balances().await?;

for balance in balances {
    println!("Asset: {}, Balance: {}", balance.asset, balance.quantity);
}
```

## API Endpoints Used

- **GET /api/v5/account/balance**: Fetches trading account balance details

## Security Considerations

1. **API Keys**: Store API keys encrypted at rest in the database
2. **Read-Only Permissions**: Use OKX API keys with read-only permissions only
3. **Rate Limiting**: OKX has rate limits; implement appropriate throttling if needed
4. **HTTPS**: All API calls use HTTPS for secure communication

## OKX API Documentation

For more information about OKX API, see:
- [OKX API Documentation](https://www.okx.com/docs-v5/en/)
- [Account Balance Endpoint](https://www.okx.com/docs-v5/en/#trading-account-rest-api-get-balance)

## Testing

Run the connector tests:

```bash
cargo test --package crypto-pocket-butler-backend --lib connectors::okx::tests
```

## Example Response Structure

The connector returns a vector of `Balance` objects:

```rust
pub struct Balance {
    pub asset: String,       // e.g., "BTC", "ETH", "USDT"
    pub quantity: String,    // Total balance
    pub available: String,   // Available balance
    pub frozen: String,      // Frozen balance
}
```

## Error Handling

The connector handles various error scenarios:
- Network errors
- Invalid API credentials
- OKX API errors (rate limits, maintenance, etc.)
- JSON parsing errors

All errors are returned as `Box<dyn Error + Send + Sync>` for flexibility.

## Future Enhancements

- Support for other account types (funding, trading)
- Support for futures balances
- Caching to reduce API calls
- Rate limit handling with automatic retry
- Websocket support for real-time balance updates

---

## EVM Wallet Connector

This module implements a connector for fetching native and ERC-20 token balances from EVM-compatible blockchain wallets.

### Supported Chains

- **Ethereum Mainnet**
- **Arbitrum**
- **Optimism**
- **Base**
- **Binance Smart Chain (BSC)**

### Features

- **Native Balance Fetching**: Fetches ETH balance for wallet addresses
- **ERC-20 Token Support**: Automatically checks common stablecoin and token balances
- **Multi-Chain Support**: Checks balances across multiple EVM chains
- **Public RPC Endpoints**: Uses reliable public RPC endpoints (LlamaRPC)
- **Async/Await**: Non-blocking I/O using Tokio runtime
- **Error Handling**: Graceful handling of RPC failures and invalid addresses

### Common Tokens Checked

The connector automatically checks balances for popular tokens on each chain:

**Ethereum:**
- USDT, USDC, DAI, WETH

**Arbitrum:**
- USDT, USDC, DAI, WETH

**Optimism:**
- USDT, USDC, DAI, WETH

**Base:**
- USDC, DAI, WETH

**Binance Smart Chain (BSC):**
- USDT, USDC, DAI, WBNB

### Usage

#### Creating a Connector

```rust
use crypto_pocket_butler_backend::connectors::evm::{EvmConnector, EvmChain};

let connector = EvmConnector::new(
    "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045".to_string(),
    vec![EvmChain::Ethereum, EvmChain::Arbitrum],
)?;
```

#### Fetching Balances

```rust
use crypto_pocket_butler_backend::connectors::ExchangeConnector;

let balances = connector.fetch_spot_balances().await?;

for balance in balances {
    println!("Asset: {}, Balance: {}", balance.asset, balance.quantity);
}
```

### RPC Endpoints

The connector uses public RPC endpoints:
- Ethereum: `https://eth.llamarpc.com`
- Arbitrum: `https://arbitrum.llamarpc.com`
- Optimism: `https://optimism.llamarpc.com`
- Base: `https://base.llamarpc.com`
- Binance Smart Chain: `https://bsc-dataseed.bnbchain.org`

### Security Considerations

1. **Public RPCs**: Uses public RPC endpoints - no API keys needed
2. **Read-Only**: Only reads blockchain data, never signs transactions
3. **Rate Limiting**: Public RPCs have rate limits; be mindful of request volume
4. **Address Validation**: Validates wallet addresses before making RPC calls

### Technology Stack

The connector is built on **Alloy**, the modern Rust toolkit for EVM interactions:
- High-performance ABI encoding/decoding
- Type-safe contract bindings via the `sol!` macro
- Async RPC provider with connection pooling
- Network-agnostic design for multi-chain support

### Testing

Run the connector tests:

```bash
cargo test --package crypto-pocket-butler-backend --lib connectors::evm::tests
```

### Example Response Structure

The connector returns a vector of `Balance` objects with chain information:

```rust
pub struct Balance {
    pub asset: String,       // e.g., "ETH-ethereum", "USDC-arbitrum"
    pub quantity: String,    // Raw balance (in wei for native, token units for ERC-20)
    pub available: String,   // Same as quantity (wallets don't have frozen balances)
    pub frozen: String,      // Always "0" for wallets
}
```

### Error Handling

The connector handles various error scenarios:
- Invalid wallet addresses
- Network/RPC errors
- Token contract call failures
- Chain-specific issues

All errors are logged but don't stop the entire sync process. If one chain fails, others continue.

### Future Enhancements

- Custom token list configuration
- Balance caching with TTL
- Support for NFT balances
- DeFi protocol position tracking
- Support for Bitcoin
- Multicall optimization for batch token queries

---

## Solana Wallet Connector

**Status**: Coming Soon

Solana wallet support is planned but temporarily on hold due to dependency version conflicts between the Solana SDK and existing project dependencies. The implementation will be added in a future update once the dependency tree is upgraded to support both stacks.

### Planned Features

- Native SOL balance fetching
- SPL token balance support (USDC, USDT, wrapped SOL, etc.)
- Uses public Solana RPC endpoints
- Associated token account derivation

### Workaround

For now, to track Solana wallet balances:
1. Use the account type "wallet" with `exchange_name` set to "solana"
2. The sync will return a friendly message indicating Solana support is coming soon
3. Check back in the next release for full Solana integration
