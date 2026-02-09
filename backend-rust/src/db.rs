use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};
use std::time::Duration;

/// Database configuration and connection pool
pub struct DbConfig {
    pub database_url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout: Duration,
    pub acquire_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/crypto_pocket_butler".to_string()),
            max_connections: 100,
            min_connections: 5,
            connect_timeout: Duration::from_secs(30),
            acquire_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600), // 10 minutes
            max_lifetime: Duration::from_secs(1800), // 30 minutes
        }
    }
}

impl DbConfig {
    /// Create a new database connection pool with the given configuration
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

        Database::connect(opt).await
    }

    /// Create a new database connection pool from environment variables
    pub async fn from_env() -> Result<DatabaseConnection, DbErr> {
        Self::default().connect().await
    }
}
