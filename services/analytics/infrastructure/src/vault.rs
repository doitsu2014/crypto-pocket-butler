//! Salt versions from Supabase Vault.
//!
//! `analytics.salt_versions` holds version labels only. The `SECURITY DEFINER`
//! function `read_analytics_salt(version)` returns the secret without a
//! cross-schema grant. The provider caches the 32-byte salts for five minutes
//! and may serve stale for at most ten; after that ingest fails closed.

use std::sync::Arc;
use std::time::{Duration, Instant};

use analytics_application::ports::{ExternalError, SaltProvider, SaltVersion};
use analytics_domain::SALT_RETENTION_MONTHS;
use async_trait::async_trait;
use base64::Engine;
use chrono::{DateTime, Months, Utc};
use sqlx::PgPool;
use tokio::sync::RwLock;

const DEFAULT_REFRESH_AFTER: Duration = Duration::from_secs(300);
const DEFAULT_MAX_STALENESS: Duration = Duration::from_secs(600);

#[derive(Debug, Clone)]
struct CachedSalts {
    current: SaltVersion,
    retained: Vec<SaltVersion>,
    fetched_at: Instant,
}

/// Reads salt versions through the Vault function.
pub struct PgSaltProvider {
    pool: PgPool,
    cache: RwLock<Option<CachedSalts>>,
    refresh_after: Duration,
    max_staleness: Duration,
}

impl PgSaltProvider {
    /// Creates the provider with the production cache windows.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            cache: RwLock::new(None),
            refresh_after: DEFAULT_REFRESH_AFTER,
            max_staleness: DEFAULT_MAX_STALENESS,
        }
    }

    async fn load(&self) -> Result<CachedSalts, ExternalError> {
        let rows: Vec<(String, DateTime<Utc>)> = sqlx::query_as(
            "SELECT version, created_at FROM salt_versions ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|error| {
            ExternalError::Unavailable(format!("salt versions could not be read: {error}"))
        })?;
        if rows.is_empty() {
            return Err(ExternalError::Refused(
                "no salt version is configured".to_string(),
            ));
        }
        let cutoff = Utc::now()
            .checked_sub_months(Months::new(
                u32::try_from(SALT_RETENTION_MONTHS).unwrap_or(12),
            ))
            .unwrap_or_else(|| Utc::now() - chrono::Duration::days(366));
        let mut retained = Vec::with_capacity(rows.len());
        for (version, created_at) in rows {
            if created_at < cutoff {
                continue;
            }
            let secret: String = sqlx::query_scalar("SELECT read_analytics_salt($1)")
                .bind(&version)
                .fetch_one(&self.pool)
                .await
                .map_err(|error| {
                    ExternalError::Refused(format!(
                        "salt version {version} could not be read: {error}"
                    ))
                })?;
            let salt = decode_salt(&secret).map_err(|reason| {
                ExternalError::Refused(format!("salt version {version} is invalid: {reason}"))
            })?;
            retained.push(SaltVersion { version, salt });
        }
        let current = retained.first().cloned().ok_or_else(|| {
            ExternalError::Refused("no salt version is inside the twelve-month window".to_string())
        })?;
        Ok(CachedSalts {
            current,
            retained,
            fetched_at: Instant::now(),
        })
    }

    async fn cached(&self) -> Result<CachedSalts, ExternalError> {
        let cached = self.cache.read().await.clone();
        if let Some(entry) = &cached {
            if entry.fetched_at.elapsed() < self.refresh_after {
                return Ok(entry.clone());
            }
        }
        match self.load().await {
            Ok(entry) => {
                *self.cache.write().await = Some(entry.clone());
                Ok(entry)
            }
            Err(error) => {
                if let Some(entry) = cached {
                    if entry.fetched_at.elapsed() <= self.max_staleness {
                        tracing::warn!(
                            error = %error,
                            "salt refresh failed; serving cached versions inside the staleness window"
                        );
                        return Ok(entry);
                    }
                }
                tracing::error!(error = %error, "salt unavailable and no fresh cache; failing closed");
                Err(error)
            }
        }
    }
}

#[async_trait]
impl SaltProvider for PgSaltProvider {
    async fn current(&self) -> Result<SaltVersion, ExternalError> {
        Ok(self.cached().await?.current)
    }

    async fn retained(&self) -> Result<Vec<SaltVersion>, ExternalError> {
        Ok(self.cached().await?.retained)
    }
}

/// Decodes a Vault salt value into exactly 32 bytes.
///
/// A 64-character hexadecimal string and a 44-character base64 string are both
/// accepted, because either is a common Vault representation; anything else is
/// used as raw bytes and must itself be 32 bytes.
///
/// # Errors
/// Returns a reason when the value is empty or does not decode to 32 bytes.
pub fn decode_salt(raw: &str) -> Result<Vec<u8>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("the salt is empty".to_string());
    }
    let bytes = if trimmed.len() == 64 && trimmed.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        hex::decode(trimmed).map_err(|error| format!("the salt is not hexadecimal: {error}"))?
    } else if trimmed.len() == 44 {
        base64::engine::general_purpose::STANDARD
            .decode(trimmed)
            .map_err(|error| format!("the salt is not base64: {error}"))?
    } else {
        trimmed.as_bytes().to_vec()
    };
    if bytes.len() != analytics_domain::SALT_BYTES {
        return Err(format!(
            "the salt is {} bytes, expected {}",
            bytes.len(),
            analytics_domain::SALT_BYTES
        ));
    }
    Ok(bytes)
}

/// Builds the `Arc<dyn SaltProvider>` the composition root passes inward.
#[must_use]
pub fn shared(pool: PgPool) -> Arc<dyn SaltProvider> {
    Arc::new(PgSaltProvider::new(pool))
}

#[cfg(test)]
mod tests {
    use super::decode_salt;
    use base64::Engine;

    #[test]
    fn decodes_a_hex_salt_to_thirty_two_bytes() {
        let salt = decode_salt(&"11".repeat(32)).expect("hex decodes");
        assert_eq!(salt.len(), 32);
        assert!(salt.iter().all(|byte| *byte == 0x11));
    }

    #[test]
    fn decodes_a_base64_salt() {
        let raw = base64::engine::general_purpose::STANDARD.encode([7u8; 32]);
        let salt = decode_salt(&raw).expect("base64 decodes");
        assert_eq!(salt, vec![7u8; 32]);
    }

    #[test]
    fn refuses_a_salt_of_the_wrong_length() {
        assert!(decode_salt("short").is_err());
    }

    #[test]
    fn refuses_an_empty_salt() {
        assert!(decode_salt("   ").is_err());
    }

    #[test]
    fn refuses_malformed_hex() {
        let raw = format!("{}zz", "11".repeat(31));
        assert!(decode_salt(&raw).is_err());
    }
}
