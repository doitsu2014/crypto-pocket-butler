//! U4 — withdraw a consent inside one transaction with its audit outbox row
//! (BR4.7, BR4.8, NFR19.1–19.3).
//!
//! The advisory lock, the newest read, the insert, and the outbox row are one
//! unit of work: the withdrawal commits with its audit intent or not at all.

use std::sync::Arc;

use identity_domain::{
    ConsentAction, ConsentEvent, ConsentEventDraft, ConsentKind, ConsentPolicy, ConsentState,
};

use super::ports::{
    AuditIntent, ConsentRepository, ConsentTransaction, ObservationSink, WithdrawalRollbackCause,
};
use crate::common::{AppError, Caller};

/// What a withdrawal did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WithdrawResult {
    /// The withdrawal was recorded.
    Withdrawn(ConsentEvent),
    /// The kind was already withdrawn; nothing was written.
    NoOp(ConsentState),
}

/// Records a withdrawal for the caller.
pub struct ConsentWithdraw {
    consent: Arc<dyn ConsentRepository>,
    observation: Arc<dyn ObservationSink>,
}

impl ConsentWithdraw {
    /// Creates the use case.
    #[must_use]
    pub fn new(consent: Arc<dyn ConsentRepository>, observation: Arc<dyn ObservationSink>) -> Self {
        Self {
            consent,
            observation,
        }
    }

    /// Withdraws one kind under the per-(user, kind) lock.
    ///
    /// # Errors
    /// Returns [`AppError::Conflict`] when the kind was never granted, and
    /// [`AppError::Unavailable`] when the event or its audit intent cannot be
    /// written.
    pub async fn withdraw(
        &self,
        caller: &Caller,
        kind: ConsentKind,
    ) -> Result<WithdrawResult, AppError> {
        let user_id = caller.user_id();
        let mut transaction = self
            .consent
            .begin_withdraw(user_id, kind)
            .await
            .map_err(AppError::from_repository)?;

        let newest = match transaction.newest(kind).await {
            Ok(newest) => newest,
            Err(error) => {
                self.roll_back(transaction, WithdrawalRollbackCause::InsertFailed)
                    .await;
                return Err(AppError::from_repository(error));
            }
        };
        let Some(grant) = newest else {
            self.roll_back_quietly(transaction).await;
            return Err(AppError::Conflict(format!(
                "consent kind '{}' was never granted",
                kind.as_str()
            )));
        };
        if grant.action() == ConsentAction::Withdrawn {
            self.roll_back_quietly(transaction).await;
            return Ok(WithdrawResult::NoOp(ConsentState::Withdrawn));
        }

        let policy = match ConsentPolicy::parse(grant.policy_version()) {
            Ok(policy) => policy,
            Err(error) => {
                self.roll_back_quietly(transaction).await;
                return Err(AppError::from_domain(error));
            }
        };
        let draft = ConsentEventDraft::withdraw(user_id, kind, policy);
        let event = match transaction.insert_event(&draft).await {
            Ok(event) => event,
            Err(error) => {
                self.roll_back(transaction, WithdrawalRollbackCause::InsertFailed)
                    .await;
                return Err(AppError::from_repository(error));
            }
        };

        let intent =
            AuditIntent::consent_withdrawn(&event, grant.policy_version(), &caller.correlation_id);
        if let Err(error) = transaction.enqueue_audit(&intent).await {
            self.roll_back(transaction, WithdrawalRollbackCause::OutboxFailed)
                .await;
            return Err(AppError::from_repository(error));
        }

        if let Err(error) = transaction.commit().await {
            self.observation
                .record_withdrawal_rollback(WithdrawalRollbackCause::InsertFailed);
            return Err(AppError::from_repository(error));
        }

        self.observation
            .record_consent_event(kind, ConsentAction::Withdrawn);
        Ok(WithdrawResult::Withdrawn(event))
    }

    async fn roll_back(
        &self,
        transaction: Box<dyn ConsentTransaction>,
        cause: WithdrawalRollbackCause,
    ) {
        if let Err(error) = transaction.rollback().await {
            tracing::warn!(%error, "consent withdrawal rollback failed");
        }
        self.observation.record_withdrawal_rollback(cause);
    }

    /// A rollback that is a normal outcome (the kind was already withdrawn or was
    /// never granted). It must not be counted as a withdrawal failure.
    async fn roll_back_quietly(&self, transaction: Box<dyn ConsentTransaction>) {
        if let Err(error) = transaction.rollback().await {
            tracing::warn!(%error, "consent withdrawal rollback failed");
        }
    }
}
