//! The C10 audit record shape. A failed run carries actor (or system), action,
//! target, timestamp, identifier-only context, and outcome (BR5.2).

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

use crate::error::DomainError;

/// The outcome a C10 event records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditOutcome {
    /// The action completed.
    Success,
    /// The action was refused.
    Denied,
    /// The action failed.
    Failed,
}

impl AuditOutcome {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Denied => "denied",
            Self::Failed => "failed",
        }
    }
}

/// One audit event, with no credential, token, or provider URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditRecord {
    actor_kind: String,
    actor_service: Option<String>,
    action: String,
    target: String,
    occurred_at: DateTime<Utc>,
    context: BTreeMap<String, String>,
    outcome: AuditOutcome,
}

impl AuditRecord {
    /// Creates an event.
    ///
    /// # Errors
    /// Returns [`DomainError::Required`] for a blank actor kind, action, or
    /// target.
    pub fn new(
        actor_kind: &str,
        actor_service: Option<&str>,
        action: &str,
        target: impl Into<String>,
        outcome: AuditOutcome,
        occurred_at: DateTime<Utc>,
        context: BTreeMap<String, String>,
    ) -> Result<Self, DomainError> {
        let actor_kind = actor_kind.trim();
        if actor_kind.is_empty() {
            return Err(DomainError::Required { field: "actorKind" });
        }
        let action = action.trim();
        if action.is_empty() {
            return Err(DomainError::Required { field: "action" });
        }
        let target = target.into();
        if target.trim().is_empty() {
            return Err(DomainError::Required { field: "target" });
        }
        Ok(Self {
            actor_kind: actor_kind.to_string(),
            actor_service: actor_service
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string),
            action: action.to_string(),
            target,
            occurred_at,
            context,
            outcome,
        })
    }

    /// `user`, `system`, or `support_agent`.
    #[must_use]
    pub fn actor_kind(&self) -> &str {
        &self.actor_kind
    }

    /// The acting service, when the event records one.
    #[must_use]
    pub fn actor_service(&self) -> Option<&str> {
        self.actor_service.as_deref()
    }

    /// The action name.
    #[must_use]
    pub fn action(&self) -> &str {
        &self.action
    }

    /// The target identifier.
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    /// When the event occurred.
    #[must_use]
    pub fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }

    /// The identifier-only context.
    #[must_use]
    pub fn context(&self) -> &BTreeMap<String, String> {
        &self.context
    }

    /// The outcome.
    #[must_use]
    pub fn outcome(&self) -> AuditOutcome {
        self.outcome
    }
}
