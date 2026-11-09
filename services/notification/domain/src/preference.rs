//! Per-type and per-channel preferences, their defaults, and the mandatory
//! email rule (BR2.1–BR2.3, BR3.1).

use chrono::{DateTime, Utc};

use crate::alert::NotificationType;
use crate::error::DomainError;
use crate::ids::{PreferenceId, UserId};

/// One delivery channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Channel {
    /// The stored alert in the user's in-app feed.
    InApp,
    /// The transactional email provider.
    Email,
}

impl Channel {
    /// Both channels.
    pub const ALL: [Self; 2] = [Self::InApp, Self::Email];

    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InApp => "in_app",
            Self::Email => "email",
        }
    }

    /// Parses a stored label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for any other value.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "in_app" => Ok(Self::InApp),
            "email" => Ok(Self::Email),
            _ => Err(DomainError::Invalid {
                field: "channel".to_string(),
                reason: "must be in_app or email".to_string(),
            }),
        }
    }
}

/// BR2.3: the default when no row exists. In-app is on for every type; email is
/// on only for the brief.
#[must_use]
pub fn default_enabled(kind: NotificationType, channel: Channel) -> bool {
    match channel {
        Channel::InApp => true,
        Channel::Email => kind == NotificationType::Brief,
    }
}

/// BR3.1: a security or revoked-key email is mandatory.
#[must_use]
pub fn is_mandatory(kind: NotificationType, channel: Channel) -> bool {
    channel == Channel::Email && kind.is_mandatory_email()
}

/// BR2.1 and BR3.1: the value delivery actually honours.
#[must_use]
pub fn effective_enabled(kind: NotificationType, channel: Channel, stored: Option<bool>) -> bool {
    if is_mandatory(kind, channel) {
        return true;
    }
    stored.unwrap_or_else(|| default_enabled(kind, channel))
}

/// BR2.1, BR3.1, and C9: the value delivery actually honours for one alert. A
/// mandatory email always wins over the stored preference and the requested
/// channel; otherwise the emitter's request narrows the preference.
#[must_use]
pub fn honoured(
    kind: NotificationType,
    channel: Channel,
    stored: Option<bool>,
    requested: crate::alert::RequestedChannel,
) -> bool {
    if is_mandatory(kind, channel) {
        return true;
    }
    requested.allows(channel) && effective_enabled(kind, channel, stored)
}

/// BR2.1: refuses to store a disabled mandatory email.
///
/// # Errors
/// Returns [`DomainError::Mandatory`] naming `<type>.<channel>`.
pub fn validate_change(
    kind: NotificationType,
    channel: Channel,
    enabled: bool,
) -> Result<(), DomainError> {
    if !enabled && is_mandatory(kind, channel) {
        return Err(DomainError::Mandatory {
            field: format!("{}.{}", kind.as_str(), channel.as_str()),
        });
    }
    Ok(())
}

/// One stored preference row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationPreference {
    id: PreferenceId,
    user_id: UserId,
    kind: NotificationType,
    channel: Channel,
    enabled: bool,
    updated_at: DateTime<Utc>,
}

impl NotificationPreference {
    /// Rebuilds a stored row. A stored `false` on a mandatory email is kept as
    /// stored; [`effective_enabled`] still honours the rule.
    #[must_use]
    pub fn stored(
        id: PreferenceId,
        user_id: UserId,
        kind: NotificationType,
        channel: Channel,
        enabled: bool,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            user_id,
            kind,
            channel,
            enabled,
            updated_at,
        }
    }

    /// Builds a row for a validated change.
    ///
    /// # Errors
    /// Returns [`DomainError::Mandatory`] when the change would disable a
    /// mandatory email.
    pub fn change(
        id: PreferenceId,
        user_id: UserId,
        kind: NotificationType,
        channel: Channel,
        enabled: bool,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        validate_change(kind, channel, enabled)?;
        Ok(Self::stored(
            id, user_id, kind, channel, enabled, updated_at,
        ))
    }

    /// The stored identifier.
    #[must_use]
    pub fn id(&self) -> PreferenceId {
        self.id
    }

    /// The owning user.
    #[must_use]
    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    /// The notification type.
    #[must_use]
    pub fn kind(&self) -> NotificationType {
        self.kind
    }

    /// The channel.
    #[must_use]
    pub fn channel(&self) -> Channel {
        self.channel
    }

    /// The stored on/off value.
    #[must_use]
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// When the row last changed.
    #[must_use]
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

/// BR2.3: one type and channel as the settings screen shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedPreference {
    /// The notification type.
    pub kind: NotificationType,
    /// The channel.
    pub channel: Channel,
    /// The value delivery honours.
    pub enabled: bool,
    /// Whether the channel ignores the stored value (BR3.1).
    pub mandatory: bool,
}

/// BR2.3: every type and channel, with stored values applied over the defaults.
#[must_use]
pub fn resolve_all(_user_id: UserId, stored: &[NotificationPreference]) -> Vec<ResolvedPreference> {
    let mut resolved = Vec::with_capacity(NotificationType::ALL.len() * Channel::ALL.len());
    for kind in NotificationType::ALL {
        for channel in Channel::ALL {
            let row = stored
                .iter()
                .find(|preference| preference.kind == kind && preference.channel == channel);
            resolved.push(ResolvedPreference {
                kind,
                channel,
                enabled: effective_enabled(kind, channel, row.map(|row| row.enabled)),
                mandatory: is_mandatory(kind, channel),
            });
        }
    }
    resolved
}
