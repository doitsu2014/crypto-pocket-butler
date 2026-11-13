//! The inline read-audit sink for the support metadata path (BR3.13, W4).
//!
//! A read event has no grant change to ride, so it is appended synchronously
//! before the response: `identity.support_read.served` for a projection that
//! leaves, `identity.support_read.denied` for a refusal, and the same denied
//! action with outcome `failed` when an owner could not answer. The application
//! layer refuses the read when this append fails.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use identity_application::ports::{
    AuditClient, Clock, RepositoryError, SupportObservation, SupportSourceFailureSink,
};
use identity_domain::{AuditOutcome, AuditRecord, SupportGrantId, UserId};

/// Writes support read events straight to C10.
pub struct AuditSupportObservation {
    audit: Arc<dyn AuditClient>,
    clock: Arc<dyn Clock>,
    source_failures: Option<Arc<dyn SupportSourceFailureSink>>,
}

impl AuditSupportObservation {
    /// Creates the sink over the audit client and the injected clock.
    #[must_use]
    pub fn new(audit: Arc<dyn AuditClient>, clock: Arc<dyn Clock>) -> Self {
        Self {
            audit,
            clock,
            source_failures: None,
        }
    }

    /// Joins the monitoring sink that names the owner that could not answer.
    #[must_use]
    pub fn with_source_failures(mut self, sink: Arc<dyn SupportSourceFailureSink>) -> Self {
        self.source_failures = Some(sink);
        self
    }

    async fn append(
        &self,
        actor: UserId,
        action: &str,
        target: String,
        outcome: AuditOutcome,
        code: Option<&str>,
        correlation_id: &str,
    ) -> Result<(), RepositoryError> {
        let mut context = BTreeMap::new();
        if let Some(code) = code {
            context.insert("code".to_string(), code.to_string());
        }
        let record = AuditRecord::new(
            Some(actor),
            action,
            target,
            outcome,
            self.clock.now(),
            context,
        )
        .map_err(|error| {
            RepositoryError::InvalidData(format!("invalid support read event: {error}"))
        })?;
        self.audit.append(&record, correlation_id).await
    }
}

#[async_trait]
impl SupportObservation for AuditSupportObservation {
    async fn served(
        &self,
        actor: UserId,
        target: UserId,
        grant_id: SupportGrantId,
        correlation_id: &str,
    ) -> Result<(), RepositoryError> {
        let grant_label = grant_id.to_string();
        let mut context = BTreeMap::new();
        context.insert("grantId".to_string(), grant_label);
        let record = AuditRecord::new(
            Some(actor),
            "identity.support_read.served",
            target.to_string(),
            AuditOutcome::Success,
            self.clock.now(),
            context,
        )
        .map_err(|error| {
            RepositoryError::InvalidData(format!("invalid support read event: {error}"))
        })?;
        self.audit.append(&record, correlation_id).await
    }

    async fn denied(
        &self,
        actor: UserId,
        target: String,
        code: &str,
        correlation_id: &str,
    ) -> Result<(), RepositoryError> {
        self.append(
            actor,
            "identity.support_read.denied",
            target,
            AuditOutcome::Denied,
            Some(code),
            correlation_id,
        )
        .await
    }

    async fn failed(
        &self,
        actor: UserId,
        target: UserId,
        code: &str,
        correlation_id: &str,
    ) -> Result<(), RepositoryError> {
        self.append(
            actor,
            "identity.support_read.denied",
            target.to_string(),
            AuditOutcome::Failed,
            Some(code),
            correlation_id,
        )
        .await
    }

    fn source_failure(&self, source: &str, cause: &str) {
        if let Some(sink) = &self.source_failures {
            sink.source_failure(source, cause);
        }
    }
}
