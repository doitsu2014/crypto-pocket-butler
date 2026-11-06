//! WF1: append an audit event (BR1.1–BR1.4, BR2.1–BR2.2).

use std::collections::BTreeMap;
use std::sync::Arc;

use audit_domain::{
    ActorKind, AuditEvent, AuditEventId, Context, EventClass, NewAuditEvent, Outcome, Sensitivity,
    UserId,
};
use chrono::{DateTime, Utc};

use crate::common::FieldError;
use crate::hashing::sha256_hex;
use crate::ports::{AppendOutcome, AuditEventRepository, Clock};

/// The authenticated emitter. The service token identifies it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Emitter {
    /// The registered emitting service, e.g. `identity`.
    pub service: String,
}

/// The raw append request, exactly as the boundary received it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppendCommand {
    /// The acting user or support agent.
    pub actor_id: Option<String>,
    /// `user`, `system`, or `support_agent`.
    pub actor_kind: String,
    /// The dotted action name.
    pub action: String,
    /// The logical target.
    pub target: String,
    /// When the action happened.
    pub occurred_at: Option<DateTime<Utc>>,
    /// Correlation and identifier metadata.
    pub context: BTreeMap<String, String>,
    /// `success`, `denied`, or `failed`.
    pub outcome: String,
    /// `routine` or `security`.
    pub sensitivity: String,
    /// A caller-supplied class, checked and never trusted.
    pub event_class: Option<String>,
    /// The legal basis the record is kept under.
    pub legal_basis: Option<String>,
    /// The deletion request the record outlives.
    pub deletion_request_id: Option<String>,
    /// The emitter's retry key.
    pub idempotency_key: Option<String>,
}

/// A stored event, as returned to the emitter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppendResult {
    /// The event identifier.
    pub id: AuditEventId,
    /// When the audit service stored it.
    pub recorded_at: DateTime<Utc>,
    /// Whether this call inserted the event or returned an earlier one.
    pub created: bool,
}

/// Why an append was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AppendError {
    /// The envelope failed validation; nothing was stored.
    #[error("the event envelope is invalid")]
    Rejected(Vec<FieldError>),
    /// The idempotency key was reused with a different payload.
    #[error("the idempotency key was reused with a different payload")]
    PayloadMismatch,
    /// The insert failed.
    #[error("the event could not be stored: {0}")]
    Repository(#[from] crate::ports::RepositoryError),
}

/// Validates and stores one event.
pub struct AppendEvent {
    events: Arc<dyn AuditEventRepository>,
    clock: Arc<dyn Clock>,
}

impl AppendEvent {
    /// Creates the use case.
    #[must_use]
    pub fn new(events: Arc<dyn AuditEventRepository>, clock: Arc<dyn Clock>) -> Self {
        Self { events, clock }
    }

    /// Validates the envelope, derives the event class, and appends.
    ///
    /// # Errors
    /// Returns [`AppendError::Rejected`] naming each offending field path with
    /// nothing stored, [`AppendError::PayloadMismatch`] for a reused key with a
    /// different payload, and [`AppendError::Repository`] on a storage failure.
    pub async fn execute(
        &self,
        emitter: &Emitter,
        command: AppendCommand,
    ) -> Result<AppendResult, AppendError> {
        let new = self.validate(command)?;
        // The hash covers the emitter's payload, not the service's receipt time:
        // a retry without `occurredAt` must still compare equal to the first
        // attempt (BR2.2).
        let payload = payload_hash(&new);
        let recorded_at = self.clock.now();
        let event = AuditEvent::record(new, AuditEventId::generate(), recorded_at)
            .map_err(|error| AppendError::Rejected(vec![FieldError::from(error)]))?;

        match self
            .events
            .append(&event, &emitter.service, &payload)
            .await
            .map_err(AppendError::Repository)?
        {
            AppendOutcome::Appended => Ok(AppendResult {
                id: event.id(),
                recorded_at,
                created: true,
            }),
            AppendOutcome::Duplicate {
                event_id,
                same_payload,
            } => {
                if !same_payload {
                    return Err(AppendError::PayloadMismatch);
                }
                let recorded_at = self
                    .events
                    .find_by_id(event_id)
                    .await
                    .ok()
                    .flatten()
                    .map_or(recorded_at, |existing| existing.recorded_at());
                Ok(AppendResult {
                    id: event_id,
                    recorded_at,
                    created: false,
                })
            }
        }
    }

    fn validate(&self, command: AppendCommand) -> Result<NewAuditEvent, AppendError> {
        let mut errors = Vec::new();

        let actor_kind = match ActorKind::parse(&command.actor_kind) {
            Ok(kind) => Some(kind),
            Err(error) => {
                errors.push(FieldError::from(error));
                None
            }
        };
        let outcome = match Outcome::parse(&command.outcome) {
            Ok(value) => Some(value),
            Err(error) => {
                errors.push(FieldError::from(error));
                None
            }
        };
        let sensitivity = match Sensitivity::parse(&command.sensitivity) {
            Ok(value) => Some(value),
            Err(error) => {
                errors.push(FieldError::from(error));
                None
            }
        };
        let event_class = match command.event_class.as_deref() {
            None => None,
            Some(raw) => match EventClass::parse(raw) {
                Ok(value) => Some(value),
                Err(error) => {
                    errors.push(FieldError::from(error));
                    None
                }
            },
        };
        let actor_id = match command.actor_id.as_deref() {
            None => None,
            Some(raw) => match UserId::parse(raw) {
                Ok(value) => Some(value),
                Err(error) => {
                    errors.push(FieldError::from(error));
                    None
                }
            },
        };
        let deletion_request_id = match command.deletion_request_id.as_deref() {
            None => None,
            Some(raw) => match audit_domain::DeletionRequestId::parse(raw) {
                Ok(value) => Some(value),
                Err(error) => {
                    errors.push(FieldError::from(error));
                    None
                }
            },
        };
        let context = match Context::parse(command.context) {
            Ok(value) => value,
            Err(error) => {
                errors.push(FieldError::from(error));
                Context::empty()
            }
        };

        if !errors.is_empty() {
            return Err(AppendError::Rejected(errors));
        }

        Ok(NewAuditEvent {
            actor_id,
            actor_kind: actor_kind.expect("validated actor kind"),
            action: command.action,
            target: command.target,
            occurred_at: command.occurred_at,
            context,
            outcome: outcome.expect("validated outcome"),
            sensitivity: sensitivity.expect("validated sensitivity"),
            event_class,
            idempotency_key: command.idempotency_key,
            legal_basis: command.legal_basis,
            deletion_request_id,
        })
    }
}

/// The deterministic payload digest used to compare a retry with its first
/// attempt. It deliberately excludes the service-assigned id and receipt time;
/// an absent `occurredAt` stays absent rather than becoming the receipt time.
fn payload_hash(new: &NewAuditEvent) -> String {
    let canonical = format!(
        "actorId={};actorKind={};action={};target={};occurredAt={};outcome={};sensitivity={};legalBasis={};deletionRequestId={};context={}",
        new.actor_id.map(|id| id.to_string()).unwrap_or_default(),
        new.actor_kind.as_str(),
        new.action.trim(),
        new.target.trim(),
        new.occurred_at
            .map(|at| at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
            .unwrap_or_default(),
        new.outcome.as_str(),
        new.sensitivity.as_str(),
        new.legal_basis.as_deref().unwrap_or("").trim(),
        new.deletion_request_id
            .map(|id| id.to_string())
            .unwrap_or_default(),
        new.context.canonical(),
    );
    sha256_hex(&canonical)
}
