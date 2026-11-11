//! U4 — the user's explicit preferences with a per-field source (BR4.1–BR4.3).
//!
//! `PreferencesService` is the earlier `users`-row update (BR3.3); the U4 use
//! cases beside it read and write the `user_preferences` table, which holds the
//! user's explicit choice and never writes u1's provider-derived profile.

pub mod ports;
pub mod read;
pub mod write;

pub use self::ports::{PreferencesRepository, UserLookup};

use std::sync::Arc;

use identity_domain::{AuditOutcome, Preferences, User, UserId};

use crate::common::{AppError, Caller};
use crate::ports::{Clock, UserRepository};
use crate::trail::{context, Trail};

/// Applies validated preference changes to the stored user row.
pub struct PreferencesService {
    users: Arc<dyn UserRepository>,
    trail: Arc<Trail>,
    clock: Arc<dyn Clock>,
}

impl PreferencesService {
    /// Creates the use case.
    #[must_use]
    pub fn new(users: Arc<dyn UserRepository>, trail: Arc<Trail>, clock: Arc<dyn Clock>) -> Self {
        Self {
            users,
            trail,
            clock,
        }
    }

    /// Applies a preference change for the owner. The values are already
    /// validated by their domain types at the HTTP boundary.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for another user's record and
    /// [`AppError::Unavailable`] when the store cannot answer.
    pub async fn update(
        &self,
        caller: &Caller,
        user_id: UserId,
        preferences: Preferences,
    ) -> Result<User, AppError> {
        caller.require_owner(user_id)?;
        let mut user = caller.user.clone();
        user.update_preferences(preferences, self.clock.now());
        self.users
            .update(&user)
            .await
            .map_err(AppError::from_repository)?;
        self.trail
            .record(
                Some(caller.user_id()),
                "identity.preferences.changed",
                user_id.to_string(),
                AuditOutcome::Success,
                &caller.correlation_id,
                context(&[]),
            )
            .await?;
        Ok(user)
    }
}
