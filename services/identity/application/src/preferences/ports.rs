//! U4 preference ports, defined inward and implemented by
//! `identity-infrastructure`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use identity_domain::{PreferenceUpdate, ProviderProfile, StoredPreferences, UserId};

use crate::ports::RepositoryError;

/// Reads and partially writes one user's explicit preferences.
#[async_trait]
pub trait PreferencesRepository: Send + Sync {
    /// Loads the stored row, or `None` before the first write (BR4.1).
    async fn load(&self, user_id: UserId) -> Result<Option<StoredPreferences>, RepositoryError>;

    /// Applies only the supplied fields and returns the stored row.
    async fn upsert(
        &self,
        user_id: UserId,
        update: &PreferenceUpdate,
        now: DateTime<Utc>,
    ) -> Result<StoredPreferences, RepositoryError>;
}

/// Reads u1's provider-derived profile values for the read fallback (BR4.1).
#[async_trait]
pub trait UserLookup: Send + Sync {
    /// The three stored profile values, or `None` when the user does not exist.
    async fn provider_profile(
        &self,
        user_id: UserId,
    ) -> Result<Option<ProviderProfile>, RepositoryError>;
}
