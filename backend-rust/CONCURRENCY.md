# Concurrency and Performance

This document describes how the Crypto Pocket Butler backend handles concurrent requests efficiently using thread pools and connection pools.

## Architecture Overview

The backend is built on **Axum** and **Tokio**, providing robust concurrent request handling out of the box.

### Key Components

1. **Tokio Async Runtime (Thread Pool)**
2. **Database Connection Pool**
3. **Async Request Handlers**

## Thread Pool (Tokio Runtime)

### Configuration

The application uses Tokio's multi-threaded runtime with the `#[tokio::main]` attribute:

```rust
#[tokio::main]
async fn main() {
    // Application startup
}
```

This automatically creates a thread pool with:
- **Worker threads**: Equal to the number of CPU cores by default
- **Work-stealing scheduler**: Efficiently distributes tasks across threads
- **Non-blocking I/O**: All I/O operations are asynchronous

### How It Works

1. Each incoming HTTP request is handled as an asynchronous task
2. Tasks are scheduled across the thread pool
3. When a task awaits I/O (database query, HTTP request), the thread is freed to handle other tasks
4. This allows thousands of concurrent requests with a small number of threads

### Environment Variables

No explicit configuration needed. Tokio automatically scales to available CPU cores.

To override the number of threads (if needed):
```bash
export TOKIO_WORKER_THREADS=8  # Optional: override default
```

## Database Connection Pool

### Configuration

The connection pool is managed by SeaORM/SQLx and configured in `src/db.rs`:

```rust
pub struct DbConfig {
    pub max_connections: u32,      // Maximum concurrent connections
    pub min_connections: u32,      // Minimum idle connections
    pub connect_timeout: Duration,  // New connection timeout
    pub acquire_timeout: Duration,  // Pool acquisition timeout
    pub idle_timeout: Duration,     // Idle connection timeout
    pub max_lifetime: Duration,     // Connection max lifetime
}
```

### Default Values

- **max_connections**: 100 (handles up to 100 concurrent database operations)
- **min_connections**: 5 (always keep 5 connections ready)
- **connect_timeout**: 30 seconds
- **acquire_timeout**: 30 seconds
- **idle_timeout**: 600 seconds (10 minutes)
- **max_lifetime**: 1800 seconds (30 minutes)

### Environment Variables

All connection pool settings can be configured via environment variables:

```bash
# Database connection string
export DATABASE_URL="postgres://user:password@localhost/dbname"

# Connection pool settings
export DB_MAX_CONNECTIONS=100        # Maximum connections (default: 100)
export DB_MIN_CONNECTIONS=5          # Minimum connections (default: 5)
export DB_CONNECT_TIMEOUT_SECS=30    # Connection timeout (default: 30)
export DB_ACQUIRE_TIMEOUT_SECS=30    # Acquire timeout (default: 30)
export DB_IDLE_TIMEOUT_SECS=600      # Idle timeout (default: 600)
export DB_MAX_LIFETIME_SECS=1800     # Max lifetime (default: 1800)
```

### How It Works

1. **Connection Reuse**: Connections are reused across requests, avoiding the overhead of establishing new connections
2. **Pool Management**: The pool automatically creates/destroys connections based on demand
3. **Timeout Handling**: If all connections are busy, requests wait up to `acquire_timeout` before failing
4. **Connection Health**: Connections are validated and recycled after `max_lifetime`

## Concurrent Request Handling

### Request Flow

```
1. Client Request → Tokio Runtime
2. Task Spawned → Thread Pool
3. Auth Middleware → JWT Validation (non-blocking)
4. Handler Function → Business Logic
5. Database Query → Connection Pool (non-blocking)
6. Response → Client
```

### Example: Concurrent Portfolio Requests

When multiple users query their portfolios simultaneously:

1. Each request runs as an independent async task
2. JWT validation happens concurrently (non-blocking)
3. Database queries use different connections from the pool
4. No request blocks another request
5. Throughput scales with available CPU cores and database connections

