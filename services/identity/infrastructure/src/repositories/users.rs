//! Postgres implementation of [`UserRepository`].
//!
//! The Supabase subject is protected inside the database by
//! `identity.protect`/`identity.unprotect`; this adapter never handles key
//! material and never stores the subject in clear text.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use identity_application::ports::{RepositoryError, UserRepository};
use identity_domain::{
    BaseCurrency, Locale, Role, ShreddingKey, TimeZone, User, UserId, UserStatus,
};
use sqlx::PgPool;
use uuid::Uuid;

use super::{invalid_data, map_sqlx_error, sha256_hex};

const USER_COLUMNS: &str =
    "id, role, status, email, email_verified, locale, base_currency, timezone, \
     created_at, updated_at, anonymised_at";

/// sqlx-backed user records.
#[derive(Debug, Clone)]
pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    role: String,
    status: String,
    email: Option<String>,
    email_verified: bool,
    locale: String,
    base_currency: String,
    timezone: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    anonymised_at: Option<DateTime<Utc>>,
}

impl UserRow {
    fn into_domain(self) -> Result<User, RepositoryError> {
        Ok(User::from_stored(
            UserId::new(self.id),
            Role::parse(&self.role).map_err(|error| invalid_data("read user", error))?,
            UserStatus::parse(&self.status).map_err(|error| invalid_data("read user", error))?,
            self.email,
            self.email_verified,
            Locale::parse(&self.locale).map_err(|error| invalid_data("read user", error))?,
            BaseCurrency::parse(&self.base_currency)
                .map_err(|error| invalid_data("read user", error))?,
            TimeZone::parse(&self.timezone).map_err(|error| invalid_data("read user", error))?,
            self.created_at,
            self.updated_at,
            self.anonymised_at,
        ))
    }
}

#[async_trait]
impl UserRepository for PgUserRepository {
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, RepositoryError> {
        let row = sqlx::query_as::<_, UserRow>(&format!(
            "SELECT {USER_COLUMNS} FROM users WHERE id = $1"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find user", error))?;
        row.map(UserRow::into_domain).transpose()
    }

    async fn find_by_auth_subject(
        &self,
        auth_subject: &str,
    ) -> Result<Option<User>, RepositoryError> {
        let hash = sha256_hex(auth_subject);
        let row = sqlx::query_as::<_, UserRow>(&format!(
            "SELECT {USER_COLUMNS} FROM users
             JOIN user_personal ON user_personal.user_id = users.id
             WHERE user_personal.auth_subject_hash = $1"
        ))
        .bind(hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find user by subject", error))?;
        row.map(UserRow::into_domain).transpose()
    }

    async fn insert(
        &self,
        user: &User,
        auth_subject: &str,
        key: &ShreddingKey,
    ) -> Result<(), RepositoryError> {
        let protected: Vec<u8> = sqlx::query_scalar("SELECT identity.protect($1, $2)")
            .bind(auth_subject)
            .bind(key.key_id())
            .fetch_one(&self.pool)
            .await
            .map_err(|error| map_sqlx_error("protect user subject", error))?;

        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| map_sqlx_error("begin user insert", error))?;
        sqlx::query(
            "INSERT INTO users
                 (id, role, status, email, email_verified, locale, base_currency, timezone,
                  created_at, updated_at, anonymised_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(user.id().as_uuid())
        .bind(user.role().as_str())
        .bind(user.status().as_str())
        .bind(user.email())
        .bind(user.email_verified())
        .bind(user.locale().as_str())
        .bind(user.base_currency().as_str())
        .bind(user.timezone().as_str())
        .bind(user.created_at())
        .bind(user.updated_at())
        .bind(user.anonymised_at())
        .execute(&mut *transaction)
        .await
        .map_err(|error| map_sqlx_error("insert user", error))?;

        sqlx::query(
            "INSERT INTO user_personal
                 (user_id, auth_subject_hash, auth_subject_protected, shredding_key_id)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(user.id().as_uuid())
        .bind(sha256_hex(auth_subject))
        .bind(protected)
        .bind(key.id().as_uuid())
        .execute(&mut *transaction)
        .await
        .map_err(|error| map_sqlx_error("insert user subject", error))?;

        transaction
            .commit()
            .await
            .map_err(|error| map_sqlx_error("commit user insert", error))
    }

    async fn update(&self, user: &User) -> Result<(), RepositoryError> {
        let affected = sqlx::query(
            "UPDATE users
             SET role = $2, status = $3, email = $4, email_verified = $5, locale = $6,
                 base_currency = $7, timezone = $8, updated_at = $9, anonymised_at = $10
             WHERE id = $1",
        )
        .bind(user.id().as_uuid())
        .bind(user.role().as_str())
        .bind(user.status().as_str())
        .bind(user.email())
        .bind(user.email_verified())
        .bind(user.locale().as_str())
        .bind(user.base_currency().as_str())
        .bind(user.timezone().as_str())
        .bind(user.updated_at())
        .bind(user.anonymised_at())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("update user", error))?
        .rows_affected();
        if affected == 0 {
            return Err(RepositoryError::Conflict(format!(
                "user '{}' does not exist",
                user.id()
            )));
        }
        Ok(())
    }

    async fn reveal_auth_subject(
        &self,
        user_id: UserId,
    ) -> Result<Option<String>, RepositoryError> {
        sqlx::query_scalar::<_, String>(
            "SELECT identity.unprotect(personal.auth_subject_protected, keys.key_id)
             FROM user_personal AS personal
             JOIN shredding_keys AS keys ON keys.id = personal.shredding_key_id
             WHERE personal.user_id = $1",
        )
        .bind(user_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("reveal user subject", error))
    }

    async fn anonymise(&self, user: &User) -> Result<(), RepositoryError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| map_sqlx_error("begin user anonymise", error))?;
        sqlx::query("DELETE FROM user_personal WHERE user_id = $1")
            .bind(user.id().as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(|error| map_sqlx_error("remove user subject", error))?;
        sqlx::query(
            "UPDATE users
             SET role = $2, status = $3, email = $4, email_verified = $5, locale = $6,
                 base_currency = $7, timezone = $8, updated_at = $9, anonymised_at = $10
             WHERE id = $1",
        )
        .bind(user.id().as_uuid())
        .bind(user.role().as_str())
        .bind(user.status().as_str())
        .bind(user.email())
        .bind(user.email_verified())
        .bind(user.locale().as_str())
        .bind(user.base_currency().as_str())
        .bind(user.timezone().as_str())
        .bind(user.updated_at())
        .bind(user.anonymised_at())
        .execute(&mut *transaction)
        .await
        .map_err(|error| map_sqlx_error("anonymise user", error))?;
        transaction
            .commit()
            .await
            .map_err(|error| map_sqlx_error("commit user anonymise", error))
    }
}

#[async_trait]
impl identity_application::ports::DueUserRepository for PgUserRepository {
    async fn list_active(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<
        Vec<identity_application::ports::DueCandidate>,
        identity_application::ports::RepositoryError,
    > {
        #[derive(sqlx::FromRow)]
        struct DueRow {
            id: Uuid,
            timezone: String,
        }
        let rows = sqlx::query_as::<_, DueRow>(
            "SELECT id, timezone FROM users
             WHERE status = 'active' AND anonymised_at IS NULL
             ORDER BY id
             LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("list active users", error))?;
        Ok(rows
            .into_iter()
            .map(|row| identity_application::ports::DueCandidate {
                user_id: UserId::new(row.id),
                timezone: row.timezone,
            })
            .collect())
    }
}
