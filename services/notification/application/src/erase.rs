//! NFR13.1: Identity's erasure replaces the user reference on the user's
//! alerts and delivery attempts with one random UUIDv4. No mapping back to the
//! user is retained, so a second call for the same user finds nothing.

use std::sync::Arc;

use notification_domain::UserId;

use crate::common::FieldError;
use crate::ports::{AlertRepository, IdGenerator, RepositoryError};
use crate::trail::Trail;

/// What one erasure did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EraseReport {
    /// Alerts rewritten.
    pub alerts_anonymised: u64,
    /// Delivery attempts rewritten.
    pub attempts_anonymised: u64,
    /// Whether the C10 write was abandoned after every retry.
    pub audit_failed: bool,
}

/// Why an erasure failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EraseError {
    /// The request failed validation.
    #[error("the erasure request is invalid")]
    Rejected(Vec<FieldError>),
    /// A storage failure.
    #[error("the erasure could not be completed: {0}")]
    Repository(#[from] RepositoryError),
}

/// Anonymises a user's notification rows.
pub struct EraseService {
    alerts: Arc<dyn AlertRepository>,
    ids: Arc<dyn IdGenerator>,
    trail: Arc<Trail>,
}

impl EraseService {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        alerts: Arc<dyn AlertRepository>,
        ids: Arc<dyn IdGenerator>,
        trail: Arc<Trail>,
    ) -> Self {
        Self { alerts, ids, trail }
    }

    /// Replaces the user reference on alerts and attempts with one fresh
    /// reference. A user with no rows returns zero and writes no audit event.
    ///
    /// # Errors
    /// Returns [`EraseError`] for an invalid identifier or a storage failure.
    pub async fn anonymise(
        &self,
        user_id: &str,
        correlation: Option<&str>,
    ) -> Result<EraseReport, EraseError> {
        let user = UserId::parse(user_id)
            .map_err(|error| EraseError::Rejected(vec![FieldError::from(error)]))?;
        let replacement = UserId::new(self.ids.next());
        let rows = self.alerts.anonymise_user(user, replacement).await?;
        let mut report = EraseReport {
            alerts_anonymised: rows.alerts,
            attempts_anonymised: rows.attempts,
            audit_failed: false,
        };
        if rows.alerts > 0 || rows.attempts > 0 {
            report.audit_failed = !self.trail.anonymised(replacement, correlation).await;
        }
        Ok(report)
    }
}
