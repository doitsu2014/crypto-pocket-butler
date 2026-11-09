//! Wire DTOs. They are mapped to and from domain types only here, at the HTTP
//! boundary; no DTO type leaks inward.

use chrono::{DateTime, Utc};
use ledger_application::{
    AnonymiseResult, CorrectionResult, IngestReport, PrivacyExport, ReconcileReport, RecordedTrade,
    TransactionPage,
};
use ledger_domain::{canonical_decimal, AssetPnl, Correction, PnlReport, Transaction};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One transaction as the C7 read returns it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionDto {
    /// The identifier.
    pub id: String,
    /// The connected account, when any.
    pub account_id: Option<String>,
    /// The asset.
    pub asset_id: String,
    /// The movement kind.
    pub kind: String,
    /// The quantity, as a decimal string.
    pub quantity: String,
    /// The unit price, absent when unknown; never zero as a stand-in.
    pub price: Option<String>,
    /// Where the price came from.
    pub price_source: Option<String>,
    /// The ISO currency the price is quoted in.
    pub currency: String,
    /// The source system.
    pub source: String,
    /// The source system's row identifier.
    pub external_ref: Option<String>,
    /// When the movement happened.
    pub executed_at: DateTime<Utc>,
    /// When it was recorded.
    pub recorded_at: DateTime<Utc>,
    /// Whether the price is missing.
    pub price_missing: bool,
}

impl TransactionDto {
    /// Maps a domain transaction.
    #[must_use]
    pub fn from_transaction(transaction: &Transaction) -> Self {
        Self {
            id: transaction.id().to_string(),
            account_id: transaction.account_id().map(|id| id.to_string()),
            asset_id: transaction.asset_id().to_string(),
            kind: transaction.kind().as_str().to_string(),
            quantity: canonical_decimal(transaction.quantity()),
            price: transaction.price().map(canonical_decimal),
            price_source: transaction
                .price_source()
                .map(|source| source.as_str().to_string()),
            currency: transaction.currency().to_string(),
            source: transaction.source().as_str().to_string(),
            external_ref: transaction.external_ref().map(str::to_string),
            executed_at: transaction.executed_at(),
            recorded_at: transaction.recorded_at(),
            price_missing: transaction.price_missing(),
        }
    }
}

/// One page of transactions.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionPageResponse {
    /// The transactions, newest first.
    pub transactions: Vec<TransactionDto>,
    /// The cursor for the next page.
    pub next_cursor: Option<String>,
}

impl TransactionPageResponse {
    /// Maps a domain page.
    #[must_use]
    pub fn from_page(page: TransactionPage) -> Self {
        Self {
            transactions: page
                .transactions
                .iter()
                .map(TransactionDto::from_transaction)
                .collect(),
            next_cursor: page.next_cursor,
        }
    }
}

/// Per-asset P&L.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetPnlDto {
    /// The asset.
    pub asset_id: String,
    /// Cost basis of the open lots.
    pub net_invested: Option<String>,
    /// Realised P&L.
    pub realized: Option<String>,
    /// Unrealised P&L.
    pub unrealized: Option<String>,
    /// Realised plus unrealised.
    pub total_return: Option<String>,
    /// The latest known unit price, in the base currency.
    pub mark_price: Option<String>,
    /// When the mark was observed.
    pub price_as_of: Option<DateTime<Utc>>,
    /// The mark's freshness label.
    pub price_freshness: String,
    /// Whether the asset's history is complete.
    pub covered: bool,
}

impl AssetPnlDto {
    fn from_asset(asset: &AssetPnl) -> Self {
        Self {
            asset_id: asset.asset_id.clone(),
            net_invested: asset.net_invested.map(canonical_decimal),
            realized: asset.realized.map(canonical_decimal),
            unrealized: asset.unrealized.map(canonical_decimal),
            total_return: asset.total_return.map(canonical_decimal),
            mark_price: asset.mark_price.map(canonical_decimal),
            price_as_of: asset.price_as_of,
            price_freshness: asset.price_freshness.clone(),
            covered: asset.covered,
        }
    }
}

