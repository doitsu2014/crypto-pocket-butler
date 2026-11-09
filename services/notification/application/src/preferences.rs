//! WF3: view and save per-type, per-channel preferences (BR2.1–BR2.3, BR3.1).

use std::sync::Arc;

use notification_domain::{
    resolve_all, validate_change, Channel, NotificationPreference, NotificationType,
    ResolvedPreference, UserId,
};

use crate::common::FieldError;
use crate::ports::{Clock, IdGenerator, PreferenceRepository, RepositoryError};
use crate::trail::Trail;

/// One requested change from the settings screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreferenceChange {
    /// The notification type label.
    pub kind: String,
    /// The channel label.
    pub channel: String,
    /// The requested value.
    pub enabled: bool,
}

/// The saved view plus any audit writes that were abandoned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedPreferences {
    /// Every type and channel after the save.
    pub preferences: Vec<ResolvedPreference>,
    /// How many C10 writes were abandoned after every retry.
    pub audit_failures: u64,
}

/// Why a preference request failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PreferenceError {
    /// The request failed validation, including a refused mandatory email.
    #[error("the preference request is invalid")]
    Rejected(Vec<FieldError>),
    /// A storage failure.
    #[error("storage failure")]
    Repository(#[from] RepositoryError),
}

/// Views and saves preferences.
pub struct PreferenceService {
    preferences: Arc<dyn PreferenceRepository>,
    ids: Arc<dyn IdGenerator>,
    clock: Arc<dyn Clock>,
    trail: Arc<Trail>,
}

impl PreferenceService {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        preferences: Arc<dyn PreferenceRepository>,
        ids: Arc<dyn IdGenerator>,
        clock: Arc<dyn Clock>,
        trail: Arc<Trail>,
    ) -> Self {
        Self {
            preferences,
            ids,
            clock,
            trail,
        }
    }

    /// BR2.3: every type and channel, with defaults overlaid by stored rows.
    ///
    /// # Errors
    /// Returns [`PreferenceError`] for an invalid user or a storage failure.
    pub async fn view(&self, user_id: &str) -> Result<Vec<ResolvedPreference>, PreferenceError> {
        let user = self.parse_user(user_id)?;
        let stored = self.preferences.list_for_user(user).await?;
        Ok(resolve_all(user, &stored))
    }

    /// BR2.2: stores every valid change and applies it to the next event.
    ///
    /// # Errors
    /// Returns [`PreferenceError::Rejected`] for an invalid request or a refused
    /// mandatory email; every refused change writes a C10 event first.
    pub async fn save(
        &self,
        user_id: &str,
        changes: Vec<PreferenceChange>,
        correlation: Option<&str>,
    ) -> Result<SavedPreferences, PreferenceError> {
        let user = self.parse_user(user_id)?;
        let now = self.clock.now();
        let mut errors = Vec::new();
        let mut rows = Vec::new();
        let mut audit_failures = 0u64;

        for change in changes {
            let kind = match NotificationType::parse(&change.kind) {
                Ok(kind) => kind,
                Err(error) => {
                    errors.push(FieldError::from(error));
                    continue;
                }
            };
            let channel = match Channel::parse(&change.channel) {
                Ok(channel) => channel,
                Err(error) => {
                    errors.push(FieldError::from(error));
                    continue;
                }
            };
            if let Err(error) = validate_change(kind, channel, change.enabled) {
                errors.push(FieldError::from(error));
                if !self
                    .trail
                    .preference_refused(user, kind, channel, correlation)
                    .await
                {
                    audit_failures += 1;
                }
                continue;
            }
            match NotificationPreference::change(
                notification_domain::PreferenceId::new(self.ids.next()),
                user,
                kind,
                channel,
                change.enabled,
                now,
            ) {
                Ok(row) => rows.push(row),
                Err(error) => errors.push(FieldError::from(error)),
            }
        }

        if !errors.is_empty() {
            return Err(PreferenceError::Rejected(errors));
        }
        self.preferences.upsert_all(&rows).await?;
        let stored = self.preferences.list_for_user(user).await?;
        Ok(SavedPreferences {
            preferences: resolve_all(user, &stored),
            audit_failures,
        })
    }

    fn parse_user(&self, user_id: &str) -> Result<UserId, PreferenceError> {
        UserId::parse(user_id)
            .map_err(|error| PreferenceError::Rejected(vec![FieldError::from(error)]))
    }
}
