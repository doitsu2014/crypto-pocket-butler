//! One delivery try for one alert and channel, the retry schedule, and the
//! derivation of the alert's terminal status (BR4.1, BR4.2).

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};

use crate::alert::AlertStatus;
use crate::constants::{MAX_ATTEMPTS, RETRY_DELAY_SECONDS};
use crate::error::DomainError;
use crate::ids::{AlertId, DeliveryAttemptId, UserId};
use crate::preference::Channel;

/// The outcome of one delivery try.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttemptStatus {
    /// Delivered.
    Sent,
    /// Failed and still retryable.
    Failed,
    /// Failed with no retries left, or with a permanent error.
    Abandoned,
}

impl AttemptStatus {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sent => "sent",
            Self::Failed => "failed",
            Self::Abandoned => "abandoned",
        }
    }

    /// Parses a stored label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for any other value.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "sent" => Ok(Self::Sent),
            "failed" => Ok(Self::Failed),
            "abandoned" => Ok(Self::Abandoned),
            _ => Err(DomainError::Invalid {
                field: "status".to_string(),
                reason: "must be sent, failed, or abandoned".to_string(),
            }),
        }
    }
}

/// Why an attempt failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// The provider permanently refused the message.
    Provider,
    /// A transient network or 5xx failure, or a C3 network failure.
    Network,
    /// No verified address, or an unverified one (BR4.2). Permanent.
    Address,
    /// The message had no deliverable content.
    Content,
}

impl ErrorCategory {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Provider => "provider",
            Self::Network => "network",
            Self::Address => "address",
            Self::Content => "content",
        }
    }

    /// Parses a stored label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for any other value.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "provider" => Ok(Self::Provider),
            "network" => Ok(Self::Network),
            "address" => Ok(Self::Address),
            "content" => Ok(Self::Content),
            _ => Err(DomainError::Invalid {
                field: "errorCategory".to_string(),
                reason: "must be provider, network, address, or content".to_string(),
            }),
        }
    }

    /// Whether the failure is permanent, so no retry is attempted (BR4.2).
    #[must_use]
    pub fn is_permanent(self) -> bool {
        matches!(self, Self::Provider | Self::Address | Self::Content)
    }
}

/// One delivery try.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryAttempt {
    id: DeliveryAttemptId,
    alert_id: AlertId,
    user_id: UserId,
    channel: Channel,
    attempt: u8,
    status: AttemptStatus,
    error_category: Option<ErrorCategory>,
    attempted_at: DateTime<Utc>,
}

impl DeliveryAttempt {
    fn build(
        id: DeliveryAttemptId,
        alert_id: AlertId,
        user_id: UserId,
        channel: Channel,
        attempt: u8,
        status: AttemptStatus,
        error_category: Option<ErrorCategory>,
        attempted_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if attempt == 0 || attempt > MAX_ATTEMPTS {
            return Err(DomainError::Invalid {
                field: "attempt".to_string(),
                reason: format!("must be between 1 and {MAX_ATTEMPTS}"),
            });
        }
        if status == AttemptStatus::Sent && error_category.is_some() {
            return Err(DomainError::Invalid {
                field: "errorCategory".to_string(),
                reason: "a sent attempt carries no error category".to_string(),
            });
        }
        if status != AttemptStatus::Sent && error_category.is_none() {
            return Err(DomainError::Invalid {
                field: "errorCategory".to_string(),
                reason: "a failed attempt names its category".to_string(),
            });
        }
        Ok(Self {
            id,
            alert_id,
            user_id,
            channel,
            attempt,
            status,
            error_category,
            attempted_at,
        })
    }

    /// A delivered attempt.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] when the attempt number is out of range.
    pub fn sent(
        id: DeliveryAttemptId,
        alert_id: AlertId,
        user_id: UserId,
        channel: Channel,
        attempt: u8,
        attempted_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        Self::build(
            id,
            alert_id,
            user_id,
            channel,
            attempt,
            AttemptStatus::Sent,
            None,
            attempted_at,
        )
    }

    /// A failed, still-retryable attempt.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] when the number is out of range or the
    /// category is absent.
    pub fn failed(
        id: DeliveryAttemptId,
        alert_id: AlertId,
        user_id: UserId,
        channel: Channel,
        attempt: u8,
        error_category: Option<ErrorCategory>,
        attempted_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        Self::build(
            id,
            alert_id,
            user_id,
            channel,
            attempt,
            AttemptStatus::Failed,
            error_category,
            attempted_at,
        )
    }

