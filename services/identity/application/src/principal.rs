//! WF1 — principal resolution and just-in-time provisioning (BR1.1–BR1.5).

use std::sync::Arc;

use chrono::{DateTime, Utc};
use identity_domain::{
    AuditOutcome, DeviceSession, DeviceSessionId, User, UserId, UserStatus, VerifiedClaims,
};

use crate::common::{AppError, Caller, PrincipalView};
use crate::ports::{
    Clock, DeviceSessionRepository, RepositoryError, ShreddingKeyRepository, UserRepository,
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
    keys: Arc<dyn ShreddingKeyRepository>,
    sessions: Arc<dyn DeviceSessionRepository>,
    trail: Arc<Trail>,
    clock: Arc<dyn Clock>,
}

impl PrincipalService {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        users: Arc<dyn UserRepository>,
        keys: Arc<dyn ShreddingKeyRepository>,
        sessions: Arc<dyn DeviceSessionRepository>,
        trail: Arc<Trail>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            users,
            keys,
            sessions,
            trail,
            clock,
        }
    }

    /// Resolves the principal, creating the user on the first authenticated
    /// request.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for a deleted account and
    /// [`AppError::Unavailable`] when the store cannot answer.
    pub async fn resolve(
        &self,
        claims: VerifiedClaims,
        correlation_id: &str,
    ) -> Result<PrincipalView, AppError> {
        let now = self.clock.now();
        let existing = self
            .users
            .find_by_auth_subject(&claims.subject)
            .await
            .map_err(AppError::from_repository)?;

        let (user, next_step) = match existing {
            Some(user) => self.refresh(user, &claims, now, correlation_id).await?,
            None => self.provision(&claims, now, correlation_id).await?,
        };

        self.record_session(&user, &claims, now).await;

        Ok(PrincipalView::new(
            user.id(),
            user.role(),
            user.status(),
            next_step,
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
        let user_id = UserId::generate();
        let key = self
            .keys
            .create_for_user(user_id, now)
            .await
            .map_err(AppError::from_repository)?;
        let mut user = User::provision(user_id, claims, now);

        match self.users.insert(&user, &claims.subject, &key).await {
            Ok(()) => {
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

        let next_step = user
            .apply_claims(claims, now)
            .map_err(AppError::from_domain)?;
        if user.is_active() {
            self.users
                .update(&user)
                .await
                .map_err(AppError::from_repository)?;
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

    async fn refresh(
        &self,
        mut user: User,
        claims: &VerifiedClaims,
        now: DateTime<Utc>,
        correlation_id: &str,
    ) -> Result<(User, identity_domain::NextStep), AppError> {
        if user.status() == UserStatus::Deleted {
            self.trail
                .record(
                    Some(user.id()),
                    "identity.principal.denied",
                    user.id().to_string(),
                    AuditOutcome::Denied,
                    correlation_id,
                    context(&[("reason", "deleted")]),
                )
                .await?;
            return Err(AppError::Forbidden(
                "this account has been deleted".to_string(),
            ));
        }

        let previous_role = user.role();
        let previous_status = user.status();
        let next_step = user
            .apply_claims(claims, now)
            .map_err(AppError::from_domain)?;
        let role_changed = user.role() != previous_role;
        let activated =
            user.status() == UserStatus::Active && previous_status != UserStatus::Active;

        if role_changed || activated {
            self.users
                .update(&user)
                .await
                .map_err(AppError::from_repository)?;
        }
        if role_changed {
            self.trail
                .record(
                    Some(user.id()),
                    "identity.role.changed",
                    user.id().to_string(),
                    AuditOutcome::Success,
                    correlation_id,
                    context(&[("role", user.role().as_str())]),
                )
                .await?;
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
