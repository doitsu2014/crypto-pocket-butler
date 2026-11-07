//! WF3 and WF4: flag evaluation and administration (BR4.1–BR4.3).
//!
//! Flags are public configuration and are cached in process for thirty seconds.
//! An administrator's change may take up to thirty seconds per replica; replicas
//! may briefly disagree, which the performance design accepts.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use analytics_domain::{FeatureFlag, FeatureFlagId, FlagStatus, FlagUpdate, NewFeatureFlag};
use chrono::{DateTime, Duration, Utc};

use crate::common::FieldError;
use crate::hashing::flag_bucket;
use crate::ports::{Clock, FlagRepository, RepositoryError};
use crate::trail::Trail;

use analytics_domain::FLAG_CACHE_SECONDS;

/// The fields a create request supplies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlagCommand {
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
pub struct FlagPatch {
    /// A replacement owner.
    pub owner: Option<String>,
    /// A replacement expiry instant.
    pub expires_at: Option<DateTime<Utc>>,
    /// A replacement rollout percentage.
    pub rollout_percentage: Option<u16>,
    /// A replacement active/disabled status.
    pub status: Option<String>,
}

/// The answer to `GET /v1/flags/{key}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlagEvaluation {
    /// The requested key.
    pub key: String,
    /// Whether the flag is on for this caller.
    pub enabled: bool,
}

/// Why a flag operation failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FlagError {
    /// The request failed validation.
    #[error("the flag request is invalid")]
    Rejected(Vec<FieldError>),
    /// The flag does not exist.
    #[error("the flag does not exist")]
    NotFound,
    /// A storage failure.
    #[error("the flag could not be read or written: {0}")]
    Repository(#[from] RepositoryError),
}

#[derive(Debug, Clone)]
struct CachedFlag {
    flag: Option<FeatureFlag>,
    fetched_at: DateTime<Utc>,
}

/// Creates, updates, deletes, lists, and evaluates flags.
pub struct FlagService {
    repository: Arc<dyn FlagRepository>,
    clock: Arc<dyn Clock>,
    trail: Arc<Trail>,
    cache: Mutex<HashMap<String, CachedFlag>>,
}

impl FlagService {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        repository: Arc<dyn FlagRepository>,
        clock: Arc<dyn Clock>,
        trail: Arc<Trail>,
    ) -> Self {
        Self {
            repository,
            clock,
            trail,
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Creates a flag (BR4.1).
    ///
    /// # Errors
    /// Returns [`FlagError::Rejected`] for invalid input or a duplicate key,
    /// and [`FlagError::Repository`] on storage failure.
    pub async fn create(
        &self,
        command: FlagCommand,
        actor: &str,
    ) -> Result<FeatureFlag, FlagError> {
        let now = self.clock.now();
        let flag = FeatureFlag::create(
            FeatureFlagId::generate(),
            NewFeatureFlag {
                key: command.key,
                owner: command.owner,
                expires_at: command.expires_at,
                rollout_percentage: command.rollout_percentage,
                status: command.status,
            },
            now,
        )
        .map_err(|error| FlagError::Rejected(vec![FieldError::from(error)]))?;

        match self.repository.insert(&flag).await {
            Ok(()) => {}
            Err(RepositoryError::Conflict(reason)) => {
                return Err(FlagError::Rejected(vec![FieldError::new("key", reason)]));
            }
            Err(error) => return Err(FlagError::Repository(error)),
        }
        self.invalidate();
        self.trail.flag_changed("created", flag.key(), actor).await;
        Ok(flag)
    }

    /// Applies a partial update (BR4.1).
    ///
    /// # Errors
    /// Returns [`FlagError::NotFound`] when the key is unknown,
    /// [`FlagError::Rejected`] for invalid input, and [`FlagError::Repository`]
    /// on storage failure.
    pub async fn update(
        &self,
        key: &str,
        patch: FlagPatch,
        actor: &str,
    ) -> Result<FeatureFlag, FlagError> {
        let now = self.clock.now();
        let mut flag = self
            .repository
            .find_by_key(key)
            .await
            .map_err(FlagError::Repository)?
            .ok_or(FlagError::NotFound)?;
        flag.apply_update(
            FlagUpdate {
                owner: patch.owner,
                expires_at: patch.expires_at,
                rollout_percentage: patch.rollout_percentage,
                status: patch.status,
            },
            now,
        )
        .map_err(|error| FlagError::Rejected(vec![FieldError::from(error)]))?;
        self.repository
            .update(&flag)
            .await
            .map_err(FlagError::Repository)?;
        self.invalidate();
        self.trail.flag_changed("updated", flag.key(), actor).await;
        Ok(flag)
    }

    /// Deletes a flag.
    ///
    /// # Errors
    /// Returns [`FlagError::NotFound`] when the key is unknown, and
    /// [`FlagError::Repository`] on storage failure.
    pub async fn delete(&self, key: &str, actor: &str) -> Result<(), FlagError> {
        let removed = self
            .repository
            .delete(key)
            .await
            .map_err(FlagError::Repository)?;
        if !removed {
            return Err(FlagError::NotFound);
        }
        self.invalidate();
        self.trail.flag_changed("deleted", key, actor).await;
        Ok(())
    }

    /// Lists flags, optionally only those past their expiry (BR4.3).
    ///
    /// # Errors
    /// Returns [`FlagError::Repository`] on storage failure.
    pub async fn list(&self, expired_only: bool) -> Result<Vec<FeatureFlag>, FlagError> {
        let now = self.clock.now();
        let mut flags = self
            .repository
            .list()
            .await
            .map_err(FlagError::Repository)?;
        if expired_only {
            flags.retain(|flag| flag.effective_status(now) == FlagStatus::Expired);
        }
        flags.sort_by(|left, right| left.key().cmp(right.key()));
        Ok(flags)
    }

    /// Evaluates one flag for one subject (BR4.2, BR4.3).
    ///
    /// # Errors
    /// Returns [`FlagError::Repository`] on storage failure.
    pub async fn evaluate(&self, key: &str, subject: &str) -> Result<FlagEvaluation, FlagError> {
        let now = self.clock.now();
        let flag = match self.cached(key, now) {
            Some(flag) => flag,
            None => {
                let flag = self
                    .repository
                    .find_by_key(key)
                    .await
                    .map_err(FlagError::Repository)?;
                self.remember(key, flag.clone(), now);
                flag
            }
        };
        let enabled = flag
            .as_ref()
            .is_some_and(|flag| flag.enabled_for_bucket(flag_bucket(subject), now));
        Ok(FlagEvaluation {
            key: key.to_string(),
            enabled,
        })
    }

    fn cached(&self, key: &str, now: DateTime<Utc>) -> Option<Option<FeatureFlag>> {
        let cache = self.cache.lock().unwrap_or_else(|error| error.into_inner());
        cache.get(key).and_then(|entry| {
            let age = now - entry.fetched_at;
            (age < Duration::seconds(FLAG_CACHE_SECONDS)).then(|| entry.flag.clone())
        })
    }

    fn remember(&self, key: &str, flag: Option<FeatureFlag>, now: DateTime<Utc>) {
        let mut cache = self.cache.lock().unwrap_or_else(|error| error.into_inner());
        cache.insert(
            key.to_string(),
            CachedFlag {
                flag,
                fetched_at: now,
            },
        );
    }

    fn invalidate(&self) {
        let mut cache = self.cache.lock().unwrap_or_else(|error| error.into_inner());
        cache.clear();
    }
}