    /// An abandoned attempt: the terminal failure for a channel.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] when the attempt number is out of range.
    pub fn abandoned(
        id: DeliveryAttemptId,
        alert_id: AlertId,
        user_id: UserId,
        channel: Channel,
        attempt: u8,
        error_category: ErrorCategory,
        attempted_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        Self::build(
            id,
            alert_id,
            user_id,
            channel,
            attempt,
            AttemptStatus::Abandoned,
            Some(error_category),
            attempted_at,
        )
    }

    /// Rebuilds a stored row.
    ///
    /// # Errors
    /// Returns [`DomainError`] when a stored label or bound no longer holds.
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: DeliveryAttemptId,
        alert_id: AlertId,
        user_id: UserId,
        channel: Channel,
        attempt: u8,
        status: &str,
        error_category: Option<ErrorCategory>,
        attempted_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let status = AttemptStatus::parse(status)?;
        Self::build(
            id,
            alert_id,
            user_id,
            channel,
            attempt,
            status,
            error_category,
            attempted_at,
        )
    }

    /// The stored identifier.
    #[must_use]
    pub fn id(&self) -> DeliveryAttemptId {
        self.id
    }

    /// The alert this attempt belongs to.
    #[must_use]
    pub fn alert_id(&self) -> AlertId {
        self.alert_id
    }

    /// The receiving user.
    #[must_use]
    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    /// The channel.
    #[must_use]
    pub fn channel(&self) -> Channel {
        self.channel
    }

    /// The one-based attempt number.
    #[must_use]
    pub fn attempt(&self) -> u8 {
        self.attempt
    }

    /// The outcome.
    #[must_use]
    pub fn status(&self) -> AttemptStatus {
        self.status
    }

    /// Why it failed, when it did.
    #[must_use]
    pub fn error_category(&self) -> Option<ErrorCategory> {
        self.error_category
    }

    /// When the try happened.
    #[must_use]
    pub fn attempted_at(&self) -> DateTime<Utc> {
        self.attempted_at
    }
}

/// What one channel has done so far.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChannelOutcome {
    /// How many attempts the channel has recorded.
    pub attempts: u8,
    /// Whether a delivery succeeded.
    pub sent: bool,
    /// Whether the channel was abandoned.
    pub abandoned: bool,
}

impl ChannelOutcome {
    /// Whether the channel has reached a terminal state.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        self.sent || self.abandoned
    }
}

/// Summarises an alert's attempts per channel.
#[must_use]
pub fn summarise(attempts: &[DeliveryAttempt]) -> BTreeMap<Channel, ChannelOutcome> {
    let mut summary: BTreeMap<Channel, ChannelOutcome> = BTreeMap::new();
    for attempt in attempts {
        let entry = summary.entry(attempt.channel).or_default();
        entry.attempts = entry.attempts.saturating_add(1);
        match attempt.status {
            AttemptStatus::Sent => entry.sent = true,
            AttemptStatus::Abandoned => entry.abandoned = true,
            AttemptStatus::Failed => {}
        }
    }
    summary
}

/// BR4.1: when the next attempt is due, or `None` when the channel is done.
#[must_use]
pub fn next_attempt_at(attempt: u8, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let index = usize::from(attempt).checked_sub(1)?;
    RETRY_DELAY_SECONDS
        .get(index)
        .map(|seconds| now + Duration::seconds(*seconds))
}

/// Derives the alert's status from its enabled channels and their outcomes.
#[must_use]
pub fn alert_status(
    enabled: &[Channel],
    outcomes: &BTreeMap<Channel, ChannelOutcome>,
) -> AlertStatus {
    if enabled.is_empty() {
        return AlertStatus::Delivered;
    }
    let mut sent = 0usize;
    let mut abandoned = 0usize;
    for channel in enabled {
        let Some(outcome) = outcomes.get(channel) else {
            return AlertStatus::Queued;
        };
        if outcome.sent {
            sent += 1;
        } else if outcome.abandoned {
            abandoned += 1;
        } else {
            return AlertStatus::Queued;
        }
    }
    if abandoned == 0 {
        AlertStatus::Delivered
    } else if sent == 0 {
        AlertStatus::Failed
    } else {
        AlertStatus::PartiallyDelivered
    }
}
