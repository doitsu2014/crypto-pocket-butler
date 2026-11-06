//! Database readiness probe.

use asset_registry_application::ports::ReadinessProbe;
use async_trait::async_trait;
use sqlx::PgPool;

/// Checks the registry database with a bounded query.
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
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}
