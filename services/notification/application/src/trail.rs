//! C10 trail writes: three attempts with 1 s, 2 s, then 5 s between them
//! (NFR1.5). A failure is counted, never swallowed into a success.
//!
//! No address, credential, token, or payload value is ever written.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use notification_domain::audit::{
    PREFERENCE_REFUSED, PRIVACY_ANONYMISED, SECURITY_ALERT_ABANDONED, SECURITY_ALERT_DELIVERED,
};
use notification_domain::{AlertId, Channel, NotificationType, UserId, AUDIT_RETRY_SECONDS};

use crate::ports::{AuditClient, AuditRecord, Clock, Pause};

/// Writes audit records with bounded retry and a persistent-failure counter.
pub struct Trail {
    client: Arc<dyn AuditClient>,
    clock: Arc<dyn Clock>,
    pause: Arc<dyn Pause>,
    failures: AtomicU64,
}

impl Trail {
    /// Creates the trail writer.
    #[must_use]
    pub fn new(client: Arc<dyn AuditClient>, clock: Arc<dyn Clock>, pause: Arc<dyn Pause>) -> Self {
        Self {
            client,
            clock,
            pause,
            failures: AtomicU64::new(0),
        }
    }

    /// How many records were abandoned after every attempt.
    #[must_use]
    pub fn failures(&self) -> u64 {
        self.failures.load(Ordering::Relaxed)
    }

    /// Writes one record. Returns whether it was accepted.
    pub async fn record(&self, record: AuditRecord) -> bool {
        for (index, delay) in AUDIT_RETRY_SECONDS.iter().enumerate() {
            let is_last = index + 1 >= AUDIT_RETRY_SECONDS.len();
            match self.client.append(&record).await {
                Ok(()) => return true,
                Err(error) => {
                    tracing::warn!(
                        action = %record.action,
                        target = %record.target,
                        error = %error,
                        "failed to write the C10 audit record"
                    );
                    self.pause
                        .pause_for(std::time::Duration::from_secs(*delay))
                        .await;
                    if is_last {
                        self.failures.fetch_add(1, Ordering::Relaxed);
                        return false;
                    }
                }
            }
        }
        false
    }

    /// A mandatory security or revoked-key email was delivered.
    pub async fn security_alert_delivered(
        &self,
        kind: NotificationType,
        alert_id: AlertId,
        user_id: UserId,
        sanitised_fields: &[String],
    ) -> bool {
        self.security_alert(
            SECURITY_ALERT_DELIVERED,
            kind,
            alert_id,
            user_id,
            "success",
            sanitised_fields,
        )
        .await
    }

    /// A mandatory security or revoked-key email was abandoned.
    pub async fn security_alert_abandoned(
        &self,
        kind: NotificationType,
        alert_id: AlertId,
        user_id: UserId,
        sanitised_fields: &[String],
    ) -> bool {
        self.security_alert(
            SECURITY_ALERT_ABANDONED,
            kind,
            alert_id,
            user_id,
            "failed",
            sanitised_fields,
        )
        .await
    }

    async fn security_alert(
        &self,
        action: &str,
        kind: NotificationType,
        alert_id: AlertId,
        user_id: UserId,
        outcome: &str,
        sanitised_fields: &[String],
    ) -> bool {
        let mut context = base_context("BR3.1", None);
        // NFR9.5: the event names the channel and the fields the content guard
        // removed. Only field names travel, never their values (BR5.1).
        context.insert("channel".to_string(), Channel::Email.as_str().to_string());
        if !sanitised_fields.is_empty() {
            context.insert("sanitisedFields".to_string(), sanitised_fields.join(","));
        }
        self.record(AuditRecord {
            actor_id: Some(user_id.to_string()),
            actor_kind: "user".to_string(),
            action: action.to_string(),
            target: format!("{}:{alert_id}", kind.as_str()),
            occurred_at: self.clock.now(),
            context,
            outcome: outcome.to_string(),
            sensitivity: "security".to_string(),
        })
        .await
    }

    /// A user tried to disable a mandatory email (BR2.1, BR3.1).
    pub async fn preference_refused(
        &self,
        user_id: UserId,
        kind: NotificationType,
        channel: Channel,
        correlation: Option<&str>,
    ) -> bool {
        let rule = format!("BR2.1.{}", channel.as_str());
        let context = base_context(&rule, correlation);
        self.record(AuditRecord {
            actor_id: Some(user_id.to_string()),
            actor_kind: "user".to_string(),
            action: PREFERENCE_REFUSED.to_string(),
            target: kind.as_str().to_string(),
            occurred_at: self.clock.now(),
            context,
            outcome: "denied".to_string(),
            sensitivity: "security".to_string(),
        })
        .await
    }

    /// Identity's erasure reached Notification (NFR13.1). The target is the
    /// non-reversible reference, never the erased user id.
    pub async fn anonymised(&self, replacement: UserId, correlation: Option<&str>) -> bool {
        self.record(AuditRecord {
            actor_id: None,
            actor_kind: "system".to_string(),
            action: PRIVACY_ANONYMISED.to_string(),
            target: replacement.to_string(),
            occurred_at: self.clock.now(),
            context: base_context("NFR13.1", correlation),
            outcome: "success".to_string(),
            sensitivity: "security".to_string(),
        })
        .await
    }
}

/// C10 accepts identifiers and metadata only: the source service, a rule id,
/// the request correlation id, and (for a delivery event) the channel and the
/// sanitised field names.
fn base_context(rule_id: &str, correlation: Option<&str>) -> BTreeMap<String, String> {
    let mut context = BTreeMap::new();
    context.insert("sourceService".to_string(), "notification".to_string());
    context.insert("ruleId".to_string(), rule_id.to_string());
    if let Some(correlation) = correlation.filter(|value| !value.is_empty()) {
        context.insert("correlationId".to_string(), correlation.to_string());
    }
    context
}
