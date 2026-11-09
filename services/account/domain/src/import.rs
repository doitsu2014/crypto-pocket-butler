//! CSV imports: format detection, row parsing, dedupe keys, per-line errors, and
//! the reconciling summary (BR5.1–BR5.4).

use chrono::{DateTime, NaiveDateTime, Utc};
use rust_decimal::Decimal;

use crate::error::DomainError;
use crate::ids::{AccountId, ImportBatchId, ImportRowErrorId, ImportedTransactionId, UserId};

/// The largest CSV the service stages in one upload.
pub const CSV_MAX_BYTES: usize = 10 * 1024 * 1024;
/// The longest file name accepted.
pub const MAX_FILE_NAME_CHARS: usize = 255;
/// The row chunk size used for batched inserts and bounded memory.
pub const INSERT_CHUNK_ROWS: usize = 500;

/// The CSV format the header matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportSource {
    /// Binance's transaction export.
    Binance,
    /// OKX's transaction export.
    Okx,
    /// The service's generic six-column format.
    Generic,
}

impl ImportSource {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Binance => "binance",
            Self::Okx => "okx",
            Self::Generic => "generic",
        }
    }

    /// Parses a stored label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown source.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "binance" => Ok(Self::Binance),
            "okx" => Ok(Self::Okx),
            "generic" => Ok(Self::Generic),
            _ => Err(DomainError::Invalid {
                field: "source",
                reason: "must be binance, okx, or generic".to_string(),
            }),
        }
    }

    /// Detects the format from the header line.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown format.
    pub fn detect(header: &str) -> Result<Self, DomainError> {
        let lowered = header.trim().to_ascii_lowercase();
        if lowered.contains("utc_time") && lowered.contains("operation") {
            return Ok(Self::Binance);
        }
        if lowered.contains("timestamp")
            && lowered.contains("type")
            && (lowered.contains("asset") || lowered.contains("coin"))
        {
            return Ok(Self::Okx);
        }
        if lowered.contains("external_id")
            || (lowered.contains("kind")
                && lowered.contains("executed_at")
                && lowered.contains("asset"))
        {
            return Ok(Self::Generic);
        }
        Err(DomainError::Invalid {
            field: "file",
            reason: "is not a supported CSV format (binance, okx, generic)".to_string(),
        })
    }
}

/// One parsed CSV row.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedTransaction {
    /// The venue's own row id, when the CSV carries one.
    pub external_id: Option<String>,
    /// The asset identifier.
    pub asset_id: String,
    /// What the row records.
    pub kind: TransactionKind,
    /// The decimal quantity.
    pub quantity: Decimal,
    /// The decimal unit price, when present.
    pub price: Option<Decimal>,
    /// When the transaction executed.
    pub executed_at: DateTime<Utc>,
}

impl ParsedTransaction {
    /// The stable dedupe key for this row (BR5.2). With an external id the key is
    /// `(accountId, externalId)`; otherwise it is
    /// `(accountId, kind, asset, quantity, executedAt)`.
    #[must_use]
    pub fn dedupe_key(&self, account_id: AccountId) -> String {
        dedupe_key_for(
            account_id,
            self.external_id.as_deref(),
            self.kind,
            &self.asset_id,
            self.quantity,
            self.executed_at,
        )
    }
}

/// Builds the dedupe key used by the unique index.
#[must_use]
pub fn dedupe_key_for(
    account_id: AccountId,
    external_id: Option<&str>,
    kind: TransactionKind,
    asset_id: &str,
    quantity: Decimal,
    executed_at: DateTime<Utc>,
) -> String {
    match external_id.map(str::trim).filter(|value| !value.is_empty()) {
        Some(external) => format!("{account_id}|ext|{external}"),
        None => format!(
            "{account_id}|row|{}|{}|{}|{}",
            kind.as_str(),
            asset_id.trim(),
            quantity.normalize(),
            executed_at.to_rfc3339()
        ),
    }
}

/// The transaction kinds the ledger consumes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionKind {
    /// A purchase.
    Buy,
    /// A sale.
    Sell,
    /// A deposit or inbound transfer.
    TransferIn,
    /// A withdrawal or outbound transfer.
    TransferOut,
    /// A fee.
    Fee,
}

