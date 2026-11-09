//! In-memory port fakes. No database, no network, no real clock.

#![allow(dead_code)]

use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration as StdDuration;

use chrono::{DateTime, Duration, Utc};
use notification_application::ports::{
    AlertRepository, AnonymisedRows, AttemptRepository, AuditClient, AuditRecord, Clock,
    ContactDirectory, DeliveryControl, EmailMessage, EmailSender, ExternalError, IdGenerator,
    InsertOutcome, Pause, PreferenceRepository, ReadinessProbe, RepositoryError, ServiceCaller,
    ServiceTokenRegistry,
};
use notification_domain::{
    Alert, AlertId, AlertStatus, Channel, DeliveryAttempt, NotificationPreference,
    NotificationType, UserId,
};
use uuid::Uuid;

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
        *self.now.lock().unwrap() += duration;
    }
}

impl Clock for MutableClock {
    fn now(&self) -> DateTime<Utc> {
        *self.now.lock().unwrap()
    }
}

/// A deterministic identifier source.
#[derive(Debug, Default)]
pub struct SequentialIds {
    counter: AtomicU64,
}

impl IdGenerator for SequentialIds {
    fn next(&self) -> Uuid {
        let value = self.counter.fetch_add(1, Ordering::Relaxed) + 1;
        Uuid::from_u128(0x1000_0000_0000_4000_8000_0000_0000_0000 + u128::from(value))
    }
}

/// A pause that records what it was asked to wait.
#[derive(Debug, Default)]
pub struct RecordingPause {
    waited: Mutex<Vec<StdDuration>>,
}

