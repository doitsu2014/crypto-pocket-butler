//! Request and response DTOs. They live only at the HTTP boundary and map to and
//! from domain and application types here; no DTO type reaches inward.

use chrono::{DateTime, Utc};
use portfolio_application::intents::IntentRequest;
use portfolio_application::privacy::PrivacyExport;
use portfolio_application::targets::TargetInput;
use portfolio_application::trades::TradeRequest;
use portfolio_application::valuation::{SnapshotDetail, SnapshotView, ValuationRead};
use portfolio_application::RunReport;
use portfolio_domain::{
    Guardrail, IntentProjection, OrderIntent, OrderSide, Portfolio, PortfolioAccount, PriceSource,
    RecordedTrade, TargetAllocation, TradeOverlay, Valuation, ValuationLine, ValuationSnapshot,
};
use rust_decimal::Decimal;
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::str::FromStr;

fn decimal_number(value: Decimal) -> Result<serde_json::Number, String> {
    serde_json::Number::from_str(&value.to_string())
        .map_err(|_| format!("{value} is not a JSON number"))
}

fn serialize_decimal<S: Serializer>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error> {
    decimal_number(*value)
        .map_err(serde::ser::Error::custom)?
        .serialize(serializer)
}

fn serialize_optional_decimal<S: Serializer>(
    value: &Option<Decimal>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match value {
        Some(value) => serialize_decimal(value, serializer),
        None => serializer.serialize_none(),
    }
}

fn decimal_value(value: serde_json::Value) -> Result<Decimal, String> {
    match value {
        serde_json::Value::Number(number) => Decimal::from_str(&number.to_string())
            .map_err(|_| "must be a decimal number".to_string()),
        serde_json::Value::String(raw) => {
            Decimal::from_str(raw.trim()).map_err(|_| "must be a decimal number".to_string())
        }
        _ => Err("must be a decimal number".to_string()),
    }
}

fn deserialize_decimal<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Decimal, D::Error> {
    decimal_value(serde_json::Value::deserialize(deserializer)?).map_err(DeError::custom)
}

fn deserialize_optional_decimal<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Decimal>, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    if value.is_null() {
        Ok(None)
    } else {
        decimal_value(value).map(Some).map_err(DeError::custom)
    }
}

/// `GET /v1/portfolios` and `POST /v1/portfolios` item.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioDto {
    /// The portfolio identifier.
    pub id: String,
    /// The display name.
    pub name: String,
    /// The ISO 4217 base currency.
    pub base_currency: String,
}

impl From<&Portfolio> for PortfolioDto {
    fn from(portfolio: &Portfolio) -> Self {
        Self {
            id: portfolio.id().to_string(),
            name: portfolio.name().to_string(),
            base_currency: portfolio.base_currency().to_string(),
        }
    }
}

/// `POST /v1/portfolios` body.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePortfolioRequest {
    /// An optional name; the default is used when absent.
    #[serde(default)]
    pub name: Option<String>,
    /// An optional base currency; the Identity preference or USD is used when
    /// absent.
    #[serde(default)]
    pub base_currency: Option<String>,
}

/// One valuation line.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValuationLineDto {
    /// The asset.
    pub asset_id: String,
    /// The aggregated quantity.
    pub quantity: String,
    /// The value in the base currency.
    pub value: Option<String>,
    /// The allocation percentage.
    #[serde(serialize_with = "serialize_optional_decimal")]
    pub allocation: Option<Decimal>,
    /// The target percentage.
    #[serde(serialize_with = "serialize_optional_decimal")]
    pub target_percent: Option<Decimal>,
    /// The drift in percentage points.
    #[serde(serialize_with = "serialize_optional_decimal")]
    pub drift_points: Option<Decimal>,
    /// The unit price.
    pub price: Option<String>,
    /// The quote source.
    pub price_source: Option<String>,
    /// `live`, `delayed`, `stale`, or `unpriced`.
    pub price_freshness: String,
    /// When the quote was observed.
    pub price_observed_at: Option<DateTime<Utc>>,
    /// The effective FX rate.
    pub fx_rate: Option<String>,
    /// The FX source.
    pub fx_source: Option<String>,
    /// Both FX leg sources.
    pub fx_derived_from: Vec<String>,
    /// When the FX rate was observed.
    pub fx_observed_at: Option<DateTime<Utc>>,
    /// `priced`, `unpriced`, or `holdings_unavailable`.
    pub data_status: String,
}

