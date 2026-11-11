//! Postgres implementation of [`UserRepository`].
//!
//! The Supabase subject is protected inside the database by
//! `identity.protect`/`identity.unprotect`; this adapter never handles key
//! material and never stores the subject in clear text.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use identity_application::ports::{RepositoryError, UserRepository};
use identity_domain::{
    BaseCurrency, Locale, Role, ShreddingKey, ShreddingKeyId, ShreddingKeyStatus, TimeZone, User,
    UserId, UserStatus,
};
use sqlx::PgPool;
use uuid::Uuid;

use super::{invalid_data, map_sqlx_error, sha256_hex};

const USER_COLUMNS: &str =
    "id, role, status, display_name, display_name_overridden, email, email_overridden, \
     email_verified, locale, base_currency, timezone, created_at, updated_at, anonymised_at";

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

    /// Inserts the `users` row inside an existing transaction.
    async fn insert_user_row(
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        user: &User,
    ) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO users
                 (id, role, status, display_name, display_name_overridden, email,
                  email_overridden, email_verified, locale, base_currency, timezone,
                  created_at, updated_at, anonymised_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
        )
        .bind(user.id().as_uuid())
        .bind(user.role().as_str())
        .bind(user.status().as_str())
        .bind(user.display_name())
        .bind(user.display_name_overridden())
        .bind(user.email())
        .bind(user.email_overridden())
        .bind(user.email_verified())
        .bind(user.locale().as_str())
        .bind(user.base_currency().as_str())
        .bind(user.timezone().as_str())
        .bind(user.created_at())
        .bind(user.updated_at())
        .bind(user.anonymised_at())
        .execute(&mut **transaction)
        .await
        .map_err(|error| map_sqlx_error("insert user", error))?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    role: String,
    status: String,
    display_name: Option<String>,
    display_name_overridden: bool,
    email: Option<String>,
    email_overridden: bool,
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
            self.display_name,
            self.display_name_overridden,
            self.email,
            self.email_overridden,
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
        .bind(&hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find user by subject", error))?;
        let Some(user) = row.map(UserRow::into_domain).transpose()? else {
            // Erasure removed the personal row but kept the pseudonymous hash,
            // so a still-valid token is matched and denied rather than treated
            // as an unknown subject (BR5.8, AC4.5.1). Returning the erased
            // record is what stops `resolve_with_origin` provisioning a new
            // account for it.
            return self.find_erased_by_hash(&hash).await;
        };
        // A hash hit whose decryptable ciphertext names another subject is a
        // collision. Fail closed: never return a record the presented subject
        // does not own (BR1.8, security design).
        match self.reveal_auth_subject(user.id()).await? {
            Some(stored) if stored != auth_subject => Err(RepositoryError::Validation(
                "the presented subject does not match the stored subject".to_string(),
            )),
            _ => Ok(Some(user)),
        }
    }

    async fn find_by_subject_hash(
        &self,
        subject_hash: &str,
    ) -> Result<Option<User>, RepositoryError> {
        let hash = subject_hash.trim();
        let row = sqlx::query_as::<_, UserRow>(&format!(
            "SELECT {USER_COLUMNS} FROM users
             JOIN user_personal ON user_personal.user_id = users.id
             WHERE user_personal.auth_subject_hash = $1"
        ))
        .bind(hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find user by subject hash", error))?;
        let Some(user) = row.map(UserRow::into_domain).transpose()? else {
            return self.find_erased_by_hash(hash).await;
        };
        Ok(Some(user))
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, RepositoryError> {
        let row = sqlx::query_as::<_, UserRow>(&format!(
            "SELECT {USER_COLUMNS} FROM users
             WHERE email = $1
             ORDER BY created_at, id
             LIMIT 1"
        ))
        .bind(email.trim())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find user by email", error))?;
        row.map(UserRow::into_domain).transpose()
    }

    async fn provision(
        &self,
        user: &User,
        auth_subject: &str,
        now: DateTime<Utc>,
    ) -> Result<ShreddingKey, RepositoryError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| map_sqlx_error("begin user provision", error))?;

        // The key reference is created before the data it protects, inside the
        // same transaction: a failed write leaves neither a key nor a user row.
        let key_row_id = ShreddingKeyId::generate();
        let vault_reference = format!("identity/user/{}", user.id());
        let key_id: i64 = sqlx::query_scalar("SELECT create_shredding_key($1)")
            .bind(&vault_reference)
            .fetch_one(&mut *transaction)
            .await
            .map_err(|error| map_sqlx_error("create shredding key", error))?;
        sqlx::query(
            "INSERT INTO shredding_keys
                 (id, user_id, vault_reference, key_id, status, created_at, destroyed_at)
             VALUES ($1, $2, $3, $4, 'active', $5, NULL)",
        )
        .bind(key_row_id.as_uuid())
        .bind(user.id().as_uuid())
        .bind(&vault_reference)
        .bind(key_id)
        .bind(now)
        .execute(&mut *transaction)
        .await
        .map_err(|error| map_sqlx_error("store shredding key", error))?;

        let protected: Vec<u8> = sqlx::query_scalar("SELECT protect($1, $2)")
            .bind(auth_subject)
            .bind(key_id)
            .fetch_one(&mut *transaction)
            .await
            .map_err(|error| map_sqlx_error("protect user subject", error))?;

        Self::insert_user_row(&mut transaction, user).await?;

        sqlx::query(
            "INSERT INTO user_personal
                 (user_id, auth_subject_hash, auth_subject_protected, shredding_key_id)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(user.id().as_uuid())
        .bind(sha256_hex(auth_subject))
        .bind(protected)
        .bind(key_row_id.as_uuid())
        .execute(&mut *transaction)
        .await
        .map_err(|error| map_sqlx_error("insert user subject", error))?;

        transaction
            .commit()
            .await
            .map_err(|error| map_sqlx_error("commit user provision", error))?;

        Ok(ShreddingKey::from_stored(
            key_row_id,
            user.id(),
            vault_reference,
            key_id,
            ShreddingKeyStatus::Active,
            now,
            None,
        ))
    }

    async fn insert(
        &self,
        user: &User,
        auth_subject: &str,
        key: &ShreddingKey,
    ) -> Result<(), RepositoryError> {
        let protected: Vec<u8> = sqlx::query_scalar("SELECT protect($1, $2)")
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
        Self::insert_user_row(&mut transaction, user).await?;

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
             SET role = $2, status = $3, display_name = $4, display_name_overridden = $5,
                 email = $6, email_overridden = $7, email_verified = $8, locale = $9,
                 base_currency = $10, timezone = $11, updated_at = $12, anonymised_at = $13
             WHERE id = $1",
        )
        .bind(user.id().as_uuid())
        .bind(user.role().as_str())
        .bind(user.status().as_str())
        .bind(user.display_name())
        .bind(user.display_name_overridden())
        .bind(user.email())
        .bind(user.email_overridden())
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
            "SELECT unprotect(personal.auth_subject_protected, keys.key_id)
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
        // The tombstone move. The hash is copied onto `users` BEFORE
        // `user_personal` leaves, in one transaction, so the subject is never in
        // neither place (security design). The status guard makes a concurrent
        // restore win: a row that is no longer pending deletion (or already
        // deleted by the transition this run committed) is never overwritten,
        // and an account restored mid-erasure fails here instead of being
        // silently erased.
        let affected = sqlx::query(
            "UPDATE users
             SET erased_subject_hash = COALESCE(
                     erased_subject_hash,
                     (SELECT auth_subject_hash FROM user_personal WHERE user_id = $1)
                 ),
                 role = $2, status = $3, display_name = $4, display_name_overridden = $5,
                 email = $6, email_overridden = $7, email_verified = $8, locale = $9,
                 base_currency = $10, timezone = $11, updated_at = $12, anonymised_at = $13
             WHERE id = $1 AND status IN ('pending_deletion', 'deleted')",
        )
        .bind(user.id().as_uuid())
        .bind(user.role().as_str())
        .bind(user.status().as_str())
        .bind(user.display_name())
        .bind(user.display_name_overridden())
        .bind(user.email())
        .bind(user.email_overridden())
        .bind(user.email_verified())
        .bind(user.locale().as_str())
        .bind(user.base_currency().as_str())
        .bind(user.timezone().as_str())
        .bind(user.updated_at())
        .bind(user.anonymised_at())
        .execute(&mut *transaction)
        .await
        .map_err(|error| map_sqlx_error("anonymise user", error))?
        .rows_affected();
        if affected == 0 {
            return Err(RepositoryError::Conflict(
                "the account is no longer pending deletion; the erasure was refused".to_string(),
            ));
        }
        sqlx::query("DELETE FROM user_personal WHERE user_id = $1")
            .bind(user.id().as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(|error| map_sqlx_error("remove user subject", error))?;
        transaction
            .commit()
            .await
            .map_err(|error| map_sqlx_error("commit user anonymise", error))
    }
}

impl PgUserRepository {
    /// Finds an erased account by its retained subject hash (BR5.8).
    async fn find_erased_by_hash(&self, hash: &str) -> Result<Option<User>, RepositoryError> {
        let row = sqlx::query_as::<_, UserRow>(&format!(
            "SELECT {USER_COLUMNS} FROM users
             WHERE erased_subject_hash = $1 AND status = 'deleted'"
        ))
        .bind(hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find erased subject", error))?;
        row.map(UserRow::into_domain).transpose()
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
