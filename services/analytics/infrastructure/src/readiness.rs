//! Readiness: Postgres, the migration table, and the current pseudonymisation
//! salt. A missing salt fails readiness and ingest, while flag reads still work.

use std::sync::Arc;

use analytics_application::ports::{ReadinessProbe, SaltProvider};
use async_trait::async_trait;
use sqlx::PgPool;

/// Checks the database and the current salt.
#[derive(Clone)]
pub struct AnalyticsReadinessProbe {
    pool: PgPool,
    salts: Arc<dyn SaltProvider>,
}

impl AnalyticsReadinessProbe {
    /// Creates the probe.
    #[must_use]
    pub fn new(pool: PgPool, salts: Arc<dyn SaltProvider>) -> Self {
        Self { pool, salts }
    }
}

#[async_trait]
impl ReadinessProbe for AnalyticsReadinessProbe {
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
        self.salts
            .current()
            .await
            .map(|_| ())
            .map_err(|error| format!("the pseudonymisation salt is unavailable: {error}"))
    }
}
