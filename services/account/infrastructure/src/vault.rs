//! The Supabase Vault adapter. A credential is stored as a secret and the service
//! keeps only its reference; key material never enters the account schema.

use account_application::ports::{ExternalError, VaultClient};
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

/// Stores, reveals, and destroys Vault secrets through the service's pool.
#[derive(Debug, Clone)]
pub struct PgVaultClient {
    pool: PgPool,
}

impl PgVaultClient {
    /// Creates the adapter over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl VaultClient for PgVaultClient {
    async fn store(&self, owner: &str, secret: &str) -> Result<String, ExternalError> {
        let name = format!("account-credential-{owner}-{}", Uuid::new_v4());
        let id: Uuid = sqlx::query_scalar("SELECT vault.create_secret($1, $2)")
            .bind(secret)
            .bind(name)
            .fetch_one(&self.pool)
            .await
            .map_err(|error| {
                ExternalError::Unavailable(format!("vault could not store the secret: {error}"))
            })?;
        Ok(id.to_string())
    }

    async fn reveal(&self, reference: &str) -> Result<String, ExternalError> {
        let id = Uuid::parse_str(reference.trim()).map_err(|_| {
            ExternalError::Refused("the Vault reference is not a secret id".to_string())
        })?;
        let secret: Option<String> = sqlx::query_scalar(
            "SELECT decrypted_secret FROM vault.decrypted_secrets WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| {
            ExternalError::Unavailable(format!("vault could not reveal the secret: {error}"))
        })?;
        secret
            .ok_or_else(|| ExternalError::Refused("the Vault secret no longer exists".to_string()))
    }

    async fn destroy(&self, reference: &str) -> Result<(), ExternalError> {
        let id = Uuid::parse_str(reference.trim()).map_err(|_| {
            ExternalError::Refused("the Vault reference is not a secret id".to_string())
        })?;
        sqlx::query("DELETE FROM vault.secrets WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|error| {
                ExternalError::Unavailable(format!("vault could not destroy the secret: {error}"))
            })?;
        Ok(())
    }
}
