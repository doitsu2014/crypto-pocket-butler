//! Versioned sqlx migrations. Every up migration has a matching down migration.

use sqlx::migrate::{MigrateError, Migrator};
use sqlx::PgPool;

/// Migrations embedded at compile time from `services/portfolio/migrations`.
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

/// Deletes snapshots older than the retention window. This runs only through the
/// migrator/operator DSN: the runtime `portfolio_svc` role is insert-only on
/// `valuation_snapshots` by grant.
///
/// # Errors
/// Returns the sqlx error when the delete fails.
pub async fn prune_snapshots(pool: &PgPool, older_than_days: i64) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM valuation_snapshots
         WHERE as_of < now() - make_interval(days => $1::int)",
    )
    .bind(older_than_days)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}
