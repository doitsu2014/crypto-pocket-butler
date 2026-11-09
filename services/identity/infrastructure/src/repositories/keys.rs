//! Postgres implementation of [`ShreddingKeyRepository`].
//!
//! Key creation and destruction happen inside `identity.create_shredding_key` and
//! `identity.destroy_shredding_key`; only the pgsodium key id crosses the
//! boundary and no key material is ever returned.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use identity_application::ports::{RepositoryError, ShreddingKeyRepository};
use identity_domain::{ShreddingKey, ShreddingKeyId, ShreddingKeyStatus, UserId};
use sqlx::PgPool;
use uuid::Uuid;

use super::{invalid_data, map_sqlx_error};

const KEY_COLUMNS: &str = "id, user_id, vault_reference, key_id, status, created_at, destroyed_at";

/// sqlx-backed shredding-key references.
#[derive(Debug, Clone)]
pub struct PgShreddingKeyRepository {
    pool: PgPool,
}

impl PgShreddingKeyRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct KeyRow {
    id: Uuid,
    user_id: Uuid,
    vault_reference: String,
    key_id: i64,
    status: String,
    created_at: DateTime<Utc>,
    destroyed_at: Option<DateTime<Utc>>,
}

impl KeyRow {
    fn into_domain(self) -> Result<ShreddingKey, RepositoryError> {
        Ok(ShreddingKey::from_stored(
            ShreddingKeyId::new(self.id),
            UserId::new(self.user_id),
            self.vault_reference,
            self.key_id,
            ShreddingKeyStatus::parse(&self.status)
                .map_err(|error| invalid_data("read shredding key", error))?,
            self.created_at,
            self.destroyed_at,
        ))
    }
}

#[async_trait]
impl ShreddingKeyRepository for PgShreddingKeyRepository {
    async fn create_for_user(
        &self,
        user_id: UserId,
        now: DateTime<Utc>,
    ) -> Result<ShreddingKey, RepositoryError> {
        let row_id = ShreddingKeyId::generate();
        let vault_reference = format!("identity/user/{user_id}");
        let key_id: i64 = sqlx::query_scalar("SELECT create_shredding_key($1)")
            .bind(&vault_reference)
            .fetch_one(&self.pool)
            .await
            .map_err(|error| map_sqlx_error("create shredding key", error))?;

        sqlx::query(
            "INSERT INTO shredding_keys
                 (id, user_id, vault_reference, key_id, status, created_at, destroyed_at)
             VALUES ($1, $2, $3, $4, 'active', $5, NULL)",
        )
        .bind(row_id.as_uuid())
        .bind(user_id.as_uuid())
        .bind(&vault_reference)
        .bind(key_id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("store shredding key", error))?;

        Ok(ShreddingKey::from_stored(
            row_id,
            user_id,
            vault_reference,
            key_id,
            ShreddingKeyStatus::Active,
            now,
            None,
        ))
    }

    async fn active_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Option<ShreddingKey>, RepositoryError> {
        let row = sqlx::query_as::<_, KeyRow>(&format!(
            "SELECT {KEY_COLUMNS} FROM shredding_keys WHERE user_id = $1 AND status = 'active'"
        ))
        .bind(user_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find shredding key", error))?;
        row.map(KeyRow::into_domain).transpose()
    }

    async fn destroy(
        &self,
        key: ShreddingKeyId,
        now: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        let key_id: Option<i64> =
            sqlx::query_scalar("SELECT key_id FROM shredding_keys WHERE id = $1")
                .bind(key.as_uuid())
                .fetch_optional(&self.pool)
                .await
                .map_err(|error| map_sqlx_error("read shredding key id", error))?;
        let Some(key_id) = key_id else {
            return Err(RepositoryError::Conflict(format!(
                "shredding key '{key}' does not exist"
            )));
        };
        sqlx::query("SELECT destroy_shredding_key($1)")
            .bind(key_id)
            .execute(&self.pool)
            .await
            .map_err(|error| map_sqlx_error("destroy shredding key", error))?;
        sqlx::query(
            "UPDATE shredding_keys SET status = 'destroyed', destroyed_at = $2 WHERE id = $1",
        )
        .bind(key.as_uuid())
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("mark shredding key destroyed", error))?;
        Ok(())
    }
}
