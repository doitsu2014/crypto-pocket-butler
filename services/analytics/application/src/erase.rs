//! WF6: erasure by hashing the raw subject under every retained salt version
//! (BR6.1).

use std::sync::Arc;

use analytics_domain::AnalyticsUserId;

use crate::common::FieldError;
use crate::hashing::hmac_sha256_hex;
use crate::ports::{ConsentRepository, EventRepository, RepositoryError, SaltProvider};
use crate::trail::Trail;

/// Why an erasure failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EraseError {
    /// The request failed validation.
    #[error("the erasure request is invalid")]
    Rejected(Vec<FieldError>),
    /// The salt versions could not be read.
    #[error("the pseudonymisation salt is unavailable: {0}")]
    SaltUnavailable(String),
    /// A storage failure.
    #[error("the erasure could not be completed: {0}")]
    Repository(#[from] RepositoryError),
}

/// Deletes a user's pseudonymous events and consent-check rows under every
/// retained salt version.
pub struct EraseService {
    events: Arc<dyn EventRepository>,
    consents: Arc<dyn ConsentRepository>,
    salts: Arc<dyn SaltProvider>,
    trail: Arc<Trail>,
}

impl EraseService {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        events: Arc<dyn EventRepository>,
        consents: Arc<dyn ConsentRepository>,
        salts: Arc<dyn SaltProvider>,
        trail: Arc<Trail>,
    ) -> Self {
        Self {
            events,
            consents,
            salts,
            trail,
        }
    }

    /// Erases every event matching the raw subject under any retained version.
    ///
    /// # Errors
    /// Returns [`EraseError`] for an invalid identifier, an unavailable salt, or
    /// a storage failure.
    pub async fn erase(&self, auth_user_id: &str, actor: &str) -> Result<u64, EraseError> {
        let user = AnalyticsUserId::parse(auth_user_id)
            .map_err(|error| EraseError::Rejected(vec![FieldError::from(error)]))?;
        let versions = self
            .salts
            .retained()
            .await
            .map_err(|error| EraseError::SaltUnavailable(error.to_string()))?;
        if versions.is_empty() {
            return Err(EraseError::SaltUnavailable(
                "no retained salt version is available".to_string(),
            ));
        }
        // The raw identifier is hashed under every retained version and then
        // dropped; only the hashes reach the delete statement.
        let refs: Vec<String> = versions
            .iter()
            .map(|version| hmac_sha256_hex(&version.salt, &user.as_uuid().to_string()))
            .collect();
        // Events and consent-check rows both carry the user reference; both are
        // deleted under every retained version. Aggregates hold no userRef and
        // are unaffected.
        let deleted = self.events.delete_by_user_refs(&refs).await?;
        self.consents.delete_by_user_refs(&refs).await?;
        if let Some(user_ref) = refs.first() {
            self.trail.erased(user_ref, actor).await;
        }
        Ok(deleted)
    }
}
