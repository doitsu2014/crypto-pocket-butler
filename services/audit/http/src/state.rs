//! Shared router state.

use std::sync::Arc;

use audit_application::export::ExportAudit;
use audit_application::ports::{Clock, ReadinessProbe, ServiceTokenRegistry, TokenValidator};
use audit_application::privacy::PrivacyDeletions;
use audit_application::query::QueryEvents;
use audit_application::retention::RetentionService;
use audit_application::support::SupportSessions;
use audit_application::AppendEvent;
use audit_application::Trail;

use crate::metrics::Metrics;
use crate::rate_limit::{Limits, RateLimiter};

/// Everything the router needs. All fields are trait objects or `Arc`s; the state
/// is cheap to clone and stateless across requests.
#[derive(Clone)]
pub struct AppState {
    /// Service name used in logs and metrics.
    pub service_name: &'static str,
    /// The append use case.
    pub append: Arc<AppendEvent>,
    /// The admin query use case.
    pub query: Arc<QueryEvents>,
    /// The export use case.
    pub export: Arc<ExportAudit>,
    /// The support-session use case.
    pub support: Arc<SupportSessions>,
    /// The privacy-deletion use case.
    pub privacy: Arc<PrivacyDeletions>,
    /// The retention use case (the report path).
    pub retention: Arc<RetentionService>,
    /// The service's own write path, used to audit denials in middleware.
    pub trail: Arc<Trail>,
    /// Internal service-token registry.
    pub service_tokens: Arc<dyn ServiceTokenRegistry>,
    /// Supabase JWT validator.
    pub token_validator: Arc<dyn TokenValidator>,
    /// Clock for middleware-side audit timestamps.
    pub clock: Arc<dyn Clock>,
    /// Readiness probe.
    pub readiness: Arc<dyn ReadinessProbe>,
    /// Request metrics.
    pub metrics: Arc<Metrics>,
    /// Per-replica rate limiter.
    pub limiter: Arc<RateLimiter>,
    /// Per-replica limits.
    pub limits: Limits,
}
