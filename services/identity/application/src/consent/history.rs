//! U4 — paged consent history, newest first (BR4.4).
//!
//! The history is read-only and append-only: no event is ever edited or removed
//! by the application.

use std::sync::Arc;

use identity_domain::ConsentEvent;

use super::ports::{ConsentRepository, HistoryCursor};
use crate::common::{AppError, Caller};

/// One page of a user's consent history.
pub struct ConsentHistory {
    consent: Arc<dyn ConsentRepository>,
}

impl ConsentHistory {
    /// Creates the use case.
    #[must_use]
    pub fn new(consent: Arc<dyn ConsentRepository>) -> Self {
        Self { consent }
    }

    /// Returns the caller's events, newest first.
    ///
    /// # Errors
    /// Returns [`AppError::Unavailable`] when the store cannot answer.
    pub async fn history(
        &self,
        caller: &Caller,
        limit: u32,
        before: Option<HistoryCursor>,
    ) -> Result<Vec<ConsentEvent>, AppError> {
        self.consent
            .history(caller.user_id(), limit.clamp(1, 100), before)
            .await
            .map_err(AppError::from_repository)
    }
}
