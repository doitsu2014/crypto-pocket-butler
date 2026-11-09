//! Reporting service package: the composition root for `reporting-api` and
//! `reporting-worker`.
//!
//! It wires infrastructure adapters into application use cases. The four Clean
//! Architecture crates never import each other outward.

pub mod config;

use std::sync::Arc;
use std::time::Duration;

use reporting_application::briefs::BriefService;
use reporting_application::generation::GenerationService;
use reporting_application::ports::{
    AuditClient, BriefRepository, Clock, DeliveryRepository, DueUsersClient, IdGenerator,
    NotificationClient, PerformanceClient, ReadinessProbe, RunRepository, ServiceTokenRegistry,
    TokenValidator, ValuationClient,
};
use reporting_application::privacy::PrivacyService;
use reporting_http::metrics::Metrics;
use reporting_http::state::{AppState, WorkerState};
use reporting_infrastructure::audit_client::HttpAuditClient;
use reporting_infrastructure::auth::{HttpJwksSource, JwksValidator};
use reporting_infrastructure::clients::{HttpDueUsers, HttpPerformance, HttpValuation};
use reporting_infrastructure::clock::SystemClock;
use reporting_infrastructure::connect;
use reporting_infrastructure::ids::UuidGenerator;
use reporting_infrastructure::notification_client::HttpNotification;
use reporting_infrastructure::readiness::PgReadinessProbe;
use reporting_infrastructure::repositories::{
    PgBriefRepository, PgDeliveryRepository, PgPrivacyRepository, PgRunRepository,
};
use reporting_infrastructure::service_token::PgServiceTokenRegistry;
use sqlx::PgPool;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

pub use config::{ApiConfig, WorkerConfig};

/// The API service identifier.
pub const SERVICE_NAME: &str = "reporting-api";
/// The generation worker identifier.
pub const WORKER_SERVICE_NAME: &str = "reporting-worker";

/// Returns the API service identifier.
#[must_use]
pub fn service_name() -> &'static str {
    SERVICE_NAME
}

struct Repositories {
    briefs: Arc<PgBriefRepository>,
    deliveries: Arc<PgDeliveryRepository>,
    runs: Arc<PgRunRepository>,
    privacy: Arc<PgPrivacyRepository>,
}

fn repositories(pool: &PgPool) -> Repositories {
    Repositories {
        briefs: Arc::new(PgBriefRepository::new(pool.clone())),
        deliveries: Arc::new(PgDeliveryRepository::new(pool.clone())),
        runs: Arc::new(PgRunRepository::new(pool.clone())),
        privacy: Arc::new(PgPrivacyRepository::new(pool.clone())),
    }
}

struct Adapters {
    due_users: Arc<dyn DueUsersClient>,
    valuation: Arc<dyn ValuationClient>,
    performance: Arc<dyn PerformanceClient>,
    notifications: Arc<dyn NotificationClient>,
    audit: Arc<dyn AuditClient>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
}

fn adapters(
    identity_url: &str,
    portfolio_url: &str,
    ledger_url: &str,
    notification_url: &str,
    audit_url: &str,
    outbound_token: &str,
) -> Adapters {
    Adapters {
        due_users: Arc::new(HttpDueUsers::new(
            identity_url.to_string(),
            outbound_token.to_string(),
        )),
        valuation: Arc::new(HttpValuation::new(
            portfolio_url.to_string(),
            outbound_token.to_string(),
        )),
        performance: Arc::new(HttpPerformance::new(
            ledger_url.to_string(),
            outbound_token.to_string(),
        )),
        notifications: Arc::new(HttpNotification::new(
            notification_url.to_string(),
            outbound_token.to_string(),
        )),
        audit: Arc::new(HttpAuditClient::new(
            audit_url.to_string(),
            outbound_token.to_string(),
        )),
        clock: Arc::new(SystemClock),
        ids: Arc::new(UuidGenerator),
    }
}

fn generation_service(repositories: &Repositories, adapters: &Adapters) -> Arc<GenerationService> {
    Arc::new(GenerationService::new(
        repositories.briefs.clone() as Arc<dyn BriefRepository>,
        repositories.deliveries.clone() as Arc<dyn DeliveryRepository>,
        adapters.due_users.clone(),
        adapters.valuation.clone(),
        adapters.performance.clone(),
        adapters.notifications.clone(),
        adapters.audit.clone(),
        repositories.runs.clone() as Arc<dyn RunRepository>,
        adapters.clock.clone(),
        adapters.ids.clone(),
    ))
}

