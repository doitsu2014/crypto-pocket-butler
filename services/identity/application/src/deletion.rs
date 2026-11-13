//! W4 — account deletion, its recovery window, and its execution worker
//! (BR5.3–BR5.6, BR6.1–BR6.7).
//!
//! The owner opens a request through [`AccountLifecycleService`]
//! (`super::account_lifecycle`); this module's worker erases a request only once
//! its recovery window has closed. The cascade is ordered so a failure is always
//! retryable, and it records the attempt/backoff bookkeeping the sweep claims on.

use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use identity_domain::{
    AuditOutcome, Contributor, DeletionRequest, DeletionRequestId, UserStatus, LEASE_SECONDS,
    SYSTEM_ACTOR,
};

use crate::account_lifecycle::AccountObservation;
use crate::common::{AppError, Caller};
use crate::ports::{
    AuthAdmin, ContributorGateway, DeletionRepository, ErasureStore, ShreddingKeyRepository,
    StatusChange, StatusTransitionStore, UserRepository,
};
use crate::trail::{context, Trail};

/// Answers the owner's deletion status and records the privacy event.
///
/// Opening a request belongs to [`crate::account_lifecycle::AccountLifecycleService`],
/// which writes the status change, the request, and the audit intent in one
/// transaction; this service reads the result back and writes the operator-facing
/// C10 event.
pub struct DeletionService {
    deletions: Arc<dyn DeletionRepository>,
    trail: Arc<Trail>,
}

impl DeletionService {
    /// Creates the use case.
    #[must_use]
    pub fn new(deletions: Arc<dyn DeletionRepository>, trail: Arc<Trail>) -> Self {
        Self { deletions, trail }
    }

    /// The owner's open deletion request, when one exists (WF9).
    ///
    /// # Errors
    /// Returns [`AppError::Unavailable`] when the store cannot answer.
    pub async fn open_request(&self, caller: &Caller) -> Result<Option<DeletionRequest>, AppError> {
        self.deletions
            .find_active_for_user(caller.user_id())
            .await
            .map_err(AppError::from_repository)
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

    /// Records the privacy event through C10.
    ///
    /// # Errors
    /// Returns [`AppError::Unavailable`] when the audit sink cannot record it.
    pub async fn record_requested(
        &self,
        caller: &Caller,
        deletion: &DeletionRequest,
    ) -> Result<(), AppError> {
        self.trail
            .record(
                Some(caller.user_id()),
                "identity.deletion.requested",
                deletion.id().to_string(),
                AuditOutcome::Success,
                &caller.correlation_id,
                context(&[]),
            )
            .await
    }
}

/// Executes one due deletion per claim, in a fixed order, resuming a retry
/// without repeating completed destruction.
pub struct DeletionWorker {
    deletions: Arc<dyn DeletionRepository>,
    users: Arc<dyn UserRepository>,
    keys: Arc<dyn ShreddingKeyRepository>,
    erasure: Arc<dyn ErasureStore>,
    contributors: Arc<dyn ContributorGateway>,
    auth_admin: Arc<dyn AuthAdmin>,
    transitions: Arc<dyn StatusTransitionStore>,
    observation: Arc<dyn AccountObservation>,
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
        erasure: Arc<dyn ErasureStore>,
        contributors: Arc<dyn ContributorGateway>,
        auth_admin: Arc<dyn AuthAdmin>,
        transitions: Arc<dyn StatusTransitionStore>,
        observation: Arc<dyn AccountObservation>,
        trail: Arc<Trail>,
    ) -> Self {
        Self {
            deletions,
            users,
            keys,
            erasure,
            contributors,
            auth_admin,
            transitions,
            observation,
            trail,
        }
    }

