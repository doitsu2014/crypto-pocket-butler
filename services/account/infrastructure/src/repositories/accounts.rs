//! Postgres implementation of [`AccountRepository`].

use account_application::ports::{AccountRepository, RepositoryError};
use account_domain::{Account, AccountId, AccountKind, AccountStatus, UserId, Venue};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{invalid_data, map_sqlx_error};

const COLUMNS: &str = "id, user_id, kind, venue, label, status, last_synced_at, created_at, \
     updated_at, anonymised_at";

/// sqlx-backed accounts.
#[derive(Debug, Clone)]
pub struct PgAccountRepository {
    pool: PgPool,
}

impl PgAccountRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct AccountRow {
    id: Uuid,
    user_id: Uuid,
    kind: String,
    venue: String,
    label: String,
    status: String,
    last_synced_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    anonymised_at: Option<DateTime<Utc>>,
}

impl AccountRow {
    fn into_domain(self) -> Result<Account, RepositoryError> {
        let kind =
            AccountKind::parse(&self.kind).map_err(|error| invalid_data("read account", error))?;
        let venue = Venue::parse_for(kind, &self.venue)
            .map_err(|error| invalid_data("read account", error))?;
        let status = AccountStatus::parse(&self.status)
            .map_err(|error| invalid_data("read account", error))?;
        Ok(Account::from_stored(
            AccountId::new(self.id),
            UserId::new(self.user_id),
            kind,
            venue,
            self.label,
            status,
            self.last_synced_at,
            self.created_at,
            self.updated_at,
            self.anonymised_at,
        ))
    }
}

#[async_trait]
impl AccountRepository for PgAccountRepository {
    async fn insert(&self, account: &Account) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO accounts
                 (id, user_id, kind, venue, label, status, last_synced_at, created_at, updated_at,
                  anonymised_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(account.id().as_uuid())
        .bind(account.user_id().as_uuid())
        .bind(account.kind().as_str())
        .bind(account.venue().as_str())
        .bind(account.label())
        .bind(account.status().as_str())
        .bind(account.last_synced_at())
        .bind(account.created_at())
        .bind(account.updated_at())
        .bind(account.anonymised_at())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("insert account", error))?;
        Ok(())
    }

    async fn find_by_id(&self, id: AccountId) -> Result<Option<Account>, RepositoryError> {
        let row = sqlx::query_as::<_, AccountRow>(&format!(
            "SELECT {COLUMNS} FROM accounts WHERE id = $1"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find account", error))?;
        row.map(AccountRow::into_domain).transpose()
    }

    async fn find_for_user(&self, user_id: UserId) -> Result<Vec<Account>, RepositoryError> {
        let rows = sqlx::query_as::<_, AccountRow>(&format!(
            "SELECT {COLUMNS} FROM accounts WHERE user_id = $1 ORDER BY created_at, id"
        ))
        .bind(user_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("list accounts", error))?;
        rows.into_iter().map(AccountRow::into_domain).collect()
    }

    async fn label_exists(
        &self,
        user_id: UserId,
        label: &str,
        except: Option<AccountId>,
    ) -> Result<bool, RepositoryError> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                 SELECT 1 FROM accounts
                 WHERE user_id = $1
                   AND anonymised_at IS NULL
                   AND lower(label) = lower($2)
                   AND ($3::uuid IS NULL OR id <> $3)
             )",
        )
        .bind(user_id.as_uuid())
        .bind(label.trim())
        .bind(except.map(|id| id.as_uuid()))
        .fetch_one(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("check account label", error))?;
        Ok(exists)
    }

    async fn update(&self, account: &Account) -> Result<(), RepositoryError> {
        let affected = sqlx::query(
            "UPDATE accounts
             SET label = $2, status = $3, last_synced_at = $4, updated_at = $5, anonymised_at = $6
             WHERE id = $1",
        )
        .bind(account.id().as_uuid())
        .bind(account.label())
        .bind(account.status().as_str())
        .bind(account.last_synced_at())
        .bind(account.updated_at())
        .bind(account.anonymised_at())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("update account", error))?
        .rows_affected();
        if affected == 0 {
            return Err(RepositoryError::Conflict(format!(
                "account '{}' does not exist",
                account.id()
            )));
        }
        Ok(())
    }

    async fn list_syncable(&self) -> Result<Vec<Account>, RepositoryError> {
        let rows = sqlx::query_as::<_, AccountRow>(&format!(
            "SELECT {COLUMNS} FROM accounts
             WHERE kind <> 'csv' AND status <> 'disconnected' AND anonymised_at IS NULL
             ORDER BY created_at, id"
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("list syncable accounts", error))?;
        rows.into_iter().map(AccountRow::into_domain).collect()
    }

    async fn anonymise_for_user(
        &self,
        user_id: UserId,
        at: DateTime<Utc>,
    ) -> Result<u64, RepositoryError> {
        Ok(sqlx::query(
            "UPDATE accounts
             SET label = 'anonymised-' || id::text, anonymised_at = $2, updated_at = $2
             WHERE user_id = $1 AND anonymised_at IS NULL",
        )
        .bind(user_id.as_uuid())
        .bind(at)
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("anonymise accounts", error))?
        .rows_affected())
    }
}