impl TransactionKind {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Buy => "buy",
            Self::Sell => "sell",
            Self::TransferIn => "transfer_in",
            Self::TransferOut => "transfer_out",
            Self::Fee => "fee",
        }
    }

    /// Parses a label or a venue's own operation name.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown kind.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_ascii_lowercase().replace(' ', "_").as_str() {
            "buy" | "purchase" => Ok(Self::Buy),
            "sell" => Ok(Self::Sell),
            "transfer_in" | "deposit" | "deposit_in" | "in" => Ok(Self::TransferIn),
            "transfer_out" | "withdrawal" | "withdraw" | "out" => Ok(Self::TransferOut),
            "fee" | "commission" => Ok(Self::Fee),
            _ => Err(DomainError::Invalid {
                field: "kind",
                reason: "must be buy, sell, transfer_in, transfer_out, or fee".to_string(),
            }),
        }
    }
}

/// A transaction row stored for Ledger to consume through C5.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedTransaction {
    id: ImportedTransactionId,
    batch_id: ImportBatchId,
    account_id: AccountId,
    external_id: Option<String>,
    dedupe_key: String,
    asset_id: String,
    kind: TransactionKind,
    quantity: Decimal,
    price: Option<Decimal>,
    executed_at: DateTime<Utc>,
}

impl ImportedTransaction {
    /// Creates a stored transaction from a parsed row.
    ///
    /// # Errors
    /// Returns [`DomainError::Required`] for a blank asset id.
    pub fn record(
        id: ImportedTransactionId,
        batch_id: ImportBatchId,
        account_id: AccountId,
        parsed: &ParsedTransaction,
    ) -> Result<Self, DomainError> {
        let asset = parsed.asset_id.trim();
        if asset.is_empty() {
            return Err(DomainError::Required { field: "assetId" });
        }
        Ok(Self {
            id,
            batch_id,
            account_id,
            external_id: parsed.external_id.clone(),
            dedupe_key: parsed.dedupe_key(account_id),
            asset_id: asset.to_string(),
            kind: parsed.kind,
            quantity: parsed.quantity,
            price: parsed.price,
            executed_at: parsed.executed_at,
        })
    }

    /// Rehydrates a stored transaction.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: ImportedTransactionId,
        batch_id: ImportBatchId,
        account_id: AccountId,
        external_id: Option<String>,
        dedupe_key: String,
        asset_id: String,
        kind: TransactionKind,
        quantity: Decimal,
        price: Option<Decimal>,
        executed_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            batch_id,
            account_id,
            external_id,
            dedupe_key,
            asset_id,
            kind,
            quantity,
            price,
            executed_at,
        }
    }

    /// The row identifier.
    #[must_use]
    pub fn id(&self) -> ImportedTransactionId {
        self.id
    }

    /// The batch that produced the row.
    #[must_use]
    pub fn batch_id(&self) -> ImportBatchId {
        self.batch_id
    }

    /// The owning account.
    #[must_use]
    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    /// The venue's own row id.
    #[must_use]
    pub fn external_id(&self) -> Option<&str> {
        self.external_id.as_deref()
    }

    /// The stable dedupe key.
    #[must_use]
    pub fn dedupe_key(&self) -> &str {
        &self.dedupe_key
    }

    /// The asset identifier.
    #[must_use]
    pub fn asset_id(&self) -> &str {
        &self.asset_id
    }

    /// The transaction kind.
    #[must_use]
    pub fn kind(&self) -> TransactionKind {
        self.kind
    }

    /// The decimal quantity.
    #[must_use]
    pub fn quantity(&self) -> Decimal {
        self.quantity
    }

    /// The decimal unit price.
    #[must_use]
    pub fn price(&self) -> Option<Decimal> {
        self.price
    }

    /// When the transaction executed.
    #[must_use]
    pub fn executed_at(&self) -> DateTime<Utc> {
        self.executed_at
    }
}

/// The batch lifecycle state. The worker owns the transition to processing and
/// completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportBatchStatus {
    /// Staged and waiting for the worker.
    Pending,
    /// Being parsed by the worker.
    Processing,
    /// Parsed and counted.
    Complete,
    /// The worker could not process the file.
    Failed,
}

