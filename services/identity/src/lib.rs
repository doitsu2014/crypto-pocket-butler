//! Identity service package: the composition root for `identity-api` and
//! `identity-worker`.
//!
//! It wires infrastructure adapters into application use cases. The four Clean
//! Architecture crates never import each other outward.

pub mod config;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use identity_application::account_lifecycle::AccountLifecycleService;
use identity_application::authorisation::catalogue::OperationCatalogue;
use identity_application::authorisation::decision::AuthorisationDecision;
use identity_application::authorisation::ports::{
    AuditSink as AuthorisationAuditPort, ObservationSink, RoleWriter, StatusReader,
};
use identity_application::authorisation::role_change::RoleChange;
use identity_application::consent::ports::{
    ConsentRepository as ConsentEventRepository, ObservationSink as ConsentObservationSink,
};
use identity_application::consent::{
    ConsentGrant, ConsentHistory, ConsentService, ConsentStateQuery, ConsentWithdraw,
};
use identity_application::deletion::{DeletionService, DeletionWorker};
use identity_application::due_users::DueUsersService;
use identity_application::export::{ExportService, ExportWorker};
use identity_application::jurisdiction::{AuditSink, CheckJurisdiction, GeoLocator, PublishList};
use identity_application::ports::{
    AuditClient, AuthAdmin, Clock, ContributorGateway, ErasureStore, ExportStore, GateTokenIssuer,
    JurisdictionGate, ReadinessProbe, ServiceTokenRegistry, TokenValidator, UserRepository,
    WakeTransport,
};
use identity_application::preferences::ports::{PreferencesRepository, UserLookup};
use identity_application::preferences::read::ReadUserPreferences;
use identity_application::preferences::write::WriteUserPreferences;
use identity_application::preferences::PreferencesService;
use identity_application::principal::PrincipalService;
use identity_application::sessions::SessionService;
use identity_application::signup_gate::SignupGateService;
use identity_application::support::SupportService;
use identity_application::trail::Trail;
use identity_application::user_registry::AdminLookup;
use identity_application::worker::PrivacyWorker;
use identity_http::metrics::Metrics;
use identity_http::worker::WorkerState;
use identity_http::AppState;
use identity_infrastructure::audit_client::{HttpAuditClient, HttpDecisionAudit};
use identity_infrastructure::auth::{HttpJwksSource, JwksValidator};
use identity_infrastructure::auth_admin::HttpAuthAdmin;
use identity_infrastructure::authorisation::audit::AuthorisationAuditSink;
use identity_infrastructure::authorisation::role_writer::SupabaseRoleWriter;
use identity_infrastructure::authorisation::status::StatusReaderLocal;
use identity_infrastructure::authorisation::status_cache::StatusCache;
use identity_infrastructure::clients::HttpGeoLocator;
use identity_infrastructure::clock::SystemClock;
use identity_infrastructure::connect;
use identity_infrastructure::consent::erasure::PgRetentionStore;
use identity_infrastructure::consent::outbox::AuditOutboxDrainer;
use identity_infrastructure::consent::repository::PgConsentEventRepository;
use identity_infrastructure::contributors::HttpContributors;
use identity_infrastructure::export_store::SupabaseStorage;
use identity_infrastructure::gate_token::Hs256GateTokens;
use identity_infrastructure::preferences::repository::PgPreferencesRepository;
use identity_infrastructure::readiness::PgReadinessProbe;
use identity_infrastructure::repositories::{
    PgConsentRepository, PgDeletionRepository, PgDeviceSessionRepository, PgExportRepository,
    PgJurisdictionListRepository, PgJurisdictionRepository, PgShreddingKeyRepository,
    PgStatusTransitionStore, PgSupportGrantRepository, PgUserRepository,
};
use identity_infrastructure::service_token::FileServiceTokens;
use identity_infrastructure::wake::HttpWake;
use tokio::net::TcpListener;
use tokio::sync::Notify;
use tracing_subscriber::EnvFilter;

pub use config::{ApiConfig, WorkerConfig};

/// The API service identifier.
pub const SERVICE_NAME: &str = "identity-api";
/// The privacy worker identifier.
pub const WORKER_SERVICE_NAME: &str = "identity-worker";
/// The liveness marker the worker container health check reads.
pub const WORKER_HEARTBEAT_PATH: &str = "/tmp/identity-worker.heartbeat";
/// The erasure pool is small: one periodic delete, not a request path.
const ERASURE_CONNECTIONS: u32 = 2;

/// Returns the API service identifier.
#[must_use]
pub fn service_name() -> &'static str {
    SERVICE_NAME
}

/// The API's built state and the background drainer the composition root starts.
pub struct ApiRuntime {
    /// The router state.
    pub state: AppState,
    /// The U4 audit-outbox drainer.
    pub drainer: Arc<AuditOutboxDrainer>,
}

