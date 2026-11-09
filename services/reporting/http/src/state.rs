//! Shared router state.

use std::sync::Arc;

use reporting_application::briefs::BriefService;
use reporting_application::generation::GenerationService;
use reporting_application::ports::{ReadinessProbe, ServiceTokenRegistry, TokenValidator};
use reporting_application::privacy::PrivacyService;

use crate::metrics::Metrics;

/// The API router state.
#[derive(Clone)]
pub struct AppState {
    /// Service name used in logs and metrics.
    pub service_name: &'static str,
    /// Owner-scoped brief reads (BR3.1).
    pub briefs: Arc<BriefService>,
    /// The C11 generation run.
    pub generation: Arc<GenerationService>,
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
    /// The C11 generation run.
    pub generation: Arc<GenerationService>,
    /// Internal service-token registry.
    pub service_tokens: Arc<dyn ServiceTokenRegistry>,
    /// Readiness probe.
    pub readiness: Arc<dyn ReadinessProbe>,
    /// Request and workflow metrics.
    pub metrics: Arc<Metrics>,
}
