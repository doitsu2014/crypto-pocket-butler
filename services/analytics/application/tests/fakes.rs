//! In-memory port fakes. No database, no network, no real clock.

use std::collections::BTreeMap;
use std::sync::Mutex;

use analytics_application::ports::{
    AuditClient, AuditRecord, Clock, ConsentChecker, ConsentRepository, EventRepository,
    ExternalError, FlagRepository, Pause, RepositoryError, SaltProvider, SaltVersion,
};
use analytics_domain::{
    sanitize_event, AnalyticsEvent, AnalyticsEventId, ConsentCheck, FeatureFlag, Metric,
    MetricAggregate, Period,
};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};

/// A clock a test moves explicitly.
#[derive(Debug)]
pub struct MutableClock {
    now: Mutex<DateTime<Utc>>,
}

impl MutableClock {
    /// A clock fixed at `now`.
    #[must_use]
    pub fn at(now: DateTime<Utc>) -> Self {
        Self {
            now: Mutex::new(now),
        }
    }

    /// Moves the clock forward.
    pub fn advance(&self, duration: Duration) {
        let mut now = self.now.lock().unwrap();
        *now += duration;
    }
}

impl Clock for MutableClock {
    fn now(&self) -> DateTime<Utc> {
        *self.now.lock().unwrap()
    }
}

/// A pause that never sleeps.
pub struct NoPause;

#[async_trait]
impl Pause for NoPause {
    async fn pause(&self) {}
}

/// An event store that also records consent rows written with an event.
#[derive(Default)]
pub struct FakeEvents {
    pub stored: Mutex<Vec<AnalyticsEvent>>,
    pub consents: Mutex<Vec<ConsentCheck>>,
    pub delete_calls: Mutex<usize>,
}

#[async_trait]
impl EventRepository for FakeEvents {
    async fn insert(
        &self,
        event: &AnalyticsEvent,
        consent: Option<&ConsentCheck>,
    ) -> Result<(), RepositoryError> {
        self.stored.lock().unwrap().push(event.clone());
        if let Some(consent) = consent {
            self.consents.lock().unwrap().push(consent.clone());
        }
        Ok(())
    }

