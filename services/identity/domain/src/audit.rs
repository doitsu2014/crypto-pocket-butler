//! The audit record Identity emits through C10. It carries identifiers and the
//! outcome only: never a credential, token, or full address (NFR19.2).

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

use crate::error::DomainError;
use crate::ids::UserId;

/// The longest dotted action name.
pub const MAX_ACTION_LEN: usize = 120;
/// The longest target label.
pub const MAX_TARGET_LEN: usize = 240;
/// The most context keys one event carries.
pub const MAX_CONTEXT_KEYS: usize = 16;
/// The longest one context value.
pub const MAX_CONTEXT_VALUE_LEN: usize = 240;

/// The recorded outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuditOutcome {
    /// The action succeeded.
    Success,
    /// The action was refused by a rule.
    Denied,
    /// The action failed at an integration boundary.
    Failed,
}

impl AuditOutcome {
    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Denied => "denied",
            Self::Failed => "failed",
        }
    }
}

/// One security or privacy event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditRecord {
    actor_id: Option<UserId>,
    action: String,
    target: String,
    outcome: AuditOutcome,
    occurred_at: DateTime<Utc>,
    context: BTreeMap<String, String>,
}

impl AuditRecord {
    /// Builds a validated audit record.
    ///
    /// # Errors
    /// Returns [`DomainError`] when the action, target, or context is malformed.
    pub fn new(
        actor_id: Option<UserId>,
        action: impl Into<String>,
        target: impl Into<String>,
        outcome: AuditOutcome,
        occurred_at: DateTime<Utc>,
        context: BTreeMap<String, String>,
    ) -> Result<Self, DomainError> {
        let action = action.into();
        let target = target.into();
        let action = action.trim();
        let target = target.trim();
        if action.is_empty() || !action.contains('.') || action.len() > MAX_ACTION_LEN {
            return Err(DomainError::Invalid {
                field: "action",
                reason: "must be a dotted action name of at most 120 characters".to_string(),
            });
        }
        if target.is_empty() || target.len() > MAX_TARGET_LEN {
            return Err(DomainError::Invalid {
                field: "target",
                reason: "must be 1-240 characters".to_string(),
            });
        }
        if context.len() > MAX_CONTEXT_KEYS {
            return Err(DomainError::TooLarge {
                field: "context",
                limit: "16 keys",
            });
        }
        for (key, value) in &context {
            if key.is_empty() || key.len() > 64 || value.len() > MAX_CONTEXT_VALUE_LEN {
                return Err(DomainError::Invalid {
                    field: "context",
                    reason: "a context key or value exceeds its bound".to_string(),
                });
            }
        }
        Ok(Self {
            actor_id,
            action: action.to_string(),
            target: target.to_string(),
            outcome,
            occurred_at,
            context,
        })
    }

    /// The acting user, when one exists.
    #[must_use]
    pub fn actor_id(&self) -> Option<UserId> {
        self.actor_id
    }

    /// The dotted action.
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

    /// When the action happened.
    #[must_use]
    pub fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }

    /// Identifier-only context.
    #[must_use]
    pub fn context(&self) -> &BTreeMap<String, String> {
        &self.context
    }
}
