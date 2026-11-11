//! U4 — partial preference write against closed sets (BR4.1, BR4.2, BR4.9).
//!
//! Only the supplied fields change. An empty update is a read, not a write, so a
//! caller cannot create a row by writing nothing.

use std::sync::Arc;

use identity_domain::{EffectivePreferences, PreferenceUpdate};

use super::ports::{PreferencesRepository, UserLookup};
use super::read::load_effective;
use crate::common::{AppError, Caller};
use crate::consent::ports::{ObservationSink, PreferenceWriteOutcome};
use crate::ports::Clock;

/// Applies a validated partial preference update.
pub struct WriteUserPreferences {
    preferences: Arc<dyn PreferencesRepository>,
    users: Arc<dyn UserLookup>,
    clock: Arc<dyn Clock>,
    observation: Arc<dyn ObservationSink>,
}

impl WriteUserPreferences {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        preferences: Arc<dyn PreferencesRepository>,
        users: Arc<dyn UserLookup>,
        clock: Arc<dyn Clock>,
        observation: Arc<dyn ObservationSink>,
    ) -> Self {
        Self {
            preferences,
            users,
            clock,
            observation,
        }
    }

    /// Applies only the supplied fields and returns the effective result.
    ///
    /// # Errors
    /// Returns [`AppError::Unavailable`] when the store cannot answer. An
    /// out-of-set value is refused at the HTTP boundary with a 422.
    pub async fn write(
        &self,
        caller: &Caller,
        update: PreferenceUpdate,
    ) -> Result<EffectivePreferences, AppError> {
        let user_id = caller.user_id();
        if update.is_empty() {
            return load_effective(self.preferences.as_ref(), self.users.as_ref(), user_id).await;
        }
        let stored = self
            .preferences
            .upsert(user_id, &update, self.clock.now())
            .await
            .map_err(AppError::from_repository)?;
        self.observation
            .record_preference_write(PreferenceWriteOutcome::Applied);
        let profile = self
            .users
            .provider_profile(user_id)
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("user"))?;
        Ok(EffectivePreferences::resolve(
            user_id,
            Some(&stored),
            &profile,
        ))
    }
}
