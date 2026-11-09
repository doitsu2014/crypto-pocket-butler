//! The C10 audit record shape. Brief failures and privacy actions carry actor
//! (or system), action, target, timestamp, identifier-only context, and outcome.

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

#[cfg(test)]
mod tests {
    use super::{AuditOutcome, AuditRecord};
    use crate::ids::UserId;
    use chrono::DateTime;
    use std::collections::BTreeMap;

    fn now() -> DateTime<chrono::Utc> {
        DateTime::from_timestamp(1_800_000_000, 0).expect("a valid instant")
    }

    #[test]
    fn a_system_event_needs_no_actor() {
        let record = AuditRecord::new(
            None,
            "reporting.brief.failed",
            "brief:1",
            AuditOutcome::Failed,
            now(),
            BTreeMap::new(),
        )
        .expect("a system event is valid");
        assert_eq!(record.actor_id(), None);
        assert_eq!(record.outcome().as_str(), "failed");
    }

    #[test]
    fn a_user_event_keeps_the_actor() {
        let user = UserId::generate();
        let record = AuditRecord::new(
            Some(user),
            "reporting.privacy.anonymise",
            "user:1",
            AuditOutcome::Success,
            now(),
            BTreeMap::new(),
        )
        .expect("a user event is valid");
        assert_eq!(record.actor_id(), Some(user));
    }

    #[test]
    fn a_blank_action_is_refused() {
        let error = AuditRecord::new(
            None,
            "  ",
            "brief:1",
            AuditOutcome::Success,
            now(),
            BTreeMap::new(),
        )
        .expect_err("a blank action is refused");
        assert_eq!(error.field(), "action");
    }

    #[test]
    fn a_blank_target_is_refused() {
        let error = AuditRecord::new(
            None,
            "reporting.brief.failed",
            "",
            AuditOutcome::Success,
            now(),
            BTreeMap::new(),
        )
        .expect_err("a blank target is refused");
        assert_eq!(error.field(), "target");
    }

    #[test]
    fn outcomes_round_trip_through_their_labels() {
        assert_eq!(AuditOutcome::Success.as_str(), "success");
        assert_eq!(AuditOutcome::Denied.as_str(), "denied");
        assert_eq!(AuditOutcome::Failed.as_str(), "failed");
    }
}
