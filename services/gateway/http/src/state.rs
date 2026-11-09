//! Shared router state.

use std::sync::Arc;

use gateway_application::authorize::AuthorizeAndRoute;
use gateway_application::ports::ReadinessProbe;

use crate::metrics::Metrics;
use crate::rate_limit::RateLimiter;

/// Everything the router needs.
#[derive(Clone)]
pub struct AppState {
    /// Service name used in logs and metrics.
    pub service_name: &'static str,
    /// Authenticate, authorize, and route use case.
    pub use_case: Arc<AuthorizeAndRoute>,
    /// Readiness probe.
    pub readiness: Arc<dyn ReadinessProbe>,
    /// Request metrics.
    pub metrics: Arc<Metrics>,
    /// Rate limiter.
    pub rate_limiter: Arc<RateLimiter>,
}
