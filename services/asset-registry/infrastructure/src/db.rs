//! Database connection setup.
//!
//! The service's own schema is selected with `search_path`; no cross-schema
//! grant is used or required.

use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// Connects to Supabase Postgres and pins every pooled connection to the
/// `registry` schema.
///
/// # Errors
/// Returns the sqlx error when the database is unreachable or the schema is not
/// available to the role.
pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(300))
        .after_connect(|connection, _meta| {
            Box::pin(async move {
                sqlx::query("SET search_path TO registry")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(database_url)
        .await
}