impl From<&ValuationLine> for ValuationLineDto {
    fn from(line: &ValuationLine) -> Self {
        Self {
            asset_id: line.asset_id().to_string(),
            quantity: line.quantity().to_string(),
            value: line.value().map(|value| value.to_string()),
            allocation: line.allocation(),
            target_percent: line.target_percent(),
            drift_points: line.drift_points(),
            price: line.price().map(|price| price.to_string()),
            price_source: line.price_source().map(ToString::to_string),
            price_freshness: line.price_freshness().as_str().to_string(),
            price_observed_at: line.price_observed_at(),
            fx_rate: line.fx_rate().map(|rate| rate.to_string()),
            fx_source: line.fx_source().map(ToString::to_string),
            fx_derived_from: line.fx_derived_from().to_vec(),
            fx_observed_at: line.fx_observed_at(),
            data_status: line.data_status().as_str().to_string(),
        }
    }
}

/// `GET /v1/portfolios/{id}` response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioValuationDto {
    /// The portfolio.
    pub portfolio_id: String,
    /// When the valuation is as-of.
    pub as_of: DateTime<Utc>,
    /// `complete`, `partial`, or `unavailable`.
    pub status: String,
    /// Whether C5 holdings were unavailable.
    pub holdings_unavailable: bool,
    /// The priced total; null when unavailable.
    pub total_value: Option<String>,
    /// The base currency.
    pub currency: String,
    /// The fraction of asset lines that are priced.
    #[serde(serialize_with = "serialize_decimal")]
    pub priced_coverage: Decimal,
    /// How many assets were excluded for having no usable price.
    pub unpriced_count: u32,
    /// How many holdings are kept as excluded labelled lines.
    pub excluded_count: u32,
    /// The per-asset lines.
    pub lines: Vec<ValuationLineDto>,
    /// Whether drift is available.
    pub drift_available: bool,
    /// The set-targets prompt, when drift is unavailable.
    pub prompt: Option<String>,
    /// Whether a stored snapshot was served.
    pub served_from_snapshot: bool,
}

impl From<&ValuationRead> for PortfolioValuationDto {
    fn from(read: &ValuationRead) -> Self {
        let valuation: &Valuation = &read.valuation;
        Self {
            portfolio_id: read.portfolio.id().to_string(),
            as_of: read.as_of,
            status: valuation.status().as_str().to_string(),
            holdings_unavailable: valuation.holdings_unavailable(),
            total_value: valuation.total_value().map(|value| value.to_string()),
            currency: valuation.currency().to_string(),
            priced_coverage: valuation.priced_coverage(),
            unpriced_count: valuation.unpriced_count(),
            excluded_count: valuation.excluded_count(),
            lines: valuation
                .lines()
                .iter()
                .map(ValuationLineDto::from)
                .collect(),
            drift_available: read.targets_configured,
            prompt: read.prompt.clone(),
            served_from_snapshot: read.served_from_snapshot,
        }
    }
}

/// One requested target.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetInputDto {
    /// The asset.
    pub asset_id: String,
    /// The target percentage.
    #[serde(deserialize_with = "deserialize_decimal")]
    pub target_percent: Decimal,
    /// The optional guardrail threshold in percentage points.
    #[serde(default, deserialize_with = "deserialize_optional_decimal")]
    pub guardrail_points: Option<Decimal>,
}

/// `PUT /v1/portfolios/{id}/targets` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetSetRequest {
    /// The target set.
    pub targets: Vec<TargetInputDto>,
}

impl TargetSetRequest {
    /// Converts the request to application inputs. The serde deserializer already
    /// refused a value that is not a decimal.
    #[must_use]
    pub fn into_inputs(self) -> Vec<TargetInput> {
        self.targets
            .into_iter()
            .map(|target| TargetInput {
                asset_id: target.asset_id,
                target_percent: target.target_percent,
                guardrail_points: target.guardrail_points,
            })
            .collect()
    }
}

/// `PUT /v1/portfolios/{id}/targets` response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetPreviewDto {
    /// The sum of the target percentages.
    #[serde(serialize_with = "serialize_decimal")]
    pub sum: Decimal,
    /// How far the sum is from 100.
    #[serde(serialize_with = "serialize_decimal")]
    pub difference: Decimal,
    /// Whether the set was saved.
    pub saved: bool,
}

/// `POST /v1/portfolios/{id}/order-intents` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderIntentRequestDto {
    /// The asset.
    pub asset_id: String,
    /// `buy` or `sell`.
    pub side: String,
    /// The quantity as a decimal string.
    pub quantity: String,
}

