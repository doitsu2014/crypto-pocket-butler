//! Versioned sqlx migrations. Every up migration has a matching down migration.

use sqlx::migrate::{MigrateError, Migrator};
use sqlx::PgPool;

/// Migrations embedded at compile time from `services/market-data/migrations`.
pub static MIGRATOR: Migrator = sqlx::migrate!("../migrations");

/// Applies every pending migration.
///
/// # Errors
/// Returns the migration error when a migration fails; the caller must not start
/// serving traffic.
pub async fn run(pool: &PgPool) -> Result<(), MigrateError> {
    MIGRATOR.run(pool).await
}

/// Reverts the most recent migration, including its down migration.
///
/// # Errors
/// Returns the migration error when the rollback fails.
pub async fn revert(pool: &PgPool) -> Result<(), MigrateError> {
    let mut connection = pool.acquire().await.map_err(MigrateError::from)?;
    MIGRATOR.undo(&mut connection, 0).await?;
    Ok(())
}
