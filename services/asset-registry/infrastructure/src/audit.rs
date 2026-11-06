//! The local structured audit sink used until the Audit service (U12) exists.
//!
//! It writes one JSON line per event and never writes a token, credential, or
//! user wallet address.

use asset_registry_application::audit::AuditEvent;
use asset_registry_application::ports::{AuditError, AuditSink};
use async_trait::async_trait;

/// Writes registry audit events as structured log lines.
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
            "registry.audit"
        );
        Ok(())
    }
}
