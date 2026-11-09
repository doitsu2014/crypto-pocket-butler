//! Database connection setup.
//!
//! The service's own schema is selected with `search_path`; no cross-schema grant
//! is used or required. The connection budget follows the infrastructure design:
//! 10 per API replica and 8 for the worker.

use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// The API budget per replica.
pub const API_CONNECTIONS: u32 = 10;
/// The worker's budget.
pub const WORKER_CONNECTIONS: u32 = 8;

/// Connects to Supabase Postgres and pins every pooled connection to the
/// `reporting` schema.
///
/// # Errors
/// Returns the sqlx error when the database is unreachable or the schema is not
/// available to the role.
pub async fn connect(database_url: &str, max_connections: u32) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .min_connections(max_connections.min(5))
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .after_connect(|connection, _meta| {
            Box::pin(async move {
                sqlx::query("SET search_path TO reporting")
                    .execute(&mut *connection)
                    .await?;
                sqlx::query("SET statement_timeout = '10s'")
                    .execute(&mut *connection)
                    .await?;
                Ok(())
            })
        })
        .connect(database_url)
        .await
}
