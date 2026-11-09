//! WF5: correct a transaction with per-field validation, a recorded old value,
//! and a C10 audit written after the commit (BR1.1, BR2.3, BR4.1, NFR1.4).

use std::sync::Arc;

use ledger_domain::{
    effective_value, parse_field_value, Correction, CorrectionField, CorrectionId, DomainError,
    PnlReport, TransactionId,
};

use crate::common::AppError;
use crate::hashing::sha256_hex;
use crate::ingest::IngestService;
use crate::ports::{
    AssetResolver, Clock, CorrectionRepository, IdGenerator, TransactionRepository,
};
use crate::trail::Trail;

/// A stored correction and the state of its follow-ups.
#[derive(Debug, Clone)]
pub struct CorrectionResult {
    /// The append-only correction row.
    pub correction: Correction,
    /// The recomputed P&L, when the rebuild succeeded.
    pub report: Option<PnlReport>,
    /// Whether the C10 audit retries were exhausted.
    pub audit_failed: bool,
    /// Whether the derived state could not be rebuilt.
    pub rebuild_failed: bool,
}

/// Records corrections.
pub struct CorrectionsService {
    transactions: Arc<dyn TransactionRepository>,
    corrections: Arc<dyn CorrectionRepository>,
    ingest: Arc<IngestService>,
    assets: Arc<dyn AssetResolver>,
    pnl: Arc<crate::reads::PnlService>,
    trail: Arc<Trail>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
}

impl CorrectionsService {
    /// Creates the use case.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        transactions: Arc<dyn TransactionRepository>,
        corrections: Arc<dyn CorrectionRepository>,
        ingest: Arc<IngestService>,
        assets: Arc<dyn AssetResolver>,
        pnl: Arc<crate::reads::PnlService>,
        trail: Arc<Trail>,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            transactions,
            corrections,
            ingest,
            assets,
            pnl,
            trail,
            clock,
            ids,
        }
    }

    /// Records a correction owned by `user_id`.
    ///
    /// # Errors
    /// Returns [`AppError::Rejected`] for a blank reason or an invalid value,
    /// [`AppError::NotFound`] for an unknown transaction, and
    /// [`AppError::Forbidden`] when the caller does not own it.
    pub async fn correct(
        &self,
        user_id: &str,
        transaction_id: TransactionId,
        field: &str,
        new_value: &str,
        reason: &str,
    ) -> Result<CorrectionResult, AppError> {
        let field = CorrectionField::parse(field).map_err(AppError::from)?;
        let subject = self.ingest.ensure_subject(user_id).await?;
        let transaction = self
            .transactions
            .find_by_id(transaction_id)
            .await?
            .ok_or(AppError::NotFound)?;
        if transaction.subject_id() != subject.id() {
            return Err(AppError::Forbidden);
        }
        if reason.trim().is_empty() {
            return Err(AppError::field("reason", "is required"));
        }
        let now = self.clock.now();
        let value = parse_field_value(field, new_value, now).map_err(AppError::from)?;
        if field == CorrectionField::AssetId {
            if let ledger_domain::FieldValue::AssetId(asset_id) = &value {
                let exists = self
                    .assets
                    .exists(asset_id)
                    .await
                    .map_err(AppError::External)?;
                if !exists {
                    return Err(AppError::field("newValue", "assetId is not registered"));
                }
            }
        }
        let existing = self
            .corrections
            .list_for_transaction(transaction_id)
            .await?;
        let old_value = effective_value(&transaction, &existing, field);
        let correction = Correction::record(
            CorrectionId::new(self.ids.next()),
            transaction_id,
            field,
            old_value,
            value.as_text(),
            reason,
            user_id,
            now,
        )
        .map_err(|error| match error {
            DomainError::Required { field } => AppError::field(field, "is required"),
            other => AppError::from(other),
        })?;
        self.corrections.insert(&correction).await?;
        let audit_failed = !self
            .trail
            .correction_recorded(
                user_id,
                &transaction_id.to_string(),
                field.as_str(),
                &sha256_hex(reason),
                "success",
            )
            .await;
        let (report, rebuild_failed) = match self.pnl.report(user_id, None).await {
            Ok(report) => (Some(report), false),
            Err(_) => {
                tracing::error!("the P&L refresh failed after a correction");
                (None, true)
            }
        };
        Ok(CorrectionResult {
            correction,
            report,
            audit_failed,
            rebuild_failed,
        })
    }
}
