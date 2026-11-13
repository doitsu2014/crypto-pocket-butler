//! WF1 — principal resolution and just-in-time provisioning (BR1.1–BR1.5).

use std::sync::Arc;

use chrono::{DateTime, Utc};
use identity_domain::{
    AuditOutcome, DeviceSession, DeviceSessionId, User, UserId, UserStatus, VerifiedClaims,
};

use crate::common::{AppError, Caller, FieldError, PrincipalView};
use crate::ports::{
    Clock, DeviceSessionRepository, GateRefusal, JurisdictionGate, RepositoryError, UserRepository,
};
use crate::trail::{context, Trail};

/// The C3 contact answer: the address only when Identity has verified it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactView {
    /// The user the answer concerns.
    pub user_id: UserId,
    /// The verified address, or `None` when there is none or it is unconfirmed.
    pub email: Option<String>,
    /// Whether a confirmed address exists.
    pub verified: bool,
}

/// Resolves a validated JWT to an internal principal, provisioning on first use.
pub struct PrincipalService {
    users: Arc<dyn UserRepository>,
    sessions: Arc<dyn DeviceSessionRepository>,
    trail: Arc<Trail>,
    gate: Arc<dyn JurisdictionGate>,
    clock: Arc<dyn Clock>,
}

impl PrincipalService {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        users: Arc<dyn UserRepository>,
        sessions: Arc<dyn DeviceSessionRepository>,
        trail: Arc<Trail>,
        gate: Arc<dyn JurisdictionGate>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            users,
            sessions,
            trail,
            gate,
            clock,
        }
    }

    /// Resolves the principal, creating the user on the first authenticated
    /// request.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for a denied status and
    /// [`AppError::Unavailable`] when the store cannot answer.
    pub async fn resolve(
        &self,
        claims: VerifiedClaims,
        correlation_id: &str,
    ) -> Result<PrincipalView, AppError> {
        Ok(self.resolve_with_origin(claims, correlation_id).await?.0)
    }

    /// Resolves the principal and reports whether this request created the
    /// record, so the C3 PUT can answer 201 on a first provision and 200 on a
    /// refresh.
    ///
    /// # Errors
    /// Returns the same refusals as [`Self::resolve`].
    pub async fn resolve_with_origin(
        &self,
        claims: VerifiedClaims,
        correlation_id: &str,
    ) -> Result<(PrincipalView, bool), AppError> {
        let now = self.clock.now();
        let existing = match self.users.find_by_auth_subject(&claims.subject).await {
            Ok(existing) => existing,
            Err(RepositoryError::Validation(detail)) => {
                // A hash hit that names another subject is a security event: the
                // record is never returned and nothing is created (fail closed).
                self.trail
                    .record(
                        None,
                        "identity.principal.collision",
                        "subject",
                        AuditOutcome::Denied,
                        correlation_id,
                        context(&[("reason", "subject_hash_collision")]),
                    )
                    .await?;
                return Err(AppError::Validation(vec![FieldError::new(
                    "subject", detail,
                )]));
            }
            Err(error) => return Err(AppError::from_repository(error)),
        };

        let (user, next_step, created) = match existing {
            Some(user) => {
                let (user, next_step) = self.refresh(user, &claims, now, correlation_id).await?;
                (user, next_step, false)
            }
            None => {
                let (user, next_step) = self.provision(&claims, now, correlation_id).await?;
                (user, next_step, true)
            }
        };

        self.record_session(&user, &claims, now).await;

        Ok((
            PrincipalView::new(user.id(), user.role(), user.status(), next_step),
            created,
        ))
    }

    /// C3: the verified contact for one user, read from the stored record.
    ///
    /// Notification calls this at delivery time and keeps the address only in
    /// memory (NFR9.3). A deleted or unknown user is `NotFound`; an unconfirmed
    /// address answers `verified: false` with no address.
    ///
    /// # Errors
    /// Returns [`AppError::NotFound`] for a deleted or unknown user and
    /// [`AppError::Unavailable`] when the store cannot answer.
    pub async fn contact(&self, user_id: UserId) -> Result<ContactView, AppError> {
        let user = self
            .users
            .find_by_id(user_id)
            .await
            .map_err(AppError::from_repository)?
            .filter(|user| !user.is_deleted())
            .ok_or(AppError::NotFound("user"))?;
        let email = user.verified_email().map(ToString::to_string);
        Ok(ContactView {
            user_id,
            verified: email.is_some(),
            email,
        })
    }

    /// Resolves the principal and loads the stored user for a use case.
    ///
    /// # Errors
    /// Returns the same refusals as [`Self::resolve`].
    pub async fn load_caller(
        &self,
        claims: VerifiedClaims,
        correlation_id: &str,
    ) -> Result<Caller, AppError> {
        let view = self.resolve(claims.clone(), correlation_id).await?;
        let user = self
            .users
            .find_by_id(view.user_id())
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("user"))?;
        Ok(Caller {
            user,
            claims,
            correlation_id: correlation_id.to_string(),
        })
    }

    async fn provision(
        &self,
        claims: &VerifiedClaims,
        now: DateTime<Utc>,
        correlation_id: &str,
    ) -> Result<(User, identity_domain::NextStep), AppError> {
        // The jurisdiction gate clears the provider before any personal data is
        // written (BR1.1, C8). The gate is in-process; a refusal stores nothing.
        self.require_gate_clearance(claims, correlation_id).await?;

        let user_id = UserId::generate();
        let user = User::provision(user_id, claims, now);

        match self.users.provision(&user, &claims.subject, now).await {
            Ok(_key) => {
                self.trail
                    .record(
                        Some(user.id()),
                        "identity.user.provisioned",
                        user.id().to_string(),
                        AuditOutcome::Success,
                        correlation_id,
                        context(&[("role", claims.role.as_str())]),
                    )
                    .await?;
            }
            Err(RepositoryError::Conflict(_)) => {
                let existing = self
                    .users
                    .find_by_auth_subject(&claims.subject)
                    .await
                    .map_err(AppError::from_repository)?
                    .ok_or_else(|| {
                        AppError::Internal("a concurrent provision vanished".to_string())
                    })?;
                return self.refresh(existing, claims, now, correlation_id).await;
            }
            Err(error) => return Err(AppError::from_repository(error)),
        }

        // The record starts active when the provider's facts support it; the
        // first authenticated request is the provisioning event itself.
        if user.is_active() {
            self.trail
                .record(
                    Some(user.id()),
                    "identity.user.activated",
                    user.id().to_string(),
                    AuditOutcome::Success,
                    correlation_id,
                    context(&[]),
                )
                .await?;
        }
        Ok((user, claims.next_step()))
    }

    /// Asks the in-process gate to clear the caller's declared country and the
    /// address the trusted caller observed (C8).
    async fn require_gate_clearance(
        &self,
        claims: &VerifiedClaims,
        correlation_id: &str,
    ) -> Result<(), AppError> {
        let (Some(country_code), Some(client_ip)) =
            (claims.country_code.as_deref(), claims.client_ip.as_deref())
        else {
            return Err(AppError::Validation(vec![FieldError::new(
                "jurisdiction",
                "the caller must declare a country and present the observed address",
            )]));
        };
        match self
            .gate
            .clear(country_code, client_ip, correlation_id)
            .await
        {
            Ok(()) => Ok(()),
            Err(GateRefusal::Denied(reason) | GateRefusal::Invalid(reason)) => {
                self.trail
                    .record(
                        None,
                        "identity.user.provision_denied",
                        "jurisdiction",
                        AuditOutcome::Denied,
                        correlation_id,
                        context(&[("reason", "gate_not_cleared")]),
                    )
                    .await?;
                Err(AppError::Validation(vec![FieldError::new(
                    "jurisdiction",
                    reason,
                )]))
            }
            Err(GateRefusal::Unavailable(detail)) => Err(AppError::Unavailable(detail)),
        }
    }

    async fn refresh(
        &self,
        mut user: User,
        claims: &VerifiedClaims,
        now: DateTime<Utc>,
        correlation_id: &str,
    ) -> Result<(User, identity_domain::NextStep), AppError> {
        // The tombstone outcome: the subject hash matched an erased account
        // (BR5.8, AC4.5.1). It is denied, never provisioned again, and audited
        // as its own event so an operator can see the difference between an
        // erased subject and a disabled one.
        if user.status() == UserStatus::Deleted {
            self.trail
                .record(
                    Some(user.id()),
                    "identity.principal.erased",
                    user.id().to_string(),
                    AuditOutcome::Denied,
                    correlation_id,
                    context(&[("reason", "subject_erased")]),
                )
                .await?;
            return Err(AppError::Forbidden(
                "this account has been deleted".to_string(),
            ));
        }
        if user.status().denies_use() {
            let reason = user.status().as_str();
            self.trail
                .record(
                    Some(user.id()),
                    "identity.principal.denied",
                    user.id().to_string(),
                    AuditOutcome::Denied,
                    correlation_id,
                    context(&[("reason", reason)]),
                )
                .await?;
            return Err(AppError::Forbidden(format!("this account is {reason}")));
        }

        let previous_role = user.role();
        let previous_status = user.status();
        let next_step = user
            .apply_claims(claims, now)
            .map_err(AppError::from_domain)?;
        let role_changed = user.role() != previous_role;
        let activated =
            user.status() == UserStatus::Active && previous_status != UserStatus::Active;

        // The stored row follows the token's role and the guarded Pending →
        // Active promotion. Role changes are not a registry event: the
        // authorisation unit owns them (no identity.role.changed here).
        if role_changed || activated {
            self.users
                .update(&user)
                .await
                .map_err(AppError::from_repository)?;
        }
        if activated {
            self.trail
                .record(
                    Some(user.id()),
                    "identity.user.activated",
                    user.id().to_string(),
                    AuditOutcome::Success,
                    correlation_id,
                    context(&[]),
                )
                .await?;
        }
        Ok((user, next_step))
    }

    /// Records the device session on each authenticated request (BR8.1). A
    /// failure here is logged rather than blocking principal resolution: the
    /// session view is not the authentication path.
    async fn record_session(&self, user: &User, claims: &VerifiedClaims, now: DateTime<Utc>) {
        let Some(session_id) = claims.session_id.as_deref() else {
            return;
        };
        let session = DeviceSession::new(
            DeviceSessionId::generate(),
            user.id(),
            session_id,
            claims.user_agent.clone(),
            claims.location.clone(),
            now,
        );
        if let Err(error) = self.sessions.upsert(&session).await {
            tracing::warn!(%error, "failed to record the device session");
        }
    }
}
