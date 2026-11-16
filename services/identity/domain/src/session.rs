//! Identity's own device-session view (BR8.1, BR8.2).
//!
//! This record stores the first-seen timestamp so the 30-day absolute session
//! cap is anchored to an individual provider session, not to the account's age
//! (FR2.1; AC2.1.3). It stores no token, factor, or full IP.

use chrono::{DateTime, Duration, Utc};

use crate::ids::{DeviceSessionId, UserId};

/// Maximum lifetime of one provider session from its first observation.
pub const ABSOLUTE_SESSION_CAP_DAYS: i64 = 30;

/// One device session row, updated on each authenticated request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceSession {
    id: DeviceSessionId,
    user_id: UserId,
    auth_session_id: String,
    device: Option<String>,
    location: Option<String>,
    first_seen_at: DateTime<Utc>,
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
            first_seen_at: now,
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
        first_seen_at: DateTime<Utc>,
        last_activity_at: DateTime<Utc>,
        revoked_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            user_id,
            auth_session_id,
            device,
            location,
            first_seen_at,
            last_activity_at,
            revoked_at,
        }
    }

    /// Applies the latest activity without resetting the absolute lifetime.
    /// A missing location clears it, because the coarse location is not retained
    /// once it cannot be supplied (BR8.1).
    pub fn touch(&mut self, device: Option<String>, location: Option<String>, now: DateTime<Utc>) {
        if device.is_some() {
            self.device = device;
        }
        self.location = location;
        self.last_activity_at = now;
    }

    /// Whether the provider session remains within its absolute lifetime.
    ///
    /// A session is expired at the exact cap boundary; refresh activity cannot
    /// extend it. A caller that needs a new session must establish a new
    /// provider session with a new session identifier.
    #[must_use]
    pub fn is_within_absolute_cap(&self, now: DateTime<Utc>) -> bool {
        now < self.absolute_expires_at()
    }

    /// Absolute expiry derived from the immutable first-seen timestamp.
    #[must_use]
    pub fn absolute_expires_at(&self) -> DateTime<Utc> {
        self.first_seen_at + Duration::days(ABSOLUTE_SESSION_CAP_DAYS)
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

    /// When Identity first observed this provider session; never reset by touch.
    #[must_use]
    pub fn first_seen_at(&self) -> DateTime<Utc> {
        self.first_seen_at
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

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone, Utc};

    use super::{DeviceSession, ABSOLUTE_SESSION_CAP_DAYS};
    use crate::ids::{DeviceSessionId, UserId};

    fn session() -> DeviceSession {
        DeviceSession::new(
            DeviceSessionId::generate(),
            UserId::generate(),
            "provider-session",
            Some("browser".to_string()),
            None,
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        )
    }

    #[test]
    fn a_new_session_is_within_the_cap_before_expiry() {
        let session = session();
        let before_expiry = session.absolute_expires_at() - Duration::seconds(1);

        assert!(session.is_within_absolute_cap(before_expiry));
        assert_eq!(
            session.first_seen_at(),
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
        );
        assert_eq!(
            session.absolute_expires_at(),
            session.first_seen_at() + Duration::days(30)
        );
    }

    #[test]
    fn the_cap_expires_at_the_exact_boundary() {
        let session = session();

        assert!(!session.is_within_absolute_cap(session.absolute_expires_at()));
        assert!(
            !session.is_within_absolute_cap(session.absolute_expires_at() + Duration::seconds(1))
        );
    }

    #[test]
    fn activity_does_not_reset_the_absolute_expiry() {
        let mut session = session();
        let original_first_seen = session.first_seen_at();
        let original_expiry = session.absolute_expires_at();
        let later_activity = original_first_seen + Duration::days(20);

        session.touch(None, Some("coarse location".to_string()), later_activity);

        assert_eq!(session.first_seen_at(), original_first_seen);
        assert_eq!(session.absolute_expires_at(), original_expiry);
        assert!(session.is_within_absolute_cap(later_activity));
        assert!(!session.is_within_absolute_cap(original_expiry));
    }

    #[test]
    fn the_absolute_cap_is_thirty_days() {
        assert_eq!(ABSOLUTE_SESSION_CAP_DAYS, 30);
    }
}
