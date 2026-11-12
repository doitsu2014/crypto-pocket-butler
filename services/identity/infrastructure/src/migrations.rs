//! Versioned sqlx migrations. Every up migration has a matching down migration.
//!
//! Migration `0005_preferences_consent` alters a live table, so it is applied
//! **statement by statement outside any transaction**: its backfills commit in
//! bounded batches, its indexes are built `CONCURRENTLY`, and its CHECKs are
//! added `NOT VALID` and then validated. PostgreSQL refuses `CREATE INDEX
//! CONCURRENTLY` (and a procedure's `COMMIT`) when a multi-statement simple query
//! is wrapped in one implicit transaction, which is why the runner executes each
//! top-level statement on its own. The other migrations are transactional and go
//! through sqlx's migrator as before.

use std::borrow::Cow;

use sqlx::migrate::{MigrateError, Migration, MigrationType, Migrator};
use sqlx::PgPool;

/// Migrations embedded at compile time from `services/identity/migrations`.
///
/// Every up/down pair is picked up when this crate recompiles; the macro tracks
/// the directory it embeds.
pub static MIGRATOR: Migrator = sqlx::migrate!("../migrations");

/// The live-table migration the statement-wise runner applies.
pub const LIVE_SAFE_VERSION: i64 = 5;

/// The migrations below `version`, applied transactionally by sqlx.
/// `ignore_missing` keeps an applied live-table migration from making the
/// subset's validation fail on the next run.
#[must_use]
pub fn migrator_before(version: i64) -> Migrator {
    subset(|migration| migration.version < version)
}

/// The migrations above `version`, applied transactionally after the live-table
/// migration. A future `0006` must never run before `0005`.
fn migrator_after(version: i64) -> Migrator {
    subset(|migration| migration.version > version)
}

/// Every transactional migration (the live-table migration is excluded).
fn transactional_migrator() -> Migrator {
    subset(|migration| migration.version != LIVE_SAFE_VERSION)
}

fn subset(keep: impl Fn(&Migration) -> bool) -> Migrator {
    Migrator {
        migrations: Cow::Owned(
            MIGRATOR
                .migrations
                .iter()
                .filter(|migration| keep(migration))
                .cloned()
                .collect(),
        ),
        ignore_missing: true,
        locking: true,
        no_tx: false,
    }
}

/// Applies every pending migration to the database behind `pool`.
///
/// # Errors
/// Returns the migration error when a migration fails; the caller must not start
/// serving traffic.
pub async fn run(pool: &PgPool) -> Result<(), MigrateError> {
    migrator_before(LIVE_SAFE_VERSION).run(pool).await?;
    apply_live_safe(pool).await?;
    migrator_after(LIVE_SAFE_VERSION).run(pool).await
}

/// Reverts the most recent migration, including its down migration.
///
/// # Errors
/// Returns the migration error when the rollback fails.
pub async fn revert(pool: &PgPool) -> Result<(), MigrateError> {
    let newest: Option<i64> =
        sqlx::query_scalar("SELECT max(version) FROM _sqlx_migrations WHERE success")
            .fetch_one(pool)
            .await
            .map_err(MigrateError::from)?;
    match newest {
        None => Ok(()),
        Some(version) if version == LIVE_SAFE_VERSION => {
            let migration = find(LIVE_SAFE_VERSION, MigrationType::ReversibleDown)
                .ok_or(MigrateError::VersionMissing(LIVE_SAFE_VERSION))?;
            run_statements(pool, &migration.sql).await?;
            sqlx::query("DELETE FROM _sqlx_migrations WHERE version = $1")
                .bind(LIVE_SAFE_VERSION)
                .execute(pool)
                .await
                .map_err(MigrateError::from)?;
            Ok(())
        }
        Some(version) => {
            let mut connection = pool.acquire().await.map_err(MigrateError::from)?;
            // `undo` reverts every applied migration above `target`, so this
            // reverts only the newest one. The live-table version is excluded
            // here; it is handled by the branch above when it is the newest.
            let result = transactional_migrator()
                .undo(&mut connection, version - 1)
                .await;
            if result.is_err() {
                // sqlx's migrator takes a session advisory lock and does not
                // release it on the error path. Without this the lock would ride
                // the pooled connection back into the pool and wedge every later
                // migration (a refused rollback must stay retryable).
                sqlx::query("SELECT pg_advisory_unlock_all()")
                    .execute(&mut *connection)
                    .await
                    .map_err(MigrateError::from)?;
            }
            result
        }
    }
}

async fn apply_live_safe(pool: &PgPool) -> Result<(), MigrateError> {
    if applied(pool, LIVE_SAFE_VERSION).await? {
        return Ok(());
    }
    let migration = find(LIVE_SAFE_VERSION, MigrationType::ReversibleUp)
        .ok_or(MigrateError::VersionMissing(LIVE_SAFE_VERSION))?;
    run_statements(pool, &migration.sql).await?;
    sqlx::query(
        "INSERT INTO _sqlx_migrations
             (version, description, success, checksum, execution_time)
         VALUES ($1, $2, TRUE, $3, -1)",
    )
    .bind(migration.version)
    .bind(&*migration.description)
    .bind(&*migration.checksum)
    .execute(pool)
    .await
    .map_err(MigrateError::from)?;
    Ok(())
}

