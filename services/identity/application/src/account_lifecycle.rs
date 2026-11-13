//! US4.1–US4.5 — the account status machine's use cases.
//!
//! An admin disables or restores with a recorded reason (BR5.2); the owner opens
//! a deletion request and a 30-day recovery window (BR5.3); the owner restores
//! inside the window and gets `410 Gone` after it (BR5.4). Every accepted change
//! writes the status, the append-only transition, and its audit intent in one
//! transaction through [`StatusTransitionStore`] (BR5.9).

use std::sync::Arc;

use chrono::{DateTime, Utc};
use identity_domain::{
    recoverable_until, require_edge, validate_reason, DeletionRequest, DeletionRequestId,
    StatusTransition, UserId, UserStatus,
};

use crate::common::{AppError, Caller};
use crate::ports::{Clock, DeletionRepository, StatusChange, StatusTransitionStore, WakeTransport};

/// The metrics surface the lifecycle emits. Implemented by the HTTP layer so the
/// application crate stays free of a metrics dependency.
pub trait AccountObservation: Send + Sync {
    /// Records one accepted transition.
    fn record_transition(&self, from: UserStatus, to: UserStatus, actor_kind: &str);
    /// Records one refused transition, by reason.
    fn record_refusal(&self, reason: &str);
    /// Records one deletion event: `requested`, `cancelled`, or `erased`.
    fn record_deletion_event(&self, event: &str);
    /// Records one failed erasure step, by step name.
    fn record_erasure_failure(&self, step: &str);
}

/// A sink that collects nothing, for callers that do not need metrics.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoAccountObservation;

impl AccountObservation for NoAccountObservation {
    fn record_transition(&self, _from: UserStatus, _to: UserStatus, _actor_kind: &str) {}
    fn record_refusal(&self, _reason: &str) {}
    fn record_deletion_event(&self, _event: &str) {}
    fn record_erasure_failure(&self, _step: &str) {}
}

/// The answer to a disable or restore.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountStatusView {
    user_id: UserId,
    status: UserStatus,
    transition: StatusTransition,
}

impl AccountStatusView {
    /// The account the change concerns.
    #[must_use]
    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    /// The account's status after the change.
    #[must_use]
    pub fn status(&self) -> UserStatus {
        self.status
    }

    /// The recorded transition.
    #[must_use]
    pub fn transition(&self) -> &StatusTransition {
        &self.transition
    }
}

/// The account lifecycle use cases.
pub struct AccountLifecycleService {
    users: Arc<dyn crate::ports::UserRepository>,
    transitions: Arc<dyn StatusTransitionStore>,
    deletions: Arc<dyn DeletionRepository>,
    wake: Arc<dyn WakeTransport>,
    observation: Arc<dyn AccountObservation>,
    clock: Arc<dyn Clock>,
}

