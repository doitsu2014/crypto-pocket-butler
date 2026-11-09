//! WF9 — account deletion and its execution worker (BR6.1–BR6.7).

use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use identity_domain::{
    AuditOutcome, Contributor, DeletionRequest, DeletionRequestId, LEASE_SECONDS,
};

use crate::common::{AppError, Caller};
use crate::ports::{
    AuthAdmin, Clock, ConsentRepository, ContributorGateway, DeletionRepository,
    ShreddingKeyRepository, UserRepository, WakeTransport,
};
use crate::trail::{context, Trail};

/// Creates deletion requests and answers status polls.
pub struct DeletionService {
    deletions: Arc<dyn DeletionRepository>,
    trail: Arc<Trail>,
    wake: Arc<dyn WakeTransport>,
    clock: Arc<dyn Clock>,
}

impl DeletionService {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        deletions: Arc<dyn DeletionRepository>,
        trail: Arc<Trail>,
        wake: Arc<dyn WakeTransport>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            deletions,
            trail,
            wake,
            clock,
        }
    }

    /// Confirms a deletion, reusing the user's active request (BR6.1).
    ///
    /// # Errors
    /// Returns [`AppError::Unavailable`] when the request or its audit event
    /// cannot be written.
    pub async fn request(&self, caller: &Caller) -> Result<DeletionRequest, AppError> {
        if let Some(active) = self
            .deletions
            .find_active_for_user(caller.user_id())
            .await
            .map_err(AppError::from_repository)?
        {
            return Ok(active);
        }
        let deletion = DeletionRequest::request(
            DeletionRequestId::generate(),
            caller.user_id(),
            self.clock.now(),
        );
        self.deletions
            .insert(&deletion)
            .await
            .map_err(AppError::from_repository)?;
        self.trail
            .record(
                Some(caller.user_id()),
                "identity.deletion.requested",
                deletion.id().to_string(),
                AuditOutcome::Success,
                &caller.correlation_id,
                context(&[]),
            )
            .await?;
        if let Err(error) = self.wake.wake().await {
            tracing::warn!(%error, "deletion wake failed; the sweep will pick the request up");
        }
        Ok(deletion)
    }

    /// Returns the owner's deletion state (BR6.4).
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for another user's request.
    pub async fn status(
        &self,
        caller: &Caller,
        deletion_id: DeletionRequestId,
    ) -> Result<DeletionRequest, AppError> {
        let deletion = self
            .deletions
            .find_by_id(deletion_id)
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("deletion"))?;
        if deletion.user_id() != caller.user_id() {
            return Err(AppError::Forbidden(
                "only the owner may read a deletion request".to_string(),
            ));
        }
        Ok(deletion)
    }
}

/// Executes one deletion per run, in a fixed order, resuming a retry without
/// repeating completed destruction.
pub struct DeletionWorker {
    deletions: Arc<dyn DeletionRepository>,
    users: Arc<dyn UserRepository>,
    keys: Arc<dyn ShreddingKeyRepository>,
    consent: Arc<dyn ConsentRepository>,
    contributors: Arc<dyn ContributorGateway>,
    auth_admin: Arc<dyn AuthAdmin>,
    trail: Arc<Trail>,
}

