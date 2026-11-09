//! The append-only audit event (BR1.1–BR1.4, BR2.1–BR2.2).

use std::collections::BTreeMap;

use chrono::{DateTime, SecondsFormat, Utc};

use crate::context::Context;
use crate::error::DomainError;
use crate::ids::{AuditEventId, DeletionRequestId, UserId};
use crate::secrets::ensure_secret_free;

/// The maximum action length.
pub const MAX_ACTION_LEN: usize = 120;
/// The maximum target length.
pub const MAX_TARGET_LEN: usize = 240;
/// The maximum idempotency-key length.
pub const MAX_IDEMPOTENCY_KEY_LEN: usize = 128;
/// The maximum legal-basis length.
pub const MAX_LEGAL_BASIS_LEN: usize = 120;

/// The actions Audit's own paths may attribute to the system. The entity
/// model's `system` default is a schema convenience, not a permission: the
/// service sets it only for these actions.
pub const SYSTEM_ACTION_ALLOW_LIST: [&str; 14] = [
    "system.job.run",
    "system.retention.run",
    "system.retention.reported",
    "system.sync.scheduled",
    "system.smoke.check",
    "system.privacy.deletion",
    "system.export.recovery",
    "system.support.expiry",
    "system.support.check_denied",
    // Service-attributed brief and privacy events (u8-reporting). A reporting
    // run acts without a user actor, so its events are system-attributed.
    "reporting.brief.failed",
    "reporting.brief.email_submitted",
    "reporting.privacy.export",
    "reporting.privacy.anonymise",
    // The documented fallback action when a finer event is refused.
    "system.reporting.brief",
];

/// Who the actor is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActorKind {
    /// A regular user.
    User,
    /// The service itself.
    System,
    /// A support agent acting inside a session.
    SupportAgent,
}

impl ActorKind {
    /// Parses the stored value.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for anything else.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "user" => Ok(Self::User),
            "system" => Ok(Self::System),
            "support_agent" => Ok(Self::SupportAgent),
            _ => Err(DomainError::Invalid {
                field: "actorKind",
                reason: "must be user, system, or support_agent".to_string(),
            }),
        }
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::System => "system",
            Self::SupportAgent => "support_agent",
        }
    }
}

/// How the action turned out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Outcome {
    /// It succeeded.
    Success,
    /// It was refused by a rule.
    Denied,
    /// It was attempted and failed.
    Failed,
}

impl Outcome {
    /// Parses the stored value.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for anything else.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "success" => Ok(Self::Success),
            "denied" => Ok(Self::Denied),
            "failed" => Ok(Self::Failed),
            _ => Err(DomainError::Invalid {
                field: "outcome",
                reason: "must be success, denied, or failed".to_string(),
            }),
        }
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Denied => "denied",
            Self::Failed => "failed",
        }
    }
}

/// How important the event is to an operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sensitivity {
    /// Reviewed in the ordinary course.
    Routine,
    /// Reviewed first.
    Security,
}

impl Sensitivity {
    /// Parses the stored value.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for anything else.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "routine" => Ok(Self::Routine),
            "security" => Ok(Self::Security),
            _ => Err(DomainError::Invalid {
                field: "sensitivity",
                reason: "must be routine or security".to_string(),
            }),
        }
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Routine => "routine",
            Self::Security => "security",
        }
    }
}

/// The failure class the audit service derives from sensitivity (ADR-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventClass {
    /// Must be stored before the caller's action commits.
    Security,
    /// Best-effort; the caller's action proceeds if the append fails.
    Domain,
}

impl EventClass {
    /// Derives the class from sensitivity. The caller never chooses it.
    #[must_use]
    pub fn for_sensitivity(sensitivity: Sensitivity) -> Self {
        match sensitivity {
            Sensitivity::Security => Self::Security,
            Sensitivity::Routine => Self::Domain,
        }
    }

    /// Parses the stored value.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for anything else.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "security" => Ok(Self::Security),
            "domain" => Ok(Self::Domain),
            _ => Err(DomainError::Invalid {
                field: "eventClass",
                reason: "must be security or domain".to_string(),
            }),
        }
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Security => "security",
            Self::Domain => "domain",
        }
    }
}

