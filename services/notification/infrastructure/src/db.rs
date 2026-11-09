//! Database connection setup.
//!
//! Every pooled connection is pinned to the `notification` schema with
//! `search_path`; no cross-schema grant is used or required.

use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// Connects to Supabase Postgres and pins every connection to `notification`.
///
/// # Errors
/// Returns the sqlx error when the database is unreachable or the schema is not
/// available to the role.
pub async fn connect(database_url: &str, max_connections: u32) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(max_connections.max(1))
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(2))
        .idle_timeout(Duration::from_secs(300))
        .after_connect(|connection, _meta| {
            Box::pin(async move {
                sqlx::query("SET search_path TO notification")
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
