//! WF4 and WF5: deliver queued alerts, retry bounded failures, keep an
//! email-off brief in app, and audit a mandatory email's outcome
//! (BR2.1, BR3.1, BR4.1–BR4.3, NFR2.5, NFR9.5).
//!
//! The alerts table is the queue. One leased worker drains it; the API replicas
//! only set the durable wake flag.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use futures::stream::{self, StreamExt};
use notification_domain::{
    alert_status, honoured, next_attempt_at, summarise, Alert, AlertId, AlertStatus, AttemptStatus,
    Channel, DeliveryAttempt, ErrorCategory, NotificationType, DELIVERY_BATCH_SIZE, LEASE_SECONDS,
    MAX_EMAILS_PER_RUN, MAX_RUN_BATCHES, PROVIDER_CONCURRENCY,
};
use uuid::Uuid;

use crate::ports::{
    AlertRepository, AttemptRepository, Clock, ContactDirectory, DeliveryControl, EmailMessage,
    EmailSender, ExternalError, IdGenerator, PreferenceRepository, RepositoryError,
};
use crate::trail::Trail;

/// What one alert's delivery did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertOutcome {
    /// The alert.
    pub alert_id: AlertId,
    /// The type label.
    pub kind: String,
    /// The status after this run.
    pub status: String,
    /// Whether an email was left for a later run by the per-run cap.
    pub deferred: bool,
}

/// One attempt recorded in this run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttemptOutcome {
    /// The channel label.
    pub channel: String,
    /// The attempt status label.
    pub status: String,
}

/// What one run did.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeliveryReport {
    /// The lease owner, acting as the run id.
    pub run_id: Uuid,
    /// Alerts examined.
    pub processed: usize,
    /// Alerts still queued after the run (retry due, or deferred).
    pub queued: usize,
    /// Alerts fully delivered.
    pub delivered: usize,
    /// Alerts with one channel abandoned.
    pub partially_delivered: usize,
    /// Alerts with every enabled channel abandoned.
    pub failed: usize,
    /// Alerts whose email was left for a later run by the cap.
    pub deferred: usize,
    /// Attempts that succeeded.
    pub attempts_sent: u64,
    /// Attempts recorded as retryable failures.
    pub attempts_failed: u64,
    /// Attempts abandoned.
    pub attempts_abandoned: u64,
    /// Mandatory emails abandoned in this run.
    pub mandatory_abandoned: usize,
    /// C10 writes abandoned after every retry.
    pub audit_failures: u64,
    /// Per-alert outcomes for metrics.
    pub alerts: Vec<AlertOutcome>,
    /// Per-attempt outcomes for metrics.
    pub attempts: Vec<AttemptOutcome>,
}

impl DeliveryReport {
    fn absorb(
        &mut self,
        outcome: AlertOutcome,
        attempts: Vec<AttemptOutcome>,
        audit_failures: u64,
    ) {
        self.processed += 1;
        match outcome.status.as_str() {
            "delivered" => self.delivered += 1,
            "partially_delivered" => self.partially_delivered += 1,
            "failed" => self.failed += 1,
            _ => self.queued += 1,
        }
        if outcome.deferred {
            self.deferred += 1;
        }
        for attempt in &attempts {
            match attempt.status.as_str() {
                "sent" => self.attempts_sent += 1,
                "failed" => self.attempts_failed += 1,
                _ => self.attempts_abandoned += 1,
            }
        }
        self.audit_failures += audit_failures;
        self.alerts.push(outcome);
        self.attempts.extend(attempts);
    }
}

/// One alert's result inside a run: its outcome, the attempts recorded, and how
/// many audit writes were abandoned.
type Delivered = Result<(AlertOutcome, Vec<AttemptOutcome>, u64), DeliveryError>;

/// Why a run failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DeliveryError {
    /// Another replica holds the delivery lease.
    #[error("another delivery run holds the lease")]
    Busy,
    /// A storage failure.
    #[error("storage failure: {0}")]
    Repository(#[from] RepositoryError),
    /// A stored row could not be represented as a domain value.
    #[error("the delivery could not be recorded: {0}")]
    Domain(String),
}