/// Builds the API application state from configuration.
///
/// # Errors
/// Returns an error when the database is unreachable, a secret file is
/// unreadable, or the service-token registry has no outbound token.
pub async fn build_api_state(config: &ApiConfig) -> anyhow::Result<ApiRuntime> {
    let pool = connect(&config.database_url, config.pool_connections).await?;

    let users = Arc::new(PgUserRepository::new(pool.clone()));
    let consent = Arc::new(PgConsentRepository::new(pool.clone()));
    let consent_events_store: Arc<dyn ConsentEventRepository> =
        Arc::new(PgConsentEventRepository::new(pool.clone()));
    let preferences_store: Arc<dyn PreferencesRepository> =
        Arc::new(PgPreferencesRepository::new(pool.clone()));
    let user_lookup: Arc<dyn UserLookup> = users.clone();
    let grants = Arc::new(PgSupportGrantRepository::new(pool.clone()));
    let sessions = Arc::new(PgDeviceSessionRepository::new(pool.clone()));
    let exports = Arc::new(PgExportRepository::new(pool.clone()));
    let deletions = Arc::new(PgDeletionRepository::new(pool.clone()));
    let rules = Arc::new(PgJurisdictionRepository::new(pool.clone()));
    let jurisdiction_lists = Arc::new(PgJurisdictionListRepository::new(pool.clone()));

    let service_tokens: Arc<dyn ServiceTokenRegistry> = Arc::new(
        FileServiceTokens::from_file(&config.service_tokens_path).map_err(anyhow::Error::msg)?,
    );
    let outbound_token = service_tokens
        .token_for(&config.token_service)
        .with_context(|| {
            format!(
                "the service-token registry has no token for '{}'",
                config.token_service
            )
        })?;

    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let metrics = Arc::new(Metrics::default());
    let observation: Arc<dyn ObservationSink> = metrics.clone();
    let status_reader: Arc<dyn StatusReader> = Arc::new(StatusCache::new(
        Arc::new(StatusReaderLocal::new(pool.clone())),
        clock.clone(),
        observation.clone(),
    ));
    let authorisation_audit: Arc<dyn AuthorisationAuditPort> = Arc::new(
        AuthorisationAuditSink::new(config.audit_url.clone(), outbound_token.clone()),
    );
    let role_writer: Arc<dyn RoleWriter> = Arc::new(SupabaseRoleWriter::new(
        config.supabase_auth_url.clone(),
        config.service_role_key.clone(),
    ));
    let authorisation = Arc::new(AuthorisationDecision::new(
        OperationCatalogue::new(),
        status_reader,
        authorisation_audit.clone(),
        observation.clone(),
        clock.clone(),
    ));
    let authorisation_users: Arc<dyn UserRepository> = users.clone();
    let role_change = Arc::new(RoleChange::new(
        authorisation_users.clone(),
        role_writer,
        authorisation_audit,
        observation,
        clock.clone(),
    ));
    let audit: Arc<dyn AuditClient> = Arc::new(HttpAuditClient::new(
        config.audit_url.clone(),
        outbound_token.clone(),
    ));
    let decision_audit: Arc<dyn AuditSink> = Arc::new(HttpDecisionAudit::new(
        config.audit_url.clone(),
        outbound_token.clone(),
    ));
    let geo: Arc<dyn GeoLocator> = Arc::new(HttpGeoLocator::new(
        config.jurisdiction_provider_url.clone(),
        config.jurisdiction_provider_key.clone(),
    ));
    let trail = Arc::new(Trail::new(audit, clock.clone()));

    let token_validator: Arc<dyn TokenValidator> = Arc::new(JwksValidator::new(
        Arc::new(HttpJwksSource::new(config.jwks_url.clone())),
        config.jwt_issuer.clone(),
        config.jwt_audience.clone(),
    ));
    let auth_admin: Arc<dyn AuthAdmin> = Arc::new(HttpAuthAdmin::new(
        config.supabase_auth_url.clone(),
        config.service_role_key.clone(),
    ));
    let store: Arc<dyn ExportStore> = Arc::new(SupabaseStorage::new(
        config.storage_url.clone(),
        config.service_role_key.clone(),
        config.export_bucket.clone(),
    ));
    let wake: Arc<dyn WakeTransport> = Arc::new(HttpWake::new(
        config.worker_url.clone(),
        outbound_token.clone(),
    ));
    let gate_issuer: Arc<dyn GateTokenIssuer> =
        Arc::new(Hs256GateTokens::new(config.gate_token_key.clone()));

    let jurisdiction_check = Arc::new(CheckJurisdiction::new(
        jurisdiction_lists.clone(),
        geo,
        decision_audit.clone(),
        clock.clone(),
    ));
    let gate: Arc<dyn JurisdictionGate> = jurisdiction_check.clone();
    let principal = Arc::new(PrincipalService::new(
        users.clone(),
        sessions.clone(),
        trail.clone(),
        gate,
        clock.clone(),
    ));
    let user_registry = Arc::new(AdminLookup::new(users.clone(), trail.clone()));
    let consent_service = Arc::new(ConsentService::new(consent, trail.clone(), clock.clone()));
    let consent_observation: Arc<dyn ConsentObservationSink> = metrics.clone();
    let consent_state = Arc::new(ConsentStateQuery::new(
        consent_events_store.clone(),
        consent_observation.clone(),
    ));
    let consent_grant = Arc::new(ConsentGrant::new(
        consent_events_store.clone(),
        consent_observation.clone(),
    ));
    let consent_withdraw = Arc::new(ConsentWithdraw::new(
        consent_events_store.clone(),
        consent_observation.clone(),
    ));
    let consent_history = Arc::new(ConsentHistory::new(consent_events_store));
    let preferences_read = Arc::new(ReadUserPreferences::new(
        preferences_store.clone(),
        user_lookup.clone(),
    ));
    let preferences_write = Arc::new(WriteUserPreferences::new(
        preferences_store,
        user_lookup,
        clock.clone(),
        consent_observation.clone(),
    ));
    let preferences = Arc::new(PreferencesService::new(
        users.clone(),
        trail.clone(),
        clock.clone(),
    ));
    let due_users_service = Arc::new(DueUsersService::new(users.clone()));
    let sessions_service = Arc::new(SessionService::new(
        sessions,
        users.clone(),
        auth_admin,
        trail.clone(),
        clock.clone(),
    ));
    let support = Arc::new(SupportService::new(
        grants,
        users.clone(),
        trail.clone(),
        clock.clone(),
    ));
    let signup_gate = Arc::new(SignupGateService::new(
        rules,
        gate_issuer,
        trail.clone(),
        clock.clone(),
    ));
    let jurisdiction_publish = Arc::new(PublishList::new(
        jurisdiction_lists,
        decision_audit,
        clock.clone(),
    ));
    let export_service = Arc::new(ExportService::new(
        exports,
        store,
        trail.clone(),
        wake.clone(),
        clock.clone(),
    ));
    let deletion_service = Arc::new(DeletionService::new(deletions.clone(), trail.clone()));
    let transitions = Arc::new(PgStatusTransitionStore::new(pool.clone()));
    let account_observation: Arc<dyn identity_application::account_lifecycle::AccountObservation> =
        metrics.clone();
    let account_lifecycle = Arc::new(AccountLifecycleService::new(
        users.clone(),
        transitions,
        deletions,
        wake.clone(),
        account_observation,
        clock.clone(),
    ));
    let drainer = Arc::new(AuditOutboxDrainer::new(
        pool.clone(),
        config.audit_url.clone(),
        outbound_token,
        metrics.clone(),
        Duration::from_secs(config.drainer_interval_seconds),
    ));
    let readiness: Arc<dyn ReadinessProbe> = Arc::new(PgReadinessProbe::new(pool));

    let state = AppState {
        service_name: SERVICE_NAME,
        principal,
        consent: consent_service,
        consent_state,
        consent_grant,
        consent_withdraw,
        consent_history,
        consent_services: Arc::new(config.consent_services.clone()),
        consent_observation,
        preferences,
        preferences_read,
        preferences_write,
        sessions: sessions_service,
        support,
        signup_gate,
        jurisdiction_check,
        jurisdiction_publish,
        exports: export_service,
        deletions: deletion_service,
        account_lifecycle,
        due_users: due_users_service,
        user_registry,
        users: authorisation_users,
        authorisation,
        role_change,
        authorize_services: Arc::new(config.authorize_services.clone()),
        token_validator,
        service_tokens,
        wake,
        readiness,
        metrics,
    };
    Ok(ApiRuntime { state, drainer })
}

