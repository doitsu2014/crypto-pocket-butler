//! U4 — answer the consent-state query and compute the missing acceptances
//! (BR4.5, BR4.6, BR4.11).
//!
//! The answer is the newest event per kind plus the required kinds that are not
//! granted. A read failure is an error; the gate never answers permissively on a
//! guess.

use std::sync::Arc;
use std::time::Instant;

use identity_domain::{ConsentStateView, UserId};

use super::ports::{ConsentRepository, ObservationSink, StateQueryOutcome};
use crate::common::AppError;

/// Answers "what is this user's consent state, and what is still missing?".
pub struct ConsentStateQuery {
    consent: Arc<dyn ConsentRepository>,
    observation: Arc<dyn ObservationSink>,
}

impl ConsentStateQuery {
    /// Creates the use case.
    #[must_use]
    pub fn new(consent: Arc<dyn ConsentRepository>, observation: Arc<dyn ObservationSink>) -> Self {
        Self {
            consent,
            observation,
        }
    }

    /// Resolves the state for one user.
    ///
    /// # Errors
    /// Returns [`AppError::Unavailable`] when the store cannot answer. The
    /// caller must fail closed.
    pub async fn state(&self, user_id: UserId) -> Result<ConsentStateView, AppError> {
        let started = Instant::now();
        match self.consent.newest_per_kind(user_id).await {
            Ok(events) => {
                let view = ConsentStateView::from_newest(user_id, events);
                self.observation.record_state_query(
                    StateQueryOutcome::Answered,
                    view.has_missing(),
                    started.elapsed(),
                );
                Ok(view)
            }
            Err(error) => {
                self.observation.record_state_query(
                    StateQueryOutcome::Error,
                    false,
                    started.elapsed(),
                );
                Err(AppError::from_repository(error))
            }
        }
    }
}
