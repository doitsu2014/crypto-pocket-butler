//! WF1: submit an alert, deduplicate by idempotency key, sanitise its content,
//! and request the mandatory-email wake (BR1.1, BR3.1, BR5.1).

use std::collections::BTreeMap;
use std::sync::Arc;

use notification_domain::{
    sanitise_payload, Alert, AlertId, NotificationType, RequestedChannel, UserId,
};

use crate::common::FieldError;
use crate::ports::{
    AlertRepository, Clock, DeliveryControl, IdGenerator, InsertOutcome, RepositoryError,
};

/// A submitted alert, before validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmitCommand {
    /// The receiving user.
    pub user_id: String,
    /// The notification type.
    pub kind: String,
    /// The caller's dedupe key.
    pub idempotency_key: String,
    /// The caller's payload; the guard and the allow-list decide what is kept.
    pub payload: BTreeMap<String, String>,
    /// The request correlation id, for the trail when it is needed.
    pub correlation_id: Option<String>,
    /// The channel the emitter asked for (C9); absent means `both`.
    pub channel: Option<String>,
}

/// Whether the alert was new or a duplicate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmitStatus {
    /// A new alert was stored and is queued.
    Queued,
    /// The key already existed; nothing was sent (BR1.1).
    Deduplicated,
}

/// What a submit returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubmitResult {
    /// The stored or existing alert.
    pub alert_id: AlertId,
    /// New or duplicate.
    pub status: SubmitStatus,
}

/// Why a submit failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SubmitError {
    /// The request failed validation.
    #[error("the alert request is invalid")]
    Rejected(Vec<FieldError>),
    /// A storage failure.
    #[error("storage failure")]
    Repository(#[from] RepositoryError),
}

/// Stores submitted alerts.
pub struct SubmitAlert {
    alerts: Arc<dyn AlertRepository>,
    control: Arc<dyn DeliveryControl>,
    ids: Arc<dyn IdGenerator>,
    clock: Arc<dyn Clock>,
}

impl SubmitAlert {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        alerts: Arc<dyn AlertRepository>,
        control: Arc<dyn DeliveryControl>,
        ids: Arc<dyn IdGenerator>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            alerts,
            control,
            ids,
            clock,
        }
    }

    /// Validates, sanitises, stores, and dedupes one alert.
    ///
    /// # Errors
    /// Returns [`SubmitError::Rejected`] for an invalid request and
    /// [`SubmitError::Repository`] when the store fails.
    pub async fn execute(&self, command: SubmitCommand) -> Result<SubmitResult, SubmitError> {
        let user = UserId::parse(&command.user_id)
            .map_err(|error| SubmitError::Rejected(vec![FieldError::from(error)]))?;
        let kind = NotificationType::parse(&command.kind)
            .map_err(|error| SubmitError::Rejected(vec![FieldError::from(error)]))?;
        let requested = RequestedChannel::parse(command.channel.as_deref())
            .map_err(|error| SubmitError::Rejected(vec![FieldError::from(error)]))?;
        let now = self.clock.now();
        let payload = sanitise_payload(kind, command.payload);
        let alert = Alert::queue(
            AlertId::new(self.ids.next()),
            user,
            kind,
            &command.idempotency_key,
            payload,
            now,
        )
        .map_err(|error| SubmitError::Rejected(vec![FieldError::from(error)]))?
        .with_requested_channel(requested);

        match self.alerts.insert(&alert).await? {
            InsertOutcome::Inserted(alert) => {
                // BR3.1 and NFR2.5: a mandatory email wakes the worker within
                // one poll; the queue carries every other alert.
                if kind.is_mandatory_email() {
                    self.control.request_wake(now).await?;
                }
                Ok(SubmitResult {
                    alert_id: alert.id(),
                    status: SubmitStatus::Queued,
                })
            }
            InsertOutcome::Duplicate(existing) => Ok(SubmitResult {
                alert_id: existing.id(),
                status: SubmitStatus::Deduplicated,
            }),
        }
    }
}
