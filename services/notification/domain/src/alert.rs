//! The notification types, their payload allow-lists, the content guard run
//! over a submitted payload, and the stored alert (BR1.1, BR3.1, BR5.1).

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

use crate::constants::{MAX_IDEMPOTENCY_KEY_LEN, MAX_PAYLOAD_VALUE_LEN};
use crate::content::is_sensitive;
use crate::error::DomainError;
use crate::ids::{AlertId, UserId};

/// One notification type from the closed set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NotificationType {
    /// A portfolio's drift crossed its guardrail (BR1.2, owned by Portfolio).
    Drift,
    /// An account sync failed.
    SyncFailed,
    /// An exchange credential was revoked (mandatory email, BR3.1).
    KeyRevoked,
    /// A security event such as a new-device sign-in (mandatory email, BR3.1).
    Security,
    /// A periodic brief (email follows the user's preference, BR4.3).
    Brief,
}

impl NotificationType {
    /// Every type, in the order the preferences screen shows them.
    pub const ALL: [Self; 5] = [
        Self::Drift,
        Self::SyncFailed,
        Self::KeyRevoked,
        Self::Security,
        Self::Brief,
    ];

    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Drift => "drift",
            Self::SyncFailed => "sync_failed",
            Self::KeyRevoked => "key_revoked",
            Self::Security => "security",
            Self::Brief => "brief",
        }
    }

    /// Parses a stored label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for any other value.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "drift" => Ok(Self::Drift),
            "sync_failed" => Ok(Self::SyncFailed),
            "key_revoked" => Ok(Self::KeyRevoked),
            "security" => Ok(Self::Security),
            "brief" => Ok(Self::Brief),
            _ => Err(DomainError::Invalid {
                field: "type".to_string(),
                reason: "must be drift, sync_failed, key_revoked, security, or brief".to_string(),
            }),
        }
    }

    /// BR3.1: whether email is mandatory for this type.
    #[must_use]
    pub fn is_mandatory_email(self) -> bool {
        matches!(self, Self::KeyRevoked | Self::Security)
    }

    /// BR5.1: the only payload keys this type may store.
    #[must_use]
    pub fn allowed_payload_fields(self) -> &'static [&'static str] {
        match self {
            Self::Drift => &["portfolioId", "assetId", "driftPoints"],
            Self::SyncFailed => &["accountId", "reason"],
            Self::KeyRevoked | Self::Security => &["event", "deviceLabel"],
            Self::Brief => &["briefId", "period", "asOf", "dataStatus", "hasUnpriced"],
        }
    }
}

/// The stored lifecycle of one alert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlertStatus {
    /// Stored and waiting for delivery.
    Queued,
    /// Every enabled channel was sent.
    Delivered,
    /// Some channels sent, at least one abandoned.
    PartiallyDelivered,
    /// Every enabled channel was abandoned.
    Failed,
}

impl AlertStatus {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Delivered => "delivered",
            Self::PartiallyDelivered => "partially_delivered",
            Self::Failed => "failed",
        }
    }

    /// Parses a stored label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for any other value.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "queued" => Ok(Self::Queued),
            "delivered" => Ok(Self::Delivered),
            "partially_delivered" => Ok(Self::PartiallyDelivered),
            "failed" => Ok(Self::Failed),
            _ => Err(DomainError::Invalid {
                field: "status".to_string(),
                reason: "must be queued, delivered, partially_delivered, or failed".to_string(),
            }),
        }
    }

    /// Whether delivery has finished, one way or the other.
    #[must_use]
    pub fn is_terminal(self) -> bool {
        !matches!(self, Self::Queued)
    }
}

/// A payload after the allow-list and the content guard have run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SanitisedPayload {
    /// The allow-listed, credential-free values.
    pub fields: BTreeMap<String, String>,
    /// Field names dropped because they carried a credential or full address.
    pub dropped: Vec<String>,
}

/// BR5.1: keeps allow-listed keys, drops sensitive values, and records the
/// dropped field names.
#[must_use]
pub fn sanitise_payload(kind: NotificationType, raw: BTreeMap<String, String>) -> SanitisedPayload {
    let allowed = kind.allowed_payload_fields();
    let mut fields = BTreeMap::new();
    let mut dropped = Vec::new();
    for (key, value) in raw {
        if !allowed.contains(&key.as_str()) {
            continue;
        }
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        if value.chars().count() > MAX_PAYLOAD_VALUE_LEN {
            continue;
        }
        if is_sensitive(value) {
            dropped.push(key);
            continue;
        }
        fields.insert(key, value.to_string());
    }
    SanitisedPayload { fields, dropped }
}

/// The channel the emitting service asked for (C9).
///
/// A brief asks for `email` so it does not add a second in-app entry; every
/// other emitter keeps the default `both`. A mandatory email still ignores an
/// in-app-only request (BR3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RequestedChannel {
    /// Write the in-app feed entry and send an email.
    Both,
    /// Only the in-app feed entry.
    InApp,
    /// Only the email.
    Email,
}

impl RequestedChannel {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Both => "both",
            Self::InApp => "in_app",
            Self::Email => "email",
        }
    }

    /// Parses a label. An absent label is `both`.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for any other value.
    pub fn parse(raw: Option<&str>) -> Result<Self, DomainError> {
        match raw.map(str::trim) {
            None | Some("") | Some("both") => Ok(Self::Both),
            Some("in_app") => Ok(Self::InApp),
            Some("email") => Ok(Self::Email),
            Some(_) => Err(DomainError::Invalid {
                field: "channel".to_string(),
                reason: "must be in_app, email, or both".to_string(),
            }),
        }
    }

    /// Whether the emitter asked for this channel.
    #[must_use]
    pub fn allows(self, channel: crate::preference::Channel) -> bool {
        match self {
            Self::Both => true,
            Self::InApp => channel == crate::preference::Channel::InApp,
            Self::Email => channel == crate::preference::Channel::Email,
        }
    }
}

