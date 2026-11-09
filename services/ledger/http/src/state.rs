//! Shared router state.

use std::sync::Arc;

use ledger_application::{
    CorrectionsService, IngestService, PnlService, PrivacyService, ReadService, ReadinessProbe,
    ReconcileService, ServiceTokenRegistry, TokenValidator, TradesService,
};

use crate::metrics::Metrics;

/// Everything both routers need.
#[derive(Clone)]
pub struct AppState {
    /// Service name used in logs and metrics.
    pub service_name: &'static str,
    /// The C5 ingest use case.
    pub ingest: Arc<IngestService>,
    /// The C7 transaction read.
    pub reads: Arc<ReadService>,
    /// The C7 P&L read.
    pub pnl: Arc<PnlService>,
    /// The correction use case.
    pub corrections: Arc<CorrectionsService>,
    /// The recorded-trade use case.
    pub trades: Arc<TradesService>,
    /// The privacy export and anonymise use case.
    pub privacy: Arc<PrivacyService>,
    /// The C11 reconcile use case.
    pub reconcile: Arc<ReconcileService>,
    /// Internal service-token registry.
    pub service_tokens: Arc<dyn ServiceTokenRegistry>,
    /// Supabase JWT validator.
    pub token_validator: Arc<dyn TokenValidator>,
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