impl RecordingPause {
    /// Every pause, in order.
    #[must_use]
    pub fn waited(&self) -> Vec<StdDuration> {
        self.waited.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl Pause for RecordingPause {
    async fn pause_for(&self, duration: StdDuration) {
        self.waited.lock().unwrap().push(duration);
    }
}

/// Everything the fake repositories share.
#[derive(Default)]
pub struct FakeStore {
    pub alerts: Mutex<Vec<Alert>>,
    pub attempts: Mutex<Vec<DeliveryAttempt>>,
    pub preferences: Mutex<Vec<NotificationPreference>>,
    pub failing: AtomicBool,
    pub calls: Mutex<BTreeMap<&'static str, u64>>,
}

impl FakeStore {
    fn bump(&self, operation: &'static str) {
        *self.calls.lock().unwrap().entry(operation).or_insert(0) += 1;
    }

    /// Makes the next repository call fail.
    pub fn fail_next(&self) {
        self.failing.store(true, Ordering::Relaxed);
    }

    /// How many times an operation was called.
    #[must_use]
    pub fn calls(&self, operation: &'static str) -> u64 {
        self.calls
            .lock()
            .unwrap()
            .get(operation)
            .copied()
            .unwrap_or(0)
    }

    fn check(&self, operation: &'static str) -> Result<(), RepositoryError> {
        self.bump(operation);
        if self.failing.swap(false, Ordering::Relaxed) {
            return Err(RepositoryError::Unavailable(format!(
                "{operation} refused by the fake"
            )));
        }
        Ok(())
    }
}

/// The alert store.
pub struct FakeAlerts {
    store: Arc<FakeStore>,
}

impl FakeAlerts {
    /// Creates the repository over a shared store.
    #[must_use]
    pub fn new(store: Arc<FakeStore>) -> Self {
        Self { store }
    }
}

#[async_trait::async_trait]
impl AlertRepository for FakeAlerts {
    async fn insert(&self, alert: &Alert) -> Result<InsertOutcome, RepositoryError> {
        self.store.check("insert_alert")?;
        let mut alerts = self.store.alerts.lock().unwrap();
        if let Some(existing) = alerts
            .iter()
            .find(|stored| stored.idempotency_key() == alert.idempotency_key())
        {
            return Ok(InsertOutcome::Duplicate(existing.clone()));
        }
        alerts.push(alert.clone());
        Ok(InsertOutcome::Inserted(alert.clone()))
    }

    async fn find_by_idempotency_key(&self, key: &str) -> Result<Option<Alert>, RepositoryError> {
        self.store.check("find_alert")?;
        Ok(self
            .store
            .alerts
            .lock()
            .unwrap()
            .iter()
            .find(|alert| alert.idempotency_key() == key)
            .cloned())
    }

    async fn list_for_user(
        &self,
        user_id: UserId,
        before: Option<(DateTime<Utc>, AlertId)>,
        limit: u32,
    ) -> Result<Vec<Alert>, RepositoryError> {
        self.store.check("list_alerts")?;
        let disabled: Vec<(NotificationType, Channel)> = self
            .store
            .preferences
            .lock()
            .unwrap()
            .iter()
            .filter(|row| row.channel() == Channel::InApp && !row.enabled())
            .map(|row| (row.kind(), row.channel()))
            .collect();
        let mut alerts: Vec<Alert> = self
            .store
            .alerts
            .lock()
            .unwrap()
            .iter()
            .filter(|alert| alert.user_id() == user_id)
            // BR2.1 and WF4.2: a type disabled for in-app never reaches the feed.
            .filter(|alert| !disabled.iter().any(|(kind, _)| *kind == alert.kind()))
            .cloned()
            .collect();
        alerts.sort_by(|left, right| {
            right
                .created_at()
                .cmp(&left.created_at())
                .then_with(|| right.id().as_uuid().cmp(&left.id().as_uuid()))
        });
        let filtered = alerts.into_iter().filter(|alert| match before {
            None => true,
            Some((created_at, id)) => {
                alert.created_at() < created_at
                    || (alert.created_at() == created_at && alert.id().as_uuid() < id.as_uuid())
            }
        });
        Ok(filtered.take(limit as usize).collect())
    }

    async fn due(&self, now: DateTime<Utc>, limit: u32) -> Result<Vec<Alert>, RepositoryError> {
        self.store.check("due_alerts")?;
        let mut alerts: Vec<Alert> = self
            .store
            .alerts
            .lock()
            .unwrap()
            .iter()
            .filter(|alert| alert.status() == AlertStatus::Queued)
            .filter(|alert| alert.next_attempt_at().is_some_and(|at| at <= now))
            .cloned()
            .collect();
        alerts.sort_by_key(Alert::next_attempt_at);
        Ok(alerts.into_iter().take(limit as usize).collect())
    }

    async fn has_due(&self, now: DateTime<Utc>) -> Result<bool, RepositoryError> {
        self.store.check("has_due")?;
        Ok(self.store.alerts.lock().unwrap().iter().any(|alert| {
            alert.status() == AlertStatus::Queued
                && alert.next_attempt_at().is_some_and(|at| at <= now)
        }))
    }

    async fn set_status(
        &self,
        id: AlertId,
        status: AlertStatus,
        next_attempt_at: Option<DateTime<Utc>>,
    ) -> Result<(), RepositoryError> {
        self.store.check("set_alert_status")?;
        let mut alerts = self.store.alerts.lock().unwrap();
        let Some(alert) = alerts.iter_mut().find(|alert| alert.id() == id) else {
            return Err(RepositoryError::Unavailable("alert is absent".to_string()));
        };
        // Rebuilding through the domain keeps the stored row well formed.
        *alert = Alert::from_stored(
            alert.id(),
            alert.user_id(),
            alert.kind(),
            alert.idempotency_key(),
            alert.payload().clone(),
            status.as_str(),
            alert.sanitised_fields().to_vec(),
            alert.created_at(),
            next_attempt_at,
        )
        .map_err(|error| RepositoryError::InvalidData(error.to_string()))?;
        Ok(())
    }

    async fn anonymise_user(
        &self,
        user_id: UserId,
        replacement: UserId,
    ) -> Result<AnonymisedRows, RepositoryError> {
        self.store.check("anonymise")?;
        let mut alerts = self.store.alerts.lock().unwrap();
        let mut alerts_changed = 0u64;
        for alert in alerts.iter_mut().filter(|alert| alert.user_id() == user_id) {
            *alert = Alert::from_stored(
                alert.id(),
                replacement,
                alert.kind(),
                alert.idempotency_key(),
                alert.payload().clone(),
                alert.status().as_str(),
                alert.sanitised_fields().to_vec(),
                alert.created_at(),
                alert.next_attempt_at(),
            )
            .map_err(|error| RepositoryError::InvalidData(error.to_string()))?;
            alerts_changed += 1;
        }
        let mut attempts = self.store.attempts.lock().unwrap();
        let mut attempts_changed = 0u64;
        for attempt in attempts
            .iter_mut()
            .filter(|attempt| attempt.user_id() == user_id)
        {
            *attempt = DeliveryAttempt::from_stored(
                attempt.id(),
                attempt.alert_id(),
                replacement,
                attempt.channel(),
                attempt.attempt(),
                attempt.status().as_str(),
                attempt.error_category(),
                attempt.attempted_at(),
            )
            .map_err(|error| RepositoryError::InvalidData(error.to_string()))?;
            attempts_changed += 1;
        }
        Ok(AnonymisedRows {
            alerts: alerts_changed,
            attempts: attempts_changed,
        })
    }

    async fn delete_before(
        &self,
        cutoff: DateTime<Utc>,
        limit: u32,
    ) -> Result<u64, RepositoryError> {
        self.store.check("delete_alerts")?;
        let mut alerts = self.store.alerts.lock().unwrap();
        let mut removed = 0u64;
        alerts.retain(|alert| {
            if removed < u64::from(limit) && alert.created_at() < cutoff {
                removed += 1;
                false
            } else {
                true
            }
        });
        Ok(removed)
    }
}

/// The attempt store.
pub struct FakeAttempts {
    store: Arc<FakeStore>,
}

impl FakeAttempts {
    /// Creates the repository over a shared store.
    #[must_use]
    pub fn new(store: Arc<FakeStore>) -> Self {
        Self { store }
    }
}

#[async_trait::async_trait]
impl AttemptRepository for FakeAttempts {
    async fn insert(&self, attempt: &DeliveryAttempt) -> Result<(), RepositoryError> {
        self.store.check("insert_attempt")?;
        self.store.attempts.lock().unwrap().push(attempt.clone());
        Ok(())
    }

    async fn list_for_alert(
        &self,
        alert_id: AlertId,
    ) -> Result<Vec<DeliveryAttempt>, RepositoryError> {
        self.store.check("list_attempts")?;
        Ok(self
            .store
            .attempts
            .lock()
            .unwrap()
            .iter()
            .filter(|attempt| attempt.alert_id() == alert_id)
            .cloned()
            .collect())
    }

    async fn delete_before(
        &self,
        cutoff: DateTime<Utc>,
        limit: u32,
    ) -> Result<u64, RepositoryError> {
        self.store.check("delete_attempts")?;
        let mut attempts = self.store.attempts.lock().unwrap();
        let mut removed = 0u64;
        attempts.retain(|attempt| {
            if removed < u64::from(limit) && attempt.attempted_at() < cutoff {
                removed += 1;
                false
            } else {
                true
            }
        });
        Ok(removed)
    }
}

/// The preference store.
pub struct FakePreferences {
    store: Arc<FakeStore>,
}

impl FakePreferences {
    /// Creates the repository over a shared store.
    #[must_use]
    pub fn new(store: Arc<FakeStore>) -> Self {
        Self { store }
    }
}

#[async_trait::async_trait]
impl PreferenceRepository for FakePreferences {
    async fn list_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<NotificationPreference>, RepositoryError> {
        Ok(self
            .store
            .preferences
            .lock()
            .unwrap()
            .iter()
            .filter(|row| row.user_id() == user_id)
            .cloned()
            .collect())
    }

    async fn upsert(&self, preference: &NotificationPreference) -> Result<(), RepositoryError> {
        let mut rows = self.store.preferences.lock().unwrap();
        rows.retain(|row| {
            !(row.user_id() == preference.user_id()
                && row.kind() == preference.kind()
                && row.channel() == preference.channel())
        });
        rows.push(preference.clone());
        Ok(())
    }

    async fn upsert_all(
        &self,
        preferences: &[NotificationPreference],
    ) -> Result<(), RepositoryError> {
        let mut rows = self.store.preferences.lock().unwrap();
        for preference in preferences {
            rows.retain(|row| {
                !(row.user_id() == preference.user_id()
                    && row.kind() == preference.kind()
                    && row.channel() == preference.channel())
            });
            rows.push(preference.clone());
        }
        Ok(())
    }
}

/// The delivery-control fake.
#[derive(Default)]
pub struct FakeControl {
    state: Mutex<ControlState>,
}

#[derive(Default)]
struct ControlState {
    wake: bool,
    lease: Option<(Uuid, DateTime<Utc>)>,
    emails_sent: u32,
}

impl FakeControl {
    /// Holds the lease as another replica would.
    pub fn hold_lease(&self, owner: Uuid, until: DateTime<Utc>) {
        self.state.lock().unwrap().lease = Some((owner, until));
    }

    /// Whether a wake flag is set.
    #[must_use]
    pub fn wake_requested_now(&self) -> bool {
        self.state.lock().unwrap().wake
    }

    /// Whether a lease is held.
    #[must_use]
    pub fn leased(&self) -> bool {
        self.state.lock().unwrap().lease.is_some()
    }
}

#[async_trait::async_trait]
impl DeliveryControl for FakeControl {
    async fn request_wake(&self, _at: DateTime<Utc>) -> Result<(), RepositoryError> {
        self.state.lock().unwrap().wake = true;
        Ok(())
    }

    async fn wake_requested(&self) -> Result<bool, RepositoryError> {
        Ok(self.state.lock().unwrap().wake)
    }

    async fn clear_wake(&self) -> Result<(), RepositoryError> {
        self.state.lock().unwrap().wake = false;
        Ok(())
    }

    async fn acquire_lease(
        &self,
        owner: Uuid,
        now: DateTime<Utc>,
        ttl_seconds: i64,
    ) -> Result<bool, RepositoryError> {
        let mut state = self.state.lock().unwrap();
        if let Some((_, until)) = state.lease {
            if until > now {
                return Ok(false);
            }
        }
        state.lease = Some((owner, now + Duration::seconds(ttl_seconds)));
        state.emails_sent = 0;
        Ok(true)
    }

    async fn reserve_email(&self, owner: Uuid, cap: u32) -> Result<bool, RepositoryError> {
        let mut state = self.state.lock().unwrap();
        if state.lease.is_none_or(|(held, _)| held != owner) {
            return Ok(false);
        }
        if state.emails_sent >= cap {
            return Ok(false);
        }
        state.emails_sent += 1;
        Ok(true)
    }

    async fn release_lease(&self, owner: Uuid) -> Result<(), RepositoryError> {
        let mut state = self.state.lock().unwrap();
        if state.lease.is_some_and(|(held, _)| held == owner) {
            state.lease = None;
        }
        Ok(())
    }
}

/// The C3 contact fake: queued answers, then a default.
pub struct FakeContacts {
    queued: Mutex<VecDeque<Result<Option<String>, ExternalError>>>,
    default: Mutex<Result<Option<String>, ExternalError>>,
    pub calls: AtomicU64,
}

impl FakeContacts {
    /// Creates the fake with one answer used for every call.
    #[must_use]
    pub fn always(answer: Result<Option<String>, ExternalError>) -> Self {
        Self {
            queued: Mutex::new(VecDeque::new()),
            default: Mutex::new(answer),
            calls: AtomicU64::new(0),
        }
    }

    /// Queues an answer ahead of the default.
    pub fn queue(&self, answer: Result<Option<String>, ExternalError>) {
        self.queued.lock().unwrap().push_back(answer);
    }
}

#[async_trait::async_trait]
impl ContactDirectory for FakeContacts {
    async fn verified_contact(&self, _user_id: &str) -> Result<Option<String>, ExternalError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let queued = self.queued.lock().unwrap().pop_front();
        match queued {
            Some(answer) => answer,
            None => self.default.lock().unwrap().clone(),
        }
    }
}

/// One provider call the fake saw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmailCall {
    /// The recipient the fake was asked to use.
    pub to: String,
    /// The subject.
    pub subject: String,
}