fn find(version: i64, kind: MigrationType) -> Option<&'static Migration> {
    MIGRATOR
        .migrations
        .iter()
        .find(|migration| migration.version == version && migration.migration_type == kind)
}

async fn applied(pool: &PgPool, version: i64) -> Result<bool, MigrateError> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM _sqlx_migrations WHERE version = $1 AND success)",
    )
    .bind(version)
    .fetch_one(pool)
    .await
    .map_err(MigrateError::from)
}

/// Executes a migration file one top-level statement at a time on a single
/// connection. Each statement is its own implicit transaction unless the script
/// opens one with `BEGIN`, which is what lets a `DO`/`CALL` block commit and a
/// `CREATE INDEX CONCURRENTLY` run outside a transaction block.
async fn run_statements(pool: &PgPool, sql: &str) -> Result<(), MigrateError> {
    let mut connection = pool.acquire().await.map_err(MigrateError::from)?;
    // The service pool pins a 10 s statement timeout; a backfill batch or a
    // concurrent index build may exceed it. The migration connection is
    // dedicated to this file, so lift the timeout for its duration.
    sqlx::query("SET statement_timeout = 0")
        .execute(&mut *connection)
        .await
        .map_err(MigrateError::from)?;
    for statement in split_statements(sql) {
        sqlx::query(&statement)
            .execute(&mut *connection)
            .await
            .map_err(MigrateError::from)?;
    }
    Ok(())
}

/// Splits a SQL script on top-level semicolons, respecting line and block
/// comments, single- and double-quoted strings, and `$tag$` dollar quoting. The
/// migration files use all four, so a naive `split(';')` would cut a function
/// body in half.
#[must_use]
pub fn split_statements(sql: &str) -> Vec<String> {
    let bytes = sql.as_bytes();
    let mut statements = Vec::new();
    let mut start = 0usize;
    let mut index = 0usize;

    while index < bytes.len() {
        match bytes[index] {
            b'\'' => {
                index += 1;
                while index < bytes.len() {
                    if bytes[index] == b'\'' {
                        if index + 1 < bytes.len() && bytes[index + 1] == b'\'' {
                            index += 2;
                            continue;
                        }
                        index += 1;
                        break;
                    }
                    index += 1;
                }
            }
            b'"' => {
                index += 1;
                while index < bytes.len() {
                    if bytes[index] == b'"' {
                        if index + 1 < bytes.len() && bytes[index + 1] == b'"' {
                            index += 2;
                            continue;
                        }
                        index += 1;
                        break;
                    }
                    index += 1;
                }
            }
            b'-' if bytes.get(index + 1) == Some(&b'-') => {
                index += 2;
                while index < bytes.len() && bytes[index] != b'\n' {
                    index += 1;
                }
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                index += 2;
                let mut depth = 1usize;
                while index < bytes.len() && depth > 0 {
                    if bytes.get(index) == Some(&b'/') && bytes.get(index + 1) == Some(&b'*') {
                        depth += 1;
                        index += 2;
                    } else if bytes.get(index) == Some(&b'*') && bytes.get(index + 1) == Some(&b'/')
                    {
                        depth -= 1;
                        index += 2;
                    } else {
                        index += 1;
                    }
                }
            }
            b'$' => {
                let mut tag_end = index + 1;
                while tag_end < bytes.len()
                    && (bytes[tag_end].is_ascii_alphanumeric() || bytes[tag_end] == b'_')
                {
                    tag_end += 1;
                }
                if bytes.get(tag_end) == Some(&b'$') {
                    let tag = &sql[index..=tag_end];
                    let body_start = tag_end + 1;
                    match sql[body_start..].find(tag) {
                        Some(offset) => index = body_start + offset + tag.len(),
                        None => index = bytes.len(),
                    }
                } else {
                    index += 1;
                }
            }
            b';' => {
                let statement = sql[start..index].trim();
                if !statement.is_empty() {
                    statements.push(statement.to_string());
                }
                index += 1;
                start = index;
            }
            _ => index += 1,
        }
    }

    let tail = sql[start..].trim();
    if !tail.is_empty() {
        statements.push(tail.to_string());
    }
    statements
}

#[cfg(test)]
mod tests {
    use super::split_statements;

    #[test]
    fn a_script_splits_on_top_level_semicolons_only() {
        // A line comment, a single-quoted semicolon, a block comment, and a
        // dollar-quoted function body must all stay inside one statement.
        let sql = "-- header; not a split\n\
                   CREATE TABLE t (a text DEFAULT 'x;y');\n\
                   /* block; */ CREATE OR REPLACE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$\n\
                   BEGIN\n  RAISE NOTICE 'a;b';\nEND\n$$;\n\
                   CALL f();";
        let statements = split_statements(sql);

        assert_eq!(statements.len(), 3);
        assert!(statements[0].contains("CREATE TABLE t"));
        assert!(statements[1].contains("FUNCTION f()"));
        assert_eq!(statements[2], "CALL f()");
    }

    #[test]
    fn a_dollar_quote_with_a_tag_is_one_statement() {
        let statements = split_statements("DO $do$ BEGIN PERFORM 1; END $do$; SELECT 1;");
        assert_eq!(statements.len(), 2);
        assert_eq!(statements[0], "DO $do$ BEGIN PERFORM 1; END $do$");
        assert_eq!(statements[1], "SELECT 1");
    }
}
