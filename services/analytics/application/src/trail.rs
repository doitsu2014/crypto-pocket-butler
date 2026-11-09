//! C10 trail writes. An audit failure is logged and counted by the caller; it
//! never carries a property value, user reference, or subject.

use std::collections::BTreeMap;
use std::sync::Arc;

use analytics_domain::audit::{
    FLAG_CREATED, FLAG_DELETED, FLAG_UPDATED, INGEST_REFUSED, PRIVACY_ERASED,
};
use analytics_domain::RefusalKind;

use crate::hashing::pseudonymous_actor_id;
use crate::ports::{AuditClient, AuditRecord, Clock};

/// The prefix length of a user reference used as an audit target. The full hash
/// is never written.
pub const TARGET_PREFIX_CHARS: usize = 12;

/// Writes audit records, logging but never failing the caller.
pub struct Trail {
    client: Arc<dyn AuditClient>,
    clock: Arc<dyn Clock>,
}

impl Trail {
    /// Creates the trail writer.
    #[must_use]
    pub fn new(client: Arc<dyn AuditClient>, clock: Arc<dyn Clock>) -> Self {
        Self { client, clock }
    }

    /// A refused ingest, with the affected user-reference hash, a category, and
    /// a correlation id. The hash becomes a pseudonymous actor UUID; the raw
    /// subject never leaves Analytics.
    pub async fn refused_ingest(
        &self,
        kind: RefusalKind,
        actor_ref: &str,
        correlation: Option<&str>,
    ) {
        // C10's context allow-list is identifiers only; the refusal category is
        // recorded as the rule id and the request correlation id beside it.
        let mut context = BTreeMap::new();
        context.insert("ruleId".to_string(), kind.as_str().to_string());
        if let Some(correlation) = correlation.filter(|value| !value.is_empty()) {
            context.insert("correlationId".to_string(), correlation.to_string());
        }
        self.record(AuditRecord {
            actor_id: Some(pseudonymous_actor_id(actor_ref)),
            actor_kind: "user".to_string(),
            action: INGEST_REFUSED.to_string(),
            target: "analytics-ingest".to_string(),
            occurred_at: self.clock.now(),
            context,
            outcome: "denied".to_string(),
            sensitivity: "security".to_string(),
        })
        .await;
    }

    /// A flag creation, update, or deletion by an administrator.
    pub async fn flag_changed(&self, action: &str, key: &str, actor: &str) {
        let action = match action {
            "created" => FLAG_CREATED,
            "deleted" => FLAG_DELETED,
            _ => FLAG_UPDATED,
        };
        self.record(AuditRecord {
            actor_id: Some(actor.to_string()),
            actor_kind: "user".to_string(),
            action: action.to_string(),
            target: key.to_string(),
            occurred_at: self.clock.now(),
            context: BTreeMap::new(),
            outcome: "success".to_string(),
            sensitivity: "security".to_string(),
        })
        .await;
    }

    /// An erasure request. The target is a user-reference prefix, never the
    /// hash; the actor is a pseudonymous UUID derived from the hash, and the
    /// acting service is named in context.
    pub async fn erased(&self, actor_ref: &str, acting_service: &str) {
        // C10's context allow-list accepts `sourceService` and `ruleId`; the
        // deleted-row count is exposed in the erasure metric, not in the trail.
        let mut context = BTreeMap::new();
        context.insert("sourceService".to_string(), acting_service.to_string());
        context.insert("ruleId".to_string(), "BR6.1".to_string());
        self.record(AuditRecord {
            actor_id: Some(pseudonymous_actor_id(actor_ref)),
            actor_kind: "user".to_string(),
            action: PRIVACY_ERASED.to_string(),
            target: actor_ref.chars().take(TARGET_PREFIX_CHARS).collect(),
            occurred_at: self.clock.now(),
            context,
            outcome: "success".to_string(),
            sensitivity: "security".to_string(),
        })
        .await;
    }

    async fn record(&self, record: AuditRecord) {
        if let Err(error) = self.client.append(&record).await {
            tracing::warn!(
                action = %record.action,
                error = %error,
                "failed to write the C10 audit record"
            );
        }
    }
}