/// The email provider fake.
pub struct FakeEmail {
    queued: Mutex<VecDeque<Result<(), ExternalError>>>,
    default: Mutex<Result<(), ExternalError>>,
    pub calls: Mutex<Vec<EmailCall>>,
}

impl FakeEmail {
    /// Creates the fake with one answer used for every call.
    #[must_use]
    pub fn always(answer: Result<(), ExternalError>) -> Self {
        Self {
            queued: Mutex::new(VecDeque::new()),
            default: Mutex::new(answer),
            calls: Mutex::new(Vec::new()),
        }
    }

    /// Queues an answer ahead of the default.
    pub fn queue(&self, answer: Result<(), ExternalError>) {
        self.queued.lock().unwrap().push_back(answer);
    }

    /// Every call the fake saw.
    #[must_use]
    pub fn calls(&self) -> Vec<EmailCall> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl EmailSender for FakeEmail {
    async fn send(&self, message: &EmailMessage) -> Result<(), ExternalError> {
        self.calls.lock().unwrap().push(EmailCall {
            to: message.to.clone(),
            subject: message.subject.clone(),
        });
        let queued = self.queued.lock().unwrap().pop_front();
        match queued {
            Some(answer) => answer,
            None => self.default.lock().unwrap().clone(),
        }
    }
}

/// The C10 audit fake.
#[derive(Default)]
pub struct FakeAudit {
    pub records: Mutex<Vec<AuditRecord>>,
    fail_times: AtomicU64,
}

impl FakeAudit {
    /// Fails the next `count` appends.
    pub fn fail_next(&self, count: u64) {
        self.fail_times.store(count, Ordering::Relaxed);
    }

