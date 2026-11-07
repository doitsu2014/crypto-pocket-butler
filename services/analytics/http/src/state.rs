//! Shared router state.

use std::sync::Arc;

use analytics_application::{
    AggregateService, Clock, EraseService, FlagService, IngestEvent, ReadinessProbe,
    RetentionService, ServiceTokenRegistry, TokenValidator,
};

use crate::metrics::Metrics;

/// Everything both routers need.
#[derive(Clone)]
pub struct AppState {
    /// Service name used in logs and metrics.
    pub service_name: &'static str,
    /// The ingest use case.
    pub ingest: Arc<IngestEvent>,
    /// The flag use case.
    pub flags: Arc<FlagService>,
    /// The aggregation use case.
    pub aggregates: Arc<AggregateService>,
    /// The retention use case.
    pub retention: Arc<RetentionService>,
    /// The erasure use case.
    pub erase: Arc<EraseService>,
    /// Internal service-token registry.
    pub service_tokens: Arc<dyn ServiceTokenRegistry>,
    /// Supabase JWT validator.
    pub token_validator: Arc<dyn TokenValidator>,
    /// Clock for deriving a flag's displayed status.
    pub clock: Arc<dyn Clock>,
    /// Readiness probe.
    pub readiness: Arc<dyn ReadinessProbe>,
    /// Request and work metrics.
    pub metrics: Arc<Metrics>,
}

const _: fn() = || {
    fn assert_router_state<T: Clone + Send + Sync + 'static>() {}
    fn assert_from_ref<T: axum::extract::FromRef<T>>() {}
    assert_router_state::<AppState>();
    assert_from_ref::<AppState>();
};
