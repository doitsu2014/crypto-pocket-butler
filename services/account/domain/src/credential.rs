//! Exchange credentials: Vault references only, verified read-only before use
//! (BR2.1–BR2.5).

use chrono::{DateTime, Utc};

use crate::error::DomainError;
use crate::ids::{AccountId, CredentialId};

/// The verification state of one credential.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialStatus {
    /// The read-only check has not completed.
    Pending,
    /// The read-only check passed.
    Verified,
    /// The key carries a trading or withdrawal permission, or is malformed.
    Rejected,
    /// A transient provider failure left the last verdict unconfirmed.
    Stale,
}

impl CredentialStatus {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Verified => "verified",
            Self::Rejected => "rejected",
            Self::Stale => "stale",
        }
    }

    /// Parses a stored label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown status.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "pending" => Ok(Self::Pending),
            "verified" => Ok(Self::Verified),
            "rejected" => Ok(Self::Rejected),
            "stale" => Ok(Self::Stale),
            _ => Err(DomainError::Invalid {
                field: "verificationStatus",
                reason: "must be one of pending, verified, rejected, stale".to_string(),
            }),
        }
    }
}

/// One read-only exchange credential, stored as a Vault secret reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credential {
    id: CredentialId,
    account_id: AccountId,
    vault_secret_ref: String,
    read_only: bool,
    verification_status: CredentialStatus,
    permissions_verified_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl Credential {
    /// Creates a pending credential over a Vault reference.
    ///
    /// # Errors
    /// Returns [`DomainError::Required`] for a blank reference.
    pub fn pending(
        id: CredentialId,
        account_id: AccountId,
        vault_secret_ref: &str,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let reference = vault_secret_ref.trim();
        if reference.is_empty() {
            return Err(DomainError::Required {
                field: "vaultSecretRef",
            });
        }
        Ok(Self {
            id,
            account_id,
            vault_secret_ref: reference.to_string(),
            read_only: false,
            verification_status: CredentialStatus::Pending,
            permissions_verified_at: None,
            created_at: now,
        })
    }

    /// Rehydrates a stored credential.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: CredentialId,
        account_id: AccountId,
        vault_secret_ref: String,
        read_only: bool,
        verification_status: CredentialStatus,
        permissions_verified_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            account_id,
            vault_secret_ref,
            read_only,
            verification_status,
            permissions_verified_at,
            created_at,
        }
    }

    /// The credential identifier.
    #[must_use]
    pub fn id(&self) -> CredentialId {
        self.id
    }

    /// The owning account.
    #[must_use]
    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    /// The Vault reference. Key material never enters the service.
    #[must_use]
    pub fn vault_secret_ref(&self) -> &str {
        &self.vault_secret_ref
    }

    /// Whether the permissions check found no trading or withdrawal right.
    #[must_use]
    pub fn read_only(&self) -> bool {
        self.read_only
    }

    /// The verification state.
    #[must_use]
    pub fn verification_status(&self) -> CredentialStatus {
        self.verification_status
    }

    /// When the permissions were last confirmed.
    #[must_use]
    pub fn permissions_verified_at(&self) -> Option<DateTime<Utc>> {
        self.permissions_verified_at
    }

    /// Creation time.
    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Whether a sync may use this credential.
    #[must_use]
    pub fn is_usable(&self) -> bool {
        self.read_only
            && matches!(
                self.verification_status,
                CredentialStatus::Verified | CredentialStatus::Stale
            )
    }

    /// Records a successful read-only verification.
    pub fn verify(&mut self, at: DateTime<Utc>) {
        self.read_only = true;
        self.verification_status = CredentialStatus::Verified;
        self.permissions_verified_at = Some(at);
    }

    /// Records a rejection. A rejected credential is never used again.
    pub fn reject(&mut self, at: DateTime<Utc>) {
        self.read_only = false;
        self.verification_status = CredentialStatus::Rejected;
        self.permissions_verified_at = Some(at);
    }

    /// Records a transient failure without changing the last verdict (WF7).
    pub fn mark_stale(&mut self, at: DateTime<Utc>) {
        self.verification_status = CredentialStatus::Stale;
        self.permissions_verified_at = Some(at);
    }

    /// Swaps in a new Vault reference after the new key verified (BR2.2).
    ///
    /// # Errors
    /// Returns [`DomainError::Required`] for a blank reference.
    pub fn replace_reference(
        &mut self,
        vault_secret_ref: &str,
        at: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        let reference = vault_secret_ref.trim();
        if reference.is_empty() {
            return Err(DomainError::Required {
                field: "vaultSecretRef",
            });
        }
        self.vault_secret_ref = reference.to_string();
        self.verify(at);
        Ok(())
    }
}
