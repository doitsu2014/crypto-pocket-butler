//! Analytics service package: the composition root for both binaries.
//!
//! It wires infrastructure adapters into application use cases. The four Clean
//! Architecture crates never import each other outward.

pub mod config;

use std::path::Path;
use std::sync::Arc;

use analytics_application::ports::{
    AggregateRepository, AuditClient, Clock, ConsentChecker, ConsentRepository, EventRepository,
    FlagRepository, SaltProvider, ServiceTokenRegistry, TokenValidator,
};
use analytics_application::{
    AggregateService, EraseService, FlagService, IngestEvent, Pause, RetentionService, Trail,
};
use analytics_http::metrics::Metrics;
use analytics_http::AppState;
use analytics_infrastructure::auth::{HttpJwksSource, JwksValidator};
use analytics_infrastructure::clients::{HttpAuditClient, HttpConsentChecker};
use analytics_infrastructure::clock::SystemClock;
use analytics_infrastructure::connect;
use analytics_infrastructure::pause::TokioPause;
use analytics_infrastructure::readiness::AnalyticsReadinessProbe;
use analytics_infrastructure::repositories::{
    PgAggregateRepository, PgConsentRepository, PgEventRepository, PgFlagRepository,
};
use analytics_infrastructure::service_token::PgServiceTokenRegistry;
use analytics_infrastructure::vault::PgSaltProvider;
use async_trait::async_trait;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

pub use config::{ApiConfig, MigrateConfig, WorkerConfig};

/// The API service identifier.
pub const SERVICE_NAME: &str = "analytics-api";
/// The worker service identifier.
pub const WORKER_SERVICE_NAME: &str = "analytics-worker";

struct Components {
    ingest: Arc<IngestEvent>,
    flags: Arc<FlagService>,
    aggregates: Arc<AggregateService>,
    retention: Arc<RetentionService>,
    erase: Arc<EraseService>,
    service_tokens: Arc<dyn ServiceTokenRegistry>,
    clock: Arc<dyn Clock>,
    readiness: Arc<dyn analytics_application::ports::ReadinessProbe>,
    metrics: Arc<Metrics>,
}

#[allow(clippy::too_many_arguments)]
async fn components(
    database_url: &str,
    pool_connections: u32,
    identity_url: &str,
    audit_url: &str,
    outbound_token: &str,
    smoke_token_path: Option<&Path>,
) -> anyhow::Result<Components> {
    let pool = connect(database_url, pool_connections).await?;

    let events: Arc<dyn EventRepository> = Arc::new(PgEventRepository::new(pool.clone()));
    let consents: Arc<dyn ConsentRepository> = Arc::new(PgConsentRepository::new(pool.clone()));
    let aggregates_repository: Arc<dyn AggregateRepository> =
        Arc::new(PgAggregateRepository::new(pool.clone()));
    let flags_repository: Arc<dyn FlagRepository> = Arc::new(PgFlagRepository::new(pool.clone()));

    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let salts: Arc<dyn SaltProvider> = Arc::new(PgSaltProvider::new(pool.clone()));
    let audit: Arc<dyn AuditClient> =
        Arc::new(HttpAuditClient::new(audit_url, outbound_token.to_string()));
    let trail = Arc::new(Trail::new(audit, clock.clone()));
    let consent_checker: Arc<dyn ConsentChecker> = Arc::new(HttpConsentChecker::new(
        identity_url,
        outbound_token.to_string(),
    ));

    let ingest = Arc::new(IngestEvent::new(
        events.clone(),
        consent_checker,
        salts.clone(),
        trail.clone(),
        clock.clone(),
    ));
    let flags = Arc::new(FlagService::new(
        flags_repository,
        clock.clone(),
        trail.clone(),
    ));
    let aggregates = Arc::new(AggregateService::new(
        events.clone(),
        aggregates_repository,
        clock.clone(),
    ));
    let pause: Arc<dyn Pause> = Arc::new(TokioPause::new(200));
    let retention = Arc::new(RetentionService::new(
        events.clone(),
        consents.clone(),
        clock.clone(),
        pause,
    ));
    let erase = Arc::new(EraseService::new(events, consents, salts.clone(), trail));

    let mut registry = PgServiceTokenRegistry::new(pool.clone());
    if let Some(path) = smoke_token_path {
        let token = std::fs::read_to_string(path)
            .map_err(|error| anyhow::anyhow!("{} is unreadable: {error}", path.display()))?;
        registry = registry.with_extra_token(token.trim(), "smoke-reader", &["smoke"]);
    }
    let service_tokens: Arc<dyn ServiceTokenRegistry> = Arc::new(registry);
    let readiness = Arc::new(AnalyticsReadinessProbe::new(pool, salts));

    Ok(Components {
        ingest,
        flags,
        aggregates,
        retention,
        erase,
        service_tokens,
        clock,
        readiness,
        metrics: Arc::new(Metrics::default()),
    })
}

