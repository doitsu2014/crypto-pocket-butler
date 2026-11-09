//! C7 privacy: export the financial rows and anonymise the mutable subject
//! (NFR9.2, NFR13.1).

use std::sync::Arc;

use ledger_domain::{Correction, LedgerSubjectId, Transaction};

use crate::common::AppError;
use crate::ports::{
    AnonymisationRepository, AnonymiserGateway, Clock, CorrectionRepository, IdGenerator,
    ServiceCaller, SubjectRepository, TransactionRepository,
};
use crate::trail::Trail;

/// The financial rows Identity assembles into the user's export.
#[derive(Debug, Clone, PartialEq)]
pub struct PrivacyExport {
    /// The user.
    pub user_id: String,
    /// The append-only transactions.
    pub transactions: Vec<Transaction>,
    /// The append-only corrections.
    pub corrections: Vec<Correction>,
}

/// The result of one anonymisation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnonymiseResult {
    /// The user.
    pub user_id: String,
    /// The non-reversible reference, when a subject existed.
    pub anonymised_ref: Option<String>,
    /// Whether an earlier request had already applied it.
    pub already_applied: bool,
    /// Whether the C10 audit retries were exhausted.
    pub audit_failed: bool,
}

/// Exports and anonymises a user's ledger data for Identity.
pub struct PrivacyService {
    transactions: Arc<dyn TransactionRepository>,
    corrections: Arc<dyn CorrectionRepository>,
    subjects: Arc<dyn SubjectRepository>,
    anonymiser: Arc<dyn AnonymiserGateway>,
    anonymisations: Arc<dyn AnonymisationRepository>,
    trail: Arc<Trail>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
}

impl PrivacyService {
    /// Creates the use case.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        transactions: Arc<dyn TransactionRepository>,
        corrections: Arc<dyn CorrectionRepository>,
        subjects: Arc<dyn SubjectRepository>,
        anonymiser: Arc<dyn AnonymiserGateway>,
        anonymisations: Arc<dyn AnonymisationRepository>,
        trail: Arc<Trail>,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            transactions,
            corrections,
            subjects,
            anonymiser,
            anonymisations,
            trail,
            clock,
            ids,
        }
    }

    /// Returns the transactions and corrections for the user.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for a caller without the export
    /// operation and [`AppError::Repository`] when a read fails.
    pub async fn export(
        &self,
        caller: &ServiceCaller,
        user_id: &str,
    ) -> Result<PrivacyExport, AppError> {
        if !caller.allows("privacy.export") {
            return Err(AppError::Forbidden);
        }
        let Some(subject) = self.subjects.find_by_user(user_id).await? else {
            return Ok(PrivacyExport {
                user_id: user_id.to_string(),
                transactions: Vec::new(),
                corrections: Vec::new(),
            });
        };
        Ok(PrivacyExport {
            user_id: user_id.to_string(),
            transactions: self.transactions.list_all(subject.id()).await?,
            corrections: self.corrections.list_for_subject(subject.id()).await?,
        })
    }

    /// Clears the subject's user id and sets a non-reversible reference. The
    /// transaction and correction rows stay untouched. A retry applies once.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for a caller other than Identity or one
    /// without the anonymise operation, and [`AppError::Rejected`] for a blank
    /// idempotency key.
    pub async fn anonymise(
        &self,
        caller: &ServiceCaller,
        user_id: &str,
        idempotency_key: &str,
    ) -> Result<AnonymiseResult, AppError> {
        if caller.service != "identity" || !caller.allows("privacy.anonymise") {
            return Err(AppError::Forbidden);
        }
        if idempotency_key.trim().is_empty() {
            return Err(AppError::field("idempotencyKey", "is required"));
        }
        let key = idempotency_key.trim();
        if let Some(existing) = self.anonymisations.find(key).await? {
            return Ok(AnonymiseResult {
                user_id: user_id.to_string(),
                anonymised_ref: Some(existing),
                already_applied: true,
                audit_failed: false,
            });
        }
        let Some(subject) = self.subjects.find_by_user(user_id).await? else {
            return Ok(AnonymiseResult {
                user_id: user_id.to_string(),
                anonymised_ref: None,
                already_applied: true,
                audit_failed: false,
            });
        };
        if let Some(existing) = subject.anonymised_ref() {
            let existing = existing.to_string();
            self.anonymisations
                .record(key, &existing, self.clock.now())
                .await?;
            return Ok(AnonymiseResult {
                user_id: user_id.to_string(),
                anonymised_ref: Some(existing),
                already_applied: true,
                audit_failed: false,
            });
        }
        let anonymised_ref = self.ids.next().to_string();
        self.anonymiser
            .anonymise(subject.id(), &anonymised_ref)
            .await?;
        self.anonymisations
            .record(key, &anonymised_ref, self.clock.now())
            .await?;
        let audit_failed = !self.trail.anonymised(&anonymised_ref, "success").await;
        Ok(AnonymiseResult {
            user_id: user_id.to_string(),
            anonymised_ref: Some(anonymised_ref),
            already_applied: false,
            audit_failed,
        })
    }

    /// The subject identifier for a user, when one exists.
    ///
    /// # Errors
    /// Returns [`AppError::Repository`] when the read fails.
    pub async fn subject_id(&self, user_id: &str) -> Result<Option<LedgerSubjectId>, AppError> {
        Ok(self
            .subjects
            .find_by_user(user_id)
            .await?
            .map(|subject| subject.id()))
    }
}
