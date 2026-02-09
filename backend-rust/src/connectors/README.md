# OKX Connector

This module implements a read-only connector for OKX exchange to fetch spot account balances.

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
