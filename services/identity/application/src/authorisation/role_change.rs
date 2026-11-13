//! The role-change use case: intent audit, provider write, outcome audit
//! (BR2.10).
//!
//! The ordering is the rule: nothing is written to the provider until the intent
//! entry is recorded, and a provider failure records `failed` rather than
//! leaving a privileged change unrecorded.

use std::sync::Arc;

use identity_domain::authorisation::CallerIdentity;
use identity_domain::{AuditOutcome, Role, UserId};

use crate::authorisation::ports::{
    AuditSink, AuditWriteContext, AuthorisationEvent, ObservationSink, RoleWriter, MAX_REASON_LEN,
};
use crate::common::{AppError, FieldError};
use crate::ports::{Clock, ExternalError, RepositoryError, UserRepository};

/// The applied change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleChangeView {
    user_id: UserId,
    previous_role: Role,
    new_role: Role,
}

impl RoleChangeView {
    /// The target user.
    #[must_use]
    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    /// The role before the change.
    #[must_use]
    pub fn previous_role(&self) -> Role {
        self.previous_role
    }

    /// The role after the change.
    #[must_use]
    pub fn new_role(&self) -> Role {
        self.new_role
    }
}

/// One admin-initiated role change.
pub struct RoleChange {
    users: Arc<dyn UserRepository>,
    roles: Arc<dyn RoleWriter>,
    audit: Arc<dyn AuditSink>,
    observation: Arc<dyn ObservationSink>,
    clock: Arc<dyn Clock>,
}

impl RoleChange {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        users: Arc<dyn UserRepository>,
        roles: Arc<dyn RoleWriter>,
        audit: Arc<dyn AuditSink>,
        observation: Arc<dyn ObservationSink>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            users,
            roles,
            audit,
            observation,
            clock,
        }
    }

    /// Changes one user's provider role.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for a non-admin, [`AppError::Validation`]
    /// for a blank or over-long reason or an unassignable role,
    /// [`AppError::NotFound`] for an unknown target, and [`AppError::Unavailable`]
    /// when the audit entry or the provider write fails.
    pub async fn change(
        &self,
        actor: &CallerIdentity,
        target: UserId,
        new_role: Role,
        reason: &str,
        correlation_id: &str,
    ) -> Result<RoleChangeView, AppError> {
        let reason = reason.trim();

        // BR2.4: administration is reachable only with the admin role. The
        // refusal is audited; a denial that cannot be audited fails closed.
        if !actor.role().is_admin() {
            let event = AuthorisationEvent::role_change_requested(
                actor.actor_label(),
                target.to_string(),
                self.clock.now(),
                None,
                new_role,
                reason,
                AuditOutcome::Denied,
            );
            if let Err(error) = self.audit.record(&event, correlation_id).await {
                return Err(self.audit_failure(error));
            }
            return Err(AppError::Forbidden(
                "only an admin may change a role".to_string(),
            ));
        }

        // BR2.10: the reason is required and bounded; nothing is written before
        // it validates.
        if reason.is_empty() {
            return Err(AppError::Validation(vec![FieldError::new(
                "reason",
                "is required",
            )]));
        }
        if reason.chars().count() > MAX_REASON_LEN {
            return Err(AppError::Validation(vec![FieldError::new(
                "reason",
                format!("must be at most {MAX_REASON_LEN} characters"),
            )]));
        }
        // A caller cannot assign the job-only system role.
        if new_role.is_system() {
            return Err(AppError::Validation(vec![FieldError::new(
                "newRole",
                "must be owner, admin, or support",
            )]));
        }

        let user = self
            .users
            .find_by_id(target)
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("user"))?;
        let previous_role = user.role();
        let subject = self
            .users
            .reveal_auth_subject(target)
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("user"))?;

        // The intent entry comes first. If it cannot be recorded, the provider
        // is never touched.
        let intent = AuthorisationEvent::role_change_requested(
            actor.actor_label(),
            target.to_string(),
            self.clock.now(),
            Some(previous_role),
            new_role,
            reason,
            AuditOutcome::Success,
        );
        if let Err(error) = self.audit.record(&intent, correlation_id).await {
            return Err(self.audit_failure(error));
        }

        match self.roles.set_role(&subject, new_role).await {
            Ok(()) => {
                let outcome = AuthorisationEvent::role_change_outcome(
                    actor.actor_label(),
                    target.to_string(),
                    self.clock.now(),
                    previous_role,
                    new_role,
                    reason,
                    AuditOutcome::Success,
                    None,
                );
                let recorded = self.audit.record(&outcome, correlation_id).await;
                self.observation.record_role_change(true);
                if let Err(error) = recorded {
                    return Err(self.audit_failure(error));
                }
                Ok(RoleChangeView {
                    user_id: target,
                    previous_role,
                    new_role,
                })
            }
            Err(error) => {
                let cause = failure_cause(&error);
                let outcome = AuthorisationEvent::role_change_outcome(
                    actor.actor_label(),
                    target.to_string(),
                    self.clock.now(),
                    previous_role,
                    new_role,
                    reason,
                    AuditOutcome::Failed,
                    Some(cause),
                );
                if let Err(audit_error) = self.audit.record(&outcome, correlation_id).await {
                    self.observation
                        .record_audit_write_failure(AuditWriteContext::RoleChange);
                    tracing::error!(
                        correlation_id,
                        %audit_error,
                        "the failed role change could not record its outcome"
                    );
                }
                self.observation.record_role_change(false);
                tracing::warn!(
                    correlation_id,
                    target = %target,
                    %error,
                    "the provider refused the role change"
                );
                Err(AppError::from_external(error))
            }
        }
    }

    /// Any audit failure on this path is a 503. A privileged change is never
    /// completed unrecorded, and the audit adapter's redaction refusal is not a
    /// client mistake, so it must not surface as a 422.
    fn audit_failure(&self, error: RepositoryError) -> AppError {
        self.observation
            .record_audit_write_failure(AuditWriteContext::RoleChange);
        tracing::warn!(%error, "the role change could not be audited");
        AppError::Unavailable("the role change could not be audited".to_string())
    }
}

/// The enum the audit record carries instead of a raw provider message.
fn failure_cause(error: &ExternalError) -> &'static str {
    match error {
        ExternalError::Refused(_) => "provider_refused",
        ExternalError::Unavailable(_) => "provider_unavailable",
    }
}