impl OrderIntentRequestDto {
    /// Converts the request to an application intent request.
    ///
    /// # Errors
    /// Returns the offending field when the side or the quantity is invalid.
    pub fn into_request(self) -> Result<IntentRequest, (&'static str, String)> {
        let side = OrderSide::parse(&self.side).map_err(|error| ("side", error.to_string()))?;
        let quantity = Decimal::from_str_exact(self.quantity.trim())
            .map_err(|_| ("quantity", "must be a decimal number".to_string()))?;
        Ok(IntentRequest {
            asset_id: self.asset_id,
            side,
            quantity,
        })
    }
}

/// The preview of an order intent.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderIntentPreviewDto {
    /// The projected allocation.
    #[serde(serialize_with = "serialize_decimal")]
    pub projected_allocation: Decimal,
    /// The projected drift in percentage points.
    #[serde(serialize_with = "serialize_optional_decimal")]
    pub projected_drift_points: Option<Decimal>,
    /// The shortfall for a sell beyond holdings.
    pub shortfall: Option<String>,
}

impl From<&IntentProjection> for OrderIntentPreviewDto {
    fn from(projection: &IntentProjection) -> Self {
        Self {
            projected_allocation: projection.projected_allocation,
            projected_drift_points: projection.projected_drift_points,
            shortfall: projection.shortfall.map(|value| value.to_string()),
        }
    }
}

/// A recorded order intent.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderIntentDto {
    /// The intent identifier.
    pub id: String,
    /// The asset.
    pub asset_id: String,
    /// `buy` or `sell`.
    pub side: String,
    /// The quantity.
    pub quantity: String,
    /// The projected allocation.
    #[serde(serialize_with = "serialize_decimal")]
    pub projected_allocation: Decimal,
    /// The projected drift.
    #[serde(serialize_with = "serialize_optional_decimal")]
    pub projected_drift_points: Option<Decimal>,
    /// The shortfall.
    pub shortfall: Option<String>,
    /// `pending` or `cancelled`.
    pub status: String,
    /// When the intent was recorded.
    pub created_at: DateTime<Utc>,
    /// When the intent was cancelled.
    pub cancelled_at: Option<DateTime<Utc>>,
}

impl From<&OrderIntent> for OrderIntentDto {
    fn from(intent: &OrderIntent) -> Self {
        Self {
            id: intent.id().to_string(),
            asset_id: intent.asset_id().to_string(),
            side: intent.side().as_str().to_string(),
            quantity: intent.quantity().to_string(),
            projected_allocation: intent.projected_allocation(),
            projected_drift_points: intent.projected_drift_points(),
            shortfall: intent.shortfall().map(|value| value.to_string()),
            status: intent.status().as_str().to_string(),
            created_at: intent.created_at(),
            cancelled_at: intent.cancelled_at(),
        }
    }
}

/// `POST /v1/portfolios/{id}/trades` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeRequestDto {
    /// The asset.
    pub asset_id: String,
    /// `buy` or `sell`.
    pub side: String,
    /// The quantity as a decimal string.
    pub quantity: String,
    /// The unit price, supplied by the user when the venue has none.
    #[serde(default)]
    pub price: Option<String>,
    /// `venue` or `user`.
    pub price_source: String,
    /// When the trade executed.
    pub executed_at: DateTime<Utc>,
    /// The client idempotency key.
    pub idempotency_key: String,
}

impl TradeRequestDto {
    /// Converts the request to an application trade request.
    ///
    /// # Errors
    /// Returns the offending field when the side, price, or quantity is
    /// invalid.
    pub fn into_request(self) -> Result<TradeRequest, (&'static str, String)> {
        let side = OrderSide::parse(&self.side).map_err(|error| ("side", error.to_string()))?;
        let price_source = PriceSource::parse(&self.price_source)
            .map_err(|error| ("priceSource", error.to_string()))?;
        let quantity = Decimal::from_str_exact(self.quantity.trim())
            .map_err(|_| ("quantity", "must be a decimal number".to_string()))?;
        let price = self
            .price
            .map(|raw| {
                Decimal::from_str_exact(raw.trim())
                    .map_err(|_| ("price", "must be a decimal number".to_string()))
            })
            .transpose()?;
        Ok(TradeRequest {
            asset_id: self.asset_id,
            side,
            quantity,
            price,
            price_source,
            executed_at: self.executed_at,
            idempotency_key: self.idempotency_key,
        })
    }
}

