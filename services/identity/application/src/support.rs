//! WF7 — support access grants (BR4.1–BR4.4).

use std::sync::Arc;

use chrono::Duration;
use identity_domain::{
    AuditOutcome, SupportAccessGrant, SupportGrantId, UserId, DEFAULT_GRANT_MINUTES,
};

use crate::common::{AppError, Caller};
use crate::ports::{Clock, SupportGrantRepository, UserRepository};
use crate::trail::{context, Trail};

/// Requests, approves, revokes, and checks support grants.
pub struct SupportService {
    grants: Arc<dyn SupportGrantRepository>,
    users: Arc<dyn UserRepository>,
    trail: Arc<Trail>,
    clock: Arc<dyn Clock>,
}

impl SupportService {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        grants: Arc<dyn SupportGrantRepository>,
        users: Arc<dyn UserRepository>,
        trail: Arc<Trail>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            grants,
            users,
            trail,
            clock,
        }
    }

    /// A support agent requests a grant for one target user (WF7 step 2).
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for a non-support caller and
    /// [`AppError::Validation`] for a blank reason.
    pub async fn request(
        &self,
        caller: &Caller,
        user_id: UserId,
        reason: &str,
    ) -> Result<SupportAccessGrant, AppError> {
        if !caller.is_support() {
            return Err(AppError::Forbidden(
                "only a support agent may request a supporting grant".to_string(),
            ));
        }
        let target = self
            .users
            .find_by_id(user_id)
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("user"))?;
        if target.is_deleted() {
            return Err(AppError::NotFound("user"));
        }
        let grant = SupportAccessGrant::request(
            SupportGrantId::generate(),
            user_id,
            caller.user_id(),
            reason,
            self.clock.now(),
        )
        .map_err(AppError::from_domain)?;
        self.grants
            .insert(&grant)
            .await
            .map_err(AppError::from_repository)?;
        let grant_label = grant.id().to_string();
        self.trail
            .record(
                Some(caller.user_id()),
                "identity.support_grant.requested",
                user_id.to_string(),
                AuditOutcome::Success,
                &caller.correlation_id,
                context(&[("grantId", grant_label.as_str())]),
            )
            .await?;
        Ok(grant)
    }

    /// A distinct admin approves a pending grant (BR4.1, BR4.2).
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for a non-admin or self-approval.
    pub async fn approve(
        &self,
        caller: &Caller,
        grant_id: SupportGrantId,
    ) -> Result<SupportAccessGrant, AppError> {
        if !caller.is_admin() {
            return Err(AppError::Forbidden(
                "only an admin may approve a support grant".to_string(),
            ));
        }
        let mut grant = self.load(grant_id).await?;
        grant
            .approve(
                caller.user_id(),
                caller.role(),
                self.clock.now(),
                Duration::minutes(DEFAULT_GRANT_MINUTES),
            )
            .map_err(AppError::from_domain)?;
        self.grants
            .update(&grant)
            .await
            .map_err(AppError::from_repository)?;
        let grant_label = grant.id().to_string();
        self.trail
            .record(
                Some(caller.user_id()),
                "identity.support_grant.approved",
                grant.user_id().to_string(),
                AuditOutcome::Success,
                &caller.correlation_id,
                context(&[("grantId", grant_label.as_str())]),
            )
            .await?;
        Ok(grant)
    }

    /// An admin revokes a pending or active grant.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for a non-admin and
    /// [`AppError::Conflict`] for a terminal grant.
    pub async fn revoke(
        &self,
        caller: &Caller,
        grant_id: SupportGrantId,
    ) -> Result<SupportAccessGrant, AppError> {
        if !caller.is_admin() {
            return Err(AppError::Forbidden(
                "only an admin may revoke a support grant".to_string(),
            ));
        }
        let mut grant = self.load(grant_id).await?;
        grant
            .revoke(self.clock.now())
            .map_err(AppError::from_domain)?;
        self.grants
            .update(&grant)
            .await
            .map_err(AppError::from_repository)?;
        let grant_label = grant.id().to_string();
        self.trail
            .record(
                Some(caller.user_id()),
                "identity.support_grant.revoked",
                grant.user_id().to_string(),
                AuditOutcome::Success,
                &caller.correlation_id,
                context(&[("grantId", grant_label.as_str())]),
            )
            .await?;
        Ok(grant)
    }

    /// Verifies a grant on the metadata read path. The audit event is written
    /// before the answer; a failed write refuses the read (BR4.3).
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for an inactive or expired grant and
    /// [`AppError::Unavailable`] when the audit event cannot be written.
    pub async fn check(
        &self,
        grant_id: SupportGrantId,
        user_id: UserId,
        correlation_id: &str,
    ) -> Result<SupportAccessGrant, AppError> {
        let now = self.clock.now();
        let mut grant = self.load(grant_id).await?;
        if grant.expire_if_due(now) {
            self.grants
                .update(&grant)
                .await
                .map_err(AppError::from_repository)?;
        }
        let valid = grant.is_valid_at(now, user_id);
        let grant_label = grant.id().to_string();
        let (action, outcome) = if valid {
            ("identity.support_read.served", AuditOutcome::Success)
        } else {
            ("identity.support_read.denied", AuditOutcome::Denied)
        };
        self.trail
            .record(
                Some(grant.support_agent_id()),
                action,
                user_id.to_string(),
                outcome,
                correlation_id,
                context(&[("grantId", grant_label.as_str())]),
            )
            .await?;
        if valid {
            Ok(grant)
        } else {
            Err(AppError::Forbidden(
                "the support grant is not active".to_string(),
            ))
        }
    }

    async fn load(&self, grant_id: SupportGrantId) -> Result<SupportAccessGrant, AppError> {
        self.grants
            .find_by_id(grant_id)
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("support grant"))
    }
}
