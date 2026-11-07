//! Shared router state.

use std::sync::Arc;

use account_application::accounts::AccountService;
use account_application::holdings::HoldingService;
use account_application::imports::ImportService;
use account_application::ports::{ReadinessProbe, ServiceTokenRegistry, TokenValidator};
use account_application::privacy::PrivacyService;
use account_application::sync::SyncService;

use crate::metrics::Metrics;

/// Everything the router needs. All fields are trait objects or `Arc`s; the state
/// is cheap to clone and stateless across requests.
#[derive(Clone)]
pub struct AppState {
    /// Service name used in logs and metrics.
    pub service_name: &'static str,
    /// Account onboarding and manual re-sync.
    pub accounts: Arc<AccountService>,
    /// Scheduled and manual sync processing.
    pub sync: Arc<SyncService>,
    /// CSV upload and processing status.
    pub imports: Arc<ImportService>,
    /// C5 holdings and transactions.
    pub holdings: Arc<HoldingService>,
    /// Privacy export and anonymise.
    pub privacy: Arc<PrivacyService>,
    /// Supabase JWT validator.
    pub token_validator: Arc<dyn TokenValidator>,
    /// Internal service-token registry.
    pub service_tokens: Arc<dyn ServiceTokenRegistry>,
    /// Readiness probe.
    pub readiness: Arc<dyn ReadinessProbe>,
    /// Request metrics.
    pub metrics: Arc<Metrics>,
}
