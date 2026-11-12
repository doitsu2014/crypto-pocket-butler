//! Ports the decision and the role change depend on, implemented outward.
//!
//! Nothing here names sqlx, reqwest, or Axum: the adapters live in
//! `identity-infrastructure` and the metrics adapter lives at the HTTP boundary.

use std::collections::BTreeMap;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use identity_domain::{AuditOutcome, DenialReason, OperationClass, Role, UserId, UserStatus};

use crate::ports::{ExternalError, RepositoryError};

/// The longest role-change reason the use case accepts.
pub const MAX_REASON_LEN: usize = 500;

/// Why a status read did not answer.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StatusReadError {
    /// The pool acquire or the statement exceeded its bound.
    #[error("the status read timed out")]
    Timeout,
    /// The source could not answer.
    #[error("the status source is unavailable: {0}")]
    Unavailable(String),
    /// The stored value is not a status.
    #[error("the stored status is malformed: {0}")]
    Malformed(String),
}

impl StatusReadError {
    /// The metric cause this error reports.
    #[must_use]
    pub fn cause(&self) -> StatusFailure {
        match self {
            Self::Timeout => StatusFailure::Timeout,
            Self::Unavailable(_) => StatusFailure::Unavailable,
            Self::Malformed(_) => StatusFailure::Malformed,
        }
    }
}

/// The bounded failure causes the status metric carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusFailure {
    /// The read exceeded its bound.
    Timeout,
    /// The source is down.
    Unavailable,
    /// A stored value is not a status; treated as unavailable, never `active`.
    Malformed,
}

impl StatusFailure {
    /// The metric label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Timeout => "timeout",
            Self::Unavailable => "unavailable",
            Self::Malformed => "malformed",
        }
    }
}

/// Reads the caller's status by internal user id.
///
/// The adapter is bounded (200 ms acquire, 100 ms statement) and does not
/// retry; the outage cache sits behind this port and is consulted only after a
/// failed read.
#[async_trait]
pub trait StatusReader: Send + Sync {
    /// Returns the stored status, or `None` when no row carries the id.
    ///
    /// # Errors
    /// Returns [`StatusReadError`] when the read timed out, the source is
    /// unavailable, or the stored value is malformed.
    async fn status_of(&self, user_id: UserId) -> Result<Option<UserStatus>, StatusReadError>;
}

/// One audit event this unit writes. The field set is fixed: there is no
/// free-form `context`, so a credential or a full wallet address has nowhere to
/// go (BR2.12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorisationEvent {
    actor: String,
    action: String,
    target: String,
    outcome: AuditOutcome,
    occurred_at: DateTime<Utc>,
    context: BTreeMap<String, String>,
}

impl AuthorisationEvent {
    /// A denial. The action is the operation name and the target names the
    /// resource kind and the caller's own id — never another user's id
    /// (BR2.12).
    #[must_use]
    pub fn denied(
        actor: impl Into<String>,
        operation: &str,
        target: impl Into<String>,
        occurred_at: DateTime<Utc>,
        reason: DenialReason,
    ) -> Self {
        let mut context = BTreeMap::new();
        context.insert("reason".to_string(), reason.as_str().to_string());
        Self {
            actor: actor.into(),
            action: operation.to_string(),
            target: target.into(),
            outcome: AuditOutcome::Denied,
            occurred_at,
            context,
        }
    }

    /// The intent entry: recorded before the provider write (BR2.10).
    #[must_use]
    pub fn role_change_requested(
        actor: impl Into<String>,
        target: impl Into<String>,
        occurred_at: DateTime<Utc>,
        previous_role: Option<Role>,
        new_role: Role,
        reason: &str,
        outcome: AuditOutcome,
    ) -> Self {
        let mut context = role_context(previous_role, new_role, reason);
        if outcome == AuditOutcome::Denied {
            context.remove("previousRole");
        }
        Self {
            actor: actor.into(),
            action: "identity.role.change.requested".to_string(),
            target: target.into(),
            outcome,
            occurred_at,
            context,
        }
    }

