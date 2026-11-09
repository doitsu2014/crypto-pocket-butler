//! Structured audit sink for gateway denials.

use async_trait::async_trait;
use gateway_application::ports::{AuditError, AuditEvent, AuditSink};

/// Writes gateway audit events as structured log lines.
#[derive(Debug, Default, Clone, Copy)]
pub struct TracingAuditSink;

#[async_trait]
impl AuditSink for TracingAuditSink {
    async fn record(&self, event: &AuditEvent) -> Result<(), AuditError> {
        tracing::info!(
            actor = %event.actor,
            action = %event.action,
            target = %event.target,
            outcome = %event.outcome,
            correlation_id = event.correlation_id.as_deref().unwrap_or(""),
            at = %event.at,
            "gateway.audit"
        );
        Ok(())
    }
}
