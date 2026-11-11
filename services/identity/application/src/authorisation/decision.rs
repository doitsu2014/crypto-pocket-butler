//! The evaluation order (W1): declared, status, support, class, ownership.
//!
//! The status read happens after the catalogue check and before any role rule,
//! so an unknown operation is denied without touching the status source. Every
//! deny is written through [`AuditSink`] before it returns; a failed write turns
//! the denial into `audit_unavailable` (BR2.11).

use std::sync::Arc;
use std::time::Instant;

use identity_domain::authorisation::{
    CallerIdentity, Decision, DenialReason, Operation, OperationClass, ResourceOwnership,
    ResourceScope, Role,
};
use identity_domain::{UserId, UserStatus};

use crate::authorisation::catalogue::OperationCatalogue;
use crate::authorisation::ports::{
    AuditSink, AuditWriteContext, AuthorisationEvent, ObservationSink, StatusReader,
};
use crate::ports::Clock;

/// A failure that is not a decision: the status source could not answer, so the
/// caller answers 503 rather than inventing a denial.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecisionError {
    /// The status read timed out, is unavailable, or returned malformed data.
    #[error("the caller's status is unavailable: {0}")]
    StatusUnavailable(String),
}

/// The in-process decision every Identity module calls, and the C4 adapter
/// wraps.
pub struct AuthorisationDecision {
    catalogue: OperationCatalogue,
    status: Arc<dyn StatusReader>,
    audit: Arc<dyn AuditSink>,
    observation: Arc<dyn ObservationSink>,
    clock: Arc<dyn Clock>,
}

impl AuthorisationDecision {
    /// Creates the decision.
    #[must_use]
    pub fn new(
        catalogue: OperationCatalogue,
        status: Arc<dyn StatusReader>,
        audit: Arc<dyn AuditSink>,
        observation: Arc<dyn ObservationSink>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            catalogue,
            status,
            audit,
            observation,
            clock,
        }
    }

    /// The catalogue, so the HTTP adapter classes the request's operation with
    /// the same table the decision uses.
    #[must_use]
    pub fn catalogue(&self) -> &OperationCatalogue {
        &self.catalogue
    }

    /// Evaluates one request.
    ///
    /// # Errors
    /// Returns [`DecisionError::StatusUnavailable`] when the status source
    /// cannot answer; the request fails closed and no decision is made.
    pub async fn decide(
        &self,
        caller: &CallerIdentity,
        operation: &Operation,
        ownership: Option<&ResourceOwnership>,
        correlation_id: &str,
    ) -> Result<Decision, DecisionError> {
        let started = Instant::now();
        // The class comes from the catalogue, never from the value the caller
        // constructed, so a request cannot promote itself.
        let class = self.catalogue.class_of(operation.name());
        let outcome = self
            .evaluate(caller, operation, class, ownership, correlation_id)
            .await;
        if let Ok(decision) = &outcome {
            self.observation
                .record_decision(class, decision.reason(), started.elapsed());
        }
        outcome
    }

    async fn evaluate(
        &self,
        caller: &CallerIdentity,
        operation: &Operation,
        class: OperationClass,
        ownership: Option<&ResourceOwnership>,
        correlation_id: &str,
    ) -> Result<Decision, DecisionError> {
        // 1. Declared? (BR2.1)
        if class == OperationClass::Undeclared {
            return self
                .deny(
                    caller,
                    operation,
                    ownership,
                    DenialReason::NotDeclared,
                    correlation_id,
                )
                .await;
        }

        // 2. Status? (BR2.7) A user caller's status is read once, after the
        // catalogue check. A missing row fails closed. The reader (the outage
        // cache in production) reports a failed read to the metrics sink.
        if let CallerIdentity::User { user_id, .. } = caller {
            let status = self
                .status
                .status_of(*user_id)
                .await
                .map_err(|error| DecisionError::StatusUnavailable(error.to_string()))?;
            if status_denies(status, class) {
                return self
                    .deny(
                        caller,
                        operation,
                        ownership,
                        DenialReason::StatusDenied,
                        correlation_id,
                    )
                    .await;
            }
        }

        // 3. Support? (BR2.4, BR2.14) Support is never an owner, never reads a
        // collection, and never reaches administration or confirmation. Its own
        // metadata class is the one exception: the u3 grant check decides it.
        if caller.role() == Role::Support && support_is_barred(class) {
            return self
                .deny(
                    caller,
                    operation,
                    ownership,
                    DenialReason::NoRole,
                    correlation_id,
                )
                .await;
        }

        // 4–7. The class decides the rest.
        match class {
            OperationClass::Administrative => {
                if caller.role().is_admin() {
                    Ok(Decision::allow(None))
                } else {
                    self.deny(
                        caller,
                        operation,
                        ownership,
                        DenialReason::NoRole,
                        correlation_id,
                    )
                    .await
                }
            }
            // Confirmation is the one operation a `pending` account reaches; a
            // job never requests it.
            OperationClass::Confirmation => {
                if caller.is_system() {
                    self.deny(
                        caller,
                        operation,
                        ownership,
                        DenialReason::NoRole,
                        correlation_id,
                    )
                    .await
                } else {
                    Ok(Decision::allow(None))
                }
            }
            // The support console's metadata read: the grant check is u3's, so
            // the decision only keeps the support role from being barred here.
            OperationClass::SupportMetadata => {
                if caller.is_system() {
                    self.deny(
                        caller,
                        operation,
                        ownership,
                        DenialReason::NoRole,
                        correlation_id,
                    )
                    .await
                } else {
                    Ok(Decision::allow(None))
                }
            }
            // A collection is allowed with the owner filter the owning service
            // must apply to every row (BR2.5, BR2.8).
            OperationClass::Collection => {
                let filter = match caller {
                    CallerIdentity::User { user_id, .. } => Some(*user_id),
                    // A job iterates one owner at a time; the filter is that
                    // owner, never "everyone".
                    CallerIdentity::System { owner, .. } => Some(*owner),
                };
                Ok(Decision::allow(filter))
            }
            // One row: the owner must equal the caller (BR2.3, BR2.5, BR2.8).
            OperationClass::OwnerScoped => {
                if owned_by_caller(caller, ownership) {
                    Ok(Decision::allow(None))
                } else {
                    self.deny(
                        caller,
                        operation,
                        ownership,
                        DenialReason::NotOwner,
                        correlation_id,
                    )
                    .await
                }
            }
            OperationClass::Undeclared => Ok(Decision::deny(DenialReason::NotDeclared)),
        }
    }

    /// Writes the audit entry, then returns the denial. An audit failure returns
    /// `audit_unavailable`, never a silent denial and never an allow.
    async fn deny(
        &self,
        caller: &CallerIdentity,
        operation: &Operation,
        ownership: Option<&ResourceOwnership>,
        reason: DenialReason,
        correlation_id: &str,
    ) -> Result<Decision, DecisionError> {
        let event = AuthorisationEvent::denied(
            caller.actor_label(),
            operation.name(),
            denial_target(caller, operation, ownership),
            self.clock.now(),
            reason,
        );
        match self.audit.record(&event, correlation_id).await {
            Ok(()) => Ok(Decision::deny(reason)),
            Err(error) => {
                self.observation
                    .record_audit_write_failure(AuditWriteContext::Denial);
                tracing::warn!(
                    correlation_id,
                    operation = operation.name(),
                    %error,
                    "a denial could not be audited; failing closed"
                );
                Ok(Decision::audit_unavailable())
            }
        }
    }
}