    /// The outcome entry for the same role change (BR2.10).
    #[must_use]
    pub fn role_change_outcome(
        actor: impl Into<String>,
        target: impl Into<String>,
        occurred_at: DateTime<Utc>,
        previous_role: Role,
        new_role: Role,
        reason: &str,
        outcome: AuditOutcome,
        failure_cause: Option<&str>,
    ) -> Self {
        let mut context = role_context(Some(previous_role), new_role, reason);
        if let Some(cause) = failure_cause {
            context.insert("failureCause".to_string(), cause.to_string());
        }
        Self {
            actor: actor.into(),
            action: "identity.role.changed".to_string(),
            target: target.into(),
            outcome,
            occurred_at,
            context,
        }
    }

    /// The acting user id, service name, or `system`.
    #[must_use]
    pub fn actor(&self) -> &str {
        &self.actor
    }

    /// The dotted action name.
    #[must_use]
    pub fn action(&self) -> &str {
        &self.action
    }

    /// The logical target.
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    /// The outcome.
    #[must_use]
    pub fn outcome(&self) -> AuditOutcome {
        self.outcome
    }

    /// When the event happened.
    #[must_use]
    pub fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }

    /// The fixed identifier-only context.
    #[must_use]
    pub fn context(&self) -> &BTreeMap<String, String> {
        &self.context
    }
}

fn role_context(
    previous_role: Option<Role>,
    new_role: Role,
    reason: &str,
) -> BTreeMap<String, String> {
    let mut context = BTreeMap::new();
    if let Some(previous) = previous_role {
        context.insert(
            "previousRole".to_string(),
            previous.provider_value().to_string(),
        );
    }
    context.insert("newRole".to_string(), new_role.provider_value().to_string());
    context.insert("reason".to_string(), reason.to_string());
    context
}

/// Writes one audit event through C9. A failure is surfaced, never swallowed;
/// the decision turns it into `audit_unavailable` (BR2.11).
#[async_trait]
pub trait AuditSink: Send + Sync {
    /// Appends one event.
    ///
    /// # Errors
    /// Returns [`RepositoryError`] when the audit service cannot accept it.
    async fn record(
        &self,
        event: &AuthorisationEvent,
        correlation_id: &str,
    ) -> Result<(), RepositoryError>;
}

/// Which write failed, for the audit-failure metric.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditWriteContext {
    /// The denial write.
    Denial,
    /// The role-change intent or outcome write.
    RoleChange,
}

impl AuditWriteContext {
    /// The metric label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Denial => "denial",
            Self::RoleChange => "role_change",
        }
    }
}

/// Writes `app_metadata.role` for one provider user (BR2.10).
#[async_trait]
pub trait RoleWriter: Send + Sync {
    /// Writes the role claim. It is never retried: a duplicate privileged change
    /// is worse than a surfaced failure.
    ///
    /// # Errors
    /// Returns [`ExternalError`] when the provider refuses or is unreachable.
    async fn set_role(&self, subject: &str, role: Role) -> Result<(), ExternalError>;
}

/// Reads `app_metadata.role` for one provider user (U10 admin lookup).
///
/// One attempt, like the writer: a failed read is surfaced so the boundary can
/// answer `503` rather than show a record with a guessed or stored role.
#[async_trait]
pub trait RoleReader: Send + Sync {
    /// Reads the role claim.
    ///
    /// # Errors
    /// Returns [`ExternalError`] when the provider refuses or is unreachable.
    async fn read_role(&self, subject: &str) -> Result<Role, ExternalError>;
}

/// Records the `authorisation_*` metrics. Implemented at the HTTP boundary,
/// where the metrics registry lives, so infrastructure never imports `http`.
pub trait ObservationSink: Send + Sync {
    /// One decision, with its class and reason.
    fn record_decision(&self, class: OperationClass, reason: DenialReason, elapsed: Duration);

    /// One failed status read.
    fn record_status_lookup_failure(&self, cause: StatusFailure);

    /// One failed audit write.
    fn record_audit_write_failure(&self, context: AuditWriteContext);

    /// The outage cache's age and size.
    fn record_cache(&self, age_seconds: i64, entries: usize);

    /// One role-change outcome.
    fn record_role_change(&self, success: bool);
}
