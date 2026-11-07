//! Time-bound, second-approved support sessions (BR3.1–BR3.3).
//!
//! The audit service is the single owner of the support-session record. Expiry
//! needs no writer: any read compares now against `expires_at`, so a session
//! cannot outlive its window because nothing ran.

use std::fmt;

use chrono::{DateTime, Duration, Utc};

use crate::error::DomainError;
use crate::ids::{SupportSessionId, UserId};

/// The shortest reason that carries information rather than a placeholder.
pub const MIN_REASON_LEN: usize = 10;
/// The longest reason the record stores.
pub const MAX_REASON_LEN: usize = 500;
/// The longest a session may stay open, in hours.
pub const MAX_SESSION_HOURS: i64 = 8;

/// What a support session may read. Portfolio contents and credentials are never
/// in scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupportScope {
    /// Account metadata.
    AccountMetadata,
    /// Support tickets.
    Tickets,
}

impl SupportScope {
    /// Parses the stored value.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for anything else.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "account_metadata" => Ok(Self::AccountMetadata),
            "tickets" => Ok(Self::Tickets),
            _ => Err(DomainError::Invalid {
                field: "scope",
                reason: "must be account_metadata or tickets".to_string(),
            }),
        }
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AccountMetadata => "account_metadata",
            Self::Tickets => "tickets",
        }
    }
}

/// The lifecycle state of a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupportSessionState {
    /// A second approver has not answered.
    Requested,
    /// Approved and inside its window.
    Active,
    /// The window passed.
    Expired,
    /// An admin ended it early.
    Revoked,
    /// Approval refused.
    Denied,
}

impl SupportSessionState {
    /// Parses the stored value.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for anything else.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "requested" => Ok(Self::Requested),
            "active" => Ok(Self::Active),
            "expired" => Ok(Self::Expired),
            "revoked" => Ok(Self::Revoked),
            "denied" => Ok(Self::Denied),
            _ => Err(DomainError::Invalid {
                field: "state",
                reason: "unknown support-session state".to_string(),
            }),
        }
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Requested => "requested",
            Self::Active => "active",
            Self::Expired => "expired",
            Self::Revoked => "revoked",
            Self::Denied => "denied",
        }
    }

    /// Whether the state is terminal.
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Expired | Self::Revoked | Self::Denied)
    }
}

/// The outcome recorded when a session reaches a terminal state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupportSessionOutcome {
    /// Approved.
    Approved,
    /// Refused.
    Denied,
    /// Ended early by an admin.
    Revoked,
    /// The window passed.
    Expired,
}

impl SupportSessionOutcome {
    /// Parses the stored value.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for anything else.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "approved" => Ok(Self::Approved),
            "denied" => Ok(Self::Denied),
            "revoked" => Ok(Self::Revoked),
            "expired" => Ok(Self::Expired),
            _ => Err(DomainError::Invalid {
                field: "outcome",
                reason: "unknown support-session outcome".to_string(),
            }),
        }
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::Denied => "denied",
            Self::Revoked => "revoked",
            Self::Expired => "expired",
        }
    }
}

/// Why a support session was refused (BR3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SupportDenial {
    /// The reason is shorter than ten characters.
    #[error("reason must be at least 10 characters")]
    ReasonTooShort,
    /// The approver is the agent.
    #[error("secondApproverId must differ from supportAgentId")]
    ApproverIsAgent,
    /// The expiry is not after the start.
    #[error("expiresAt must be later than startedAt")]
    ExpiryNotInFuture,
    /// The expiry is more than eight hours after the start.
    #[error("expiresAt must be at most eight hours after startedAt")]
    ExpiryBeyondWindow,
}

impl SupportDenial {
    /// A stable rule code recorded on the denial event.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::ReasonTooShort => "BR3.1.reason",
            Self::ApproverIsAgent => "BR3.1.approver",
            Self::ExpiryNotInFuture => "BR3.1.expiry",
            Self::ExpiryBeyondWindow => "BR3.1.window",
        }
    }

    /// The field path the denial concerns.
    #[must_use]
    pub fn field(self) -> &'static str {
        match self {
            Self::ReasonTooShort => "reason",
            Self::ApproverIsAgent => "secondApproverId",
            Self::ExpiryNotInFuture | Self::ExpiryBeyondWindow => "expiresAt",
        }
    }
}