/// Drains the alert queue under one lease.
pub struct DeliveryService {
    alerts: Arc<dyn AlertRepository>,
    preferences: Arc<dyn PreferenceRepository>,
    attempts: Arc<dyn AttemptRepository>,
    control: Arc<dyn DeliveryControl>,
    contacts: Arc<dyn ContactDirectory>,
    email: Arc<dyn EmailSender>,
    trail: Arc<Trail>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
    batch_size: u32,
    max_emails: u32,
    concurrency: usize,
    max_batches: u32,
}

impl DeliveryService {
    /// Creates the use case with the production limits.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        alerts: Arc<dyn AlertRepository>,
        preferences: Arc<dyn PreferenceRepository>,
        attempts: Arc<dyn AttemptRepository>,
        control: Arc<dyn DeliveryControl>,
        contacts: Arc<dyn ContactDirectory>,
        email: Arc<dyn EmailSender>,
        trail: Arc<Trail>,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            alerts,
            preferences,
            attempts,
            control,
            contacts,
            email,
            trail,
            clock,
            ids,
            batch_size: DELIVERY_BATCH_SIZE,
            max_emails: MAX_EMAILS_PER_RUN,
            concurrency: PROVIDER_CONCURRENCY,
            max_batches: MAX_RUN_BATCHES,
        }
    }

    /// Overrides the run limits; used by tests and by the launch tuning.
    #[must_use]
    pub fn with_limits(&self, batch_size: u32, max_emails: u32, concurrency: usize) -> Self {
        Self {
            alerts: self.alerts.clone(),
            preferences: self.preferences.clone(),
            attempts: self.attempts.clone(),
            control: self.control.clone(),
            contacts: self.contacts.clone(),
            email: self.email.clone(),
            trail: self.trail.clone(),
            clock: self.clock.clone(),
            ids: self.ids.clone(),
            batch_size: batch_size.max(1),
            max_emails,
            concurrency: concurrency.max(1),
            max_batches: self.max_batches,
        }
    }

    /// Polls the durable wake flag and the queue, and runs when either has
    /// work (NFR2.5). `None` means there was nothing to do.
    ///
    /// # Errors
    /// Returns [`DeliveryError`] when the store fails.
    pub async fn poll(&self) -> Result<Option<DeliveryReport>, DeliveryError> {
        let now = self.clock.now();
        let woken = self.control.wake_requested().await?;
        if !woken && !self.alerts.has_due(now).await? {
            return Ok(None);
        }
        match self.run().await {
            Ok(report) => Ok(Some(report)),
            // Another replica holds the lease; it will clear the flag.
            Err(DeliveryError::Busy) => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Runs one delivery pass under the single-flight lease.
    ///
    /// # Errors
    /// Returns [`DeliveryError::Busy`] when another replica holds the lease,
    /// and [`DeliveryError`] on a storage failure.
    pub async fn run(&self) -> Result<DeliveryReport, DeliveryError> {
        let now = self.clock.now();
        let owner = self.ids.next();
        if !self
            .control
            .acquire_lease(owner, now, LEASE_SECONDS)
            .await?
        {
            return Err(DeliveryError::Busy);
        }
        let result = self.drain(now, owner).await;
        if let Err(error) = self.control.release_lease(owner).await {
            tracing::warn!(error = %error, "failed to release the delivery lease");
        }
        match result {
            Ok(report) => {
                if let Err(error) = self.control.clear_wake().await {
                    tracing::warn!(error = %error, "failed to clear the delivery wake flag");
                }
                Ok(report)
            }
            Err(error) => Err(error),
        }
    }

    async fn drain(
        &self,
        now: DateTime<Utc>,
        owner: Uuid,
    ) -> Result<DeliveryReport, DeliveryError> {
        let mut report = DeliveryReport {
            run_id: owner,
            ..DeliveryReport::default()
        };
        for _ in 0..self.max_batches {
            let due = self.alerts.due(now, self.batch_size).await?;
            if due.is_empty() {
                break;
            }
            let outcomes: Vec<Delivered> =
                stream::iter(due.into_iter().map(|alert| self.deliver(alert, owner, now)))
                    .buffer_unordered(self.concurrency)
                    .collect()
                    .await;
            let mut deferred_any = false;
            for outcome in outcomes {
                let (outcome, attempts, audit_failures) = outcome?;
                deferred_any |= outcome.deferred;
                report.absorb(outcome, attempts, audit_failures);
            }
            if deferred_any {
                // A deferred alert is due again in a minute, so the next batch
                // would only defer it again.
                break;
            }
        }
        Ok(report)
    }

    async fn deliver(
        &self,
        alert: Alert,
        owner: Uuid,
        run_now: DateTime<Utc>,
    ) -> Result<(AlertOutcome, Vec<AttemptOutcome>, u64), DeliveryError> {
        // NFR1.3: an attempt's timestamp is the moment it is made, not the run's
        // start, so a retry scheduled after a long batch is measured correctly.
        let now = self.clock.now().max(run_now);
        let stored = self.preferences.list_for_user(alert.user_id()).await?;
        let enabled: Vec<Channel> = Channel::ALL
            .iter()
            .copied()
            .filter(|channel| {
                let value = stored
                    .iter()
                    .find(|row| row.kind() == alert.kind() && row.channel() == *channel)
                    .map(notification_domain::NotificationPreference::enabled);
                honoured(alert.kind(), *channel, value, alert.requested_channel())
            })
            .collect();
        let existing = self.attempts.list_for_alert(alert.id()).await?;
        let mut outcomes = summarise(&existing);
        let mut recorded = Vec::new();
        let mut deferred = false;
        let mut next_at: Option<DateTime<Utc>> = None;

        if enabled.contains(&Channel::InApp)
            && !outcomes
                .get(&Channel::InApp)
                .is_some_and(notification_domain::ChannelOutcome::is_terminal)
        {
            let attempt = DeliveryAttempt::sent(
                notification_domain::DeliveryAttemptId::new(self.ids.next()),
                alert.id(),
                alert.user_id(),
                Channel::InApp,
                1,
                now,
            )
            .map_err(|error| DeliveryError::Domain(error.to_string()))?;
            self.attempts.insert(&attempt).await?;
            let entry = outcomes.entry(Channel::InApp).or_default();
            entry.attempts = entry.attempts.saturating_add(1);
            entry.sent = true;
            recorded.push(attempt);
        }

        if enabled.contains(&Channel::Email) {
            let email_outcome = outcomes.get(&Channel::Email).copied().unwrap_or_default();
            if !email_outcome.is_terminal() {
                let number = email_outcome.attempts.saturating_add(1);
                if !self.control.reserve_email(owner, self.max_emails).await? {
                    // NFR20.4: the atomic allowance is spent; never exceed the
                    // provider budget, and let the next run take the remainder.
                    deferred = true;
                    next_at = next_attempt_at(1, now);
                } else {
                    let attempt = self.attempt_email(&alert, number, now).await?;
                    if attempt.status() == AttemptStatus::Failed {
                        next_at = next_attempt_at(number, now);
                    }
                    self.attempts.insert(&attempt).await?;
                    let entry = outcomes.entry(Channel::Email).or_default();
                    entry.attempts = entry.attempts.saturating_add(1);
                    match attempt.status() {
                        AttemptStatus::Sent => entry.sent = true,
                        AttemptStatus::Abandoned => entry.abandoned = true,
                        AttemptStatus::Failed => {}
                    }
                    recorded.push(attempt);
                }
            }
        }

        let status = if deferred {
            AlertStatus::Queued
        } else {
            alert_status(&enabled, &outcomes)
        };
        let next = if status == AlertStatus::Queued {
            Some(next_at.unwrap_or(now))
        } else {
            None
        };
        if status != alert.status() || next != alert.next_attempt_at() {
            self.alerts.set_status(alert.id(), status, next).await?;
        }

        let mut audit_failures = 0u64;
        if alert.kind().is_mandatory_email() && status.is_terminal() {
            let written = if status == AlertStatus::Delivered {
                self.trail
                    .security_alert_delivered(
                        alert.kind(),
                        alert.id(),
                        alert.user_id(),
                        alert.sanitised_fields(),
                    )
                    .await
            } else {
                self.trail
                    .security_alert_abandoned(
                        alert.kind(),
                        alert.id(),
                        alert.user_id(),
                        alert.sanitised_fields(),
                    )
                    .await
            };
            if !written {
                audit_failures += 1;
            }
        }

        let attempts = recorded
            .iter()
            .map(|attempt| AttemptOutcome {
                channel: attempt.channel().as_str().to_string(),
                status: attempt.status().as_str().to_string(),
            })
            .collect();
        Ok((
            AlertOutcome {
                alert_id: alert.id(),
                kind: alert.kind().as_str().to_string(),
                status: status.as_str().to_string(),
                deferred,
            },
            attempts,
            audit_failures,
        ))
    }

    /// One email try: the verified address is read from C3 every time and never
    /// stored (BR4.2, NFR9.3).
    async fn attempt_email(
        &self,
        alert: &Alert,
        number: u8,
        now: DateTime<Utc>,
    ) -> Result<DeliveryAttempt, DeliveryError> {
        let attempt_id = notification_domain::DeliveryAttemptId::new(self.ids.next());
        let built = |status: AttemptStatus, category: Option<ErrorCategory>| {
            match status {
                AttemptStatus::Sent => DeliveryAttempt::sent(
                    attempt_id,
                    alert.id(),
                    alert.user_id(),
                    Channel::Email,
                    number,
                    now,
                ),
                AttemptStatus::Abandoned => DeliveryAttempt::abandoned(
                    attempt_id,
                    alert.id(),
                    alert.user_id(),
                    Channel::Email,
                    number,
                    category.unwrap_or(ErrorCategory::Provider),
                    now,
                ),
                AttemptStatus::Failed => DeliveryAttempt::failed(
                    attempt_id,
                    alert.id(),
                    alert.user_id(),
                    Channel::Email,
                    number,
                    category,
                    now,
                ),
            }
            .map_err(|error| DeliveryError::Domain(error.to_string()))
        };

        let address = match self
            .contacts
            .verified_contact(&alert.user_id().to_string())
            .await
        {
            Ok(Some(address)) => address,
            // A missing or unverified address is permanent (BR4.2).
            Ok(None) => return built(AttemptStatus::Abandoned, Some(ErrorCategory::Address)),
            Err(ExternalError::Refused(_)) => {
                return built(AttemptStatus::Abandoned, Some(ErrorCategory::Address))
            }
            Err(ExternalError::Unavailable(_)) => return self.transient(number, now, &built),
        };

        let message = EmailMessage {
            to: address,
            subject: subject_for(alert.kind()).to_string(),
            body: body_for(alert),
        };
        match self.email.send(&message).await {
            Ok(()) => built(AttemptStatus::Sent, None),
            Err(ExternalError::Refused(_)) => {
                built(AttemptStatus::Abandoned, Some(ErrorCategory::Provider))
            }
            Err(ExternalError::Unavailable(_)) => self.transient(number, now, &built),
        }
    }

    fn transient(
        &self,
        number: u8,
        now: DateTime<Utc>,
        built: &impl Fn(AttemptStatus, Option<ErrorCategory>) -> Result<DeliveryAttempt, DeliveryError>,
    ) -> Result<DeliveryAttempt, DeliveryError> {
        if next_attempt_at(number, now).is_none() {
            // BR4.1: the third failure is abandoned.
            built(AttemptStatus::Abandoned, Some(ErrorCategory::Network))
        } else {
            built(AttemptStatus::Failed, Some(ErrorCategory::Network))
        }
    }
}

/// The subject line per type. It carries no payload value.
fn subject_for(kind: NotificationType) -> &'static str {
    match kind {
        NotificationType::Drift => "Portfolio drift alert",
        NotificationType::SyncFailed => "Account sync failed",
        NotificationType::KeyRevoked => "Exchange key revoked",
        NotificationType::Security => "Security notice",
        NotificationType::Brief => "Your periodic brief",
    }
}

/// The body is built from allow-listed, already-sanitised values only.
fn body_for(alert: &Alert) -> String {
    let mut body = subject_for(alert.kind()).to_string();
    for (key, value) in alert.payload() {
        body.push('\n');
        body.push_str(key);
        body.push_str(": ");
        body.push_str(value);
    }
    if !alert.sanitised_fields().is_empty() {
        body.push_str("\nSome details were withheld for your security.");
    }
    body
}