    async fn events_between(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<AnalyticsEvent>, RepositoryError> {
        Ok(self
            .stored
            .lock()
            .unwrap()
            .iter()
            .filter(|event| event.occurred_at() >= from && event.occurred_at() < to)
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn delete_before(
        &self,
        cutoff: DateTime<Utc>,
        limit: u32,
    ) -> Result<u64, RepositoryError> {
        *self.delete_calls.lock().unwrap() += 1;
        let mut stored = self.stored.lock().unwrap();
        let mut removed = 0u64;
        stored.retain(|event| {
            if removed < u64::from(limit) && event.occurred_at() < cutoff {
                removed += 1;
                false
            } else {
                true
            }
        });
        Ok(removed)
    }

    async fn delete_by_user_refs(&self, user_refs: &[String]) -> Result<u64, RepositoryError> {
        let mut stored = self.stored.lock().unwrap();
        let mut removed = 0u64;
        stored.retain(|event| {
            let matches = event
                .user_ref()
                .is_some_and(|user_ref| user_refs.iter().any(|value| value == user_ref));
            if matches {
                removed += 1;
            }
            !matches
        });
        Ok(removed)
    }
}

/// A consent-check store.
#[derive(Default)]
pub struct FakeConsents {
    pub stored: Mutex<Vec<ConsentCheck>>,
}

#[async_trait]
impl ConsentRepository for FakeConsents {
    async fn insert(&self, check: &ConsentCheck) -> Result<(), RepositoryError> {
        self.stored.lock().unwrap().push(check.clone());
        Ok(())
    }

    async fn delete_before(
        &self,
        cutoff: DateTime<Utc>,
        limit: u32,
    ) -> Result<u64, RepositoryError> {
        let mut stored = self.stored.lock().unwrap();
        let mut removed = 0u64;
        stored.retain(|check| {
            if removed < u64::from(limit) && check.checked_at() < cutoff {
                removed += 1;
                false
            } else {
                true
            }
        });
        Ok(removed)
    }

    async fn delete_by_user_refs(&self, user_refs: &[String]) -> Result<u64, RepositoryError> {
        let mut stored = self.stored.lock().unwrap();
        let mut removed = 0u64;
        stored.retain(|check| {
            let matches = check
                .user_ref()
                .is_some_and(|user_ref| user_refs.iter().any(|value| value == user_ref));
            if matches {
                removed += 1;
            }
            !matches
        });
        Ok(removed)
    }
}

/// A C3 consent checker with a settable answer.
pub struct FakeConsentChecker {
    result: Mutex<Result<bool, ExternalError>>,
    pub calls: Mutex<usize>,
}

impl FakeConsentChecker {
    /// A checker that always returns `result`.
    #[must_use]
    pub fn new(result: Result<bool, ExternalError>) -> Self {
        Self {
            result: Mutex::new(result),
            calls: Mutex::new(0),
        }
    }
}

#[async_trait]
impl ConsentChecker for FakeConsentChecker {
    async fn analytics_granted(&self, _user_id: &str) -> Result<bool, ExternalError> {
        *self.calls.lock().unwrap() += 1;
        self.result.lock().unwrap().clone()
    }
}

/// A salt provider with settable current and retained answers.
pub struct FakeSalts {
    current: Mutex<Result<SaltVersion, ExternalError>>,
    retained: Mutex<Result<Vec<SaltVersion>, ExternalError>>,
}

impl FakeSalts {
    /// A provider that works for both calls.
    #[must_use]
    pub fn working(current: SaltVersion, retained: Vec<SaltVersion>) -> Self {
        Self {
            current: Mutex::new(Ok(current)),
            retained: Mutex::new(Ok(retained)),
        }
    }

    /// A provider whose current salt read fails.
    #[must_use]
    pub fn failing_current(error: ExternalError) -> Self {
        Self {
            current: Mutex::new(Err(error)),
            retained: Mutex::new(Ok(Vec::new())),
        }
    }

    /// A provider whose retained read fails.
    #[must_use]
    pub fn failing_retained(error: ExternalError) -> Self {
        Self {
            current: Mutex::new(Ok(salt("v1"))),
            retained: Mutex::new(Err(error)),
        }
    }
}

#[async_trait]
impl SaltProvider for FakeSalts {
    async fn current(&self) -> Result<SaltVersion, ExternalError> {
        self.current.lock().unwrap().clone()
    }

    async fn retained(&self) -> Result<Vec<SaltVersion>, ExternalError> {
        self.retained.lock().unwrap().clone()
    }
}

/// Builds a deterministic salt version.
#[must_use]
pub fn salt(version: &str) -> SaltVersion {
    let byte = u8::try_from(version.len()).unwrap_or(1);
    SaltVersion {
        version: version.to_string(),
        salt: vec![byte; 32],
    }
}

/// An audit client that records what it was asked to write.
#[derive(Default)]
pub struct FakeAudit {
    pub records: Mutex<Vec<AuditRecord>>,
}

#[async_trait]
impl AuditClient for FakeAudit {
    async fn append(&self, record: &AuditRecord) -> Result<(), ExternalError> {
        self.records.lock().unwrap().push(record.clone());
        Ok(())
    }
}

/// A flag store.
#[derive(Default)]
pub struct FakeFlags {
    pub flags: Mutex<Vec<FeatureFlag>>,
}

impl FakeFlags {
    /// Replaces a flag's status behind the service's cache.
    pub fn replace_status(&self, key: &str, status: analytics_domain::FlagStatus) {
        let mut flags = self.flags.lock().unwrap();
        for flag in flags.iter_mut() {
            if flag.key() == key {
                let update = analytics_domain::FlagUpdate {
                    status: Some(status.as_str().to_string()),
                    ..analytics_domain::FlagUpdate::default()
                };
                flag.apply_update(update, flag.updated_at()).unwrap();
            }
        }
    }
}

#[async_trait]
impl FlagRepository for FakeFlags {
    async fn insert(&self, flag: &FeatureFlag) -> Result<(), RepositoryError> {
        let mut flags = self.flags.lock().unwrap();
        if flags.iter().any(|existing| existing.key() == flag.key()) {
            return Err(RepositoryError::Conflict(
                "the key already exists".to_string(),
            ));
        }
        flags.push(flag.clone());
        Ok(())
    }

    async fn update(&self, flag: &FeatureFlag) -> Result<(), RepositoryError> {
        let mut flags = self.flags.lock().unwrap();
        if let Some(existing) = flags
            .iter_mut()
            .find(|existing| existing.key() == flag.key())
        {
            *existing = flag.clone();
        }
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool, RepositoryError> {
        let mut flags = self.flags.lock().unwrap();
        let before = flags.len();
        flags.retain(|flag| flag.key() != key);
        Ok(flags.len() != before)
    }

    async fn find_by_key(&self, key: &str) -> Result<Option<FeatureFlag>, RepositoryError> {
        Ok(self
            .flags
            .lock()
            .unwrap()
            .iter()
            .find(|flag| flag.key() == key)
            .cloned())
    }

    async fn list(&self) -> Result<Vec<FeatureFlag>, RepositoryError> {
        Ok(self.flags.lock().unwrap().clone())
    }
}

/// An aggregate store.
#[derive(Default)]
pub struct FakeAggregates {
    pub stored: Mutex<Vec<MetricAggregate>>,
}

#[async_trait]
impl analytics_application::ports::AggregateRepository for FakeAggregates {
    async fn upsert(&self, aggregate: &MetricAggregate) -> Result<(), RepositoryError> {
        let mut stored = self.stored.lock().unwrap();
        if let Some(existing) = stored.iter_mut().find(|existing| {
            existing.metric() == aggregate.metric()
                && existing.period() == aggregate.period()
                && existing.cohort() == aggregate.cohort()
                && existing.period_start() == aggregate.period_start()
        }) {
            *existing = aggregate.clone();
        } else {
            stored.push(aggregate.clone());
        }
        Ok(())
    }

    async fn list(
        &self,
        metric: Option<Metric>,
        period: Option<Period>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        limit: u32,
    ) -> Result<Vec<MetricAggregate>, RepositoryError> {
        Ok(self
            .stored
            .lock()
            .unwrap()
            .iter()
            .filter(|aggregate| metric.is_none_or(|value| aggregate.metric() == value))
            .filter(|aggregate| period.is_none_or(|value| aggregate.period() == value))
            .filter(|aggregate| from.is_none_or(|value| aggregate.period_start() >= value))
            .filter(|aggregate| to.is_none_or(|value| aggregate.period_start() < value))
            .take(limit as usize)
            .cloned()
            .collect())
    }
}

/// One `events_between` request: `(from, to, limit)`.
pub type EventWindowCall = (DateTime<Utc>, DateTime<Utc>, u32);

/// A synthetic event-store window that can prove the closed day survives a
/// prior-week volume far above the old single-window cap.
pub struct DenseEvents {
    pub day_start: DateTime<Utc>,
    pub closed: Vec<AnalyticsEvent>,
    pub prior_count: usize,
    pub calls: Mutex<Vec<EventWindowCall>>,
}

#[async_trait]
impl EventRepository for DenseEvents {
    async fn insert(
        &self,
        _event: &AnalyticsEvent,
        _consent: Option<&ConsentCheck>,
    ) -> Result<(), RepositoryError> {
        Ok(())
    }

    async fn events_between(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<AnalyticsEvent>, RepositoryError> {
        self.calls.lock().unwrap().push((from, to, limit));
        let mut events: Vec<AnalyticsEvent> = self
            .closed
            .iter()
            .filter(|event| event.occurred_at() >= from && event.occurred_at() < to)
            .cloned()
            .collect();
        if to - from > Duration::days(8) {
            for index in 0..self.prior_count {
                let when = self.day_start - Duration::days(1 + (index % 6) as i64);
                events.push(synthetic_event(&format!("prior-{index}"), when));
            }
        }
        events.truncate(limit as usize);
        Ok(events)
    }

    async fn delete_before(
        &self,
        _cutoff: DateTime<Utc>,
        _limit: u32,
    ) -> Result<u64, RepositoryError> {
        Ok(0)
    }

    async fn delete_by_user_refs(&self, _user_refs: &[String]) -> Result<u64, RepositoryError> {
        Ok(0)
    }
}

fn synthetic_event(user_ref: &str, when: DateTime<Utc>) -> AnalyticsEvent {
    AnalyticsEvent::from_stored(
        AnalyticsEventId::generate(),
        "valuation_read",
        when,
        BTreeMap::new(),
        Some(user_ref.to_string()),
        "analytics",
        when,
    )
    .expect("the synthetic event is catalogued")
}

/// Builds a stored event for a fact fixture.
#[must_use]
pub fn stored_event(
    name: &str,
    when: DateTime<Utc>,
    user_ref: &str,
    properties: BTreeMap<String, String>,
) -> AnalyticsEvent {
    let sanitized = sanitize_event(name, when, properties).expect("fixture event is in catalogue");
    AnalyticsEvent::store(
        AnalyticsEventId::generate(),
        sanitized,
        Some(user_ref.to_string()),
        when,
    )
}