/// Runs the API until shutdown.
///
/// # Errors
/// Returns an error when state cannot be built or the listener cannot bind.
pub async fn run(config: ApiConfig) -> anyhow::Result<()> {
    init_tracing();
    let runtime = build_api_state(&config).await?;
    let listener = TcpListener::bind(("0.0.0.0", config.port)).await?;
    tracing::info!(
        service = SERVICE_NAME,
        port = config.port,
        "service listening"
    );
    let drainer = runtime.drainer.clone();
    let drainer_handle = tokio::spawn(async move { drainer.run_forever().await });
    identity_http::server::serve(listener, runtime.state).await?;
    drainer_handle.abort();
    tracing::info!(service = SERVICE_NAME, "service stopped");
    Ok(())
}

/// Runs the privacy worker until shutdown.
///
/// # Errors
/// Returns an error when the database is unreachable or the listener cannot bind.
pub async fn run_worker(config: WorkerConfig) -> anyhow::Result<()> {
    init_tracing();
    let pool = connect(&config.database_url, config.pool_connections).await?;

    let users = Arc::new(PgUserRepository::new(pool.clone()));
    let keys = Arc::new(PgShreddingKeyRepository::new(pool.clone()));
    let erasure_pool = connect(&config.retention_database_url, ERASURE_CONNECTIONS).await?;
    let erasure: Arc<dyn ErasureStore> = Arc::new(PgRetentionStore::new(erasure_pool));
    let exports = Arc::new(PgExportRepository::new(pool.clone()));
    let deletions = Arc::new(PgDeletionRepository::new(pool.clone()));

    let service_tokens: Arc<dyn ServiceTokenRegistry> = Arc::new(
        FileServiceTokens::from_file(&config.service_tokens_path).map_err(anyhow::Error::msg)?,
    );
    let outbound_token = service_tokens
        .token_for(&config.token_service)
        .with_context(|| {
            format!(
                "the service-token registry has no token for '{}'",
                config.token_service
            )
        })?;

    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let audit: Arc<dyn AuditClient> = Arc::new(HttpAuditClient::new(
        config.audit_url.clone(),
        outbound_token.clone(),
    ));
    let trail = Arc::new(Trail::new(audit, clock));
    let contributors: Arc<dyn ContributorGateway> = Arc::new(HttpContributors::new(
        config.account_url.clone(),
        config.ledger_url.clone(),
        config.portfolio_url.clone(),
        outbound_token,
    ));
    let auth_admin: Arc<dyn AuthAdmin> = Arc::new(HttpAuthAdmin::new(
        config.supabase_auth_url.clone(),
        config.service_role_key.clone(),
    ));
    let store: Arc<dyn ExportStore> = Arc::new(SupabaseStorage::new(
        config.storage_url.clone(),
        config.service_role_key.clone(),
        config.export_bucket.clone(),
    ));

    let export_worker = Arc::new(ExportWorker::new(
        exports,
        contributors.clone(),
        store,
        trail.clone(),
    ));
    let worker_metrics = Arc::new(Metrics::default());
    let deletion_worker = Arc::new(DeletionWorker::new(
        deletions.clone(),
        users,
        keys,
        erasure,
        contributors,
        auth_admin,
        Arc::new(PgStatusTransitionStore::new(pool.clone())),
        worker_metrics.clone(),
        trail,
    ));
    let worker = Arc::new(PrivacyWorker::new(export_worker, deletion_worker));

    let notifier = Arc::new(Notify::new());
    let readiness: Arc<dyn ReadinessProbe> = Arc::new(PgReadinessProbe::new(pool));
    let state = WorkerState {
        service_name: WORKER_SERVICE_NAME,
        notifier: notifier.clone(),
        service_tokens,
        readiness,
        metrics: worker_metrics,
    };

    let listener = TcpListener::bind(("0.0.0.0", config.port)).await?;
    tracing::info!(
        service = WORKER_SERVICE_NAME,
        port = config.port,
        "worker listening"
    );
    let supervisor = tokio::spawn(supervisor_loop(
        worker,
        notifier,
        config.sweep_interval_seconds,
    ));
    identity_http::server::serve_worker(listener, state).await?;
    supervisor.abort();
    tracing::info!(service = WORKER_SERVICE_NAME, "worker stopped");
    Ok(())
}

/// One pass runs the export and deletion workers; the loop then waits for the
/// sweep interval, a wake call, or shutdown.
async fn supervisor_loop(worker: Arc<PrivacyWorker>, notifier: Arc<Notify>, interval: u64) {
    let worker_id = format!("{WORKER_SERVICE_NAME}-{}", std::process::id());
    loop {
        let report = worker.run_once(&worker_id, chrono::Utc::now()).await;
        if report.exports_processed + report.deletions_processed > 0 {
            tracing::info!(
                exports = report.exports_processed,
                deletions = report.deletions_processed,
                "privacy sweep processed work"
            );
        }
        write_heartbeat();
        tokio::select! {
            () = tokio::time::sleep(Duration::from_secs(interval)) => {}
            () = notifier.notified() => {}
            () = shutdown_signal() => break,
        }
    }
}

/// The worker has an inbound listener, but the heartbeat lets a container health
/// check distinguish a live process from a wedged supervisor.
fn write_heartbeat() {
    let now = chrono::Utc::now().to_rfc3339();
    if let Err(error) = std::fs::write(WORKER_HEARTBEAT_PATH, now) {
        tracing::warn!(%error, "failed to write the worker heartbeat");
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .json()
        .with_env_filter(filter)
        .try_init();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}
