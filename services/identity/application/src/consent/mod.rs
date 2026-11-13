//! U4 — consent events, the consent-state gate, and the withdrawal outbox.
//!
//! `ConsentService` is the earlier append-only purpose history (BR3.1, BR3.2);
//! the U4 use cases beside it record the event shape BR4.4–BR4.11 define. Both
//! live in one module because they share the consent store and its ports.

pub mod grant;
pub mod history;
pub mod ports;
pub mod state;
pub mod withdraw;

pub use self::grant::ConsentGrant;
pub use self::history::ConsentHistory;
pub use self::ports::{
    AuditIntent, ConsentRepository as ConsentEventRepository, ConsentTransaction, DeliveryOutcome,
    DrainerObservation, HistoryCursor, ObservationSink, PreferenceWriteOutcome, StateQueryOutcome,
    WithdrawalRollbackCause,
};
pub use self::state::ConsentStateQuery;
pub use self::withdraw::{ConsentWithdraw, WithdrawResult};

use std::sync::Arc;

use identity_domain::{
    AuditOutcome, ConsentPurpose, ConsentRecord, ConsentRecordId, ConsentSource, ConsentView,
    UserId,
};

use crate::common::{AppError, Caller};
use crate::ports::{Clock, ConsentRepository};
use crate::trail::{context, Trail};

/// Appends and reads the earlier purpose-based consent decisions.
pub struct ConsentService {
    consent: Arc<dyn ConsentRepository>,
    trail: Arc<Trail>,
    clock: Arc<dyn Clock>,
}

impl ConsentService {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        consent: Arc<dyn ConsentRepository>,
        trail: Arc<Trail>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            consent,
            trail,
            clock,
        }
    }

    /// Appends one decision for the caller. The history is never edited.
    ///
    /// # Errors
    /// Returns [`AppError::Unavailable`] when the record or its audit event
    /// cannot be written.
    pub async fn append(
        &self,
        caller: &Caller,
        purpose: ConsentPurpose,
        granted: bool,
        source: ConsentSource,
    ) -> Result<ConsentRecord, AppError> {
        let record = ConsentRecord::record(
            ConsentRecordId::generate(),
            caller.user_id(),
            purpose,
            granted,
            source,
            self.clock.now(),
        );
        self.consent
            .append(&record)
            .await
            .map_err(AppError::from_repository)?;
        self.trail
            .record(
                Some(caller.user_id()),
                "identity.consent.recorded",
                caller.user_id().to_string(),
                AuditOutcome::Success,
                &caller.correlation_id,
                context(&[
                    ("purpose", purpose.as_str()),
                    ("granted", if granted { "true" } else { "false" }),
                ]),
            )
            .await?;
        Ok(record)
    }

    /// Returns the newest analytics and marketing decisions (C3 `getConsent`).
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for another user's consent and
    /// [`AppError::Unavailable`] when the store cannot answer.
    pub async fn get(&self, caller: &Caller, user_id: UserId) -> Result<ConsentView, AppError> {
        caller.require_owner_or_admin(user_id)?;
        let analytics = self
            .consent
            .latest(user_id, ConsentPurpose::Analytics)
            .await
            .map_err(AppError::from_repository)?;
        let marketing = self
            .consent
            .latest(user_id, ConsentPurpose::Marketing)
            .await
            .map_err(AppError::from_repository)?;
        let view = ConsentView::from_latest(user_id, analytics.as_ref(), marketing.as_ref());
        self.trail
            .record(
                Some(caller.user_id()),
                "identity.consent.read",
                user_id.to_string(),
                AuditOutcome::Success,
                &caller.correlation_id,
                context(&[]),
            )
            .await?;
        Ok(view)
    }
}
