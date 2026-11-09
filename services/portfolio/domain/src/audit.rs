//! The C10 audit record shape. Target changes, guardrail changes, recorded
//! trades, intent cancellations, and privacy actions carry actor (or system),
//! action, target, timestamp, identifier-only context, and outcome.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

use crate::error::DomainError;
use crate::ids::UserId;

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

/// One audit event, with no credential, token, or full address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditRecord {
    actor_id: Option<UserId>,
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
    /// Returns [`DomainError::Required`] for a blank action or target.
    pub fn new(
        actor_id: Option<UserId>,
        action: &str,
        target: impl Into<String>,
        outcome: AuditOutcome,
        occurred_at: DateTime<Utc>,
        context: BTreeMap<String, String>,
    ) -> Result<Self, DomainError> {
        let action = action.trim();
        if action.is_empty() {
            return Err(DomainError::Required { field: "action" });
        }
        let target = target.into();
        if target.trim().is_empty() {
            return Err(DomainError::Required { field: "target" });
        }
        Ok(Self {
            actor_id,
            action: action.to_string(),
            target,
            occurred_at,
            context,
            outcome,
        })
    }

    /// The acting user, absent for a system action.
    #[must_use]
    pub fn actor_id(&self) -> Option<UserId> {
        self.actor_id
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
