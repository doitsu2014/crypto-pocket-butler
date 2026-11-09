//! Readiness: Postgres reaches and the migrations are applied. C5 and C6
//! outages must not fail readiness (NFR1.2), so only the service's own database
//! is checked.

use async_trait::async_trait;
use portfolio_application::ports::ReadinessProbe;
use sqlx::PgPool;

/// Checks the portfolio database.
#[derive(Debug, Clone)]
pub struct PgReadinessProbe {
    pool: PgPool,
}

impl PgReadinessProbe {
    /// Creates the probe over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ReadinessProbe for PgReadinessProbe {
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
