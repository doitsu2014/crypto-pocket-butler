//! Database connection setup.
//!
//! The service's own schema is selected with `search_path`; no cross-schema
//! grant is used or required. The connection budget is the isolation: five
//! appends, one query, two exports.

use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// The append budget per replica.
pub const APPEND_CONNECTIONS: u32 = 5;
/// The query budget per replica.
pub const QUERY_CONNECTIONS: u32 = 1;
/// The export budget per replica.
pub const EXPORT_CONNECTIONS: u32 = 2;

/// Connects to Supabase Postgres and pins every pooled connection to the `audit`
/// schema.
///
/// # Errors
/// Returns the sqlx error when the database is unreachable or the schema is not
/// available to the role.
pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(APPEND_CONNECTIONS + QUERY_CONNECTIONS + EXPORT_CONNECTIONS)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(2))
        .idle_timeout(Duration::from_secs(300))
        .after_connect(|connection, _meta| {
            Box::pin(async move {
                sqlx::query("SET search_path TO audit")
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
