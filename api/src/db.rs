use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};
use std::time::Duration;

/// Database configuration and connection pool
/// 
/// This struct provides configuration for the database connection pool,
/// which is essential for handling concurrent requests efficiently.
/// The connection pool maintains a set of reusable database connections
/// to avoid the overhead of creating new connections for each request.
pub struct DbConfig {
    pub database_url: String,
    /// Maximum number of connections in the pool (supports concurrent requests)
    pub max_connections: u32,
    /// Minimum number of idle connections to maintain
    pub min_connections: u32,
    /// Maximum time to wait for a new connection
    pub connect_timeout: Duration,
    /// Maximum time to wait to acquire a connection from the pool
    pub acquire_timeout: Duration,
    /// Maximum idle time before a connection is closed
    pub idle_timeout: Duration,
    /// Maximum lifetime of a connection before it's closed and recreated
    pub max_lifetime: Duration,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/crypto_pocket_butler".to_string()),
            // Connection pool configuration optimized for concurrent requests
            max_connections: std::env::var("DB_MAX_CONNECTIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100),
            min_connections: std::env::var("DB_MIN_CONNECTIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
            connect_timeout: Duration::from_secs(
                std::env::var("DB_CONNECT_TIMEOUT_SECS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(30)
            ),
            acquire_timeout: Duration::from_secs(
                std::env::var("DB_ACQUIRE_TIMEOUT_SECS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(30)
            ),
            idle_timeout: Duration::from_secs(
                std::env::var("DB_IDLE_TIMEOUT_SECS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(600) // 10 minutes
            ),
            max_lifetime: Duration::from_secs(
                std::env::var("DB_MAX_LIFETIME_SECS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1800) // 30 minutes
            ),
        }
    }
}

impl DbConfig {
    /// Create a new database connection pool with the given configuration
    /// 
    /// This establishes a connection pool that can handle multiple concurrent
    /// database operations efficiently. The pool automatically manages connections,
    /// reusing them across requests to minimize overhead.
    pub async fn connect(&self) -> Result<DatabaseConnection, DbErr> {
        let mut opt = ConnectOptions::new(&self.database_url);
        opt.max_connections(self.max_connections)
            .min_connections(self.min_connections)
            .connect_timeout(self.connect_timeout)
            .acquire_timeout(self.acquire_timeout)
            .idle_timeout(self.idle_timeout)
            .max_lifetime(self.max_lifetime)
            .sqlx_logging(true)
            .sqlx_logging_level(log::LevelFilter::Info);

        tracing::info!(
            "Initializing database connection pool with max_connections={}, min_connections={}",
            self.max_connections,
            self.min_connections
        );

        Database::connect(opt).await
    }

    /// Create a new database connection pool from environment variables
    /// 
    /// Environment variables for configuration:
    /// - DATABASE_URL: Database connection string
    /// - DB_MAX_CONNECTIONS: Maximum number of connections (default: 100)
    /// - DB_MIN_CONNECTIONS: Minimum number of connections (default: 5)
    /// - DB_CONNECT_TIMEOUT_SECS: Connection timeout in seconds (default: 30)
    /// - DB_ACQUIRE_TIMEOUT_SECS: Acquire timeout in seconds (default: 30)
    /// - DB_IDLE_TIMEOUT_SECS: Idle timeout in seconds (default: 600)
    /// - DB_MAX_LIFETIME_SECS: Max lifetime in seconds (default: 1800)
    pub async fn from_env() -> Result<DatabaseConnection, DbErr> {
        Self::default().connect().await
    }
}
