//! The account status machine, its append-only transition record, and the
//! recovery-window and erasure-retry arithmetic (US4.1–US4.5, US7.2,
//! BR5.1–BR5.10).
//!
//! U5 owns every status change: `pending → active` (the interim promotion),
//! `active ↔ disabled` (an admin), `active → pending_deletion` (the owner), and
//! `pending_deletion → active | deleted` (the owner restores, or the window
//! closes). No other edge exists, and [`require_edge`] is the one place that
//! decides it.

use chrono::{DateTime, Days, NaiveTime, TimeZone, Utc};

use crate::error::DomainError;
use crate::ids::{StatusTransitionId, UserId};
use crate::user::UserStatus;

/// The longest admin reason the transition record stores.
pub const MAX_REASON_LEN: usize = 500;
/// The longest actor label the transition record stores.
pub const MAX_ACTOR_LEN: usize = 120;
/// The recovery window, in calendar days (FR17.3, BR5.3).
pub const RECOVERY_WINDOW_DAYS: u64 = 30;
/// The attempts after which the sweep parks a stuck erasure (reliability design).
pub const MAX_ERASURE_ATTEMPTS: u32 = 5;
/// The actor recorded for a change the sweep makes, never a service name.
pub const SYSTEM_ACTOR: &str = "system";

/// The six permitted edges (BR5.1). Everything else is refused.
const EDGES: [(UserStatus, UserStatus); 6] = [
    (UserStatus::Pending, UserStatus::Active),
    (UserStatus::Active, UserStatus::Disabled),
    (UserStatus::Active, UserStatus::PendingDeletion),
    (UserStatus::Disabled, UserStatus::Active),
    (UserStatus::PendingDeletion, UserStatus::Active),
    (UserStatus::PendingDeletion, UserStatus::Deleted),
];

/// Whether `from → to` is one of the permitted edges.
#[must_use]
pub fn is_permitted(from: UserStatus, to: UserStatus) -> bool {
    EDGES.contains(&(from, to))
}

/// Refuses anything but a permitted edge (BR5.1).
///
/// # Errors
/// Returns [`DomainError::InvalidTransition`] for any other pair.
pub fn require_edge(from: UserStatus, to: UserStatus) -> Result<(), DomainError> {
    if is_permitted(from, to) {
        Ok(())
    } else {
        Err(DomainError::InvalidTransition {
            entity: "user status",
            from: from.as_str(),
            to: to.as_str(),
        })
    }
}

/// Validates and trims a required admin reason (BR5.2).
///
/// # Errors
/// Returns [`DomainError::Required`] for a blank reason and
/// [`DomainError::TooLarge`] beyond [`MAX_REASON_LEN`].
pub fn validate_reason(raw: &str) -> Result<String, DomainError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(DomainError::Required { field: "reason" });
    }
    if value.chars().count() > MAX_REASON_LEN {
        return Err(DomainError::TooLarge {
            field: "reason",
            limit: "500 characters",
        });
    }
    Ok(value.to_string())
}

/// Validates an optional reason. A blank value is treated as absent; a supplied
/// one obeys the same bound as [`validate_reason`].
///
/// # Errors
/// Returns [`DomainError::TooLarge`] beyond [`MAX_REASON_LEN`].
pub fn optional_reason(raw: Option<&str>) -> Result<Option<String>, DomainError> {
    match raw.map(str::trim) {
        None | Some("") => Ok(None),
        Some(value) if value.chars().count() > MAX_REASON_LEN => Err(DomainError::TooLarge {
            field: "reason",
            limit: "500 characters",
        }),
        Some(value) => Ok(Some(value.to_string())),
    }
}

/// Midnight UTC on the 30th calendar day after `requested_at` (BR5.3).
///
/// A calendar-day deadline is predictable for the person and the sweep, at the
/// cost that some windows are a few hours longer than 30 × 24 hours. That
/// difference is recorded in the unit's entities design.
#[must_use]
pub fn recoverable_until(requested_at: DateTime<Utc>) -> DateTime<Utc> {
    let deadline = requested_at.date_naive() + Days::new(RECOVERY_WINDOW_DAYS);
    Utc.from_utc_datetime(&deadline.and_time(NaiveTime::MIN))
}

/// Minutes to wait after the `attempt_count`-th failure: 1, 2, 4, 8, 16, then
/// capped at one hour (reliability design).
#[must_use]
pub fn backoff_minutes(attempt_count: u32) -> i64 {
    let exponent = attempt_count.saturating_sub(1).min(6);
    (1i64 << exponent).min(60)
}

/// One recorded status change, append-only (BR5.9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusTransition {
    id: StatusTransitionId,
    user_id: UserId,
    from_status: UserStatus,
    to_status: UserStatus,
    actor: String,
    reason: Option<String>,
    occurred_at: DateTime<Utc>,
    seq: Option<i64>,
}

impl StatusTransition {
    /// Creates a transition, validating the edge, the actor, and the reason.
    ///
    /// # Errors
    /// Returns [`DomainError::Required`] for a blank actor,
    /// [`DomainError::TooLarge`] for an over-long actor, and
    /// [`DomainError::InvalidTransition`] for an edge outside BR5.1.
    pub fn new(
        id: StatusTransitionId,
        user_id: UserId,
        from_status: UserStatus,
        to_status: UserStatus,
        actor: &str,
        reason: Option<&str>,
        occurred_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        require_edge(from_status, to_status)?;
        let actor = actor.trim();
        if actor.is_empty() {
            return Err(DomainError::Required { field: "actor" });
        }
        if actor.chars().count() > MAX_ACTOR_LEN {
            return Err(DomainError::TooLarge {
                field: "actor",
                limit: "120 characters",
            });
        }
        Ok(Self {
            id,
            user_id,
            from_status,
            to_status,
            actor: actor.to_string(),
            reason: optional_reason(reason)?,
            occurred_at,
            seq: None,
        })
    }

    /// Rebuilds a stored transition with its database-assigned sequence.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: StatusTransitionId,
        user_id: UserId,
        from_status: UserStatus,
        to_status: UserStatus,
        actor: String,
        reason: Option<String>,
        occurred_at: DateTime<Utc>,
        seq: i64,
    ) -> Self {
        Self {
            id,
            user_id,
            from_status,
            to_status,
            actor,
            reason,
            occurred_at,
            seq: Some(seq),
        }
    }

    /// The event identifier.
    #[must_use]
    pub fn id(&self) -> StatusTransitionId {
        self.id
    }

    /// The account whose status changed.
    #[must_use]
    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    /// The status before the change.
    #[must_use]
    pub fn from_status(&self) -> UserStatus {
        self.from_status
    }

    /// The status after the change.
    #[must_use]
    pub fn to_status(&self) -> UserStatus {
        self.to_status
    }

    /// The actor: an owner id, an admin id, or `system`.
    #[must_use]
    pub fn actor(&self) -> &str {
        &self.actor
    }

    /// The recorded reason, when one was required.
    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }

    /// When the change was recorded.
    #[must_use]
    pub fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }

    /// The monotonic sequence the database assigns, once stored.
    #[must_use]
    pub fn seq(&self) -> Option<i64> {
        self.seq
    }
}