/// A recorded trade.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordedTradeDto {
    /// The trade identifier.
    pub id: String,
    /// The Ledger transaction identifier.
    pub ledger_transaction_id: Option<String>,
    /// The asset.
    pub asset_id: String,
    /// `buy` or `sell`.
    pub side: String,
    /// The quantity.
    pub quantity: String,
    /// The unit price.
    pub price: String,
    /// `venue` or `user`.
    pub price_source: String,
    /// When the trade executed.
    pub executed_at: DateTime<Utc>,
}

impl From<&RecordedTrade> for RecordedTradeDto {
    fn from(trade: &RecordedTrade) -> Self {
        Self {
            id: trade.id().to_string(),
            ledger_transaction_id: trade.ledger_transaction_id().map(ToString::to_string),
            asset_id: trade.asset_id().to_string(),
            side: trade.side().as_str().to_string(),
            quantity: trade.quantity().to_string(),
            price: trade.price().to_string(),
            price_source: trade.price_source().as_str().to_string(),
            executed_at: trade.executed_at(),
        }
    }
}

/// One snapshot in a history response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotDto {
    /// The snapshot identifier.
    pub id: String,
    /// When the valuation was taken.
    pub as_of: DateTime<Utc>,
    /// The priced total.
    pub total_value: String,
    /// The base currency.
    pub currency: String,
    /// The frozen-inputs digest.
    pub inputs_digest: String,
}

impl From<&SnapshotView> for SnapshotDto {
    fn from(view: &SnapshotView) -> Self {
        Self::from_snapshot(&view.snapshot)
    }
}

impl SnapshotDto {
    /// Maps a stored snapshot summary. Reproduction is not performed here.
    #[must_use]
    pub fn from_snapshot(snapshot: &ValuationSnapshot) -> Self {
        Self {
            id: snapshot.id().to_string(),
            as_of: snapshot.as_of(),
            total_value: snapshot.total_value().to_string(),
            currency: snapshot.currency().to_string(),
            inputs_digest: snapshot.inputs_digest().to_string(),
        }
    }
}

/// One snapshot reproduced on demand.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotDetailDto {
    /// The snapshot identifier.
    pub id: String,
    /// When the valuation was taken.
    pub as_of: DateTime<Utc>,
    /// The priced total.
    pub total_value: String,
    /// The base currency.
    pub currency: String,
    /// The frozen-inputs digest.
    pub inputs_digest: String,
    /// Whether the digest and reproduction verified.
    pub verified: bool,
}

impl From<&SnapshotDetail> for SnapshotDetailDto {
    fn from(detail: &SnapshotDetail) -> Self {
        Self {
            id: detail.snapshot.id().to_string(),
            as_of: detail.snapshot.as_of(),
            total_value: detail.snapshot.total_value().to_string(),
            currency: detail.snapshot.currency().to_string(),
            inputs_digest: detail.snapshot.inputs_digest().to_string(),
            verified: detail.verified,
        }
    }
}

/// One account membership in the privacy export.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MembershipDto {
    /// The membership identifier.
    pub id: String,
    /// The portfolio.
    pub portfolio_id: String,
    /// The account.
    pub account_id: String,
    /// When the account joined.
    pub added_at: DateTime<Utc>,
}

impl From<&PortfolioAccount> for MembershipDto {
    fn from(row: &PortfolioAccount) -> Self {
        Self {
            id: row.id().to_string(),
            portfolio_id: row.portfolio_id().to_string(),
            account_id: row.account_id().to_string(),
            added_at: row.added_at(),
        }
    }
}

/// One target in the privacy export.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetDto {
    /// The portfolio.
    pub portfolio_id: String,
    /// The asset.
    pub asset_id: String,
    /// The target percentage.
    pub target_percent: String,
}

impl From<&TargetAllocation> for TargetDto {
    fn from(target: &TargetAllocation) -> Self {
        Self {
            portfolio_id: target.portfolio_id().to_string(),
            asset_id: target.asset_id().to_string(),
            target_percent: target.target_percent().to_string(),
        }
    }
}

/// One guardrail in the privacy export.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailDto {
    /// The portfolio.
    pub portfolio_id: String,
    /// The asset.
    pub asset_id: String,
    /// The threshold in percentage points.
    pub threshold_points: String,
    /// Whether the drift currently sits outside.
    pub is_outside: bool,
}

impl From<&Guardrail> for GuardrailDto {
    fn from(guardrail: &Guardrail) -> Self {
        Self {
            portfolio_id: guardrail.portfolio_id().to_string(),
            asset_id: guardrail.asset_id().to_string(),
            threshold_points: guardrail.threshold_points().to_string(),
            is_outside: guardrail.is_outside(),
        }
    }
}

