//! The audit event shape shared by every registry change (BR3.1).

use chrono::{DateTime, Utc};

use crate::ports::AuditError;

/// One auditable registry event.
///
/// Tokens, credentials, and user wallet addresses never appear here; the target
/// is an internal registry identifier or a public contract address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEvent {
    /// The authenticated actor, or `system`.
    pub actor: String,
    /// What happened, e.g. `asset.create`.
    pub action: String,
    /// The internal target identifier.
    pub target: String,
    /// `success`, `refused`, or `denied`.
    pub outcome: String,
    /// The request correlation id, when one exists.
    pub correlation_id: Option<String>,
    /// When the event was recorded.
    pub at: DateTime<Utc>,
}

impl AuditEvent {
    /// Creates an audit event.
    ///
    /// # Errors
    /// Returns [`AuditError::Rejected`] when the actor, action, target, or outcome
    /// is blank.
    pub fn new(
        actor: impl Into<String>,
        action: impl Into<String>,
        target: impl Into<String>,
        outcome: impl Into<String>,
        correlation_id: Option<String>,
        at: DateTime<Utc>,
    ) -> Result<Self, AuditError> {
        let actor = actor.into();
        let action = action.into();
        let target = target.into();
        let outcome = outcome.into();
        if actor.trim().is_empty()
            || action.trim().is_empty()
            || target.trim().is_empty()
            || outcome.trim().is_empty()
        {
            return Err(AuditError::Rejected(
                "audit event requires actor, action, target, and outcome".to_string(),
            ));
        }
        Ok(Self {
            actor,
            action,
            target,
            outcome,
            correlation_id,
            at,
        })
    }
}