impl DeletionWorker {
    /// Creates the worker.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        deletions: Arc<dyn DeletionRepository>,
        users: Arc<dyn UserRepository>,
        keys: Arc<dyn ShreddingKeyRepository>,
        consent: Arc<dyn ConsentRepository>,
        contributors: Arc<dyn ContributorGateway>,
        auth_admin: Arc<dyn AuthAdmin>,
        trail: Arc<Trail>,
    ) -> Self {
        Self {
            deletions,
            users,
            keys,
            consent,
            contributors,
            auth_admin,
            trail,
        }
    }

    /// Runs one deletion pass. Returns how many requests it processed.
    ///
    /// # Errors
    /// Returns [`AppError`] when the store cannot be reached. A failed step is
    /// recorded on the request, which stays retryable.
    pub async fn run_once(&self, worker: &str, now: DateTime<Utc>) -> Result<usize, AppError> {
        let Some(mut deletion) = self
            .deletions
            .claim_next(worker, now, Duration::seconds(LEASE_SECONDS))
            .await
            .map_err(AppError::from_repository)?
        else {
            return Ok(0);
        };
        let user_id = deletion.user_id();
        let correlation = format!("deletion-{}", deletion.id());

        for contributor in Contributor::ALL {
            if deletion.progress().is_anonymised(contributor) {
                continue;
            }
            let idempotency_key = format!("{}:{}", deletion.id(), contributor.as_str());
            match self
                .contributors
                .anonymise(contributor, user_id, &idempotency_key)
                .await
            {
                Ok(categories) => {
                    deletion.record_retained(categories);
                    deletion.record_anonymised(contributor);
                    self.deletions
                        .update(&deletion)
                        .await
                        .map_err(AppError::from_repository)?;
                }
                Err(error) => {
                    self.fail(&mut deletion, error.to_string(), now, &correlation)
                        .await?;
                    return Ok(1);
                }
            }
        }

        if !deletion.progress().auth_deleted() {
            let subject = match self
                .users
                .reveal_auth_subject(user_id)
                .await
                .map_err(AppError::from_repository)?
            {
                Some(subject) => subject,
                None => {
                    self.fail(
                        &mut deletion,
                        "the Supabase Auth subject is unavailable".to_string(),
                        now,
                        &correlation,
                    )
                    .await?;
                    return Ok(1);
                }
            };
            match self.auth_admin.delete_user(&subject).await {
                Ok(()) => {
                    deletion.record_auth_deleted();
                    self.deletions
                        .update(&deletion)
                        .await
                        .map_err(AppError::from_repository)?;
                }
                Err(error) => {
                    self.fail(&mut deletion, error.to_string(), now, &correlation)
                        .await?;
                    return Ok(1);
                }
            }
        }

        if !deletion.progress().identity_anonymised() {
            let mut user = self
                .users
                .find_by_id(user_id)
                .await
                .map_err(AppError::from_repository)?
                .ok_or(AppError::NotFound("user"))?;
            user.anonymise(now);
            self.users
                .anonymise(&user)
                .await
                .map_err(AppError::from_repository)?;
            self.consent
                .delete_for_user(user_id)
                .await
                .map_err(AppError::from_repository)?;
            deletion.record_identity_anonymised(now);
            self.deletions
                .update(&deletion)
                .await
                .map_err(AppError::from_repository)?;
        }

        if !deletion.progress().key_destroyed() {
            if let Some(key) = self
                .keys
                .active_for_user(user_id)
                .await
                .map_err(AppError::from_repository)?
            {
                self.keys
                    .destroy(key.id(), now)
                    .await
                    .map_err(AppError::from_repository)?;
            }
            deletion
                .record_key_destroyed(now)
                .map_err(AppError::from_domain)?;
            self.deletions
                .update(&deletion)
                .await
                .map_err(AppError::from_repository)?;
        }

        let retained = deletion.progress().retained().to_vec();
        let retained_label = retained
            .iter()
            .map(|category| category.category().to_string())
            .collect::<Vec<_>>()
            .join(",");
        let retained_label = truncate(&retained_label, 200);
        if let Err(error) = self
            .trail
            .record(
                Some(user_id),
                "identity.deletion.completed",
                deletion.id().to_string(),
                AuditOutcome::Success,
                &correlation,
                context(&[("retainedCategories", retained_label.as_str())]),
            )
            .await
        {
            self.fail(&mut deletion, error.to_string(), now, &correlation)
                .await?;
            return Ok(1);
        }

        deletion
            .complete(retained, now)
            .map_err(AppError::from_domain)?;
        self.deletions
            .update(&deletion)
            .await
            .map_err(AppError::from_repository)?;
        Ok(1)
    }

    async fn fail(
        &self,
        deletion: &mut DeletionRequest,
        reason: String,
        now: DateTime<Utc>,
        correlation: &str,
    ) -> Result<(), AppError> {
        let reason = truncate(&reason, 500);
        deletion
            .fail(reason.clone(), now)
            .map_err(AppError::from_domain)?;
        self.deletions
            .update(deletion)
            .await
            .map_err(AppError::from_repository)?;
        let context_reason = truncate(&reason, 200);
        if let Err(error) = self
            .trail
            .record(
                Some(deletion.user_id()),
                "identity.deletion.failed",
                deletion.id().to_string(),
                AuditOutcome::Failed,
                correlation,
                context(&[("reason", context_reason.as_str())]),
            )
            .await
        {
            tracing::warn!(%error, "failed to audit a failed deletion step");
        }
        Ok(())
    }
}

fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}