/// The C7 P&L answer with its coverage.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PnlDto {
    /// When the figures were computed.
    pub as_of: DateTime<Utc>,
    /// The user's base currency; every figure is in it.
    pub currency: String,
    /// When the FX rate was observed.
    pub fx_as_of: Option<DateTime<Utc>>,
    /// Whether any non-base amount was converted.
    pub fx_estimated: bool,
    /// Cost basis of open lots; null when unavailable.
    pub net_invested: Option<String>,
    /// Realised P&L; null when unavailable.
    pub realized: Option<String>,
    /// Unrealised P&L; null when unavailable.
    pub unrealized: Option<String>,
    /// Realised plus unrealised.
    pub total_return: Option<String>,
    /// Consumed plus remaining acquisition cost.
    pub total_acquisition_cost: Option<String>,
    /// Whether C6 marks were unavailable; affected holdings are unpriced.
    pub marks_unavailable: bool,
    /// Whether C5 holdings were unavailable.
    pub holdings_unavailable: bool,
    /// Whether a non-base amount could not be converted through C6 FX.
    pub fx_unavailable: bool,
    /// The coverage share, 0 to 1.
    pub coverage: f64,
    /// `complete`, `partial`, or `unavailable`.
    pub coverage_label: String,
    /// The uncovered assets, named and excluded.
    pub coverage_gap: Vec<String>,
    /// Per-asset figures.
    pub assets: Vec<AssetPnlDto>,
}

impl PnlDto {
    /// Maps a domain report.
    #[must_use]
    pub fn from_report(report: &PnlReport) -> Self {
        Self {
            as_of: report.as_of,
            currency: report.currency.clone(),
            fx_as_of: report.fx_as_of,
            fx_estimated: report.fx_estimated,
            net_invested: report.net_invested.map(canonical_decimal),
            realized: report.realized.map(canonical_decimal),
            unrealized: report.unrealized.map(canonical_decimal),
            total_return: report.total_return.map(canonical_decimal),
            total_acquisition_cost: report.total_acquisition_cost.map(canonical_decimal),
            marks_unavailable: report.marks_unavailable,
            holdings_unavailable: report.holdings_unavailable,
            fx_unavailable: report.fx_unavailable,
            coverage: report
                .coverage
                .to_string()
                .parse::<f64>()
                .unwrap_or_default(),
            coverage_label: report.label.as_str().to_string(),
            coverage_gap: report.coverage_gap.clone(),
            assets: report.assets.iter().map(AssetPnlDto::from_asset).collect(),
        }
    }
}

/// A correction request.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorrectionRequest {
    /// The restated field.
    pub field: String,
    /// The replacement value, as text.
    pub new_value: String,
    /// Why the correction was made.
    pub reason: String,
}

/// A stored correction.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CorrectionDto {
    /// The correction identifier.
    pub id: String,
    /// The corrected transaction.
    pub transaction_id: String,
    /// The restated field.
    pub field: String,
    /// The value that was replaced.
    pub old_value: String,
    /// The replacement value.
    pub new_value: String,
    /// When it was recorded.
    pub corrected_at: DateTime<Utc>,
    /// Whether the C10 audit retries were exhausted.
    pub audit_failed: bool,
    /// Whether derived state could not be rebuilt.
    pub rebuild_failed: bool,
}

impl CorrectionDto {
    /// Maps a correction result.
    #[must_use]
    pub fn from_result(result: &CorrectionResult) -> Self {
        Self {
            id: result.correction.id().to_string(),
            transaction_id: result.correction.transaction_id().to_string(),
            field: result.correction.field().as_str().to_string(),
            old_value: result.correction.old_value().to_string(),
            new_value: result.correction.new_value().to_string(),
            corrected_at: result.correction.corrected_at(),
            audit_failed: result.audit_failed,
            rebuild_failed: result.rebuild_failed,
        }
    }
}

/// A recorded-trade request.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeRequest {
    /// The asset.
    pub asset_id: String,
    /// `buy` or `sell`.
    pub kind: String,
    /// The quantity, as a decimal string.
    pub quantity: String,
    /// The user-supplied unit price, as a decimal string.
    #[serde(default)]
    pub price: Option<String>,
    /// The ISO currency the price is quoted in; absent means the base currency.
    #[serde(default)]
    pub currency: Option<String>,
    /// When the trade executed.
    pub executed_at: DateTime<Utc>,
}

/// A stored recorded trade.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeResponse {
    /// The transaction.
    pub transaction: TransactionDto,
    /// Whether the C10 audit retries were exhausted.
    pub audit_failed: bool,
    /// Whether derived state could not be rebuilt.
    pub rebuild_failed: bool,
}

