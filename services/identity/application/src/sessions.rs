//! WF3 — the device-session view and revocation (BR8.1, BR8.2).

use std::sync::Arc;

use identity_domain::{AuditOutcome, DeviceSession, DeviceSessionId, UserId};

use crate::common::{AppError, Caller};
use crate::ports::{AuthAdmin, Clock, DeviceSessionRepository, UserRepository};
use crate::trail::{context, Trail};

/// Lists and revokes the caller's device sessions.
pub struct SessionService {
    sessions: Arc<dyn DeviceSessionRepository>,
    users: Arc<dyn UserRepository>,
    auth_admin: Arc<dyn AuthAdmin>,
    trail: Arc<Trail>,
    clock: Arc<dyn Clock>,
}

impl SessionService {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        sessions: Arc<dyn DeviceSessionRepository>,
        users: Arc<dyn UserRepository>,
        auth_admin: Arc<dyn AuthAdmin>,
        trail: Arc<Trail>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            sessions,
            users,
            auth_admin,
            trail,
            clock,
        }
    }

    /// Lists the user's sessions, newest activity first.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for another user's list.
    pub async fn list(
        &self,
        caller: &Caller,
        user_id: UserId,
    ) -> Result<Vec<DeviceSession>, AppError> {
        caller.require_owner_or_admin(user_id)?;
        let sessions = self
            .sessions
            .list_for_user(user_id)
            .await
            .map_err(AppError::from_repository)?;
        self.trail
            .record(
                Some(caller.user_id()),
                "identity.sessions.read",
                user_id.to_string(),
                AuditOutcome::Success,
                &caller.correlation_id,
                context(&[]),
            )
            .await?;
        Ok(sessions)
    }

    /// Revokes one session through Supabase Auth and marks the row revoked.
    ///
    /// Revoking the current session requires `confirm_current`, because the SPA
    /// must warn first (BR8.2).
    ///
    /// # Errors
    /// Returns [`AppError::Conflict`] when the current session was not
    /// confirmed, [`AppError::Unavailable`] when Auth cannot be reached, and
    /// [`AppError::NotFound`] for an unknown session.
    pub async fn revoke(
        &self,
        caller: &Caller,
        user_id: UserId,
        session_id: DeviceSessionId,
        confirm_current: bool,
    ) -> Result<(), AppError> {
        caller.require_owner_or_admin(user_id)?;
        let mut session = self
            .sessions
            .find(session_id)
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("session"))?;
        if session.user_id() != user_id {
            return Err(AppError::NotFound("session"));
        }
        if session.is_revoked() {
            return Ok(());
        }
        if caller.claims.session_id.as_deref() == Some(session.auth_session_id())
            && !confirm_current
        {
            return Err(AppError::Conflict(
                "confirm the current session revocation; the user will be signed out".to_string(),
            ));
        }

        let subject = self
            .users
            .reveal_auth_subject(user_id)
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("user"))?;
        self.auth_admin
            .revoke_session(&subject, session.auth_session_id())
            .await
            .map_err(AppError::from_external)?;

        session.revoke(self.clock.now());
        self.sessions
            .update(&session)
            .await
            .map_err(AppError::from_repository)?;
        let session_label = session.id().to_string();
        self.trail
            .record(
                Some(caller.user_id()),
                "identity.session.revoked",
                user_id.to_string(),
                AuditOutcome::Success,
                &caller.correlation_id,
                context(&[("sessionId", session_label.as_str())]),
            )
            .await?;
        Ok(())
    }
}
