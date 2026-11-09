//! Provider keys from Supabase Vault.
//!
//! The `market_data` schema owns `read_provider_secret(text)` as a
//! `SECURITY DEFINER` function, so the service never needs a cross-schema grant.
//! A key is revealed only for the provider call and is never logged, echoed, or
//! stored.

use async_trait::async_trait;
use market_data_application::ports::{ExternalError, ProviderSecrets};
use sqlx::PgPool;

/// Reads provider keys through the Vault function.
#[derive(Debug, Clone)]
pub struct PgProviderSecrets {
    pool: PgPool,
}

impl PgProviderSecrets {
    /// Creates the adapter over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProviderSecrets for PgProviderSecrets {
    async fn provider_key(&self, source: &str) -> Result<Option<String>, ExternalError> {
        let secret = sqlx::query_scalar::<_, Option<String>>("SELECT read_provider_secret($1)")
            .bind(source)
            .fetch_one(&self.pool)
            .await
            .map_err(|error| {
                // A missing grant is a fatal misconfiguration: the use case stops
                // the run, records it failed, and writes the C10 event. The driver
                // message names no secret value.
                if matches!(
                    &error,
                    sqlx::Error::Database(database)
                        if database.code().as_deref() == Some("42501")
                ) {
                    ExternalError::Refused(format!(
                        "the provider secret function was refused: {error}"
                    ))
                } else {
                    ExternalError::Unavailable(format!(
                        "the provider key could not be read: {error}"
                    ))
                }
            })?;
        Ok(secret.filter(|value| !value.trim().is_empty()))
    }
}
