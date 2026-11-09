//! Feature flags with governance metadata and deterministic rollout
//! (BR4.1–BR4.3).

use chrono::{DateTime, Utc};

use crate::constants::{MAX_FLAG_KEY_LEN, MAX_OWNER_LEN};
use crate::error::DomainError;
use crate::ids::FeatureFlagId;

/// The persisted status of a flag. `expired` is derived from `expiresAt`;
/// writes accept only `active` and `disabled`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagStatus {
    /// Eligible for evaluation.
    Active,
    /// Switched off by an administrator.
    Disabled,
    /// Past its expiry; evaluated off and listed for cleanup.
    Expired,
}

impl FlagStatus {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Disabled => "disabled",
            Self::Expired => "expired",
        }
    }

    /// Parses a stored label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for any other value.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "active" => Ok(Self::Active),
            "disabled" => Ok(Self::Disabled),
            "expired" => Ok(Self::Expired),
            _ => Err(DomainError::Invalid {
                field: "status",
                reason: "must be active, disabled, or expired".to_string(),
            }),
        }
    }

    /// Parses a status an administrator may write. `expired` is derived and is
    /// therefore refused.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for any other value.
    pub fn parse_writable(raw: &str) -> Result<Self, DomainError> {
        match Self::parse(raw)? {
            Self::Expired => Err(DomainError::Invalid {
                field: "status",
                reason: "expired is derived from expiresAt and cannot be set".to_string(),
            }),
            writable => Ok(writable),
        }
    }
}

/// The fields a create request supplies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewFeatureFlag {
    /// The unique key.
    pub key: String,
    /// The accountable owner.
    pub owner: String,
    /// The mandatory expiry instant.
    pub expires_at: DateTime<Utc>,
    /// The rollout percentage, 0 to 100.
    pub rollout_percentage: u16,
    /// The requested status; defaults to disabled.
    pub status: Option<String>,
}

/// A partial update.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FlagUpdate {
    /// A replacement owner.
    pub owner: Option<String>,
    /// A replacement expiry instant.
    pub expires_at: Option<DateTime<Utc>>,
    /// A replacement rollout percentage.
    pub rollout_percentage: Option<u16>,
    /// A replacement active/disabled status.
    pub status: Option<String>,
}

/// One governed feature flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureFlag {
    id: FeatureFlagId,
    key: String,
    owner: String,
    expires_at: DateTime<Utc>,
    rollout_percentage: u8,
    status: FlagStatus,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl FeatureFlag {
    /// Validates and creates a flag (BR4.1).
    ///
    /// # Errors
    /// Returns a domain error when the key is malformed, the owner is blank, the
    /// expiry is absent or past, the rollout is above 100, or the status is
    /// expired.
    pub fn create(
        id: FeatureFlagId,
        new: NewFeatureFlag,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let key = validate_key(&new.key)?;
        let owner = validate_owner(&new.owner)?;
        if new.expires_at <= now {
            return Err(DomainError::Invalid {
                field: "expiresAt",
                reason: "must be in the future".to_string(),
            });
        }
        let rollout_percentage = validate_rollout(new.rollout_percentage)?;
        let status = match new.status.as_deref() {
            None => FlagStatus::Disabled,
            Some(raw) => FlagStatus::parse_writable(raw)?,
        };
        Ok(Self {
            id,
            key,
            owner,
            expires_at: new.expires_at,
            rollout_percentage,
            status,
            created_at: now,
            updated_at: now,
        })
    }

    /// Rebuilds a stored row.
    ///
    /// # Errors
    /// Returns a domain error when a stored value is outside its allowed set.
    pub fn from_stored(
        id: FeatureFlagId,
        key: &str,
        owner: &str,
        expires_at: DateTime<Utc>,
        rollout_percentage: u16,
        status: &str,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            id,
            key: validate_key(key)?,
            owner: validate_owner(owner)?,
            expires_at,
            rollout_percentage: validate_rollout(rollout_percentage)?,
            status: FlagStatus::parse(status)?,
            created_at,
            updated_at,
        })
    }

    /// Applies an administrator's partial update (BR4.1).
    ///
    /// # Errors
    /// Returns a domain error when a supplied field is invalid.
    pub fn apply_update(
        &mut self,
        update: FlagUpdate,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        if let Some(owner) = update.owner {
            self.owner = validate_owner(&owner)?;
        }
        if let Some(expires_at) = update.expires_at {
            if expires_at <= now {
                return Err(DomainError::Invalid {
                    field: "expiresAt",
                    reason: "must be in the future".to_string(),
                });
            }
            self.expires_at = expires_at;
        }
        if let Some(rollout) = update.rollout_percentage {
            self.rollout_percentage = validate_rollout(rollout)?;
        }
        if let Some(status) = update.status {
            self.status = FlagStatus::parse_writable(&status)?;
        }
        self.updated_at = now;
        Ok(())
    }

    /// The derived status at `now` (BR4.3).
    #[must_use]
    pub fn effective_status(&self, now: DateTime<Utc>) -> FlagStatus {
        if self.status == FlagStatus::Expired || self.expires_at <= now {
            FlagStatus::Expired
        } else {
            self.status
        }
    }

    /// Whether the flag is on for a deterministic bucket (BR4.2).
    #[must_use]
    pub fn enabled_for_bucket(&self, bucket: u8, now: DateTime<Utc>) -> bool {
        self.effective_status(now) == FlagStatus::Active
            && u16::from(bucket) < u16::from(self.rollout_percentage)
    }

    /// The identifier.
    #[must_use]
    pub fn id(&self) -> FeatureFlagId {
        self.id
    }

    /// The unique key.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    /// The accountable owner.
    #[must_use]
    pub fn owner(&self) -> &str {
        &self.owner
    }

    /// The expiry instant.
    #[must_use]
    pub fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }

    /// The rollout percentage.
    #[must_use]
    pub fn rollout_percentage(&self) -> u8 {
        self.rollout_percentage
    }

    /// The stored status.
    #[must_use]
    pub fn status(&self) -> FlagStatus {
        self.status
    }

    /// When the flag was created.
    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// When the flag was last changed.
    #[must_use]
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

fn validate_key(raw: &str) -> Result<String, DomainError> {
    let key = raw.trim();
    if key.is_empty() {
        return Err(DomainError::Required { field: "key" });
    }
    if key.chars().count() > MAX_FLAG_KEY_LEN {
        return Err(DomainError::TooLarge {
            field: "key",
            limit: "the flag-key limit",
        });
    }
    if !key
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(DomainError::Invalid {
            field: "key",
            reason: "may contain only letters, digits, '-', '_', and '.'".to_string(),
        });
    }
    Ok(key.to_string())
}

fn validate_owner(raw: &str) -> Result<String, DomainError> {
    let owner = raw.trim();
    if owner.is_empty() {
        return Err(DomainError::Required { field: "owner" });
    }
    if owner.chars().count() > MAX_OWNER_LEN {
        return Err(DomainError::TooLarge {
            field: "owner",
            limit: "the owner-label limit",
        });
    }
    Ok(owner.to_string())
}

fn validate_rollout(raw: u16) -> Result<u8, DomainError> {
    if raw > 100 {
        return Err(DomainError::Invalid {
            field: "rolloutPercentage",
            reason: "must be between 0 and 100".to_string(),
        });
    }
    Ok(raw as u8)
}