impl ImportBatchStatus {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Processing => "processing",
            Self::Complete => "complete",
            Self::Failed => "failed",
        }
    }

    /// Parses a stored label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown status.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "pending" => Ok(Self::Pending),
            "processing" => Ok(Self::Processing),
            "complete" => Ok(Self::Complete),
            "failed" => Ok(Self::Failed),
            _ => Err(DomainError::Invalid {
                field: "status",
                reason: "must be pending, processing, complete, or failed".to_string(),
            }),
        }
    }
}

/// One CSV import into a csv account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportBatch {
    id: ImportBatchId,
    account_id: AccountId,
    source: ImportSource,
    file_name: String,
    status: ImportBatchStatus,
    imported_count: u32,
    skipped_count: u32,
    failed_count: u32,
    created_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
}

impl ImportBatch {
    /// Creates a pending batch.
    ///
    /// # Errors
    /// Returns [`DomainError::Required`] for a blank file name and
    /// [`DomainError::TooLarge`] for one longer than [`MAX_FILE_NAME_CHARS`].
    pub fn new(
        id: ImportBatchId,
        account_id: AccountId,
        source: ImportSource,
        file_name: &str,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let name = file_name.trim();
        if name.is_empty() {
            return Err(DomainError::Required { field: "fileName" });
        }
        if name.chars().count() > MAX_FILE_NAME_CHARS {
            return Err(DomainError::TooLarge {
                field: "fileName",
                limit: "255 characters",
            });
        }
        Ok(Self {
            id,
            account_id,
            source,
            file_name: name.to_string(),
            status: ImportBatchStatus::Pending,
            imported_count: 0,
            skipped_count: 0,
            failed_count: 0,
            created_at: now,
            completed_at: None,
        })
    }

    /// Rehydrates a stored batch.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: ImportBatchId,
        account_id: AccountId,
        source: ImportSource,
        file_name: String,
        status: ImportBatchStatus,
        imported_count: u32,
        skipped_count: u32,
        failed_count: u32,
        created_at: DateTime<Utc>,
        completed_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            account_id,
            source,
            file_name,
            status,
            imported_count,
            skipped_count,
            failed_count,
            created_at,
            completed_at,
        }
    }

    /// The batch identifier.
    #[must_use]
    pub fn id(&self) -> ImportBatchId {
        self.id
    }

    /// The owning account.
    #[must_use]
    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    /// The detected format.
    #[must_use]
    pub fn source(&self) -> ImportSource {
        self.source
    }

    /// The uploaded file name.
    #[must_use]
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    /// The lifecycle state.
    #[must_use]
    pub fn status(&self) -> ImportBatchStatus {
        self.status
    }

    /// Rows stored.
    #[must_use]
    pub fn imported_count(&self) -> u32 {
        self.imported_count
    }

    /// Duplicate rows skipped.
    #[must_use]
    pub fn skipped_count(&self) -> u32 {
        self.skipped_count
    }

    /// Rows that failed validation.
    #[must_use]
    pub fn failed_count(&self) -> u32 {
        self.failed_count
    }

    /// When the upload was staged.
    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// When processing finished.
    #[must_use]
    pub fn completed_at(&self) -> Option<DateTime<Utc>> {
        self.completed_at
    }

    /// Marks the batch processing.
    pub fn begin_processing(&mut self) {
        self.status = ImportBatchStatus::Processing;
    }

    /// Records the reconciling counts and completes the batch (BR5.4).
    pub fn complete(&mut self, imported: u32, skipped: u32, failed: u32, at: DateTime<Utc>) {
        self.imported_count = imported;
        self.skipped_count = skipped;
        self.failed_count = failed;
        self.status = ImportBatchStatus::Complete;
        self.completed_at = Some(at);
    }

    /// Records a batch-level failure while keeping the counts so far.
    pub fn fail(&mut self, at: DateTime<Utc>) {
        self.status = ImportBatchStatus::Failed;
        self.completed_at = Some(at);
    }
}

/// One bad CSV row, with its line number and reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportRowError {
    id: ImportRowErrorId,
    batch_id: ImportBatchId,
    line_number: u32,
    reason: String,
}

