//! Database connection setup.
//!
//! Every pooled connection is pinned to the `ledger` schema with `search_path`;
//! no cross-schema grant is used or required.

use std::str::FromStr;
use std::time::Duration;

use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgSslMode};
use sqlx::PgPool;

fn options(database_url: &str) -> Result<PgConnectOptions, sqlx::Error> {
    Ok(PgConnectOptions::from_str(database_url)?.ssl_mode(PgSslMode::Require))
}

/// Connects to Supabase Postgres and pins every connection to `ledger`. TLS is
/// required (NFR9.1); the DSN is a secret and never logged.
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
                sqlx::query("SET search_path TO ledger")
                    .execute(&mut *connection)
                    .await?;
                sqlx::query("SET statement_timeout = '10s'")
                    .execute(&mut *connection)
                    .await?;
                Ok(())
            })
        })
        .connect_with(options(database_url)?)
        .await
}

/// Opens one short-lived connection pinned to `ledger`, for the anonymiser role.
/// The caller closes it immediately after the single update.
///
/// # Errors
/// Returns the sqlx error when the database is unreachable.
pub async fn connect_once(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(1)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(3))
        .idle_timeout(Duration::from_secs(1))
        .after_connect(|connection, _meta| {
            Box::pin(async move {
                sqlx::query("SET search_path TO ledger")
                    .execute(&mut *connection)
                    .await?;
                sqlx::query("SET statement_timeout = '5s'")
                    .execute(&mut *connection)
                    .await?;
                Ok(())
            })
        })
        .connect_with(options(database_url)?)
        .await
}
