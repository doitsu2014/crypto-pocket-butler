//! The outage-only status cache: consulted only after a failed read, TTL 30 s,
//! cap 10 000 entries, LRU.
//!
//! Normal requests always read status (NFR24.1). The cache exists so a status
//! source blip does not take every service down, and it is bounded so a disable
//! takes effect within 30 seconds at worst. It is never a second source of
//! truth and never answers an `active` past its window.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use identity_application::authorisation::ports::{ObservationSink, StatusReadError, StatusReader};
use identity_application::ports::Clock;
use identity_domain::{UserId, UserStatus};
use uuid::Uuid;

/// The production window.
pub const CACHE_TTL: Duration = Duration::seconds(30);
/// The production cap.
pub const CACHE_CAPACITY: usize = 10_000;

#[derive(Debug, Clone)]
struct CacheEntry {
    status: UserStatus,
    stored_at: DateTime<Utc>,
    last_used: u64,
}

#[derive(Debug, Default)]
struct CacheState {
    entries: HashMap<Uuid, CacheEntry>,
    order: u64,
}

/// The outage cache over another status reader.
pub struct StatusCache {
    inner: Arc<dyn StatusReader>,
    clock: Arc<dyn Clock>,
    observation: Arc<dyn ObservationSink>,
    ttl: Duration,
    capacity: usize,
    state: Mutex<CacheState>,
}

impl StatusCache {
    /// Creates the cache with the production window and cap.
    #[must_use]
    pub fn new(
        inner: Arc<dyn StatusReader>,
        clock: Arc<dyn Clock>,
        observation: Arc<dyn ObservationSink>,
    ) -> Self {
        Self::with_limits(inner, clock, observation, CACHE_TTL, CACHE_CAPACITY)
    }

    /// Creates the cache with explicit bounds, for tests.
    #[must_use]
    pub fn with_limits(
        inner: Arc<dyn StatusReader>,
        clock: Arc<dyn Clock>,
        observation: Arc<dyn ObservationSink>,
        ttl: Duration,
        capacity: usize,
    ) -> Self {
        Self {
            inner,
            clock,
            observation,
            ttl,
            capacity: capacity.max(1),
            state: Mutex::new(CacheState::default()),
        }
    }

    /// Locks the state; a poisoned lock still holds consistent data because
    /// every critical section is allocation-only.
    fn state(&self) -> std::sync::MutexGuard<'_, CacheState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Records a successful read and returns it, evicting the least recently
    /// used entry when the cache is full.
    fn store(&self, user_id: Uuid, status: UserStatus) {
        let now = self.clock.now();
        let mut state = self.state();
        state.order += 1;
        let order = state.order;
        let is_new = !state.entries.contains_key(&user_id);
        if is_new && state.entries.len() >= self.capacity {
            if let Some(victim) = state
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(id, _)| *id)
            {
                state.entries.remove(&victim);
            }
        }
        state.entries.insert(
            user_id,
            CacheEntry {
                status,
                stored_at: now,
                last_used: order,
            },
        );
    }

    /// Drops an entry, so a deleted or vanished user is never served from it.
    fn remove(&self, user_id: Uuid) {
        self.state().entries.remove(&user_id);
    }

    /// Looks up a live entry, refreshing its recency, and reports the gauges.
    fn lookup(&self, user_id: Uuid) -> Option<UserStatus> {
        let now = self.clock.now();
        let (status, age_seconds, entries) = {
            let mut state = self.state();
            let entry = state.entries.get(&user_id).cloned()?;
            let age = now.signed_duration_since(entry.stored_at);
            if age > self.ttl {
                state.entries.remove(&user_id);
                return None;
            }
            state.order += 1;
            let order = state.order;
            if let Some(entry) = state.entries.get_mut(&user_id) {
                entry.last_used = order;
            }
            (entry.status, age.num_seconds(), state.entries.len())
        };
        self.observation.record_cache(age_seconds, entries);
        Some(status)
    }
}

#[async_trait]
impl StatusReader for StatusCache {
    async fn status_of(&self, user_id: UserId) -> Result<Option<UserStatus>, StatusReadError> {
        match self.inner.status_of(user_id).await {
            Ok(Some(status)) => {
                self.store(user_id.as_uuid(), status);
                Ok(Some(status))
            }
            Ok(None) => {
                self.remove(user_id.as_uuid());
                Ok(None)
            }
            Err(error) => {
                // The cache sees every failed read, including the ones it masks,
                // so it is the one place the failure counter increments.
                self.observation.record_status_lookup_failure(error.cause());
                match self.lookup(user_id.as_uuid()) {
                    Some(status) => Ok(Some(status)),
                    None => Err(error),
                }
            }
        }
    }
}
