//! WF7: the audit side of account deletion (BR4.3).
//!
//! The API records the request; the retention worker — the only holder of a
//! `DELETE` grant — executes it. There is no queue: the worker polls the
//! `requested` rows, which is why the API can answer 202 without holding a
//! destructive credential.

use std::sync::Arc;

use audit_domain::{
    classify, Context, DeletionRequestId, Outcome, PrivacyDeletion, PrivacyDeletionId,
    PrivacyDeletionState, Sensitivity, UserId,
};

use crate::common::FieldError;
use crate::ports::{Clock, PrivacyDeletionRepository, RepositoryError};
use crate::trail::Trail;

/// The maximum number of a subject's events one deletion examines.
pub const MAX_DELETION_SCAN: u32 = 100_000;

/// The raw deletion request Identity sends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestCommand {
    /// Identity's deletion request id.
    pub deletion_request_id: String,
    /// The user being deleted.
    pub subject_user_id: String,
}

/// Why a deletion operation was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PrivacyError {
    /// The request failed validation.
    #[error("the deletion request is invalid")]
    Rejected(Vec<FieldError>),
    /// Storage failed.
    #[error("the deletion could not be stored: {0}")]
    Repository(String),
    /// The scan and the delete disagreed; the deletion is marked failed.
    #[error("the deletion could not be completed: {0}")]
    Inconsistent(String),
}

/// Records and executes privacy deletions.
pub struct PrivacyDeletions {
    deletions: Arc<dyn PrivacyDeletionRepository>,
    clock: Arc<dyn Clock>,
    trail: Trail,
}

impl PrivacyDeletions {
    /// Creates the service.
    #[must_use]
    pub fn new(
        deletions: Arc<dyn PrivacyDeletionRepository>,
        clock: Arc<dyn Clock>,
        trail: Trail,
    ) -> Self {
        Self {
            deletions,
            clock,
            trail,
        }
    }

    /// Records a deletion request. A repeated request returns the recorded row.
    ///
    /// # Errors
    /// Returns [`PrivacyError::Rejected`] for a malformed id and
    /// [`PrivacyError::Repository`] on a storage failure.
    pub async fn request(
        &self,
        caller_service: &str,
        command: RequestCommand,
    ) -> Result<PrivacyDeletion, PrivacyError> {
        let mut errors = Vec::new();
        let request = match DeletionRequestId::parse(&command.deletion_request_id) {
            Ok(value) => Some(value),
            Err(error) => {
                errors.push(FieldError::from(error));
                None
            }
        };
        let subject = match UserId::parse(&command.subject_user_id) {
            Ok(value) => Some(value),
            Err(error) => {
                errors.push(FieldError::from(error));
                None
            }
        };
        if !errors.is_empty() {
            return Err(PrivacyError::Rejected(errors));
        }
        let request = request.expect("validated request");
        let subject = subject.expect("validated subject");

        let deletion = PrivacyDeletion::requested(
            PrivacyDeletionId::generate(),
            request,
            subject,
            self.clock.now(),
        );
        match self.deletions.insert(&deletion).await {
            Ok(()) => {
                let _ = self
                    .trail
                    .record_system(
                        "system.privacy.deletion",
                        &request.to_string(),
                        Outcome::Success,
                        Sensitivity::Security,
                        source_context(caller_service),
                    )
                    .await;
                Ok(deletion)
            }
            Err(RepositoryError::Conflict(_)) => {
                let existing = self
                    .deletions
                    .find_by_request(request)
                    .await
                    .map_err(|error| PrivacyError::Repository(error.to_string()))?;
                existing.ok_or_else(|| {
                    PrivacyError::Repository(
                        "the deletion conflicted but no record is visible".to_string(),
                    )
                })
            }
            Err(error) => Err(PrivacyError::Repository(error.to_string())),
        }
    }

    /// Executes up to `limit` waiting requests.
    ///
    /// # Errors
    /// Returns [`PrivacyError::Repository`] when the queue cannot be read. A
    /// single failed deletion is recorded on its row and does not stop the rest.
    pub async fn execute_pending(&self, limit: u32) -> Result<Vec<PrivacyDeletion>, PrivacyError> {
        let waiting = self
            .deletions
            .next_requested(limit)
            .await
            .map_err(|error| PrivacyError::Repository(error.to_string()))?;
        let mut executed = Vec::with_capacity(waiting.len());
        for deletion in waiting {
            executed.push(self.execute(deletion).await?);
        }
        Ok(executed)
    }

    /// Executes one deletion: classify, delete the unlabelled rows, and record
    /// the result.
    ///
    /// # Errors
    /// Returns [`PrivacyError::Repository`] when the scan or update fails.
    pub async fn execute(
        &self,
        deletion: PrivacyDeletion,
    ) -> Result<PrivacyDeletion, PrivacyError> {
        if deletion.state() != PrivacyDeletionState::Requested {
            return Ok(deletion);
        }
        let subject = deletion.subject_user_id();
        let labels = self
            .deletions
            .labels_for_subject(subject, MAX_DELETION_SCAN)
            .await
            .map_err(|error| PrivacyError::Repository(error.to_string()))?;
        let report = classify(&labels);
        let deleted = self
            .deletions
            .delete_unlabelled_for_subject(subject)
            .await
            .map_err(|error| PrivacyError::Repository(error.to_string()))?;

        if deleted != report.deleted as u64 {
            let failed = deletion
                .fail(
                    "the scanned and deleted record counts disagree",
                    self.clock.now(),
                )
                .map_err(|error| PrivacyError::Repository(error.to_string()))?;
            let _ = self.deletions.update(&failed).await;
            let _ = self
                .trail
                .record_system(
                    "system.privacy.deletion",
                    &failed.deletion_request_id().to_string(),
                    Outcome::Failed,
                    Sensitivity::Security,
                    source_context("audit_retention"),
                )
                .await;
            return Err(PrivacyError::Inconsistent(
                "the scanned and deleted record counts disagree".to_string(),
            ));
        }

        let completed = deletion
            .complete(report, self.clock.now())
            .map_err(|error| PrivacyError::Repository(error.to_string()))?;
        self.deletions
            .update(&completed)
            .await
            .map_err(|error| PrivacyError::Repository(error.to_string()))?;
        let _ = self
            .trail
            .record_system(
                "system.privacy.deletion",
                &completed.deletion_request_id().to_string(),
                Outcome::Success,
                Sensitivity::Security,
                source_context("audit_retention"),
            )
            .await;
        Ok(completed)
    }
}

fn source_context(service: &str) -> Context {
    let mut context = Context::empty();
    let _ = context.insert("sourceService", service);
    context
}
