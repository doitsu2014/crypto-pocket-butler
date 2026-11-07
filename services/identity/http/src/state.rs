//! Shared router state.

use std::sync::Arc;

use identity_application::consent::ConsentService;
use identity_application::deletion::DeletionService;
use identity_application::export::ExportService;
use identity_application::ports::{
    ReadinessProbe, ServiceTokenRegistry, TokenValidator, WakeTransport,
};
use identity_application::preferences::PreferencesService;
use identity_application::principal::PrincipalService;
use identity_application::sessions::SessionService;
use identity_application::signup_gate::SignupGateService;
use identity_application::support::SupportService;

use crate::metrics::Metrics;

/// Everything the router needs. All fields are trait objects or `Arc`s; the state
/// is cheap to clone and stateless across requests.
#[derive(Clone)]
pub struct AppState {
    /// Service name used in logs and metrics.
    pub service_name: &'static str,
    /// Principal resolution and provisioning.
    pub principal: Arc<PrincipalService>,
    /// Consent append and read.
    pub consent: Arc<ConsentService>,
    /// Preference updates.
    pub preferences: Arc<PreferencesService>,
    /// Session listing and revocation.
    pub sessions: Arc<SessionService>,
    /// Support-grant use cases.
    pub support: Arc<SupportService>,
    /// The pre-auth jurisdiction gate.
    pub signup_gate: Arc<SignupGateService>,
    /// Export request and status.
    pub exports: Arc<ExportService>,
    /// Deletion request and status.
    pub deletions: Arc<DeletionService>,
    /// Supabase JWT validator.
    pub token_validator: Arc<dyn TokenValidator>,
    /// Internal service-token registry.
    pub service_tokens: Arc<dyn ServiceTokenRegistry>,
    /// Worker wake transport for the C11 run endpoint.
    pub wake: Arc<dyn WakeTransport>,
    /// Readiness probe.
    pub readiness: Arc<dyn ReadinessProbe>,
    /// Request metrics.
    pub metrics: Arc<Metrics>,
}
