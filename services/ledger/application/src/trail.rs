//! C10 trail writes: commit first, then audit with bounded retry, then surface a
//! persistent failure without rolling the committed row back (BR4.1, NFR1.4).
//!
//! No amount, credential, token, or full address is ever written.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use ledger_domain::AUDIT_RETRY_SECONDS;

use crate::ports::{AuditClient, AuditRecord, Clock, Pause};

/// Writes audit records with three attempts and 1 s / 2 s / 5 s backoff.
pub struct Trail {
    client: Arc<dyn AuditClient>,
    clock: Arc<dyn Clock>,
    pause: Arc<dyn Pause>,
}

impl Trail {
    /// Creates the trail writer.
    #[must_use]
    pub fn new(client: Arc<dyn AuditClient>, clock: Arc<dyn Clock>, pause: Arc<dyn Pause>) -> Self {
        Self {
            client,
            clock,
            pause,
        }
    }

    /// Writes one record. Returns whether it was accepted; a failure is logged
    /// and counted by the caller, never swallowed into a success.
    pub async fn record(&self, record: AuditRecord) -> bool {
        let attempts = AUDIT_RETRY_SECONDS.len();
        for (index, delay) in AUDIT_RETRY_SECONDS.iter().enumerate() {
            let is_last = index + 1 >= attempts;
            match self.client.append(&record).await {
                Ok(()) => return true,
                Err(error) => {
                    tracing::warn!(
                        action = %record.action,
                        error = %error,
                        "failed to write the C10 audit record"
                    );
                    self.pause.pause_for(Duration::from_secs(*delay)).await;
                    if is_last {
                        return false;
                    }
                }
            }
        }
        false
    }

    /// A recorded correction: actor, transaction, field, a reason hash, and the
    /// outcome. The free-text reason stays only on the correction row.
    pub async fn correction_recorded(
        &self,
        actor: &str,
        transaction: &str,
        field: &str,
        reason_hash: &str,
        outcome: &str,
    ) -> bool {
        let mut context = BTreeMap::new();
        context.insert("transactionId".to_string(), transaction.to_string());
        context.insert("field".to_string(), field.to_string());
        context.insert("reasonHash".to_string(), reason_hash.to_string());
        context.insert("sourceService".to_string(), "ledger".to_string());
        self.record(AuditRecord {
            actor_id: Some(actor.to_string()),
            actor_kind: "user".to_string(),
            action: "ledger.transaction.corrected".to_string(),
            target: transaction.to_string(),
            occurred_at: self.clock.now(),
            context,
            outcome: outcome.to_string(),
            sensitivity: "security".to_string(),
        })
        .await
    }

    /// A recorded trade.
    pub async fn trade_recorded(&self, actor: &str, transaction: &str) -> bool {
        let mut context = BTreeMap::new();
        context.insert("transactionId".to_string(), transaction.to_string());
        context.insert("sourceService".to_string(), "ledger".to_string());
        self.record(AuditRecord {
            actor_id: Some(actor.to_string()),
            actor_kind: "user".to_string(),
            action: "ledger.trade.recorded".to_string(),
            target: transaction.to_string(),
            occurred_at: self.clock.now(),
            context,
            outcome: "success".to_string(),
            sensitivity: "security".to_string(),
        })
        .await
    }

    /// A privacy anonymisation. The target is the non-reversible reference.
    pub async fn anonymised(&self, anonymised_ref: &str, outcome: &str) -> bool {
        self.record(self.anonymised_record(anonymised_ref, outcome))
            .await
    }

    fn anonymised_record(&self, anonymised_ref: &str, outcome: &str) -> AuditRecord {
        let mut context = BTreeMap::new();
        context.insert("sourceService".to_string(), "identity".to_string());
        context.insert("ruleId".to_string(), "NFR9.2".to_string());
        AuditRecord {
            actor_id: None,
            actor_kind: "system".to_string(),
            action: "ledger.privacy.anonymised".to_string(),
            target: anonymised_ref.to_string(),
            occurred_at: self.clock.now(),
            context,
            outcome: outcome.to_string(),
            sensitivity: "security".to_string(),
        }
    }
}
