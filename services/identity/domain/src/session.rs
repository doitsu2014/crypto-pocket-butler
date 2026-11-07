//! Identity's own device-session view (BR8.1, BR8.2). It stores no token,
//! factor, or full IP.

use chrono::{DateTime, Utc};

use crate::ids::{DeviceSessionId, UserId};

/// One device session row, updated on each authenticated request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceSession {
    id: DeviceSessionId,
    user_id: UserId,
    auth_session_id: String,
    device: Option<String>,
    location: Option<String>,
    last_activity_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
}

impl DeviceSession {
    /// Creates a row for a session Identity has not seen before.
    #[must_use]
    pub fn new(
        id: DeviceSessionId,
        user_id: UserId,
        auth_session_id: impl Into<String>,
        device: Option<String>,
        location: Option<String>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            user_id,
            auth_session_id: auth_session_id.into(),
            device,
            location,
            last_activity_at: now,
            revoked_at: None,
        }
    }

    /// Rebuilds a row from stored columns.
    #[must_use]
    pub fn from_stored(
        id: DeviceSessionId,
        user_id: UserId,
        auth_session_id: String,
        device: Option<String>,
        location: Option<String>,
        last_activity_at: DateTime<Utc>,
        revoked_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            user_id,
            auth_session_id,
            device,
            location,
            last_activity_at,
            revoked_at,
        }
    }

    /// Applies the latest activity. A missing location clears it, because the
    /// coarse location is not retained once it cannot be supplied (BR8.1).
    pub fn touch(&mut self, device: Option<String>, location: Option<String>, now: DateTime<Utc>) {
        if device.is_some() {
            self.device = device;
        }
        self.location = location;
        self.last_activity_at = now;
    }

    /// Marks the row revoked after Supabase Auth ended the session.
    pub fn revoke(&mut self, now: DateTime<Utc>) {
        self.revoked_at = Some(now);
    }

    /// The row identifier.
    #[must_use]
    pub fn id(&self) -> DeviceSessionId {
        self.id
    }

    /// The owner.
    #[must_use]
    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    /// The Supabase Auth session identifier.
    #[must_use]
    pub fn auth_session_id(&self) -> &str {
        &self.auth_session_id
    }

    /// A short client label.
    #[must_use]
    pub fn device(&self) -> Option<&str> {
        self.device.as_deref()
    }

    /// Coarse city/country, never the full IP.
    #[must_use]
    pub fn location(&self) -> Option<&str> {
        self.location.as_deref()
    }

    /// The last observed activity.
    #[must_use]
    pub fn last_activity_at(&self) -> DateTime<Utc> {
        self.last_activity_at
    }

    /// When the session was revoked.
    #[must_use]
    pub fn revoked_at(&self) -> Option<DateTime<Utc>> {
        self.revoked_at
    }

    /// Whether the session is already revoked.
    #[must_use]
    pub fn is_revoked(&self) -> bool {
        self.revoked_at.is_some()
    }
}
