//! Postgres implementation of [`CredentialRepository`].

use account_application::ports::{CredentialRepository, RepositoryError};
use account_domain::{AccountId, Credential, CredentialId, CredentialStatus, UserId};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{invalid_data, map_sqlx_error};

const COLUMNS: &str = "id, account_id, vault_secret_ref, read_only, verification_status, \
     permissions_verified_at, created_at";

/// sqlx-backed credentials.
#[derive(Debug, Clone)]
pub struct PgCredentialRepository {
    pool: PgPool,
}

impl PgCredentialRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct CredentialRow {
    id: Uuid,
    account_id: Uuid,
    vault_secret_ref: String,
    read_only: bool,
    verification_status: String,
    permissions_verified_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl CredentialRow {
    fn into_domain(self) -> Result<Credential, RepositoryError> {
        Ok(Credential::from_stored(
            CredentialId::new(self.id),
            AccountId::new(self.account_id),
            self.vault_secret_ref,
            self.read_only,
            CredentialStatus::parse(&self.verification_status)
                .map_err(|error| invalid_data("read credential", error))?,
            self.permissions_verified_at,
            self.created_at,
        ))
    }
}

#[async_trait]
impl CredentialRepository for PgCredentialRepository {
    async fn insert(&self, credential: &Credential) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO credentials
                 (id, account_id, vault_secret_ref, read_only, verification_status,
                  permissions_verified_at, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(credential.id().as_uuid())
        .bind(credential.account_id().as_uuid())
        .bind(credential.vault_secret_ref())
        .bind(credential.read_only())
        .bind(credential.verification_status().as_str())
        .bind(credential.permissions_verified_at())
        .bind(credential.created_at())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("insert credential", error))?;
        Ok(())
    }

    async fn find_by_id(&self, id: CredentialId) -> Result<Option<Credential>, RepositoryError> {
        let row = sqlx::query_as::<_, CredentialRow>(&format!(
            "SELECT {COLUMNS} FROM credentials WHERE id = $1"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find credential", error))?;
        row.map(CredentialRow::into_domain).transpose()
    }

    async fn find_for_account(
        &self,
        account_id: AccountId,
    ) -> Result<Option<Credential>, RepositoryError> {
        let row = sqlx::query_as::<_, CredentialRow>(&format!(
            "SELECT {COLUMNS} FROM credentials WHERE account_id = $1"
        ))
        .bind(account_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find account credential", error))?;
        row.map(CredentialRow::into_domain).transpose()
    }

    async fn update(&self, credential: &Credential) -> Result<(), RepositoryError> {
        let affected = sqlx::query(
            "UPDATE credentials
             SET vault_secret_ref = $2, read_only = $3, verification_status = $4,
                 permissions_verified_at = $5
             WHERE id = $1",
        )
        .bind(credential.id().as_uuid())
        .bind(credential.vault_secret_ref())
        .bind(credential.read_only())
        .bind(credential.verification_status().as_str())
        .bind(credential.permissions_verified_at())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("update credential", error))?
        .rows_affected();
        if affected == 0 {
            return Err(RepositoryError::Conflict(format!(
                "credential '{}' does not exist",
                credential.id()
            )));
        }
        Ok(())
    }

    async fn list_verified(&self) -> Result<Vec<Credential>, RepositoryError> {
        let rows = sqlx::query_as::<_, CredentialRow>(&format!(
            "SELECT {COLUMNS} FROM credentials
             WHERE verification_status IN ('verified', 'stale') AND read_only"
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("list verified credentials", error))?;
        rows.into_iter().map(CredentialRow::into_domain).collect()
    }

    async fn list_for_user(&self, user_id: UserId) -> Result<Vec<Credential>, RepositoryError> {
        let rows = sqlx::query_as::<_, CredentialRow>(
            "SELECT c.id, c.account_id, c.vault_secret_ref, c.read_only, c.verification_status,
                    c.permissions_verified_at, c.created_at
             FROM credentials c
             JOIN accounts a ON a.id = c.account_id
             WHERE a.user_id = $1
             ORDER BY c.created_at, c.id",
        )
        .bind(user_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("list user credentials", error))?;
        rows.into_iter().map(CredentialRow::into_domain).collect()
    }

    async fn delete_for_user(&self, user_id: UserId) -> Result<u64, RepositoryError> {
        Ok(sqlx::query(
            "DELETE FROM credentials c
             USING accounts a
             WHERE c.account_id = a.id AND a.user_id = $1",
        )
        .bind(user_id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("delete user credentials", error))?
        .rows_affected())
    }
}