### Code Example

All handlers are async and use the connection pool efficiently:

```rust
pub async fn list_portfolios(
    State(db): State<DatabaseConnection>,  // From connection pool
    Extension(token): Extension<KeycloakToken<String>>,
) -> Result<Json<Vec<PortfolioResponse>>, ApiError> {
    let user = get_or_create_user(&db, &token).await?;  // Non-blocking DB query
    
    let portfolios = portfolios::Entity::find()
        .filter(portfolios::Column::UserId.eq(user.id))
        .all(&db)  // Non-blocking DB query
        .await?;
    
    Ok(Json(portfolios.into_iter().map(|p| p.into()).collect()))
}
```

## Performance Characteristics

### Throughput

- **Async I/O**: Can handle thousands of concurrent requests
- **Thread Pool**: Scales with CPU cores (typically 8-16 threads)
- **Connection Pool**: Limits concurrent database operations (default: 100)

### Bottlenecks

The main bottleneck is typically the database connection pool:
- If `max_connections=100`, only 100 concurrent database operations can run
- Requests exceeding this limit wait for up to `acquire_timeout`
- Increase `DB_MAX_CONNECTIONS` if you need higher database concurrency

### Recommended Settings

For different workloads:

**Low Traffic** (< 10 req/s):
```bash
DB_MAX_CONNECTIONS=20
DB_MIN_CONNECTIONS=2
```

**Medium Traffic** (10-100 req/s):
```bash
DB_MAX_CONNECTIONS=50
DB_MIN_CONNECTIONS=5
```

**High Traffic** (> 100 req/s):
```bash
DB_MAX_CONNECTIONS=200
DB_MIN_CONNECTIONS=10
```

**Note**: Ensure your database server can handle the configured number of connections.

## Thread Safety

All components are thread-safe:

1. **DatabaseConnection**: Thread-safe connection pool
2. **KeycloakAuthInstance**: Wrapped in `Arc` for shared access
3. **Request Handlers**: Isolated state per request
4. **User Creation**: Uses database transactions for race condition protection

## Monitoring

### Logs

The application logs key concurrency information:

```
INFO: Tokio runtime: multi-threaded with 8 worker threads
INFO: Initializing database connection pool with max_connections=100, min_connections=5
INFO: Database connection pool established
INFO: Server ready to handle concurrent requests
```

### Metrics to Monitor

In production, monitor:
- **Active connections**: Should stay below `max_connections`
- **Connection wait time**: Should be < `acquire_timeout`
- **Request latency**: P50, P95, P99
- **Thread pool utilization**: CPU usage per worker thread

## Best Practices

1. **Don't Block**: Never use blocking I/O in handlers (use async equivalents)
2. **Connection Pool Size**: Set `max_connections` based on database capacity
3. **Timeouts**: Configure appropriate timeouts for your use case
4. **Connection Health**: Keep `max_lifetime` reasonable to recycle connections
5. **Load Testing**: Test with realistic concurrent loads before production

## Load Testing Example

Using `hey` or `wrk` to test concurrent requests:

```bash
# Install hey
go install github.com/rakyll/hey@latest

# Test with 100 concurrent users, 10000 requests
hey -n 10000 -c 100 -H "Authorization: Bearer YOUR_JWT_TOKEN" \
    http://localhost:3000/v1/portfolios
```

Expected results with default configuration:
- **Requests/sec**: 1000-5000 (depending on hardware)
- **Success rate**: 100%
- **P99 latency**: < 100ms (with warm connection pool)

## Troubleshooting

### "acquire_timeout exceeded"

**Cause**: All connections in the pool are busy
**Solution**: Increase `DB_MAX_CONNECTIONS` or optimize slow queries

### High CPU usage

**Cause**: CPU-bound operations in handlers
**Solution**: Offload CPU-intensive work to blocking thread pool or background tasks

### Connection pool exhaustion