/// Whether the account's status denies the operation (BR2.7). A missing record
/// fails closed; `pending` permits only confirmation.
fn status_denies(status: Option<UserStatus>, class: OperationClass) -> bool {
    match status {
        None => true,
        Some(status) => {
            status.denies_use()
                || (status == UserStatus::Pending && class != OperationClass::Confirmation)
        }
    }
}

/// Whether the support role is barred from the operation's class (BR2.14).
fn support_is_barred(class: OperationClass) -> bool {
    matches!(
        class,
        OperationClass::OwnerScoped
            | OperationClass::Collection
            | OperationClass::Administrative
            | OperationClass::Confirmation
    )
}

/// Whether the ownership assertion names the owner the caller may act for. A
/// job is bound to the one owner it iterates for and cannot reach another's row
/// (BR2.8).
fn owned_by_caller(caller: &CallerIdentity, ownership: Option<&ResourceOwnership>) -> bool {
    let Some(ownership) = ownership else {
        return false;
    };
    if ownership.scope() != ResourceScope::Resource {
        return false;
    }
    let Some(resource_owner) = ownership.resource_owner_id() else {
        return false;
    };
    caller.owner() == Some(resource_owner)
}

/// The audit target: the resource kind and the caller's own id, never another
/// user's id (BR2.12).
fn denial_target(
    caller: &CallerIdentity,
    operation: &Operation,
    ownership: Option<&ResourceOwnership>,
) -> String {
    let actor = caller
        .user_id()
        .map(|id: UserId| id.to_string())
        .unwrap_or_else(|| "system".to_string());
    match ownership {
        Some(ownership) if ownership.scope() == ResourceScope::Resource => {
            format!("{}:{actor}", ownership.resource_kind())
        }
        Some(ownership) => format!("{}:collection:{actor}", ownership.resource_kind()),
        None => format!("{}:{actor}", operation.name()),
    }
}