/// The inputs a support agent supplies when asking for access.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportSessionRequest {
    /// The support agent who asked.
    pub support_agent_id: UserId,
    /// The user whose data is in scope.
    pub subject_user_id: UserId,
    /// Why the access is needed.
    pub reason: String,
    /// A different operator who approved the session.
    pub second_approver_id: UserId,
    /// What the session may read.
    pub scope: SupportScope,
    /// When the request was made.
    pub started_at: DateTime<Utc>,
    /// The instant the session stops granting access.
    pub expires_at: DateTime<Utc>,
}

impl SupportSessionRequest {
    /// Builds a request, enforcing only the shapes that make a record storable.
    ///
    /// # Errors
    /// Returns [`DomainError::Required`] for a blank reason and
    /// [`DomainError::TooLarge`] when the reason exceeds 500 characters.
    pub fn new(
        support_agent_id: UserId,
        subject_user_id: UserId,
        reason: &str,
        second_approver_id: UserId,
        scope: SupportScope,
        started_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let reason = reason.trim().to_string();
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
            support_agent_id,
            subject_user_id,
            reason,
            second_approver_id,
            scope,
            started_at,
            expires_at,
        })
    }

    /// BR3.1: checks the reason, the independent approver, and the expiry window.
    ///
    /// # Errors
    /// Returns [`SupportDenial`] naming the rule that refused the request.
    pub fn check(&self) -> Result<(), SupportDenial> {
        if self.reason.chars().count() < MIN_REASON_LEN {
            return Err(SupportDenial::ReasonTooShort);
        }
        if self.second_approver_id == self.support_agent_id {
            return Err(SupportDenial::ApproverIsAgent);
        }
        if self.expires_at <= self.started_at {
            return Err(SupportDenial::ExpiryNotInFuture);
        }
        if self.expires_at - self.started_at > Duration::hours(MAX_SESSION_HOURS) {
            return Err(SupportDenial::ExpiryBeyondWindow);
        }
        Ok(())
    }
}

impl fmt::Display for SupportSessionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One stored support session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportSession {
    id: SupportSessionId,
    request: SupportSessionRequest,
    state: SupportSessionState,
    approved_at: Option<DateTime<Utc>>,
    ended_at: Option<DateTime<Utc>>,
    outcome: Option<SupportSessionOutcome>,
}

impl SupportSession {
    /// Records a support agent's request. BR3.1 is checked by the caller before
    /// this point; an invalid request is stored with [`SupportSession::deny`].
    #[must_use]
    pub fn request(request: SupportSessionRequest, id: SupportSessionId) -> Self {
        Self {
            id,
            request,
            state: SupportSessionState::Requested,
            approved_at: None,
            ended_at: None,
            outcome: None,
        }
    }

    /// Records a second operator's approval, moving the request to `active`.
    ///
    /// Only the named second approver may approve, and only while the request is
    /// still `requested` and inside its window. Approval is not a terminal state,
    /// so `outcome` stays absent until the session expires, is revoked, or is
    /// denied (entity model: `outcome` is set only on a terminal state).
    ///
    /// # Errors
    /// Returns [`DomainError::Contradiction`] when `actor` is not the stored
    /// second approver, and [`DomainError::InvalidTransition`] when the session is
    /// no longer requested or has already expired.
    pub fn approve(mut self, actor: UserId, at: DateTime<Utc>) -> Result<Self, DomainError> {
        if actor != self.request.second_approver_id {
            return Err(DomainError::Contradiction {
                field: "secondApproverId",
                reason: "only the named second approver may approve the session".to_string(),
            });
        }
        let current = self.state_at(at);
        if current != SupportSessionState::Requested {
            return Err(DomainError::InvalidTransition {
                entity: "support session",
                from: current.as_str(),
                to: "active",
            });
        }
        if let Err(denial) = self.request.check() {
            return Err(DomainError::Invalid {
                field: denial.field(),
                reason: denial.to_string(),
            });
        }
        self.state = SupportSessionState::Active;
        self.approved_at = Some(at);
        self.outcome = None;
        Ok(self)
    }

