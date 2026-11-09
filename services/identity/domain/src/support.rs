//! Time-bound, second-approved support access grants (BR4.1–BR4.4).

use chrono::{DateTime, Duration, Utc};

use crate::claims::Role;
use crate::error::DomainError;
pub use crate::ids::{SupportGrantId, UserId};

/// The default and maximum grant window.
pub const DEFAULT_GRANT_MINUTES: i64 = 60;
/// The maximum grant window in minutes.
pub const MAX_GRANT_MINUTES: i64 = 60;
/// The longest reason the grant stores.
pub const MAX_REASON_LEN: usize = 500;

/// The grant lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupportGrantStatus {
    /// Waiting for a distinct admin.
    PendingApproval,
    /// Approved and inside its window.
    Active,
    /// The window passed.
    Expired,
    /// An admin ended it early.
    Revoked,
}

impl SupportGrantStatus {
    /// Parses a stored value.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for anything else.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "pending_approval" => Ok(Self::PendingApproval),
            "active" => Ok(Self::Active),
            "expired" => Ok(Self::Expired),
            "revoked" => Ok(Self::Revoked),
            _ => Err(DomainError::Invalid {
                field: "status",
                reason: "must be pending_approval, active, expired, or revoked".to_string(),
            }),
        }
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PendingApproval => "pending_approval",
            Self::Active => "active",
            Self::Expired => "expired",
            Self::Revoked => "revoked",
        }
    }

    /// Whether the state is terminal.
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Expired | Self::Revoked)
    }
}

/// A support user's reason-recorded, admin-approved read grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportAccessGrant {
    id: SupportGrantId,
    user_id: UserId,
    support_agent_id: UserId,
    reason: String,
    status: SupportGrantStatus,
    requested_at: DateTime<Utc>,
    approved_by: Option<UserId>,
    approved_at: Option<DateTime<Utc>>,
    expires_at: Option<DateTime<Utc>>,
    revoked_at: Option<DateTime<Utc>>,
}

impl SupportAccessGrant {
    /// Requests a grant (WF7 step 2). The reason must carry information.
    ///
    /// # Errors
    /// Returns [`DomainError`] when the reason is blank or too long.
    pub fn request(
        id: SupportGrantId,
        user_id: UserId,
        support_agent_id: UserId,
        reason: &str,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let reason = reason.trim();
        if reason.is_empty() {
            return Err(DomainError::Required { field: "reason" });
        }
        if reason.len() > MAX_REASON_LEN {
            return Err(DomainError::TooLarge {
                field: "reason",
                limit: "500 characters",
            });
        }
        Ok(Self {
            id,
            user_id,
            support_agent_id,
            reason: reason.to_string(),
            status: SupportGrantStatus::PendingApproval,
            requested_at: now,
            approved_by: None,
            approved_at: None,
            expires_at: None,
            revoked_at: None,
        })
    }

    /// Approves a pending grant (BR4.1, BR4.2). The approver must be an admin
    /// distinct from the requester, and the window is at most 60 minutes.
    ///
    /// # Errors
    /// Returns [`DomainError::Denied`] for a non-admin or self-approval and
    /// [`DomainError::Invalid`] for a window outside the allowed bound.
    pub fn approve(
        &mut self,
        approver_id: UserId,
        approver_role: Role,
        now: DateTime<Utc>,
        window: Duration,
    ) -> Result<(), DomainError> {
        if self.status != SupportGrantStatus::PendingApproval {
            return Err(DomainError::InvalidTransition {
                entity: "support grant",
                from: self.status.as_str(),
                to: "active",
            });
        }
        if !approver_role.is_admin() {
            return Err(DomainError::Denied {
                reason: "only an admin may approve a support grant".to_string(),
            });
        }
        if approver_id == self.support_agent_id {
            return Err(DomainError::Denied {
                reason: "the requester cannot approve their own grant".to_string(),
            });
        }
        let minutes = window.num_minutes();
        if minutes <= 0 || minutes > MAX_GRANT_MINUTES {
            return Err(DomainError::Invalid {
                field: "expiresAt",
                reason: "the grant window must be between 1 and 60 minutes".to_string(),
            });
        }
        self.approved_by = Some(approver_id);
        self.approved_at = Some(now);
        self.expires_at = Some(now + window);
        self.status = SupportGrantStatus::Active;
        Ok(())
    }

    /// Revokes a pending or active grant.
    ///
    /// # Errors
    /// Returns [`DomainError::InvalidTransition`] for a terminal grant.
    pub fn revoke(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        if self.status.is_terminal() {
            return Err(DomainError::InvalidTransition {
                entity: "support grant",
                from: self.status.as_str(),
                to: "revoked",
            });
        }
        self.status = SupportGrantStatus::Revoked;
        self.revoked_at = Some(now);
        Ok(())
    }

    /// The state as of `now`, without writing: an active grant whose window
    /// elapsed reads as expired (BR4.2).
    #[must_use]
    pub fn effective_status(&self, now: DateTime<Utc>) -> SupportGrantStatus {
        match (self.status, self.expires_at) {
            (SupportGrantStatus::Active, Some(expires_at)) if now >= expires_at => {
                SupportGrantStatus::Expired
            }
            (status, _) => status,
        }
    }

    /// Persists the expiry when the window has elapsed.
    pub fn expire_if_due(&mut self, now: DateTime<Utc>) -> bool {
        if self.effective_status(now) == SupportGrantStatus::Expired
            && self.status != SupportGrantStatus::Expired
        {
            self.status = SupportGrantStatus::Expired;
            return true;
        }
        false
    }

    /// Whether the grant authorises a read of `user_id`'s metadata at `now`.
    #[must_use]
    pub fn is_valid_at(&self, now: DateTime<Utc>, user_id: UserId) -> bool {
        self.user_id == user_id && self.effective_status(now) == SupportGrantStatus::Active
    }

    /// Rebuilds a grant from stored columns.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: SupportGrantId,
        user_id: UserId,
        support_agent_id: UserId,
        reason: String,
        status: SupportGrantStatus,
        requested_at: DateTime<Utc>,
        approved_by: Option<UserId>,
        approved_at: Option<DateTime<Utc>>,
        expires_at: Option<DateTime<Utc>>,
        revoked_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            user_id,
            support_agent_id,
            reason,
            status,
            requested_at,
            approved_by,
            approved_at,
            expires_at,
            revoked_at,
        }
    }

    /// The grant identifier.
    #[must_use]
    pub fn id(&self) -> SupportGrantId {
        self.id
    }

    /// The user whose metadata the grant covers.
    #[must_use]
    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    /// The requesting support user.
    #[must_use]
    pub fn support_agent_id(&self) -> UserId {
        self.support_agent_id
    }

    /// The recorded reason.
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// The stored state; use [`Self::effective_status`] on a read path.
    #[must_use]
    pub fn status(&self) -> SupportGrantStatus {
        self.status
    }

    /// When the grant was requested.
    #[must_use]
    pub fn requested_at(&self) -> DateTime<Utc> {
        self.requested_at
    }

    /// The distinct approver.
    #[must_use]
    pub fn approved_by(&self) -> Option<UserId> {
        self.approved_by
    }

    /// When the grant was approved.
    #[must_use]
    pub fn approved_at(&self) -> Option<DateTime<Utc>> {
        self.approved_at
    }

    /// When the window ends.
    #[must_use]
    pub fn expires_at(&self) -> Option<DateTime<Utc>> {
        self.expires_at
    }

    /// When the grant was revoked.
    #[must_use]
    pub fn revoked_at(&self) -> Option<DateTime<Utc>> {
        self.revoked_at
    }
}