/// One notification event for one user, after dedupe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alert {
    id: AlertId,
    user_id: UserId,
    kind: NotificationType,
    idempotency_key: String,
    payload: BTreeMap<String, String>,
    status: AlertStatus,
    sanitised_fields: Vec<String>,
    created_at: DateTime<Utc>,
    next_attempt_at: Option<DateTime<Utc>>,
    requested: RequestedChannel,
}

impl Alert {
    /// Stores a newly submitted alert, due immediately.
    ///
    /// # Errors
    /// Returns [`DomainError`] when the user reference or the idempotency key
    /// is not well formed.
    pub fn queue(
        id: AlertId,
        user_id: UserId,
        kind: NotificationType,
        idempotency_key: &str,
        payload: SanitisedPayload,
        created_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if user_id.as_uuid().is_nil() {
            return Err(DomainError::Invalid {
                field: "userId".to_string(),
                reason: "must be a real user reference".to_string(),
            });
        }
        let key = idempotency_key.trim();
        if key.is_empty() {
            return Err(DomainError::Required {
                field: "idempotencyKey".to_string(),
            });
        }
        if key.chars().count() > MAX_IDEMPOTENCY_KEY_LEN {
            return Err(DomainError::TooLarge {
                field: "idempotencyKey".to_string(),
                limit: format!("{MAX_IDEMPOTENCY_KEY_LEN} characters"),
            });
        }
        Ok(Self {
            id,
            user_id,
            kind,
            idempotency_key: key.to_string(),
            payload: payload.fields,
            status: AlertStatus::Queued,
            sanitised_fields: payload.dropped,
            created_at,
            next_attempt_at: Some(created_at),
            requested: RequestedChannel::Both,
        })
    }

    /// Rebuilds a stored row.
    ///
    /// # Errors
    /// Returns [`DomainError`] when a stored label or bound no longer holds.
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: AlertId,
        user_id: UserId,
        kind: NotificationType,
        idempotency_key: &str,
        payload: BTreeMap<String, String>,
        status: &str,
        sanitised_fields: Vec<String>,
        created_at: DateTime<Utc>,
        next_attempt_at: Option<DateTime<Utc>>,
    ) -> Result<Self, DomainError> {
        if user_id.as_uuid().is_nil() {
            return Err(DomainError::Invalid {
                field: "userId".to_string(),
                reason: "must be a real user reference".to_string(),
            });
        }
        let key = idempotency_key.trim();
        if key.is_empty() {
            return Err(DomainError::Required {
                field: "idempotencyKey".to_string(),
            });
        }
        Ok(Self {
            id,
            user_id,
            kind,
            idempotency_key: key.to_string(),
            payload,
            status: AlertStatus::parse(status)?,
            sanitised_fields,
            created_at,
            next_attempt_at,
            requested: RequestedChannel::Both,
        })
    }

    /// Rebuilds a stored row that carries its requested channel (C9).
    ///
    /// # Errors
    /// Returns [`DomainError`] when the row or the channel label is invalid.
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored_with_channel(
        id: AlertId,
        user_id: UserId,
        kind: NotificationType,
        idempotency_key: &str,
        payload: BTreeMap<String, String>,
        status: &str,
        sanitised_fields: Vec<String>,
        created_at: DateTime<Utc>,
        next_attempt_at: Option<DateTime<Utc>>,
        requested_channel: &str,
    ) -> Result<Self, DomainError> {
        let requested = RequestedChannel::parse(Some(requested_channel))?;
        Ok(Self::from_stored(
            id,
            user_id,
            kind,
            idempotency_key,
            payload,
            status,
            sanitised_fields,
            created_at,
            next_attempt_at,
        )?
        .with_requested_channel(requested))
    }

    /// Records the channel the emitter asked for.
    #[must_use]
    pub fn with_requested_channel(mut self, channel: RequestedChannel) -> Self {
        self.requested = channel;
        self
    }

    /// The channel the emitter asked for.
    #[must_use]
    pub fn requested_channel(&self) -> RequestedChannel {
        self.requested
    }

    /// The stored identifier.
    #[must_use]
    pub fn id(&self) -> AlertId {
        self.id
    }

    /// The receiving user.
    #[must_use]
    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    /// The notification type.
    #[must_use]
    pub fn kind(&self) -> NotificationType {
        self.kind
    }

    /// The dedupe key (BR1.1).
    #[must_use]
    pub fn idempotency_key(&self) -> &str {
        &self.idempotency_key
    }

    /// The allow-listed, credential-free payload.
    #[must_use]
    pub fn payload(&self) -> &BTreeMap<String, String> {
        &self.payload
    }

    /// The lifecycle state.
    #[must_use]
    pub fn status(&self) -> AlertStatus {
        self.status
    }

    /// Field names dropped by the content guard.
    #[must_use]
    pub fn sanitised_fields(&self) -> &[String] {
        &self.sanitised_fields
    }

    /// When the alert was stored.
    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// When the next delivery attempt is due.
    #[must_use]
    pub fn next_attempt_at(&self) -> Option<DateTime<Utc>> {
        self.next_attempt_at
    }
}