**Cause**: Long-running queries holding connections
**Solution**: 
- Optimize queries
- Set statement timeouts in PostgreSQL
- Increase `DB_MAX_CONNECTIONS`

## External API Concurrency Control

### Overview

The backend implements bounded concurrency and rate limiting for external API calls to prevent overwhelming external services and respect rate limits. This uses the **bulkhead pattern** for fault isolation.

### Rate Limiters

Rate limiters are implemented using Tokio semaphores in `src/concurrency/mod.rs`:

```rust
use crate::concurrency::RateLimiter;

// Create a rate limiter for CoinGecko API
let rate_limiter = RateLimiter::coingecko();

// Acquire permit before making API call
let _permit = rate_limiter.acquire().await?;
// Make API call
```

### Available Rate Limiters

#### CoinGecko API
- **Max concurrent**: 5 requests
- **Min delay**: 2 seconds between requests
- **Rate limit**: ~30 requests/minute (demo plan)
- **Usage**: Automatically applied in `CoinGeckoConnector`

```rust
let connector = CoinGeckoConnector::new();
// Rate limiting is built-in, no need to manually acquire permits
let coins = connector.fetch_top_coins(100).await?;
```

#### OKX API
- **Max concurrent**: 3 requests
- **Min delay**: 100ms between requests
- **Usage**: For OKX exchange API calls

```rust
let rate_limiter = RateLimiter::okx();
let _permit = rate_limiter.acquire().await?;
// Make OKX API call
```

#### EVM RPC Calls
- **Max concurrent**: 5 chain requests
- **Min delay**: 50ms between requests
- **Usage**: For blockchain RPC calls (Ethereum, Arbitrum, etc.)

```rust
// Automatically applied in EvmConnector
let connector = EvmConnector::new(wallet_address, chains)?;
// Chains are fetched in parallel with rate limiting
let balances = connector.fetch_spot_balances().await?;
```

### Parallel Chain Fetching

The EVM connector fetches balances from multiple chains **in parallel** instead of sequentially:

```rust
// OLD: Sequential (5x slower for 5 chains)
for chain in chains {
    fetch_balance(chain).await;  // One at a time
}

// NEW: Parallel (bounded concurrency)
let tasks = chains.map(|chain| async move {
    let _permit = rate_limiter.acquire().await;
    fetch_balance(chain).await
});
futures::join_all(tasks).await;
```

**Performance Impact**: ~5x faster for multi-chain wallets

### Benefits

1. **Prevents Rate Limiting**: Respects API rate limits automatically
2. **Fault Isolation**: Failures in one service don't affect others
3. **Resource Protection**: Prevents overwhelming external services
4. **Predictable Performance**: Bounded concurrency ensures stable behavior

## Database Batching

### Overview

Database operations are batched to reduce roundtrips and improve performance. Instead of inserting/updating one record at a time, multiple records are batched in a single query.

### Batched Operations

#### Price Collection (`src/jobs/price_collection.rs`)

```rust
// OLD: N individual inserts for N prices
for price in prices {
    Insert::one(price).exec(db).await?;  // 1 DB roundtrip per price
}

// NEW: Single batch insert
Insert::many(prices)
    .on_conflict(...)
    .exec(db).await?;  // 1 DB roundtrip for all prices
```

**Performance Impact**: Reduces DB roundtrips from N to 1

#### Contract Addresses Collection

Batches contract inserts with a maximum batch size of 100:

```rust
// Collect contracts in a batch
let mut contracts = Vec::new();
for asset in assets {
    contracts.push(prepare_contract(asset));
    
    // Batch insert every 100 contracts
    if contracts.len() >= 100 {
        Insert::many(contracts).exec(db).await?;
        contracts.clear();
    }
}
```

**Performance Impact**: Reduces DB roundtrips by ~100x

#### Top Coins Collection

Batches ranking inserts for all coins in a single transaction:

```rust
// Collect all rankings first
let mut rankings = Vec::new();
for coin in coins {
    rankings.push(prepare_ranking(coin));
}

// Single batch insert
Insert::many(rankings)
    .on_conflict(...)
    .exec(db).await?;
```

**Performance Impact**: Reduces DB roundtrips from N to 1

### Idempotency

All batch operations use `ON CONFLICT` clauses to maintain idempotency:

```rust
Insert::many(records)
    .on_conflict(
        OnConflict::columns([...])  // Unique constraint columns
        .update_columns([...])       // Columns to update on conflict
        .to_owned(),
    )
    .exec(db).await?;
```

This ensures that running jobs multiple times produces the same result.

## Caching Layer

### Overview

A simple in-memory cache reduces redundant API calls and database queries using the `moka` crate.

### Available Caches

#### Price Cache (`src/cache.rs`)

Caches latest asset prices with a 60-second TTL:

```rust
use crate::cache::PriceCache;

let cache = PriceCache::new();

// Check cache first
if let Some(price) = cache.get(&asset_id).await {
    return price;
}

// Fetch from DB/API and cache
let price = fetch_price(asset_id).await?;
cache.insert(asset_id, price).await;
```

**Configuration**:
- **Max capacity**: 10,000 assets
- **TTL**: 60 seconds
- **Use case**: Portfolio valuation, price lookups

#### Chain Data Cache

Caches blockchain RPC responses with a 30-second TTL:

```rust
use crate::cache::ChainDataCache;

let cache = ChainDataCache::new();
let key = format!("{}:{}:balance", chain, address);

if let Some(cached) = cache.get(&key).await {
    return cached;
}

let data = fetch_chain_data(chain, address).await?;
cache.insert(key, data.clone()).await;
```

**Configuration**:
- **Max capacity**: 1,000 responses
- **TTL**: 30 seconds
- **Use case**: Wallet balance queries, chain data

### Cache Benefits

1. **Reduced API Calls**: Avoid redundant external requests
2. **Lower Latency**: In-memory access is much faster than API calls
3. **Cost Savings**: Fewer API calls = lower costs for paid tiers
4. **Graceful Degradation**: Cache continues serving during brief API outages

### Cache Invalidation

Caches use **time-based expiration** (TTL):
- Entries automatically expire after TTL
- No manual invalidation needed
- Short TTLs ensure data freshness

## Performance Summary

### Before Optimizations

- **EVM chains**: Sequential fetching (5x slower for 5 chains)
- **Price storage**: N DB roundtrips for N prices
- **Contract storage**: N DB roundtrips for N contracts
- **No rate limiting**: Risk of hitting API rate limits
- **No caching**: Every request hits external APIs/DB

### After Optimizations

- **EVM chains**: Parallel fetching with bounded concurrency
- **Price storage**: 1 DB roundtrip per batch
- **Contract storage**: 1 DB roundtrip per 100 contracts
- **Rate limiting**: Respects API limits automatically
- **Caching**: Hot data served from memory

### Expected Performance Improvements

| Operation | Before | After | Improvement |
|-----------|--------|-------|-------------|
| Multi-chain balance fetch | 5-10s | 1-2s | 5x faster |
| Price collection (100 assets) | 100 DB roundtrips | 1 DB roundtrip | 100x fewer queries |
| Contract collection (100 assets) | 100 DB roundtrips | 1-2 DB roundtrips | 50-100x fewer queries |
| Cached price lookup | 10-50ms (DB) | <1ms (memory) | 10-50x faster |

## Further Reading

- [Tokio Runtime Documentation](https://docs.rs/tokio/latest/tokio/runtime/)
- [SeaORM Connection Pool](https://www.sea-ql.org/SeaORM/docs/install-and-config/connection/)
- [Axum Concurrency Guide](https://docs.rs/axum/latest/axum/)
- [Moka Cache Documentation](https://docs.rs/moka/latest/moka/)
- [Tokio Semaphore](https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html)