/// Builds the API application state from configuration.
///
/// # Errors
/// Returns an error when the database is unreachable or a secret file is
/// unreadable.
pub async fn build_api_state(config: &ApiConfig) -> anyhow::Result<AppState> {
    let pool = connect(&config.database_url, config.pool_connections).await?;
    let repositories = repositories(&pool);
    let adapters = adapters(
        &config.identity_url,
        &config.portfolio_url,
        &config.ledger_url,
        &config.notification_url,
        &config.audit_url,
        &config.outbound_token,
    );
    let generation = generation_service(&repositories, &adapters);
    let briefs = Arc::new(BriefService::new(
        repositories.briefs.clone(),
        repositories.deliveries.clone(),
    ));
    let privacy = Arc::new(PrivacyService::new(
        repositories.briefs.clone(),
        repositories.privacy.clone(),
        adapters.audit.clone(),
        adapters.clock.clone(),
        adapters.ids.clone(),
    ));
    let token_validator: Arc<dyn TokenValidator> = Arc::new(JwksValidator::new(
        Arc::new(HttpJwksSource::new(config.jwks_url.clone())),
        config.jwt_issuer.clone(),
        config.jwt_audience.clone(),
    ));
    let readiness: Arc<dyn ReadinessProbe> = Arc::new(PgReadinessProbe::new(pool.clone()));
    let service_tokens: Arc<dyn ServiceTokenRegistry> =
        Arc::new(PgServiceTokenRegistry::with_outbound(
            pool,
            &[(
                config.token_service.as_str(),
                config.outbound_token.as_str(),
            )],
        ));

    Ok(AppState {
        service_name: SERVICE_NAME,
        briefs,
        generation,
        privacy,
        token_validator,
        service_tokens,
        readiness,
        metrics: Arc::new(Metrics::default()),
    })
}

/// Builds the worker state and the service it runs.
///
/// # Errors
/// Returns an error when the database is unreachable or a secret file is
/// unreadable.
pub async fn build_worker_state(
    config: &WorkerConfig,
) -> anyhow::Result<(WorkerState, Arc<GenerationService>)> {
    let pool = connect(&config.database_url, config.pool_connections).await?;
    let repositories = repositories(&pool);
    let adapters = adapters(
        &config.identity_url,
        &config.portfolio_url,
        &config.ledger_url,
        &config.notification_url,
        &config.audit_url,
        &config.outbound_token,
    );
    let generation = generation_service(&repositories, &adapters);
    let readiness: Arc<dyn ReadinessProbe> = Arc::new(PgReadinessProbe::new(pool.clone()));
    let service_tokens: Arc<dyn ServiceTokenRegistry> =
        Arc::new(PgServiceTokenRegistry::with_outbound(
            pool,
            &[(
                config.token_service.as_str(),
                config.outbound_token.as_str(),
            )],
        ));
    let state = WorkerState {
        service_name: WORKER_SERVICE_NAME,
        generation: generation.clone(),
        service_tokens,
        readiness,
        metrics: Arc::new(Metrics::default()),
    };
    Ok((state, generation))
}

/// Runs the API until shutdown.
///
/// # Errors
/// Returns an error when state cannot be built or the listener cannot bind.
pub async fn run(config: ApiConfig) -> anyhow::Result<()> {
    init_tracing();
    let state = build_api_state(&config).await?;
    let listener = TcpListener::bind(("0.0.0.0", config.port)).await?;
    tracing::info!(
        service = SERVICE_NAME,
        port = config.port,
        "service listening"
    );
    reporting_http::server::serve(listener, state).await?;
    tracing::info!(service = SERVICE_NAME, "service stopped");
    Ok(())
}

/// Runs the generation worker until shutdown.
///
/// # Errors
/// Returns an error when the database is unreachable or the listener cannot
/// bind.
pub async fn run_worker(config: WorkerConfig) -> anyhow::Result<()> {
    init_tracing();
    let (state, generation) = build_worker_state(&config).await?;
    let metrics = state.metrics.clone();
    let listener = TcpListener::bind(("0.0.0.0", config.port)).await?;
    tracing::info!(
        service = WORKER_SERVICE_NAME,
        port = config.port,
        "worker listening"
    );
    let supervisor = tokio::spawn(supervisor_loop(
        generation,
        metrics,
        config.run_interval_seconds,
    ));
    reporting_http::server::serve_worker(listener, state).await?;
    supervisor.abort();
    tracing::info!(service = WORKER_SERVICE_NAME, "worker stopped");
    Ok(())
}

async fn supervisor_loop(
    generation: Arc<GenerationService>,
    metrics: Arc<Metrics>,
    run_interval_seconds: u64,
) {
    let owner = format!("{WORKER_SERVICE_NAME}-{}", std::process::id());
    loop {
        match generation.run_scheduled(&owner).await {
            Ok(report) => {
                metrics.record_run(report.failed > 0, report.duration_seconds);
                metrics.record_briefs(report.generated, report.failed, report.skipped);
                metrics.record_email(report.email_submitted, report.email_failed);
                tracing::info!(
                    due = report.due,
                    generated = report.generated,
                    failed = report.failed,
                    skipped = report.skipped,
                    "brief generation finished"
                );
            }
            Err(error) => {
                metrics.record_run(true, 0.0);
                tracing::error!(%error, "the brief generation run failed");
            }
        }
        tokio::select! {
            () = tokio::time::sleep(Duration::from_secs(run_interval_seconds.max(1))) => {}
            () = shutdown_signal() => break,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_service_identifiers_name_the_two_listeners() {
        assert_eq!(service_name(), "reporting-api");
        assert_eq!(WORKER_SERVICE_NAME, "reporting-worker");
    }
}
