//! The audit-trail helper. Every use case records through this one path, so the
//! actor, action, target, timestamp, context, and outcome are always present
//! (NFR19.1).

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use identity_domain::{AuditOutcome, AuditRecord, UserId};

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

/// Writes security and privacy events through C10.
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
    pub fn now(&self) -> DateTime<Utc> {
        self.clock.now()
    }

    /// Records one event. The caller decides whether a failure refuses the
    /// action (support and privacy paths) or is only logged.
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
}