impl TradeResponse {
    /// Maps a recorded trade.
    #[must_use]
    pub fn from_trade(trade: &RecordedTrade) -> Self {
        Self {
            transaction: TransactionDto::from_transaction(&trade.transaction),
            audit_failed: trade.audit_failed,
            rebuild_failed: trade.rebuild_failed,
        }
    }
}

/// A stored correction in an export.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedCorrectionDto {
    /// The correction identifier.
    pub id: String,
    /// The corrected transaction.
    pub transaction_id: String,
    /// The restated field.
    pub field: String,
    /// The value that was replaced.
    pub old_value: String,
    /// The replacement value.
    pub new_value: String,
    /// The user's reason.
    pub reason: String,
    /// The acting user.
    pub actor_id: String,
    /// When it was recorded.
    pub corrected_at: DateTime<Utc>,
}

impl ExportedCorrectionDto {
    fn from_correction(correction: &Correction) -> Self {
        Self {
            id: correction.id().to_string(),
            transaction_id: correction.transaction_id().to_string(),
            field: correction.field().as_str().to_string(),
            old_value: correction.old_value().to_string(),
            new_value: correction.new_value().to_string(),
            reason: correction.reason().to_string(),
            actor_id: correction.actor_id().to_string(),
            corrected_at: correction.corrected_at(),
        }
    }
}

/// The C7 privacy export.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyExportDto {
    /// The user.
    pub user_id: String,
    /// The append-only transactions.
    pub transactions: Vec<TransactionDto>,
    /// The append-only corrections.
    pub corrections: Vec<ExportedCorrectionDto>,
}

impl PrivacyExportDto {
    /// Maps a privacy export.
    #[must_use]
    pub fn from_export(export: &PrivacyExport) -> Self {
        Self {
            user_id: export.user_id.clone(),
            transactions: export
                .transactions
                .iter()
                .map(TransactionDto::from_transaction)
                .collect(),
            corrections: export
                .corrections
                .iter()
                .map(ExportedCorrectionDto::from_correction)
                .collect(),
        }
    }
}

/// An anonymise request.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnonymiseRequest {
    /// The user.
    pub user_id: Uuid,
    /// The caller's idempotency key.
    pub idempotency_key: String,
}

/// The anonymise result.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnonymiseResponse {
    /// The user.
    pub user_id: String,
    /// The non-reversible reference, when a subject existed.
    pub anonymised_ref: Option<String>,
    /// Whether an earlier request had already applied it.
    pub already_applied: bool,
    /// Whether the C10 audit retries were exhausted.
    pub audit_failed: bool,
}

impl AnonymiseResponse {
    /// Maps an anonymise result.
    #[must_use]
    pub fn from_result(result: &AnonymiseResult) -> Self {
        Self {
            user_id: result.user_id.clone(),
            anonymised_ref: result.anonymised_ref.clone(),
            already_applied: result.already_applied,
            audit_failed: result.audit_failed,
        }
    }
}

/// An ingest run's result.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestResponse {
    /// How many rows were stored.
    pub stored: usize,
    /// How many rows were skipped as duplicates.
    pub duplicates: u64,
    /// How many rows were refused, with a reason but no amount.
    pub refused: Vec<RefusedRowDto>,
}

impl IngestResponse {
    /// Maps an ingest report.
    #[must_use]
    pub fn from_report(report: &IngestReport) -> Self {
        Self {
            stored: report.stored.len(),
            duplicates: report.duplicates,
            refused: report
                .refused
                .iter()
                .map(|row| RefusedRowDto {
                    external_ref: row.external_ref.clone(),
                    reason: row.reason.clone(),
                })
                .collect(),
        }
    }
}

/// One refused row.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefusedRowDto {
    /// The source row identifier, when present.
    pub external_ref: Option<String>,
    /// Why the row was refused.
    pub reason: String,
}

/// An ingest-run request.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestRunRequest {
    /// The user whose C5 rows should be ingested.
    pub user_id: Uuid,
}

/// The reconcile result.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconcileResponse {
    /// `completed` or `skipped`.
    pub status: String,
    /// How many subjects were rebuilt.
    pub subjects: u64,
    /// How many transaction-volume batches were used.
    pub batches: u64,
    /// How many transactions were replayed.
    pub transactions: u64,
}

impl ReconcileResponse {
    /// Maps a reconcile report.
    #[must_use]
    pub fn from_report(report: &ReconcileReport) -> Self {
        Self {
            status: report.status.as_str().to_string(),
            subjects: report.subjects,
            batches: report.batches,
            transactions: report.transactions,
        }
    }
}
