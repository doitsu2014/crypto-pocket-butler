//! U4 — record one consent grant (BR4.8).
//!
//! A grant supplies its own policy version; the database assigns the identifier,
//! the sequence, and the timestamp.

use std::sync::Arc;

use identity_domain::{ConsentAction, ConsentEvent, ConsentEventDraft, ConsentKind, ConsentPolicy};

use super::ports::{ConsentRepository, ObservationSink};
use crate::common::{AppError, Caller};

/// Records a grant for the caller.
pub struct ConsentGrant {
    consent: Arc<dyn ConsentRepository>,
    observation: Arc<dyn ObservationSink>,
}

impl ConsentGrant {
    /// Creates the use case.
    #[must_use]
    pub fn new(consent: Arc<dyn ConsentRepository>, observation: Arc<dyn ObservationSink>) -> Self {
        Self {
            consent,
            observation,
        }
    }

    /// Appends the `granted` event. The policy version is already validated at
    /// the boundary.
    ///
    /// # Errors
    /// Returns [`AppError::Unavailable`] when the event cannot be written; the
    /// caller never receives a claimed success.
    pub async fn grant(
        &self,
        caller: &Caller,
        kind: ConsentKind,
        policy: ConsentPolicy,
    ) -> Result<ConsentEvent, AppError> {
        let draft = ConsentEventDraft::grant(caller.user_id(), kind, policy);
        let event = self
            .consent
            .insert(&draft)
            .await
            .map_err(AppError::from_repository)?;
        self.observation
            .record_consent_event(kind, ConsentAction::Granted);
        Ok(event)
    }
}
