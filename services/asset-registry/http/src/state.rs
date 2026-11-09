//! Shared router state.

use std::sync::Arc;

use asset_registry_application::admin::RegistryAdmin;
use asset_registry_application::ports::{
    AssetRepository, AuditSink, Clock, ReadinessProbe, TokenValidator,
};
use asset_registry_application::resolve_asset::ResolveAsset;
use asset_registry_application::resolve_price::ResolvePrice;

use crate::metrics::Metrics;

/// Everything the router needs. All fields are trait objects or `Arc`s; the state
/// is cheap to clone and stateless across requests.
#[derive(Clone)]
pub struct AppState {
    /// Service name used in logs and metrics.
    pub service_name: &'static str,
    /// Asset resolution use case.
    pub resolve_asset: Arc<ResolveAsset>,
    /// Price resolution use case.
    pub resolve_price: Arc<ResolvePrice>,
    /// Registry administration use case.
    pub admin: Arc<RegistryAdmin>,
    /// Asset lookup for the price route.
    pub assets: Arc<dyn AssetRepository>,
    /// JWT validator (the service validates the forwarded token again).
    pub token_validator: Arc<dyn TokenValidator>,
    /// Audit sink for admin denials and registry changes.
    pub audit: Arc<dyn AuditSink>,
    /// Clock for audit timestamps.
    pub clock: Arc<dyn Clock>,
    /// Readiness probe.
    pub readiness: Arc<dyn ReadinessProbe>,
    /// Request metrics.
    pub metrics: Arc<Metrics>,
}
