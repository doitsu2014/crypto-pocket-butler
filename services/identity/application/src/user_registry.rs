//! U1 — the admin user lookup and the record read within the metadata boundary
//! (BR1.9, US7.1).
//!
//! The registry never returns portfolio contents, credentials, tokens, or the
//! provider subject. An admin may look a user up by exact email; the owner or an
//! admin may read one record by id. Every refusal is audited.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use identity_domain::{is_email_shaped, AuditOutcome, User, UserId, UserStatus, MAX_EMAIL_LEN};
use sha2::{Digest, Sha256};

use crate::common::{AppError, Caller, FieldError};
use crate::ports::UserRepository;
use crate::trail::{context, Trail};

/// The SHA-256 hex digest used for the subject lookup index. The subject itself
/// is stored only as deterministic ciphertext.
#[must_use]
pub fn subject_hash(subject: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(subject.as_bytes());
    hex_encode(hasher.finalize().as_slice())
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

/// One user record as the metadata boundary exposes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserRecordView {
    user_id: UserId,
    display_name: Option<String>,
    email: Option<String>,
    email_verified: bool,
    status: UserStatus,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl UserRecordView {
    /// Maps a stored user at the boundary.
    #[must_use]
    pub fn from_user(user: &User) -> Self {
        Self {
            user_id: user.id(),
            display_name: user.display_name().map(ToString::to_string),
            email: user.email().map(ToString::to_string),
            email_verified: user.email_verified(),
            status: user.status(),
            created_at: user.created_at(),
            updated_at: user.updated_at(),
        }
    }

    /// The user identifier.
    #[must_use]
    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    /// The display name, when one is stored.
    #[must_use]
    pub fn display_name(&self) -> Option<&str> {
        self.display_name.as_deref()
    }

    /// The stored address, when one is stored.
    #[must_use]
    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }

    /// Whether the stored address is confirmed.
    #[must_use]
    pub fn email_verified(&self) -> bool {
        self.email_verified
    }

    /// The lifecycle state.
    #[must_use]
    pub fn status(&self) -> UserStatus {
        self.status
    }

    /// When the record was created.
    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// When the record last changed.
    #[must_use]
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

/// The admin and owner lookup use cases.
pub struct AdminLookup {
    users: Arc<dyn UserRepository>,
    trail: Arc<Trail>,
}

impl AdminLookup {
    /// Creates the use case.
    #[must_use]
    pub fn new(users: Arc<dyn UserRepository>, trail: Arc<Trail>) -> Self {
        Self { users, trail }
    }

    /// Looks one user up by exact email address. Only an admin may search.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for a caller without the admin role,
    /// [`AppError::Validation`] for a malformed address, [`AppError::NotFound`]
    /// for no exact match, and [`AppError::Unavailable`] when the store cannot
    /// answer.
    pub async fn by_email(&self, caller: &Caller, email: &str) -> Result<UserRecordView, AppError> {
        if !caller.is_admin() {
            self.audit(
                caller,
                "identity.admin.user_lookup",
                "users",
                AuditOutcome::Denied,
                Some("not_admin"),
            )
            .await?;
            return Err(AppError::Forbidden(
                "only an administrator may look a user up".to_string(),
            ));
        }
        let normalized = validate_email(email)?;
        let user = self
            .users
            .find_by_email(&normalized)
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("user"))?;
        self.audit(
            caller,
            "identity.admin.user_lookup",
            user.id().to_string(),
            AuditOutcome::Success,
            None,
        )
        .await?;
        Ok(UserRecordView::from_user(&user))
    }

    /// Reads one user record by id. The owner or an admin may read it.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for another user without the admin role,
    /// [`AppError::NotFound`] for an unknown id, and [`AppError::Unavailable`]
    /// when the store cannot answer.
    pub async fn by_id(
        &self,
        caller: &Caller,
        user_id: UserId,
    ) -> Result<UserRecordView, AppError> {
        let is_owner = caller.user_id() == user_id;
        if !is_owner && !caller.is_admin() {
            self.audit(
                caller,
                "identity.user.read",
                user_id.to_string(),
                AuditOutcome::Denied,
                Some("not_owner"),
            )
            .await?;
            return Err(AppError::Forbidden(
                "the resource belongs to another user".to_string(),
            ));
        }
        let user = self
            .users
            .find_by_id(user_id)
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("user"))?;
        self.audit(
            caller,
            if is_owner {
                "identity.user.read"
            } else {
                "identity.admin.user_lookup"
            },
            user.id().to_string(),
            AuditOutcome::Success,
            None,
        )
        .await?;
        Ok(UserRecordView::from_user(&user))
    }

    /// Looks one user up by internal id for an internal service caller.
    ///
    /// # Errors
    /// Returns [`AppError::NotFound`] for an unknown id and
    /// [`AppError::Unavailable`] when the store cannot answer.
    pub async fn internal_by_id(&self, user_id: UserId) -> Result<UserRecordView, AppError> {
        let user = self
            .users
            .find_by_id(user_id)
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("user"))?;
        Ok(UserRecordView::from_user(&user))
    }

    /// Looks one user up by subject hash for an internal service caller.
    ///
    /// # Errors
    /// Returns [`AppError::NotFound`] for an unknown hash,
    /// [`AppError::Gone`] for an erased subject — the tombstone C3 consumers
    /// must not mistake for "not provisioned" (BR5.8, AC4.5.1) — and
    /// [`AppError::Unavailable`] when the store cannot answer.
    pub async fn internal_by_subject_hash(
        &self,
        subject_hash: &str,
    ) -> Result<UserRecordView, AppError> {
        let user = self
            .users
            .find_by_subject_hash(subject_hash)
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("user"))?;
        if user.is_deleted() {
            return Err(AppError::Gone(
                "the subject's account has been erased".to_string(),
            ));
        }
        Ok(UserRecordView::from_user(&user))
    }

    /// Looks one user up by exact email for an internal service caller.
    ///
    /// # Errors
    /// Returns [`AppError::Validation`] for a malformed address,
    /// [`AppError::NotFound`] for no exact match, and
    /// [`AppError::Unavailable`] when the store cannot answer.
    pub async fn internal_by_email(&self, email: &str) -> Result<UserRecordView, AppError> {
        let normalized = validate_email(email)?;
        let user = self
            .users
            .find_by_email(&normalized)
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("user"))?;
        Ok(UserRecordView::from_user(&user))
    }

    async fn audit(
        &self,
        caller: &Caller,
        action: &str,
        target: impl Into<String>,
        outcome: AuditOutcome,
        reason: Option<&str>,
    ) -> Result<(), AppError> {
        let mut fields = context(&[("role", caller.role().as_str())]);
        if let Some(reason) = reason {
            fields.insert("reason".to_string(), reason.to_string());
        }
        self.trail
            .record(
                Some(caller.user_id()),
                action,
                target,
                outcome,
                &caller.correlation_id,
                fields,
            )
            .await
    }
}

/// Validates an email supplied to the exact-match admin lookup.
fn validate_email(raw: &str) -> Result<String, AppError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(AppError::Validation(vec![FieldError::new(
            "email",
            "is required",
        )]));
    }
    if value.chars().count() > MAX_EMAIL_LEN || !is_email_shaped(value) {
        return Err(AppError::Validation(vec![FieldError::new(
            "email",
            "must be an RFC 5322 address",
        )]));
    }
    Ok(value.to_string())
}