impl ImportRowError {
    /// Records one bad row. The line number is the 1-based file line.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for line 0 and
    /// [`DomainError::Required`] for a blank reason.
    pub fn record(
        id: ImportRowErrorId,
        batch_id: ImportBatchId,
        line_number: u32,
        reason: &str,
    ) -> Result<Self, DomainError> {
        if line_number == 0 {
            return Err(DomainError::Invalid {
                field: "lineNumber",
                reason: "must be at least 1".to_string(),
            });
        }
        let reason = reason.trim();
        if reason.is_empty() {
            return Err(DomainError::Required { field: "reason" });
        }
        Ok(Self {
            id,
            batch_id,
            line_number,
            reason: reason.chars().take(300).collect(),
        })
    }

    /// Rehydrates a stored row error.
    #[must_use]
    pub fn from_stored(
        id: ImportRowErrorId,
        batch_id: ImportBatchId,
        line_number: u32,
        reason: String,
    ) -> Self {
        Self {
            id,
            batch_id,
            line_number,
            reason,
        }
    }

    /// The error identifier.
    #[must_use]
    pub fn id(&self) -> ImportRowErrorId {
        self.id
    }

    /// The batch.
    #[must_use]
    pub fn batch_id(&self) -> ImportBatchId {
        self.batch_id
    }

    /// The 1-based file line.
    #[must_use]
    pub fn line_number(&self) -> u32 {
        self.line_number
    }

    /// The reason, without the row's values.
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

/// The reconciling result of one import (BR5.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportSummary {
    /// Rows stored.
    pub imported: u32,
    /// Duplicate rows skipped.
    pub skipped: u32,
    /// Rows that failed validation.
    pub failed: u32,
    /// The per-line errors.
    pub row_errors: Vec<ImportRowError>,
}

impl ImportSummary {
    /// Whether the three counts reconcile to the data rows.
    #[must_use]
    pub fn reconciles(&self, data_rows: u32) -> bool {
        self.imported + self.skipped + self.failed == data_rows
    }
}

/// Splits one CSV line, honouring quoted fields and doubled quotes.
///
/// # Errors
/// Returns a message when a quoted field is unterminated. The line's values are
/// never included in the message.
pub fn parse_csv_line(line: &str) -> Result<Vec<String>, String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut characters = line.chars().peekable();
    let mut in_quotes = false;
    while let Some(character) = characters.next() {
        match character {
            '"' if in_quotes => {
                if characters.peek() == Some(&'"') {
                    current.push('"');
                    characters.next();
                } else {
                    in_quotes = false;
                }
            }
            '"' if current.trim().is_empty() => {
                in_quotes = true;
                current.clear();
            }
            ',' if !in_quotes => {
                fields.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(character),
        }
    }
    if in_quotes {
        return Err("a quoted field is unterminated".to_string());
    }
    fields.push(current.trim().to_string());
    Ok(fields)
}

/// Parses a timestamp from a generic RFC 3339 value or a venue's plain form.
///
/// # Errors
/// Returns a message when the value is not a known timestamp shape.
pub fn parse_executed_at(raw: &str) -> Result<DateTime<Utc>, String> {
    let value = raw.trim();
    if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
        return Ok(parsed.with_timezone(&Utc));
    }
    for format in ["%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S"] {
        if let Ok(parsed) = NaiveDateTime::parse_from_str(value, format) {
            return Ok(parsed.and_utc());
        }
    }
    if let Ok(date) = chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        if let Some(midnight) = date.and_hms_opt(0, 0, 0) {
            return Ok(midnight.and_utc());
        }
    }
    Err("is not a recognised timestamp".to_string())
}

/// A privacy-export view of one account, carrying no credential reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportedAccount {
    /// The account identifier.
    pub id: AccountId,
    /// The owner.
    pub user_id: UserId,
    /// The source kind.
    pub kind: String,
    /// The venue or chain.
    pub venue: String,
    /// The display label.
    pub label: String,
    /// The lifecycle state.
    pub status: String,
    /// The last successful sync.
    pub last_synced_at: Option<DateTime<Utc>>,
}