/// One overlay in the privacy export.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayDto {
    /// The portfolio.
    pub portfolio_id: String,
    /// The asset.
    pub asset_id: String,
    /// The quantity delta.
    pub quantity_delta: String,
    /// When it was applied.
    pub applied_at: DateTime<Utc>,
}

impl From<&TradeOverlay> for OverlayDto {
    fn from(overlay: &TradeOverlay) -> Self {
        Self {
            portfolio_id: overlay.portfolio_id().to_string(),
            asset_id: overlay.asset_id().to_string(),
            quantity_delta: overlay.quantity_delta().to_string(),
            applied_at: overlay.applied_at(),
        }
    }
}

/// `GET /v1/internal/privacy/export` response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyExportDto {
    /// The user.
    pub user_id: String,
    /// The portfolios.
    pub portfolios: Vec<PortfolioDto>,
    /// The membership rows.
    pub memberships: Vec<MembershipDto>,
    /// The targets.
    pub targets: Vec<TargetDto>,
    /// The guardrails.
    pub guardrails: Vec<GuardrailDto>,
    /// The snapshot summaries.
    pub snapshots: Vec<SnapshotDto>,
    /// The live overlays.
    pub overlays: Vec<OverlayDto>,
    /// The order intents.
    pub intents: Vec<OrderIntentDto>,
    /// The recorded trades.
    pub trades: Vec<RecordedTradeDto>,
}

impl From<PrivacyExport> for PrivacyExportDto {
    fn from(export: PrivacyExport) -> Self {
        Self {
            user_id: export.user_id.to_string(),
            portfolios: export.portfolios.iter().map(PortfolioDto::from).collect(),
            memberships: export.memberships.iter().map(MembershipDto::from).collect(),
            targets: export.targets.iter().map(TargetDto::from).collect(),
            guardrails: export.guardrails.iter().map(GuardrailDto::from).collect(),
            snapshots: export
                .snapshots
                .iter()
                .map(SnapshotDto::from_snapshot)
                .collect(),
            overlays: export.overlays.iter().map(OverlayDto::from).collect(),
            intents: export.intents.iter().map(OrderIntentDto::from).collect(),
            trades: export.trades.iter().map(RecordedTradeDto::from).collect(),
        }
    }
}

/// `POST /v1/internal/privacy/anonymise` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnonymiseRequest {
    /// The user.
    pub user_id: String,
    /// The caller's idempotency key.
    pub idempotency_key: String,
}

/// `POST /v1/internal/privacy/anonymise` response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnonymiseResponse {
    /// The user.
    pub user_id: String,
    /// The categories kept under a legal basis.
    pub retained_categories: Vec<String>,
    /// Whether an earlier request already applied it.
    pub already_applied: bool,
}

/// The C11 run request.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRequest {
    /// The caller's instance identifier; a generated one is used when absent.
    #[serde(default)]
    pub owner: Option<String>,
}

/// The C11 run result.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunResultDto {
    /// The job name.
    pub job_name: String,
    /// `succeeded`, `failed`, or `skipped`.
    pub status: String,
    /// When the run started.
    pub started_at: Option<DateTime<Utc>>,
    /// A short non-secret detail.
    pub detail: Option<String>,
    /// Portfolios processed.
    pub portfolios: u32,
    /// Snapshots written.
    pub snapshots: u32,
    /// Portfolios skipped.
    pub skipped: u32,
    /// Portfolios that failed.
    pub failed: u32,
    /// Guardrail alerts that will be retried.
    pub guardrail_alerts_failed: u32,
    /// Unmatched overlays expired after seven days.
    pub overlays_expired: u64,
    /// How long the run took.
    pub duration_seconds: u64,
}

impl RunResultDto {
    /// Maps a run report.
    #[must_use]
    pub fn from_report(report: &RunReport) -> Self {
        Self {
            job_name: portfolio_application::run::RUN_JOB.to_string(),
            status: if report.failed > 0 {
                "failed".to_string()
            } else {
                "succeeded".to_string()
            },
            started_at: report.started_at,
            detail: None,
            portfolios: report.portfolios,
            snapshots: report.snapshots,
            skipped: report.skipped,
            failed: report.failed,
            guardrail_alerts_failed: report.guardrail_alerts_failed,
            overlays_expired: report.overlays_expired,
            duration_seconds: report.duration_seconds,
        }
    }
}
