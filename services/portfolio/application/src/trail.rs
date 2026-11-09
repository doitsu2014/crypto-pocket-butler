//! The audit-trail helper. Every use case records through this one path, so the
//! actor, action, target, timestamp, context, and outcome are always present.

use std::collections::BTreeMap;
use std::sync::Arc;

use portfolio_domain::{AuditOutcome, AuditRecord, UserId};

use crate::common::AppError;
use crate::ports::{AuditClient, Clock};

/// Builds the identifier-only context map for an audit event.
#[must_use]
pub fn context(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect()
}

/// Writes portfolio, target, guardrail, trade, intent, and privacy events
/// through C10.
pub struct Trail {
    audit: Arc<dyn AuditClient>,
    clock: Arc<dyn Clock>,
}

impl Trail {
    /// Creates the trail over the audit client and the clock.
    #[must_use]
    pub fn new(audit: Arc<dyn AuditClient>, clock: Arc<dyn Clock>) -> Self {
        Self { audit, clock }
    }

    /// The current instant, from the injected clock.
    #[must_use]
    pub fn now(&self) -> chrono::DateTime<chrono::Utc> {
        self.clock.now()
    }

    /// Records one event.
    ///
    /// # Errors
    /// Returns [`AppError::Unavailable`] when the audit sink cannot record the
    /// event, and [`AppError::Internal`] when the event shape is invalid.
    pub async fn record(
        &self,
        actor: Option<UserId>,
        action: &str,
        target: impl Into<String>,
        outcome: AuditOutcome,
        correlation_id: &str,
        context: BTreeMap<String, String>,
    ) -> Result<(), AppError> {
        let record = AuditRecord::new(actor, action, target, outcome, self.clock.now(), context)
            .map_err(AppError::from_domain)?;
        self.audit
            .append(&record, correlation_id)
            .await
            .map_err(AppError::from_repository)
    }

    /// Records one event with the two context keys the C10 allow-list accepts
    /// from this service: `sourceService` and `ruleId`. The adapter adds
    /// `correlationId`.
    ///
    /// # Errors
    /// Returns the same refusals as [`Self::record`].
    pub async fn record_action(
        &self,
        actor: Option<UserId>,
        action: &str,
        target: impl Into<String>,
        outcome: AuditOutcome,
        correlation_id: &str,
        rule_id: &str,
    ) -> Result<(), AppError> {
        self.record(
            actor,
            action,
            target,
            outcome,
            correlation_id,
            context(&[("sourceService", "portfolio"), ("ruleId", rule_id)]),
        )
        .await
    }
}