    /// Stores a refused request as a `denied` record.
    #[must_use]
    pub fn deny(request: SupportSessionRequest, id: SupportSessionId) -> Self {
        Self {
            id,
            request,
            state: SupportSessionState::Denied,
            approved_at: None,
            ended_at: None,
            outcome: Some(SupportSessionOutcome::Denied),
        }
    }

    /// Ends an active session early.
    ///
    /// # Errors
    /// Returns [`DomainError::InvalidTransition`] when the session is not active
    /// at `at`, because a revoked or expired session is immutable.
    pub fn revoke(mut self, at: DateTime<Utc>) -> Result<Self, DomainError> {
        let current = self.state_at(at);
        if current != SupportSessionState::Active {
            return Err(DomainError::InvalidTransition {
                entity: "support session",
                from: current.as_str(),
                to: "revoked",
            });
        }
        self.state = SupportSessionState::Revoked;
        self.ended_at = Some(at);
        self.outcome = Some(SupportSessionOutcome::Revoked);
        Ok(self)
    }

    /// The state at `now`: a session past its expiry reads as expired even if no
    /// writer ever set that (BR3.2).
    #[must_use]
    pub fn state_at(&self, now: DateTime<Utc>) -> SupportSessionState {
        if matches!(
            self.state,
            SupportSessionState::Active | SupportSessionState::Requested
        ) && now > self.request.expires_at
        {
            return SupportSessionState::Expired;
        }
        self.state
    }

    /// Whether the session grants a read at `now`.
    #[must_use]
    pub fn is_active_at(&self, now: DateTime<Utc>) -> bool {
        self.state_at(now) == SupportSessionState::Active
    }

    /// Whether the session covers `scope`.
    #[must_use]
    pub fn covers(&self, scope: SupportScope) -> bool {
        self.request.scope == scope
    }

    /// The session identifier.
    #[must_use]
    pub fn id(&self) -> SupportSessionId {
        self.id
    }

    /// The support agent.
    #[must_use]
    pub fn support_agent_id(&self) -> UserId {
        self.request.support_agent_id
    }

    /// The user whose data is in scope.
    #[must_use]
    pub fn subject_user_id(&self) -> UserId {
        self.request.subject_user_id
    }

    /// Why the access is needed.
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.request.reason
    }

    /// The independent approver.
    #[must_use]
    pub fn second_approver_id(&self) -> UserId {
        self.request.second_approver_id
    }

    /// What the session may read.
    #[must_use]
    pub fn scope(&self) -> SupportScope {
        self.request.scope
    }

    /// When the request was made.
    #[must_use]
    pub fn started_at(&self) -> DateTime<Utc> {
        self.request.started_at
    }

    /// The instant the session stops granting access.
    #[must_use]
    pub fn expires_at(&self) -> DateTime<Utc> {
        self.request.expires_at
    }

    /// When the second approver agreed.
    #[must_use]
    pub fn approved_at(&self) -> Option<DateTime<Utc>> {
        self.approved_at
    }

    /// When the session ended early.
    #[must_use]
    pub fn ended_at(&self) -> Option<DateTime<Utc>> {
        self.ended_at
    }

    /// The stored state, without the derived expiry.
    #[must_use]
    pub fn stored_state(&self) -> SupportSessionState {
        self.state
    }

    /// The terminal outcome, when there is one.
    #[must_use]
    pub fn outcome(&self) -> Option<SupportSessionOutcome> {
        self.outcome
    }

    /// Rebuilds a session from storage.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: SupportSessionId,
        request: SupportSessionRequest,
        state: SupportSessionState,
        approved_at: Option<DateTime<Utc>>,
        ended_at: Option<DateTime<Utc>>,
        outcome: Option<SupportSessionOutcome>,
    ) -> Self {
        Self {
            id,
            request,
            state,
            approved_at,
            ended_at,
            outcome,
        }
    }
}
