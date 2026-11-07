//! Shared router state.

use std::sync::Arc;

use market_data_application::ports::{ReadinessProbe, ServiceTokenRegistry};
use market_data_application::{FxService, QuoteService, RefreshService};

use crate::metrics::Metrics;

/// Everything the router needs. All fields are trait objects or `Arc`s; the
/// state is cheap to clone and stateless across requests.
#[derive(Clone)]
pub struct AppState {
    /// Service name used in logs and metrics.
    pub service_name: &'static str,
    /// C6 prices.
    pub quotes: Arc<QuoteService>,
    /// C6 FX rates.
    pub fx: Arc<FxService>,
    /// C11 refresh runs.
    pub refresh: Arc<RefreshService>,
    /// Internal service-token registry.
    pub service_tokens: Arc<dyn ServiceTokenRegistry>,
    /// Readiness probe.
    pub readiness: Arc<dyn ReadinessProbe>,
    /// Request and workflow metrics.
    pub metrics: Arc<Metrics>,
}
