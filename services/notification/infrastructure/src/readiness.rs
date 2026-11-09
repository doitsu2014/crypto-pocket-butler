//! Readiness: Postgres and its migrations only. A provider or C3 outage must
//! not fail readiness (NFR1.7).

use async_trait::async_trait;
use notification_application::ports::ReadinessProbe;
use sqlx::PgPool;

/// Checks the database and the migration table.
#[derive(Clone)]
pub struct NotificationReadinessProbe {
    pool: PgPool,
}

impl NotificationReadinessProbe {
    /// Creates the probe.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ReadinessProbe for NotificationReadinessProbe {
    async fn check(&self) -> Result<(), String> {
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|error| error.to_string())?;
        let applied: i64 =
            sqlx::query_scalar("SELECT count(*) FROM _sqlx_migrations WHERE success")
                .fetch_one(&self.pool)
                .await
                .map_err(|error| error.to_string())?;
        if applied == 0 {
            return Err("no migration has been applied".to_string());
        }
        Ok(())
    }
}
