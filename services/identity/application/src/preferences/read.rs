//! U4 — read the effective preferences with a per-field source (BR4.1).
//!
//! The read never creates a row: before the first write every field is a default
//! with `source: "default"`.

use std::sync::Arc;

use identity_domain::{EffectivePreferences, UserId};

use super::ports::{PreferencesRepository, UserLookup};
use crate::common::{AppError, Caller};

/// Reads the caller's effective preferences.
pub struct ReadUserPreferences {
    preferences: Arc<dyn PreferencesRepository>,
    users: Arc<dyn UserLookup>,
}

impl ReadUserPreferences {
    /// Creates the use case.
    #[must_use]
    pub fn new(preferences: Arc<dyn PreferencesRepository>, users: Arc<dyn UserLookup>) -> Self {
        Self { preferences, users }
    }

    /// Resolves `COALESCE(preference, provider, default)` for the caller.
    ///
    /// # Errors
    /// Returns [`AppError::Unavailable`] when the store cannot answer.
    pub async fn read(&self, caller: &Caller) -> Result<EffectivePreferences, AppError> {
        load_effective(
            self.preferences.as_ref(),
            self.users.as_ref(),
            caller.user_id(),
        )
        .await
    }
}

/// The shared resolution both the read and the write return.
pub(crate) async fn load_effective(
    preferences: &dyn PreferencesRepository,
    users: &dyn UserLookup,
    user_id: UserId,
) -> Result<EffectivePreferences, AppError> {
    let stored = preferences
        .load(user_id)
        .await
        .map_err(AppError::from_repository)?;
    let profile = users
        .provider_profile(user_id)
        .await
        .map_err(AppError::from_repository)?
        .ok_or(AppError::NotFound("user"))?;
    Ok(EffectivePreferences::resolve(
        user_id,
        stored.as_ref(),
        &profile,
    ))
}
