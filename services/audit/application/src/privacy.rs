//! WF7: the audit side of account deletion (BR4.3).
//!
//! The API records the request; the retention worker — the only holder of a
//! `DELETE` grant — executes it. There is no queue: the worker polls the
//! `requested` rows, which is why the API can answer 202 without holding a
//! destructive credential.

use std::sync::Arc;

use audit_domain::{
    classify, AuditEventId, Context, DeletionReport, DeletionRequestId, Outcome, PrivacyDeletion,
    PrivacyDeletionId, PrivacyDeletionState, Sensitivity, UserId,
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
    /// The caller is not Identity, the only service allowed to start a deletion.
    #[error("forbidden: {0}")]
    Forbidden(String),
    /// The request failed validation.
    #[error("the deletion request is invalid")]
    Rejected(Vec<FieldError>),
    /// Storage failed.
    #[error("the deletion could not be stored: {0}")]
    Repository(String),
    /// The scan and the delete disagreed. Kept for callers of earlier releases;
    /// a count difference is now recorded on the row rather than raised.
    #[error("the deletion could not be completed: {0}")]
    Inconsistent(String),
}

/// The only service allowed to ask Audit to delete a user's records.
pub const DELETION_OWNER_SERVICE: &str = "identity";

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

        // WF7: Identity owns the account deletion and is the only service that
        // may start the audit side of it. Any other registered emitter is refused
        // and the refusal is audited, because a deletion is destructive.
        if caller_service != DELETION_OWNER_SERVICE {
            let _ = self
                .trail
                .record_system(
                    "system.privacy.deletion",
                    &request.to_string(),
                    Outcome::Denied,
                    Sensitivity::Security,
                    source_context(caller_service),
                )
                .await;
            return Err(PrivacyError::Forbidden(
                "only Identity may request a privacy deletion".to_string(),
            ));
        }

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
            match self.execute(deletion).await {
                Ok(done) => executed.push(done),
                // One failed request must not stop the rest; it stays
                // `requested` and the next tick retries it.
                Err(error) => {
                    tracing::error!(%error, "a privacy deletion failed and will be retried");
                }
            }
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
        let scanned = classify(&labels);
        // Delete exactly the bounded, ordered set that was just scanned. The
        // previous unbounded delete could remove rows outside the scan and then
        // report a mismatch after the destructive step, leaving the row
        // unreconciled. The actual count is what the row records.
        let ids: Vec<AuditEventId> = labels
            .iter()
            .filter(|label| label.legal_basis.is_none())
            .map(|label| label.id)
            .collect();
        let deleted = self
            .deletions
            .delete_for_subject(subject, &ids)
            .await
            .map_err(|error| PrivacyError::Repository(error.to_string()))?;
        let report = DeletionReport {
            deleted: deleted as i64,
            kept_with_legal_basis: scanned.kept_with_legal_basis,
            defects: scanned.defects,
        };

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