/// The fields an emitter supplies when appending an event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAuditEvent {
    /// The acting user or support agent; absent for system events.
    pub actor_id: Option<UserId>,
    /// Who the actor is.
    pub actor_kind: ActorKind,
    /// The dotted action name.
    pub action: String,
    /// The logical identifier the action applied to.
    pub target: String,
    /// When the action happened; `None` records the receipt time.
    pub occurred_at: Option<DateTime<Utc>>,
    /// Correlation and identifier metadata only.
    pub context: Context,
    /// How the action turned out.
    pub outcome: Outcome,
    /// How important the event is.
    pub sensitivity: Sensitivity,
    /// A caller-supplied class, checked against the derived one and never trusted.
    pub event_class: Option<EventClass>,
    /// The emitter's retry key, when it sent one.
    pub idempotency_key: Option<String>,
    /// The legal basis this record is kept under past a deletion request.
    pub legal_basis: Option<String>,
    /// The deletion request this record outlived.
    pub deletion_request_id: Option<DeletionRequestId>,
}

/// One stored audit event. Every field is set at insert time and never changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEvent {
    id: AuditEventId,
    actor_id: Option<UserId>,
    actor_kind: ActorKind,
    action: String,
    target: String,
    occurred_at: DateTime<Utc>,
    recorded_at: DateTime<Utc>,
    context: Context,
    outcome: Outcome,
    sensitivity: Sensitivity,
    event_class: EventClass,
    idempotency_key: Option<String>,
    legal_basis: Option<String>,
    deletion_request_id: Option<DeletionRequestId>,
}

impl AuditEvent {
    /// Validates and records an event at the audit service's receipt time.
    ///
    /// # Errors
    /// Returns [`DomainError`] naming each failing field path; a rejection stores
    /// nothing.
    pub fn record(
        new: NewAuditEvent,
        id: AuditEventId,
        recorded_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let action = validate_action(&new.action)?;
        let target = validate_target(&new.target)?;

        if new.actor_kind == ActorKind::System {
            if new.actor_id.is_some() {
                return Err(DomainError::Contradiction {
                    field: "actorId",
                    reason: "a system event carries no actorId".to_string(),
                });
            }
            if !SYSTEM_ACTION_ALLOW_LIST.contains(&action.as_str()) {
                return Err(DomainError::Contradiction {
                    field: "action",
                    reason: "a system-attributed event must use an allow-listed action".to_string(),
                });
            }
        } else if new.actor_id.is_none() {
            return Err(DomainError::Required { field: "actorId" });
        }

        let derived = EventClass::for_sensitivity(new.sensitivity);
        if let Some(supplied) = new.event_class {
            if supplied != derived {
                return Err(DomainError::Contradiction {
                    field: "eventClass",
                    reason: format!(
                        "eventClass {} contradicts the class derived from sensitivity ({})",
                        supplied.as_str(),
                        derived.as_str()
                    ),
                });
            }
        }

        let idempotency_key = validate_idempotency_key(new.idempotency_key)?;
        let legal_basis = validate_legal_basis(new.legal_basis)?;

        Ok(Self {
            id,
            actor_id: new.actor_id,
            actor_kind: new.actor_kind,
            action,
            target,
            occurred_at: new.occurred_at.unwrap_or(recorded_at),
            recorded_at,
            context: new.context,
            outcome: new.outcome,
            sensitivity: new.sensitivity,
            event_class: derived,
            idempotency_key,
            legal_basis,
            deletion_request_id: new.deletion_request_id,
        })
    }

    /// The event identifier.
    #[must_use]
    pub fn id(&self) -> AuditEventId {
        self.id
    }

    /// The acting user, when there is one.
    #[must_use]
    pub fn actor_id(&self) -> Option<UserId> {
        self.actor_id
    }

    /// Who the actor is.
    #[must_use]
    pub fn actor_kind(&self) -> ActorKind {
        self.actor_kind
    }

    /// The dotted action name.
    #[must_use]
    pub fn action(&self) -> &str {
        &self.action
    }

    /// The logical target identifier.
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    /// When the action happened.
    #[must_use]
    pub fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }

    /// When the audit service stored the event.
    #[must_use]
    pub fn recorded_at(&self) -> DateTime<Utc> {
        self.recorded_at
    }

    /// The event metadata.
    #[must_use]
    pub fn context(&self) -> &Context {
        &self.context
    }

    /// How the action turned out.
    #[must_use]
    pub fn outcome(&self) -> Outcome {
        self.outcome
    }

    /// How important the event is.
    #[must_use]
    pub fn sensitivity(&self) -> Sensitivity {
        self.sensitivity
    }

    /// The derived failure class.
    #[must_use]
    pub fn event_class(&self) -> EventClass {
        self.event_class
    }

    /// The emitter's retry key.
    #[must_use]
    pub fn idempotency_key(&self) -> Option<&str> {
        self.idempotency_key.as_deref()
    }

    /// The legal basis this record is kept under.
    #[must_use]
    pub fn legal_basis(&self) -> Option<&str> {
        self.legal_basis.as_deref()
    }

    /// The deletion request this record outlived.
    #[must_use]
    pub fn deletion_request_id(&self) -> Option<DeletionRequestId> {
        self.deletion_request_id
    }

    /// A deterministic representation used to compare retried payloads (BR2.2).
    #[must_use]
    pub fn canonical_payload(&self) -> String {
        format!(
            "actorId={};actorKind={};action={};target={};occurredAt={};outcome={};sensitivity={};legalBasis={};deletionRequestId={};context={}",
            self.actor_id.map(|id| id.to_string()).unwrap_or_default(),
            self.actor_kind.as_str(),
            self.action,
            self.target,
            self.occurred_at
                .to_rfc3339_opts(SecondsFormat::Millis, true),
            self.outcome.as_str(),
            self.sensitivity.as_str(),
            self.legal_basis.as_deref().unwrap_or(""),
            self.deletion_request_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
            self.context.canonical(),
        )
    }
}