impl AccountLifecycleService {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        users: Arc<dyn crate::ports::UserRepository>,
        transitions: Arc<dyn StatusTransitionStore>,
        deletions: Arc<dyn DeletionRepository>,
        wake: Arc<dyn WakeTransport>,
        observation: Arc<dyn AccountObservation>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            users,
            transitions,
            deletions,
            wake,
            observation,
            clock,
        }
    }

    /// W1: an admin disables an active account with a recorded reason.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for a non-admin, [`AppError::Validation`]
    /// for a missing or over-long reason, [`AppError::NotFound`] for an unknown
    /// account, and [`AppError::Conflict`] for an edge outside BR5.1.
    pub async fn disable(
        &self,
        caller: &Caller,
        target: UserId,
        reason: &str,
    ) -> Result<AccountStatusView, AppError> {
        self.transition_by_admin(caller, target, UserStatus::Disabled, reason)
            .await
    }

    /// W1: an admin restores a disabled account with a recorded reason.
    ///
    /// # Errors
    /// As [`Self::disable`].
    pub async fn restore(
        &self,
        caller: &Caller,
        target: UserId,
        reason: &str,
    ) -> Result<AccountStatusView, AppError> {
        self.transition_by_admin(caller, target, UserStatus::Active, reason)
            .await
    }

    async fn transition_by_admin(
        &self,
        caller: &Caller,
        target: UserId,
        to: UserStatus,
        reason: &str,
    ) -> Result<AccountStatusView, AppError> {
        if !caller.is_admin() {
            self.observation.record_refusal("not_admin");
            return Err(AppError::Forbidden(
                "only an administrator may disable or restore an account".to_string(),
            ));
        }
        let reason = validate_reason(reason).map_err(|error| {
            self.observation.record_refusal(error.field());
            AppError::from_domain(error)
        })?;
        let user = self
            .users
            .find_by_id(target)
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("user"))?;
        let change = StatusChange {
            user_id: target,
            from: user.status(),
            to,
            actor: caller.user_id().to_string(),
            actor_kind: "admin",
            reason: Some(reason),
            occurred_at: self.clock.now(),
        };
        let transition = self.apply(&change, "admin").await?;
        Ok(AccountStatusView {
            user_id: target,
            status: to,
            transition,
        })
    }

    /// W2: the owner opens a deletion request and the 30-day window.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for another user's account,
    /// [`AppError::Conflict`] when a request is already open or the status
    /// cannot reach `pending_deletion`, and [`AppError::Unavailable`] when the
    /// request cannot be written.
    pub async fn request_deletion(&self, caller: &Caller) -> Result<DeletionRequest, AppError> {
        let now = self.clock.now();
        if self
            .deletions
            .find_active_for_user(caller.user_id())
            .await
            .map_err(AppError::from_repository)?
            .is_some()
        {
            self.observation.record_refusal("already_open");
            return Err(AppError::Conflict(
                "a deletion request is already open".to_string(),
            ));
        }
        let user = self
            .users
            .find_by_id(caller.user_id())
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("user"))?;
        if let Err(error) = require_edge(user.status(), UserStatus::PendingDeletion) {
            self.observation.record_refusal("illegal_edge");
            return Err(AppError::Conflict(format!(
                "an account that is {} cannot request deletion: {error}",
                user.status().as_str()
            )));
        }

        let request = DeletionRequest::request(DeletionRequestId::generate(), user.id(), now);
        let change = StatusChange {
            user_id: user.id(),
            from: user.status(),
            to: UserStatus::PendingDeletion,
            actor: caller.user_id().to_string(),
            actor_kind: "owner",
            reason: None,
            occurred_at: now,
        };
        self.apply_open(&change, &request).await?;
        self.observation.record_deletion_event("requested");
        if let Err(error) = self.wake.wake().await {
            tracing::warn!(%error, "deletion wake failed; the sweep will pick the request up");
        }
        Ok(request)
    }

    /// The owner's open deletion request (U7 — the recovery surface's read).
    ///
    /// The open request is the one whose status is neither cancelled nor
    /// erased. Read-only: the read neither opens a request nor moves the
    /// account, and it writes no audit event.
    ///
    /// # Errors
    /// Returns [`AppError::NotFound`] when no request is open — another tab may
    /// have restored the account or the erasure may have completed — and
    /// [`AppError::Unavailable`] when the store cannot answer.
    pub async fn open_deletion(&self, caller: &Caller) -> Result<DeletionRequest, AppError> {
        self.deletions
            .find_active_for_user(caller.user_id())
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("deletion"))
    }

    /// W3: the owner restores inside the window.
    ///
    /// # Errors
    /// Returns [`AppError::Gone`] when the window has closed,
    /// [`AppError::Conflict`] when no request is open or the account is not
    /// `pending_deletion`, and [`AppError::Unavailable`] when the store cannot
    /// answer.
    pub async fn cancel_deletion(&self, caller: &Caller) -> Result<DeletionRequest, AppError> {
        let now = self.clock.now();
        let mut request = self
            .deletions
            .find_active_for_user(caller.user_id())
            .await
            .map_err(AppError::from_repository)?
            .ok_or_else(|| AppError::Conflict("no deletion request is open".to_string()))?;
        if now >= request.recoverable_until() {
            self.observation.record_refusal("window_closed");
            return Err(AppError::Gone(format!(
                "the recovery window closed at {} and the account is due for erasure",
                request.recoverable_until().to_rfc3339()
            )));
        }
        let user = self
            .users
            .find_by_id(caller.user_id())
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("user"))?;
        request.cancel(now).map_err(AppError::from_domain)?;
        let change = StatusChange {
            user_id: user.id(),
            from: user.status(),
            to: UserStatus::Active,
            actor: caller.user_id().to_string(),
            actor_kind: "owner",
            reason: None,
            occurred_at: now,
        };
        self.apply_cancel(&change, &request).await?;
        self.observation.record_deletion_event("cancelled");
        Ok(request)
    }

    /// The recovery deadline the API reports as `recoveryEndsAt`.
    #[must_use]
    pub fn recovery_deadline(&self, requested_at: DateTime<Utc>) -> DateTime<Utc> {
        recoverable_until(requested_at)
    }

    async fn apply(
        &self,
        change: &StatusChange,
        actor_kind: &str,
    ) -> Result<StatusTransition, AppError> {
        match self.transitions.apply(change).await {
            Ok(transition) => {
                self.observation
                    .record_transition(change.from, change.to, actor_kind);
                Ok(transition)
            }
            Err(error) => {
                let refusal = refusal_reason(&error);
                self.observation.record_refusal(refusal);
                Err(AppError::from_repository(error))
            }
        }
    }

    async fn apply_open(
        &self,
        change: &StatusChange,
        request: &DeletionRequest,
    ) -> Result<StatusTransition, AppError> {
        match self.transitions.open_deletion(change, request).await {
            Ok(transition) => {
                self.observation
                    .record_transition(change.from, change.to, "owner");
                Ok(transition)
            }
            Err(error) => {
                let refusal = refusal_reason(&error);
                self.observation.record_refusal(refusal);
                Err(AppError::from_repository(error))
            }
        }
    }

    async fn apply_cancel(
        &self,
        change: &StatusChange,
        request: &DeletionRequest,
    ) -> Result<StatusTransition, AppError> {
        match self.transitions.cancel_deletion(change, request).await {
            Ok(transition) => {
                self.observation
                    .record_transition(change.from, change.to, "owner");
                Ok(transition)
            }
            Err(error) => {
                let refusal = refusal_reason(&error);
                self.observation.record_refusal(refusal);
                Err(AppError::from_repository(error))
            }
        }
    }
}

/// Maps a store refusal to the refusal label the metric carries.
fn refusal_reason(error: &crate::ports::RepositoryError) -> &'static str {
    match error {
        crate::ports::RepositoryError::Conflict(_) => "conflict",
        crate::ports::RepositoryError::Forbidden(_) => "forbidden",
        crate::ports::RepositoryError::Validation(_) => "validation",
        crate::ports::RepositoryError::InvalidData(_) => "invalid_data",
        crate::ports::RepositoryError::Unavailable(_) => "unavailable",
    }
}
