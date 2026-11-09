//! WF5: CSV upload, format detection, row parsing, dedupe, per-line errors, and
//! the reconciling summary (BR5.1–BR5.4).

use std::collections::HashSet;
use std::sync::Arc;

use account_domain::import::{
    parse_csv_line, parse_executed_at, ImportBatchStatus, INSERT_CHUNK_ROWS,
};
use account_domain::{
    parse_quantity, AccountId, AuditOutcome, ImportBatch, ImportBatchId, ImportRowError,
    ImportRowErrorId, ImportSource, ImportSummary, ImportedTransaction, ImportedTransactionId,
    ParsedTransaction, TransactionKind, CSV_MAX_BYTES,
};

use crate::common::{AppError, Caller, FieldError};
use crate::ports::{AccountRepository, Clock, ImportRepository};
use crate::trail::{context, Trail};

/// Stages and processes CSV imports.
pub struct ImportService {
    accounts: Arc<dyn AccountRepository>,
    imports: Arc<dyn ImportRepository>,
    trail: Arc<Trail>,
    clock: Arc<dyn Clock>,
}

impl ImportService {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        accounts: Arc<dyn AccountRepository>,
        imports: Arc<dyn ImportRepository>,
        trail: Arc<Trail>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            accounts,
            imports,
            trail,
            clock,
        }
    }

    /// Stages a CSV upload for a csv account and returns the pending batch.
    ///
    /// # Errors
    /// Returns a forbidden refusal for another user's account, a validation
    /// refusal for a non-CSV account, an unknown format, or an oversized file.
    pub async fn upload(
        &self,
        caller: &Caller,
        account_id: AccountId,
        file_name: &str,
        bytes: &[u8],
    ) -> Result<ImportBatch, AppError> {
        let account = self
            .accounts
            .find_by_id(account_id)
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("account"))?;
        caller.require_owner(account.user_id())?;
        if !account.is_csv() {
            return Err(AppError::Validation(vec![FieldError::new(
                "accountId",
                "is not a CSV account",
            )]));
        }
        if bytes.is_empty() {
            return Err(AppError::Validation(vec![FieldError::new(
                "file", "is empty",
            )]));
        }
        if bytes.len() > CSV_MAX_BYTES {
            return Err(AppError::Validation(vec![FieldError::new(
                "file",
                "exceeds the 10 MiB import cap",
            )]));
        }
        let content = std::str::from_utf8(bytes).map_err(|_| {
            AppError::Validation(vec![FieldError::new("file", "is not valid UTF-8")])
        })?;
        let header = content.lines().next().unwrap_or_default();
        let source = ImportSource::detect(header).map_err(AppError::from_domain)?;
        let batch = ImportBatch::new(
            ImportBatchId::generate(),
            account_id,
            source,
            file_name,
            self.clock.now(),
        )
        .map_err(AppError::from_domain)?;
        self.imports
            .create_batch(&batch, bytes)
            .await
            .map_err(AppError::from_repository)?;
        Ok(batch)
    }

    /// Reads one batch for its owner.
    ///
    /// # Errors
    /// Returns [`AppError::NotFound`] for an unknown batch and
    /// [`AppError::Forbidden`] for another user's batch.
    pub async fn find(
        &self,
        caller: &Caller,
        batch_id: ImportBatchId,
    ) -> Result<ImportBatch, AppError> {
        let batch = self.load(batch_id).await?;
        let account = self
            .accounts
            .find_by_id(batch.account_id())
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("account"))?;
        caller.require_owner(account.user_id())?;
        Ok(batch)
    }

    /// Claims and processes the oldest pending batch.
    ///
    /// # Errors
    /// Returns a repository refusal when the batch cannot be read or written.
    pub async fn process_pending(
        &self,
        correlation_id: &str,
    ) -> Result<Option<ImportSummary>, AppError> {
        let Some(batch) = self
            .imports
            .claim_pending()
            .await
            .map_err(AppError::from_repository)?
        else {
            return Ok(None);
        };
        self.process(batch.id(), correlation_id).await.map(Some)
    }

    /// Parses one batch, imports its rows, records the per-line errors, deletes
    /// the staged bytes, and returns the reconciling summary.
    ///
    /// # Errors
    /// Returns [`AppError::NotFound`] for an unknown batch and a repository
    /// refusal when a write fails.
    pub async fn process(
        &self,
        batch_id: ImportBatchId,
        correlation_id: &str,
    ) -> Result<ImportSummary, AppError> {
        let mut batch = self.load(batch_id).await?;
        if batch.status() == ImportBatchStatus::Complete {
            return Ok(ImportSummary {
                imported: batch.imported_count(),
                skipped: batch.skipped_count(),
                failed: batch.failed_count(),
                row_errors: Vec::new(),
            });
        }
        if batch.status() == ImportBatchStatus::Failed {
            return Err(AppError::Conflict(
                "the batch previously failed".to_string(),
            ));
        }
        if batch.status() == ImportBatchStatus::Pending {
            batch.begin_processing();
            self.imports
                .update_batch(&batch)
                .await
                .map_err(AppError::from_repository)?;
        }

        let Some(bytes) = self
            .imports
            .staged_bytes(batch_id)
            .await
            .map_err(AppError::from_repository)?
        else {
            batch.fail(self.clock.now());
            self.imports
                .update_batch(&batch)
                .await
                .map_err(AppError::from_repository)?;
            return Err(AppError::Unavailable(
                "the staged upload is missing".to_string(),
            ));
        };
        let content = match std::str::from_utf8(&bytes) {
            Ok(content) => content,
            Err(_) => {
                batch.fail(self.clock.now());
                self.imports
                    .update_batch(&batch)
                    .await
                    .map_err(AppError::from_repository)?;
                return Err(AppError::Unavailable(
                    "the staged upload is not valid UTF-8".to_string(),
                ));
            }
        };

        let now = self.clock.now();
        let mut pending: Vec<ImportedTransaction> = Vec::new();
        let mut row_errors: Vec<ImportRowError> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        let mut imported: u32 = 0;
        let mut skipped: u32 = 0;
        let mut failed: u32 = 0;

        for (offset, line) in content.lines().skip(1).enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let line_number = (offset + 2) as u32;
            match self.parse_row(batch.source(), line) {
                Ok(parsed) => {
                    let key = parsed.dedupe_key(batch.account_id());
                    if seen.contains(&key)
                        || self
                            .imports
                            .dedupe_exists(batch.account_id(), &key)
                            .await
                            .map_err(AppError::from_repository)?
                    {
                        skipped += 1;
                        continue;
                    }
                    seen.insert(key);
                    match ImportedTransaction::record(
                        ImportedTransactionId::generate(),
                        batch.id(),
                        batch.account_id(),
                        &parsed,
                    ) {
                        Ok(row) => pending.push(row),
                        Err(error) => {
                            failed += 1;
                            if let Ok(record) = ImportRowError::record(
                                ImportRowErrorId::generate(),
                                batch.id(),
                                line_number,
                                &error.to_string(),
                            ) {
                                row_errors.push(record);
                            }
                        }
                    }
                }
                Err(reason) => {
                    failed += 1;
                    if let Ok(record) = ImportRowError::record(
                        ImportRowErrorId::generate(),
                        batch.id(),
                        line_number,
                        &reason,
                    ) {
                        row_errors.push(record);
                    }
                }
            }
            if pending.len() >= INSERT_CHUNK_ROWS {
                let (inserted, conflicts) = self.flush(&mut pending).await?;
                imported += inserted;
                skipped += conflicts;
            }
        }
        let (inserted, conflicts) = self.flush(&mut pending).await?;
        imported += inserted;
        skipped += conflicts;

        self.imports
            .insert_row_errors(&row_errors)
            .await
            .map_err(AppError::from_repository)?;
        batch.complete(imported, skipped, failed, now);
        self.imports
            .update_batch(&batch)
            .await
            .map_err(AppError::from_repository)?;
        if let Err(error) = self.imports.delete_staging(batch_id).await {
            tracing::warn!(%error, "the staged upload could not be deleted");
        }

        self.trail
            .record(
                None,
                "account.import.completed",
                batch_id.to_string(),
                AuditOutcome::Success,
                correlation_id,
                context(&[
                    ("batchId", &batch_id.to_string()),
                    ("imported", &imported.to_string()),
                    ("skipped", &skipped.to_string()),
                    ("failed", &failed.to_string()),
                ]),
            )
            .await?;

        Ok(ImportSummary {
            imported,
            skipped,
            failed,
            row_errors,
        })
    }

    async fn load(&self, batch_id: ImportBatchId) -> Result<ImportBatch, AppError> {
        self.imports
            .find_batch(batch_id)
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("import batch"))
    }

    async fn flush(&self, rows: &mut Vec<ImportedTransaction>) -> Result<(u32, u32), AppError> {
        let mut inserted = 0u64;
        for chunk in rows.chunks(INSERT_CHUNK_ROWS) {
            inserted += self
                .imports
                .insert_transactions(chunk)
                .await
                .map_err(AppError::from_repository)?;
        }
        let conflicts = rows.len() as u32 - inserted as u32;
        rows.clear();
        Ok((inserted as u32, conflicts))
    }

    fn parse_row(&self, source: ImportSource, line: &str) -> Result<ParsedTransaction, String> {
        let fields = parse_csv_line(line)?;
        match source {
            ImportSource::Generic => {
                let external = fields.first().map(|value| value.trim().to_string());
                let kind = required(&fields, 1, "kind")?;
                let asset = required(&fields, 2, "asset")?;
                let quantity = quantity(&fields, 3)?;
                let price = optional_price(&fields, 4)?;
                let executed_at = parse_executed_at(required(&fields, 5, "executed_at")?)?;
                Ok(ParsedTransaction {
                    external_id: external.filter(|value| !value.is_empty()),
                    asset_id: asset.to_string(),
                    kind: TransactionKind::parse(kind).map_err(|error| error.to_string())?,
                    quantity,
                    price,
                    executed_at,
                })
            }
            ImportSource::Binance => {
                let executed_at = parse_executed_at(required(&fields, 0, "UTC_Time")?)?;
                let kind = required(&fields, 2, "Operation")?;
                let asset = required(&fields, 3, "Coin")?;
                let quantity = absolute_quantity(&fields, 4)?;
                Ok(ParsedTransaction {
                    external_id: None,
                    asset_id: asset.to_string(),
                    kind: TransactionKind::parse(kind).map_err(|error| error.to_string())?,
                    quantity,
                    price: None,
                    executed_at,
                })
            }
            ImportSource::Okx => {
                let executed_at = parse_executed_at(required(&fields, 0, "timestamp")?)?;
                let kind = required(&fields, 1, "type")?;
                let asset = required(&fields, 2, "asset")?;
                let quantity = absolute_quantity(&fields, 3)?;
                let price = optional_price(&fields, 4)?;
                Ok(ParsedTransaction {
                    external_id: None,
                    asset_id: asset.to_string(),
                    kind: TransactionKind::parse(kind).map_err(|error| error.to_string())?,
                    quantity,
                    price,
                    executed_at,
                })
            }
        }
    }
}

fn required<'a>(fields: &'a [String], index: usize, name: &str) -> Result<&'a str, String> {
    fields
        .get(index)
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("is missing the {name} column"))
}

fn quantity(fields: &[String], index: usize) -> Result<rust_decimal::Decimal, String> {
    let raw = required(fields, index, "quantity")?;
    parse_quantity(raw).map_err(|error| error.to_string())
}

/// Venue exports write withdrawals and fees as negatives; the stored quantity is
/// the absolute movement and the kind carries the direction.
fn absolute_quantity(fields: &[String], index: usize) -> Result<rust_decimal::Decimal, String> {
    let raw = required(fields, index, "quantity")?;
    let absolute = raw.trim().trim_start_matches(['-', '+']);
    parse_quantity(absolute).map_err(|error| error.to_string())
}

fn optional_price(
    fields: &[String],
    index: usize,
) -> Result<Option<rust_decimal::Decimal>, String> {
    match fields
        .get(index)
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        Some(raw) => parse_quantity(raw)
            .map(Some)
            .map_err(|error| error.to_string()),
        None => Ok(None),
    }
}
