//! Readiness: Postgres reaches, the migrations are applied, and Vault is present
//! (the reliability design's `/readyz`).

use account_application::ports::ReadinessProbe;
use async_trait::async_trait;
use sqlx::PgPool;

/// Checks the account database and its Vault dependency.
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
        let vault: Option<String> = sqlx::query_scalar("SELECT to_regclass('vault.secrets')::text")
            .fetch_one(&self.pool)
            .await
            .map_err(|error| error.to_string())?;
        if vault.is_none() {
            return Err("Supabase Vault is not available".to_string());
        }
        Ok(())
    }
}