/// Builds the API state from configuration.
///
/// # Errors
/// Returns an error when the database is unreachable, the salt table is
/// unreadable, or any secret file is missing.
pub async fn build_state(config: &ApiConfig) -> anyhow::Result<AppState> {
    let components = components(
        &config.database_url,
        config.pool_connections,
        &config.identity_url,
        &config.audit_url,
        &config.outbound_token,
        config.smoke_token_path.as_deref(),
    )
    .await?;

    let token_validator: Arc<dyn TokenValidator> = Arc::new(JwksValidator::new(
        Arc::new(HttpJwksSource::new(config.jwks_url.clone())),
        config.jwt_issuer.clone(),
        config.jwt_audience.clone(),
    ));

    Ok(AppState {
        service_name: SERVICE_NAME,
        ingest: components.ingest,
        flags: components.flags,
        aggregates: components.aggregates,
        retention: components.retention,
        erase: components.erase,
        service_tokens: components.service_tokens,
        token_validator,
        clock: components.clock,
        readiness: components.readiness,
        metrics: components.metrics,
    })
}

/// A validator for the worker, whose router serves no JWT path.
struct WorkerTokenValidator;

#[async_trait]
impl TokenValidator for WorkerTokenValidator {
    async fn validate(
        &self,
        _token: &str,
    ) -> Result<analytics_application::Principal, analytics_application::AuthError> {
        Err(analytics_application::AuthError::Missing)
    }
}

/// Builds the worker state from configuration.
///
/// # Errors
/// Returns an error when the database is unreachable or a secret file is missing.
pub async fn build_worker_state(config: &WorkerConfig) -> anyhow::Result<AppState> {
    let components = components(
        &config.database_url,
        config.pool_connections,
        &config.identity_url,
        &config.audit_url,
        &config.outbound_token,
        config.smoke_token_path.as_deref(),
    )
    .await?;

    Ok(AppState {
        service_name: WORKER_SERVICE_NAME,
        ingest: components.ingest,
        flags: components.flags,
        aggregates: components.aggregates,
        retention: components.retention,
        erase: components.erase,
        service_tokens: components.service_tokens,
        token_validator: Arc::new(WorkerTokenValidator),
        clock: components.clock,
        readiness: components.readiness,
        metrics: components.metrics,
    })
}

/// Runs the API until shutdown.
///
/// # Errors
/// Returns an error when state cannot be built or the listener cannot bind.
pub async fn run_api(config: ApiConfig) -> anyhow::Result<()> {
    init_tracing();
    let state = build_state(&config).await?;
    let listener = TcpListener::bind(("0.0.0.0", config.port)).await?;
    tracing::info!(
        service = SERVICE_NAME,
        port = config.port,
        "service listening"
    );
    analytics_http::server::serve(listener, analytics_http::api_router(state)).await?;
    tracing::info!(service = SERVICE_NAME, "service stopped");
    Ok(())
}

/// Runs the worker until shutdown.
///
/// # Errors
/// Returns an error when state cannot be built or the listener cannot bind.
pub async fn run_worker(config: WorkerConfig) -> anyhow::Result<()> {
    init_tracing();
    let state = build_worker_state(&config).await?;
    let listener = TcpListener::bind(("0.0.0.0", config.port)).await?;
    tracing::info!(
        service = WORKER_SERVICE_NAME,
        port = config.port,
        "worker listening"
    );
    analytics_http::server::serve(listener, analytics_http::worker_router(state)).await?;
    tracing::info!(service = WORKER_SERVICE_NAME, "worker stopped");
    Ok(())
}

/// Applies the migrations and exits.
///
/// # Errors
/// Returns an error when the database is unreachable or a migration fails.
pub async fn run_migrate(config: MigrateConfig) -> anyhow::Result<()> {
    init_tracing();
    let pool = connect(&config.database_url, 2).await?;
    analytics_infrastructure::migrations::run(&pool).await?;
    tracing::info!("analytics migrations applied");
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .json()
        .with_env_filter(filter)
        .try_init();
}
