//! Shared router state.

use std::sync::Arc;

use portfolio_application::intents::IntentService;
use portfolio_application::portfolios::PortfolioService;
use portfolio_application::ports::{ReadinessProbe, ServiceTokenRegistry, TokenValidator};
use portfolio_application::privacy::PrivacyService;
use portfolio_application::run::ValuationRunService;
use portfolio_application::targets::TargetService;
use portfolio_application::trades::TradeService;
use portfolio_application::valuation::ValuationService;

use crate::metrics::Metrics;

/**
 * The API router state, including the guardrail evaluator used after a recorded
 * trade (BR6.1).
 */
#[derive(Clone)]
pub struct AppState {
    /// Service name used in logs and metrics.
    pub service_name: &'static str,
    /// Portfolio listing, creation, and membership.
    pub portfolios: Arc<PortfolioService>,
    /// Valuation reads and snapshots.
    pub valuation: Arc<ValuationService>,
    /// Target preview and save.
    pub targets: Arc<TargetService>,
    /// Order intents.
    pub intents: Arc<IntentService>,
    /// Recorded trades.
    pub trades: Arc<TradeService>,
    /// Guardrail evaluation after a recorded trade.
    pub guardrails: Arc<portfolio_application::guardrails::GuardrailService>,
    /// Privacy export and anonymise.
    pub privacy: Arc<PrivacyService>,
    /// Supabase JWT validator.
    pub token_validator: Arc<dyn TokenValidator>,
    /// Internal service-token registry.
    pub service_tokens: Arc<dyn ServiceTokenRegistry>,
    /// Readiness probe.
    pub readiness: Arc<dyn ReadinessProbe>,
    /// Request and workflow metrics.
    pub metrics: Arc<Metrics>,
}

/// Everything the worker's listener needs.
#[derive(Clone)]
pub struct WorkerState {
    /// Service name used in logs and metrics.
    pub service_name: &'static str,
    /// The scheduled valuation run.
    pub runs: Arc<ValuationRunService>,
    /// Internal service-token registry.
    pub service_tokens: Arc<dyn ServiceTokenRegistry>,
    /// Readiness probe.
    pub readiness: Arc<dyn ReadinessProbe>,
    /// Request and workflow metrics.
    pub metrics: Arc<Metrics>,
}