fn validate_action(raw: &str) -> Result<String, DomainError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(DomainError::Required { field: "action" });
    }
    if value.len() > MAX_ACTION_LEN {
        return Err(DomainError::TooLarge {
            field: "action",
            limit: "120 characters",
        });
    }
    let shaped = value.contains('.')
        && !value.starts_with('.')
        && !value.ends_with('.')
        && !value.contains("..");
    if !shaped
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
    {
        return Err(DomainError::Invalid {
            field: "action",
            reason: "must be a lowercase dotted noun.verb name".to_string(),
        });
    }
    Ok(value.to_string())
}

fn validate_target(raw: &str) -> Result<String, DomainError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(DomainError::Required { field: "target" });
    }
    if value.len() > MAX_TARGET_LEN {
        return Err(DomainError::TooLarge {
            field: "target",
            limit: "240 characters",
        });
    }
    if value.chars().any(char::is_control) {
        return Err(DomainError::Invalid {
            field: "target",
            reason: "must not contain control characters".to_string(),
        });
    }
    ensure_secret_free("target", value)?;
    Ok(value.to_string())
}

fn validate_idempotency_key(raw: Option<String>) -> Result<Option<String>, DomainError> {
    let Some(value) = raw else {
        return Ok(None);
    };
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(DomainError::Required {
            field: "idempotencyKey",
        });
    }
    if value.len() > MAX_IDEMPOTENCY_KEY_LEN {
        return Err(DomainError::TooLarge {
            field: "idempotencyKey",
            limit: "128 characters",
        });
    }
    if value.chars().any(char::is_control) {
        return Err(DomainError::Invalid {
            field: "idempotencyKey",
            reason: "must not contain control characters".to_string(),
        });
    }
    Ok(Some(value))
}

fn validate_legal_basis(raw: Option<String>) -> Result<Option<String>, DomainError> {
    let Some(value) = raw else {
        return Ok(None);
    };
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(DomainError::Required {
            field: "legalBasis",
        });
    }
    if value.len() > MAX_LEGAL_BASIS_LEN {
        return Err(DomainError::TooLarge {
            field: "legalBasis",
            limit: "120 characters",
        });
    }
    if value.chars().any(char::is_control) {
        return Err(DomainError::Invalid {
            field: "legalBasis",
            reason: "must not contain control characters".to_string(),
        });
    }
    ensure_secret_free("legalBasis", &value)?;
    Ok(Some(value))
}

/// Rebuilds an event from storage without re-validating the emitter input.
///
/// # Errors
/// Returns [`DomainError`] when a stored value cannot be represented, which the
/// adapter surfaces as invalid data rather than silently repairing.
#[allow(clippy::too_many_arguments)]
pub fn from_stored(
    id: AuditEventId,
    actor_id: Option<UserId>,
    actor_kind: ActorKind,
    action: String,
    target: String,
    occurred_at: DateTime<Utc>,
    recorded_at: DateTime<Utc>,
    context: BTreeMap<String, String>,
    outcome: Outcome,
    sensitivity: Sensitivity,
    event_class: EventClass,
    idempotency_key: Option<String>,
    legal_basis: Option<String>,
    deletion_request_id: Option<DeletionRequestId>,
) -> Result<AuditEvent, DomainError> {
    let context = Context::parse(context)?;
    if EventClass::for_sensitivity(sensitivity) != event_class {
        return Err(DomainError::Contradiction {
            field: "eventClass",
            reason: "stored event class contradicts stored sensitivity".to_string(),
        });
    }
    Ok(AuditEvent {
        id,
        actor_id,
        actor_kind,
        action,
        target,
        occurred_at,
        recorded_at,
        context,
        outcome,
        sensitivity,
        event_class,
        idempotency_key,
        legal_basis,
        deletion_request_id,
    })
}
