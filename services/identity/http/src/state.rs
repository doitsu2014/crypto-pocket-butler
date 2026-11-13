//! Shared router state.

use std::sync::Arc;

use identity_application::account_lifecycle::AccountLifecycleService;
use identity_application::authorisation::decision::AuthorisationDecision;
use identity_application::authorisation::role_change::RoleChange;
use identity_application::consent::ports::ObservationSink as ConsentObservationSink;
use identity_application::consent::{
    ConsentGrant, ConsentHistory, ConsentService, ConsentStateQuery, ConsentWithdraw,
};
use identity_application::deletion::DeletionService;
use identity_application::export::ExportService;
use identity_application::jurisdiction::{CheckJurisdiction, PublishList};
use identity_application::ports::{
    ReadinessProbe, ServiceTokenRegistry, TokenValidator, UserRepository, WakeTransport,
};
use identity_application::preferences::read::ReadUserPreferences;
use identity_application::preferences::write::WriteUserPreferences;
use identity_application::preferences::PreferencesService;
use identity_application::principal::PrincipalService;
use identity_application::profile::ProfileService;
use identity_application::sessions::SessionService;
use identity_application::signup_gate::SignupGateService;
use identity_application::support::SupportService;
use identity_application::user_registry::AdminLookup;

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
    /// U4: the consent-state query.
    pub consent_state: Arc<ConsentStateQuery>,
    /// U4: the consent grant.
    pub consent_grant: Arc<ConsentGrant>,
    /// U4: the consent withdrawal.
    pub consent_withdraw: Arc<ConsentWithdraw>,
    /// U4: the consent history.
    pub consent_history: Arc<ConsentHistory>,
    /// U4: the services allowed on the C6 consent-state route.
    pub consent_services: Arc<Vec<String>>,
    /// U4: the metric sink the handlers record refusals into.
    pub consent_observation: Arc<dyn ConsentObservationSink>,
    /// Preference updates.
    pub preferences: Arc<PreferencesService>,
    /// U7: the profile read and the two profile writes.
    pub profile: Arc<ProfileService>,
    /// U4: the effective preference read.
    pub preferences_read: Arc<ReadUserPreferences>,
    /// U4: the partial preference write.
    pub preferences_write: Arc<WriteUserPreferences>,
    /// Session listing and revocation.
    pub sessions: Arc<SessionService>,
    /// Support-grant use cases.
    pub support: Arc<SupportService>,
    /// The pre-auth jurisdiction gate.
    pub signup_gate: Arc<SignupGateService>,
    /// The runtime jurisdiction check (C8).
    pub jurisdiction_check: Arc<CheckJurisdiction>,
    /// The jurisdiction list publish (W3).
    pub jurisdiction_publish: Arc<PublishList>,
    /// Export request and status.
    pub exports: Arc<ExportService>,
    /// Deletion request and status.
    pub deletions: Arc<DeletionService>,
    /// U5: the account status machine and the recovery window.
    pub account_lifecycle: Arc<AccountLifecycleService>,
    /// C3: the reporting due-user read.
    pub due_users: Arc<identity_application::due_users::DueUsersService>,
    /// U1: the admin lookup and the owner/admin record read.
    pub user_registry: Arc<AdminLookup>,
    /// The user records the C4 adapter resolves the token's subject to.
    pub users: Arc<dyn UserRepository>,
    /// U2: the in-process decision the C4 route adapts.
    pub authorisation: Arc<AuthorisationDecision>,
    /// U2: the admin role change.
    pub role_change: Arc<RoleChange>,
    /// U2: the services allowed on the C4 endpoint.
    pub authorize_services: Arc<Vec<String>>,
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
