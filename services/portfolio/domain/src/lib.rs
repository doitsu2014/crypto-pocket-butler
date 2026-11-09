//! Portfolio domain: portfolios and membership, targets and guardrails,
//! backend valuation with provenance, immutable snapshots, advisory order
//! intents, recorded trades with a ledger overlay, and the guardrail crossing
//! state machine.
//!
//! This crate holds entities, value types, and business rules only. It imports
//! no framework and no sibling crate.

pub mod audit;
pub mod error;
pub mod guardrail;
pub mod ids;
pub mod intent;
pub mod membership;
pub mod overlay;
pub mod portfolio;
pub mod service;
pub mod snapshot;
pub mod target;
pub mod trade;
pub mod valuation;

pub use audit::{AuditOutcome, AuditRecord};
pub use error::DomainError;
pub use guardrail::{crossing_key, Guardrail, GuardrailDraft, GuardrailTransition};
pub use ids::{
    AccountId, GuardrailId, OrderIntentId, PortfolioAccountId, PortfolioId, RecordedTradeId,
    TargetAllocationId, TradeOverlayId, UserId, ValuationRunId, ValuationSnapshotId,
};
pub use intent::{project_intent, IntentProjection, IntentStatus, OrderIntent, OrderSide};
pub use membership::PortfolioAccount;
pub use overlay::TradeOverlay;
pub use portfolio::{
    normalise_currency, validate_name, Portfolio, DEFAULT_BASE_CURRENCY, DEFAULT_PORTFOLIO_NAME,
    MAX_NAME_CHARS,
};
pub use service::{service_allowed, Operation};
pub use snapshot::{hour_bucket, ValuationSnapshot};
pub use target::{
    preview_targets, require_target_for_guardrail, validate_target, TargetAllocation, TargetPreview,
};
pub use trade::{canonical_trade_key, PriceSource, RecordedTrade};
pub use valuation::{
    apply_overlays, compute_valuation, parse_decimal, DataStatus, Freshness, FxRate, Holding,
    Quote, Valuation, ValuationLine, ValuationStatus,
};