    /// Runs one erasure pass. Returns how many requests it processed.
    ///
    /// # Errors
    /// Returns [`AppError`] when the store cannot be reached. A failed step is
    /// recorded on the request, which stays retryable with a bounded backoff.
    pub async fn run_once(&self, worker: &str, now: DateTime<Utc>) -> Result<usize, AppError> {
        let Some(mut deletion) = self
            .deletions
            .claim_next(worker, now, Duration::seconds(LEASE_SECONDS))
            .await
            .map_err(AppError::from_repository)?
        else {
            return Ok(0);
        };
        // BR5.6: a second run over an erased request changes nothing and reports
        // success.
        if deletion.erased_at().is_some() {
            return Ok(1);
        }
        let user_id = deletion.user_id();
        let correlation = format!("deletion-{}", deletion.id());

        let mut user = match self.users.find_by_id(user_id).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                self.fail(
                    &mut deletion,
                    "user",
                    "the user record is unavailable",
                    now,
                    &correlation,
                )
                .await?;
                return Ok(1);
            }
            Err(error) => {
                return Err(AppError::from_repository(error));
            }
        };

        // W4.1: a concurrent restore wins. The request is closed so it is never
        // re-claimed, and nothing is erased. A retry after the transition
        // already committed continues the cascade instead (`deleted` is this
        // run's own outcome, not a restore).
        if !matches!(
            user.status(),
            UserStatus::PendingDeletion | UserStatus::Deleted
        ) {
            deletion.cancel(now).map_err(AppError::from_domain)?;
            self.deletions
                .update(&deletion)
                .await
                .map_err(AppError::from_repository)?;
            self.observation.record_deletion_event("cancelled");
            return Ok(1);
        }

        // 1. The Supabase Auth identity leaves first: a failure here leaves the
        //    local account intact and retryable.
        if !deletion.progress().auth_deleted() {
            match self.users.reveal_auth_subject(user_id).await {
                Ok(Some(subject)) => match self.auth_admin.delete_user(&subject).await {
                    Ok(()) => {}
                    Err(error) => {
                        self.fail(&mut deletion, "auth", &error.to_string(), now, &correlation)
                            .await?;
                        return Ok(1);
                    }
                },
                // An absent subject means the identity is already gone; the step
                // is idempotent, never a failure.
                Ok(None) => {}
                Err(error) => {
                    self.fail(
                        &mut deletion,
                        "auth",
                        &format!("the subject is unavailable: {error}"),
                        now,
                        &correlation,
                    )
                    .await?;
                    return Ok(1);
                }
            }
            deletion.record_auth_deleted();
            self.deletions
                .update(&deletion)
                .await
                .map_err(AppError::from_repository)?;
        }

        // 2. The status change, its append-only transition, and its audit intent
        //    commit together (BR5.9).
        if user.status() == UserStatus::PendingDeletion {
            let change = StatusChange {
                user_id,
                from: UserStatus::PendingDeletion,
                to: UserStatus::Deleted,
                actor: SYSTEM_ACTOR.to_string(),
                actor_kind: "system",
                reason: None,
                occurred_at: now,
            };
            if let Err(error) = self.transitions.apply(&change).await {
                self.fail(
                    &mut deletion,
                    "transition",
                    &error.to_string(),
                    now,
                    &correlation,
                )
                .await?;
                return Ok(1);
            }
            user.transition(UserStatus::Deleted, now)
                .map_err(AppError::from_domain)?;
            self.observation.record_transition(
                UserStatus::PendingDeletion,
                UserStatus::Deleted,
                "system",
            );
        }

        // 3. The guarded tombstone move; it copies the subject hash before
        //    `user_personal` leaves.
        if !deletion.progress().identity_anonymised() {
            user.anonymise(now);
            if let Err(error) = self.users.anonymise(&user).await {
                self.fail(
                    &mut deletion,
                    "tombstone",
                    &error.to_string(),
                    now,
                    &correlation,
                )
                .await?;
                return Ok(1);
            }
            deletion.record_identity_anonymised(now);
            self.deletions
                .update(&deletion)
                .await
                .map_err(AppError::from_repository)?;
        }

        // 4. Each service that owns a slice of the account erases it through its
        //    own path, recorded step by step so a retry resumes.
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
                    self.fail(
                        &mut deletion,
                        "contributor",
                        &error.to_string(),
                        now,
                        &correlation,
                    )
                    .await?;
                    return Ok(1);
                }
            }
        }

        // 5. The shredding key is destroyed only after every other destructive
        //    step (BR6.3).
        if !deletion.progress().key_destroyed() {
            if let Some(key) = self
                .keys
                .active_for_user(user_id)
                .await
                .map_err(AppError::from_repository)?
            {
                if let Err(error) = self.keys.destroy(key.id(), now).await {
                    self.fail(&mut deletion, "key", &error.to_string(), now, &correlation)
                        .await?;
                    return Ok(1);
                }
            }
            deletion
                .record_key_destroyed(now)
                .map_err(AppError::from_domain)?;
            self.deletions
                .update(&deletion)
                .await
                .map_err(AppError::from_repository)?;
        }

        // 6. Consent events and preferences leave through the retention role.
        if let Err(error) = self.erasure.erase_consent(user_id).await {
            self.fail(
                &mut deletion,
                "consent",
                &format!("the retention delete failed: {error}"),
                now,
                &correlation,
            )
            .await?;
            return Ok(1);
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
            self.fail(
                &mut deletion,
                "audit",
                &error.to_string(),
                now,
                &correlation,
            )
            .await?;
            return Ok(1);
        }

        deletion.record_erased(retained, now);
        self.deletions
            .update(&deletion)
            .await
            .map_err(AppError::from_repository)?;
        self.observation.record_deletion_event("erased");
        Ok(1)
    }

    async fn fail(
        &self,
        deletion: &mut DeletionRequest,
        step: &str,
        reason: &str,
        now: DateTime<Utc>,
        correlation: &str,
    ) -> Result<(), AppError> {
        let reason = truncate(reason, 500);
        self.observation.record_erasure_failure(step);
        deletion
            .fail(reason.clone(), now)
            .map_err(AppError::from_domain)?;
        deletion
            .record_attempt_failure(now)
            .map_err(AppError::from_domain)?;
        self.deletions
            .update(deletion)
            .await
            .map_err(AppError::from_repository)?;
        tracing::warn!(
            account = %deletion.user_id(),
            step,
            attempts = deletion.attempt_count(),
            "an erasure step failed; the request backs off"
        );
        let context_reason = truncate(&reason, 200);
        if let Err(error) = self
            .trail
            .record(
                Some(deletion.user_id()),
                "identity.deletion.failed",
                deletion.id().to_string(),
                AuditOutcome::Failed,
                correlation,
                context(&[("reason", context_reason.as_str()), ("step", step)]),
            )
            .await
        {
            tracing::warn!(%error, "failed to audit a failed deletion step");
        }
        Ok(())
    }
}

/// Keeps the retained categories the cascade collected, so the finished request
/// still reports what a contributor keeps under a legal basis.
fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}