    /// Every record appended.
    #[must_use]
    pub fn records(&self) -> Vec<AuditRecord> {
        self.records.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl AuditClient for FakeAudit {
    async fn append(&self, record: &AuditRecord) -> Result<(), ExternalError> {
        let remaining = self.fail_times.load(Ordering::Relaxed);
        if remaining > 0 {
            self.fail_times.store(remaining - 1, Ordering::Relaxed);
            return Err(ExternalError::Unavailable(
                "C10 refused by the fake".to_string(),
            ));
        }
        self.records.lock().unwrap().push(record.clone());
        Ok(())
    }
}

/// A readiness probe with a fixed answer.
pub struct Ready(pub bool);

#[async_trait::async_trait]
impl ReadinessProbe for Ready {
    async fn check(&self) -> Result<(), String> {
        if self.0 {
            Ok(())
        } else {
            Err("a dependency is down".to_string())
        }
    }
}

/// Fixed service tokens.
pub struct FakeServiceTokens {
    /// Token to caller.
    pub entries: Vec<(String, ServiceCaller)>,
}

#[async_trait::async_trait]
impl ServiceTokenRegistry for FakeServiceTokens {
    async fn resolve(&self, token: &str) -> Result<Option<ServiceCaller>, RepositoryError> {
        Ok(self
            .entries
            .iter()
            .find(|(candidate, _)| candidate == token)
            .map(|(_, caller)| caller.clone()))
    }
}

/// A channel label for a fixture.
pub fn channel(raw: &str) -> Channel {
    Channel::parse(raw).unwrap()
}
