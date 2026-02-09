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

## Further Reading

- [Tokio Runtime Documentation](https://docs.rs/tokio/latest/tokio/runtime/)
- [SeaORM Connection Pool](https://www.sea-ql.org/SeaORM/docs/install-and-config/connection/)
- [Axum Concurrency Guide](https://docs.rs/axum/latest/axum/)
