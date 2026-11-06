//! The audit service's own write path.
//!
//! Every denial, export, run, and deletion the service records goes through
//! [`Trail`], so the service's own events obey the same validation and
//! idempotency rules as those of another emitter.

use std::sync::Arc;

use audit_domain::{
    ActorKind, AuditEvent, AuditEventId, Context, EventClass, NewAuditEvent, Outcome, Sensitivity,
    UserId,
};
use chrono::{DateTime, Utc};

use crate::hashing::sha256_hex;
use crate::ports::{AuditEventRepository, Clock, RepositoryError};

/// The emitting-service name used for Audit's own events.
pub const AUDIT_SERVICE: &str = "audit";

/// The service's own write path.
#[derive(Clone)]
pub struct Trail {
    events: Arc<dyn AuditEventRepository>,
    clock: Arc<dyn Clock>,
}

impl Trail {
    /// Creates the write path.
    #[must_use]
    pub fn new(events: Arc<dyn AuditEventRepository>, clock: Arc<dyn Clock>) -> Self {
        Self { events, clock }
    }

    /// The current time.
    #[must_use]
    pub fn now(&self) -> DateTime<Utc> {
        self.clock.now()
    }

    /// Records an event built by the caller.
    ///
    /// # Errors
    /// Returns [`RepositoryError`] when the insert fails.
    pub async fn record(&self, event: &AuditEvent) -> Result<AuditEventId, RepositoryError> {
        let payload = sha256_hex(&event.canonical_payload());
        match self.events.append(event, AUDIT_SERVICE, &payload).await? {
            crate::ports::AppendOutcome::Appended => Ok(event.id()),
            crate::ports::AppendOutcome::Duplicate { event_id, .. } => Ok(event_id),
        }
    }

    /// Records a system-attributed event from a validated shape.
    ///
    /// # Errors
    /// Returns [`RepositoryError`] when the event is refused by validation or the
    /// insert fails.
    pub async fn record_system(
        &self,
        action: &str,
        target: &str,
        outcome: Outcome,
        sensitivity: Sensitivity,
        context: Context,
    ) -> Result<AuditEventId, RepositoryError> {
        let now = self.clock.now();
        let new = NewAuditEvent {
            actor_id: None,
            actor_kind: ActorKind::System,
            action: action.to_string(),
            target: target.to_string(),
            occurred_at: Some(now),
            context,
            outcome,
            sensitivity,
            event_class: Some(EventClass::for_sensitivity(sensitivity)),
            idempotency_key: None,
            legal_basis: None,
            deletion_request_id: None,
        };
        let event = AuditEvent::record(new, AuditEventId::generate(), now)
            .map_err(|error| RepositoryError::InvalidData(error.to_string()))?;
        self.record(&event).await
    }

    /// Records an event attributed to a user or support agent.
    ///
    /// # Errors
    /// Returns [`RepositoryError`] when the event is refused by validation or the
    /// insert fails.
    pub async fn record_as(
        &self,
        actor_id: Option<UserId>,
        actor_kind: ActorKind,
        action: &str,
        target: &str,
        outcome: Outcome,
        sensitivity: Sensitivity,
        context: Context,
    ) -> Result<AuditEventId, RepositoryError> {
        let now = self.clock.now();
        let new = NewAuditEvent {
            actor_id,
            actor_kind,
            action: action.to_string(),
            target: target.to_string(),
            occurred_at: Some(now),
            context,
            outcome,
            sensitivity,
            event_class: Some(EventClass::for_sensitivity(sensitivity)),
            idempotency_key: None,
            legal_basis: None,
            deletion_request_id: None,
        };
        let event = AuditEvent::record(new, AuditEventId::generate(), now)
            .map_err(|error| RepositoryError::InvalidData(error.to_string()))?;
        self.record(&event).await
    }

    /// Records an operator-attributed event.
    ///
    /// # Errors
    /// Returns [`RepositoryError`] when the insert fails.
    pub async fn record_user(
        &self,
        actor_id: UserId,
        action: &str,
        target: &str,
        outcome: Outcome,
        sensitivity: Sensitivity,
        context: Context,
    ) -> Result<AuditEventId, RepositoryError> {
        self.record_as(
            Some(actor_id),
            ActorKind::User,
            action,
            target,
            outcome,
            sensitivity,
            context,
        )
        .await
    }

    /// Records a support-agent-attributed event.
    ///
    /// # Errors
    /// Returns [`RepositoryError`] when the insert fails.
    pub async fn record_agent(
        &self,
        actor_id: UserId,
        action: &str,
        target: &str,
        sensitivity: Sensitivity,
        context: Context,
    ) -> Result<AuditEventId, RepositoryError> {
        self.record_as(
            Some(actor_id),
            ActorKind::SupportAgent,
            action,
            target,
            Outcome::Success,
            sensitivity,
            context,
        )
        .await
    }

    /// Records a denial as a security event.
    ///
    /// # Errors
    /// Returns [`RepositoryError`] when the insert fails.
    pub async fn record_denial(
        &self,
        actor_id: Option<UserId>,
        actor_kind: ActorKind,
        action: &str,
        target: &str,
        context: Context,
    ) -> Result<AuditEventId, RepositoryError> {
        self.record_as(
            actor_id,
            actor_kind,
            action,
            target,
            Outcome::Denied,
            Sensitivity::Security,
            context,
        )
        .await
    }
}
