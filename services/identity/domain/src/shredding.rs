//! The per-subject key reference (BR6.3). Key material never leaves Vault; this
//! record holds only the Vault reference and the pgsodium key id.

use chrono::{DateTime, Utc};

use crate::error::DomainError;
use crate::ids::{ShreddingKeyId, UserId};

/// The key lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShreddingKeyStatus {
    /// The key protects the user's Identity fields.
    Active,
    /// The key was destroyed on deletion; terminal.
    Destroyed,
}

impl ShreddingKeyStatus {
    /// Parses a stored value.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for anything else.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "active" => Ok(Self::Active),
            "destroyed" => Ok(Self::Destroyed),
            _ => Err(DomainError::Invalid {
                field: "status",
                reason: "must be active or destroyed".to_string(),
            }),
        }
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Destroyed => "destroyed",
        }
    }
}

/// The reference to one per-subject key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShreddingKey {
    id: ShreddingKeyId,
    user_id: UserId,
    vault_reference: String,
    key_id: i64,
    status: ShreddingKeyStatus,
    created_at: DateTime<Utc>,
    destroyed_at: Option<DateTime<Utc>>,
}

impl ShreddingKey {
    /// Rebuilds a key reference from stored columns.
    #[must_use]
    pub fn from_stored(
        id: ShreddingKeyId,
        user_id: UserId,
        vault_reference: String,
        key_id: i64,
        status: ShreddingKeyStatus,
        created_at: DateTime<Utc>,
        destroyed_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            user_id,
            vault_reference,
            key_id,
            status,
            created_at,
            destroyed_at,
        }
    }

    /// Marks the key destroyed.
    ///
    /// # Errors
    /// Returns [`DomainError::InvalidTransition`] when the key is already
    /// destroyed.
    pub fn mark_destroyed(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        if self.status == ShreddingKeyStatus::Destroyed {
            return Err(DomainError::InvalidTransition {
                entity: "shredding key",
                from: self.status.as_str(),
                to: "destroyed",
            });
        }
        self.status = ShreddingKeyStatus::Destroyed;
        self.destroyed_at = Some(now);
        Ok(())
    }

    /// The record identifier.
    #[must_use]
    pub fn id(&self) -> ShreddingKeyId {
        self.id
    }

    /// The owner.
    #[must_use]
    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    /// The Vault secret reference; never the key material.
    #[must_use]
    pub fn vault_reference(&self) -> &str {
        &self.vault_reference
    }

    /// The pgsodium key id the protect/unprotect functions take.
    #[must_use]
    pub fn key_id(&self) -> i64 {
        self.key_id
    }

    /// The lifecycle state.
    #[must_use]
    pub fn status(&self) -> ShreddingKeyStatus {
        self.status
    }

    /// When the key was created.
    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// When the key was destroyed.
    #[must_use]
    pub fn destroyed_at(&self) -> Option<DateTime<Utc>> {
        self.destroyed_at
    }
}
