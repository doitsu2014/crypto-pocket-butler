//! Shared router state.

use std::sync::Arc;

use notification_application::ports::{
    Clock, ReadinessProbe, ServiceTokenRegistry, TokenValidator,
};
use notification_application::{
    DeliveryService, EraseService, FeedService, PreferenceService, RetentionService, SubmitAlert,
    Trail,
};

use crate::metrics::Metrics;

/// Everything both routers need.
#[derive(Clone)]
pub struct AppState {
    /// Service name used in logs and metrics.
    pub service_name: &'static str,
    /// The submit use case.
    pub submit: Arc<SubmitAlert>,
    /// The preference use case.
    pub preferences: Arc<PreferenceService>,
    /// The in-app feed use case.
    pub feed: Arc<FeedService>,
    /// The delivery use case.
    pub delivery: Arc<DeliveryService>,
    /// The retention use case.
    pub retention: Arc<RetentionService>,
    /// The erasure use case.
    pub erase: Arc<EraseService>,
    /// The C10 trail, whose abandoned-write count feeds the metric.
    pub trail: Arc<Trail>,
    /// Internal service-token registry.
    pub service_tokens: Arc<dyn ServiceTokenRegistry>,
    /// Supabase JWT validator.
    pub token_validator: Arc<dyn TokenValidator>,
    /// Readiness probe.
    pub readiness: Arc<dyn ReadinessProbe>,
    /// Clock for run timestamps.
    pub clock: Arc<dyn Clock>,
    /// Request and work metrics.
    pub metrics: Arc<Metrics>,
}

const _: fn() = || {
    fn assert_router_state<T: Clone + Send + Sync + 'static>() {}
    fn assert_from_ref<T: axum::extract::FromRef<T>>() {}
    assert_router_state::<AppState>();
    assert_from_ref::<AppState>();
};
