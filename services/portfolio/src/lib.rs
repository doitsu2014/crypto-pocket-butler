//! Portfolio service package: the composition root for `portfolio-api` and
//! `portfolio-worker`.
//!
//! It wires infrastructure adapters into application use cases. The four Clean
//! Architecture crates never import each other outward.

pub mod config;

use std::sync::Arc;
use std::time::Duration;

use portfolio_application::guardrails::GuardrailService;
use portfolio_application::intents::IntentService;
use portfolio_application::portfolios::PortfolioService;
use portfolio_application::ports::{
    AuditClient, Clock, GuardrailRepository, HoldingsClient, IdGenerator, LedgerClient,
    MarketDataClient, NotificationClient, ReadinessProbe, RunRepository, ServiceTokenRegistry,
    TargetRepository, TokenValidator, UserPreferenceClient,
};
use portfolio_application::privacy::PrivacyService;
use portfolio_application::run::ValuationRunService;
use portfolio_application::targets::TargetService;
use portfolio_application::trades::TradeService;
use portfolio_application::trail::Trail;
use portfolio_application::valuation::ValuationService;
use portfolio_http::metrics::Metrics;
use portfolio_http::state::{AppState, WorkerState};
use portfolio_infrastructure::audit_client::HttpAuditClient;
use portfolio_infrastructure::auth::{HttpJwksSource, JwksValidator};
use portfolio_infrastructure::clients::{
    HttpHoldingsClient, HttpIdentityPreferences, HttpLedgerClient, HttpMarketDataClient,
};
use portfolio_infrastructure::clock::SystemClock;
use portfolio_infrastructure::connect;
use portfolio_infrastructure::ids::UuidGenerator;
use portfolio_infrastructure::notification_client::HttpNotificationClient;
use portfolio_infrastructure::readiness::PgReadinessProbe;
use portfolio_infrastructure::repositories::{
    PgIntentRepository, PgMembershipRepository, PgOverlayRepository, PgPortfolioRepository,
    PgPrivacyRepository, PgRunRepository, PgSnapshotRepository, PgTargetRepository,
    PgTradeRepository,
};
use portfolio_infrastructure::service_token::PgServiceTokenRegistry;
use portfolio_infrastructure::wake::HttpWake;
use sqlx::PgPool;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

pub use config::{ApiConfig, WorkerConfig};

/// The API service identifier.
pub const SERVICE_NAME: &str = "portfolio-api";
/// The valuation worker identifier.
pub const WORKER_SERVICE_NAME: &str = "portfolio-worker";

/// Returns the API service identifier.
#[must_use]
pub fn service_name() -> &'static str {
    SERVICE_NAME
}

struct Repositories {
    portfolios: Arc<PgPortfolioRepository>,
    memberships: Arc<PgMembershipRepository>,
    targets: Arc<PgTargetRepository>,
    snapshots: Arc<PgSnapshotRepository>,
    overlays: Arc<PgOverlayRepository>,
    intents: Arc<PgIntentRepository>,
    trades: Arc<PgTradeRepository>,
    runs: Arc<PgRunRepository>,
    privacy: Arc<PgPrivacyRepository>,
}

fn repositories(pool: &PgPool) -> Repositories {
    Repositories {
        portfolios: Arc::new(PgPortfolioRepository::new(pool.clone())),
        memberships: Arc::new(PgMembershipRepository::new(pool.clone())),
        targets: Arc::new(PgTargetRepository::new(pool.clone())),
        snapshots: Arc::new(PgSnapshotRepository::new(pool.clone())),
        overlays: Arc::new(PgOverlayRepository::new(pool.clone())),
        intents: Arc::new(PgIntentRepository::new(pool.clone())),
        trades: Arc::new(PgTradeRepository::new(pool.clone())),
        runs: Arc::new(PgRunRepository::new(pool.clone())),
        privacy: Arc::new(PgPrivacyRepository::new(pool.clone())),
    }
}

struct Adapters {
    holdings: Arc<dyn HoldingsClient>,
    market: Arc<dyn MarketDataClient>,
    ledger: Arc<dyn LedgerClient>,
    preferences: Arc<dyn UserPreferenceClient>,
    audit: Arc<dyn AuditClient>,
    notification: Arc<dyn NotificationClient>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
}

#[allow(clippy::too_many_arguments)]
fn adapters(
    account_url: &str,
    market_data_url: &str,
    ledger_url: &str,
    identity_url: &str,
    notification_url: &str,
    audit_url: &str,
    outbound_token: &str,
) -> Adapters {
    Adapters {
        holdings: Arc::new(HttpHoldingsClient::new(
            account_url.to_string(),
            outbound_token.to_string(),
        )),
        market: Arc::new(HttpMarketDataClient::new(
            market_data_url.to_string(),
            outbound_token.to_string(),
        )),
        ledger: Arc::new(HttpLedgerClient::new(
            ledger_url.to_string(),
            outbound_token.to_string(),
        )),
        preferences: Arc::new(HttpIdentityPreferences::new(
            identity_url.to_string(),
            outbound_token.to_string(),
        )),
        audit: Arc::new(HttpAuditClient::new(
            audit_url.to_string(),
            outbound_token.to_string(),
        )),
        notification: Arc::new(HttpNotificationClient::new(
            notification_url.to_string(),
            outbound_token.to_string(),
        )),
        clock: Arc::new(SystemClock),
        ids: Arc::new(UuidGenerator),
    }
}

/// Builds the API application state from configuration.
///
/// # Errors
/// Returns an error when the database is unreachable or a secret file is
/// unreadable.
pub async fn build_api_state(config: &ApiConfig) -> anyhow::Result<AppState> {
    let pool = connect(&config.database_url, config.pool_connections).await?;
    let repositories = repositories(&pool);
    let service_tokens = Arc::new(PgServiceTokenRegistry::with_outbound(
        pool.clone(),
        &[(
            config.token_service.as_str(),
            config.outbound_token.as_str(),
        )],
    ));
    let adapters = adapters(
        &config.account_url,
        &config.market_data_url,
        &config.ledger_url,
        &config.identity_url,
        &config.notification_url,
        &config.audit_url,
        &config.outbound_token,
    );
    let trail = Arc::new(Trail::new(adapters.audit.clone(), adapters.clock.clone()));

    let target_repo: Arc<dyn TargetRepository> = repositories.targets.clone();
    let guardrail_repo: Arc<dyn GuardrailRepository> = repositories.targets.clone();
    let valuation = Arc::new(ValuationService::new(
        repositories.portfolios.clone(),
        repositories.memberships.clone(),
        target_repo.clone(),
        repositories.snapshots.clone(),
        repositories.overlays.clone(),
        adapters.holdings.clone(),
        adapters.market.clone(),
        adapters.clock.clone(),
        adapters.ids.clone(),
    ));
    let portfolios = Arc::new(PortfolioService::new(
        repositories.portfolios.clone(),
        repositories.memberships.clone(),
        adapters.preferences.clone(),
        adapters.clock.clone(),
        adapters.ids.clone(),
    ));
    let targets = Arc::new(TargetService::new(
        repositories.portfolios.clone(),
        target_repo.clone(),
        trail.clone(),
        adapters.clock.clone(),
        adapters.ids.clone(),
    ));
    let intents = Arc::new(IntentService::new(
        repositories.portfolios.clone(),
        repositories.intents.clone(),
        adapters.market.clone(),
        valuation.clone(),
        trail.clone(),
        adapters.clock.clone(),
        adapters.ids.clone(),
    ));
    let trades = Arc::new(TradeService::new(
        repositories.portfolios.clone(),
        repositories.trades.clone(),
        adapters.ledger.clone(),
        trail.clone(),
        adapters.clock.clone(),
        adapters.ids.clone(),
    ));
    let guardrails = Arc::new(GuardrailService::new(
        target_repo,
        guardrail_repo,
        adapters.notification.clone(),
        adapters.clock.clone(),
    ));
    let privacy = Arc::new(PrivacyService::new(
        repositories.portfolios.clone(),
        repositories.memberships.clone(),
        repositories.targets.clone(),
        repositories.snapshots.clone(),
        repositories.overlays.clone(),
        repositories.intents.clone(),
        repositories.trades.clone(),
        repositories.privacy.clone(),
        trail,
        adapters.clock.clone(),
        adapters.ids.clone(),
    ));
    let token_validator: Arc<dyn TokenValidator> = Arc::new(JwksValidator::new(
        Arc::new(HttpJwksSource::new(config.jwks_url.clone())),
        config.jwt_issuer.clone(),
        config.jwt_audience.clone(),
    ));
    let readiness: Arc<dyn ReadinessProbe> = Arc::new(PgReadinessProbe::new(pool));
    let service_tokens: Arc<dyn ServiceTokenRegistry> = service_tokens;

    Ok(AppState {
        service_name: SERVICE_NAME,
        portfolios,
        valuation,
        targets,
        intents,
        trades,
        guardrails,
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
/// Returns an error when the database is unreachable, a secret file is
/// unreadable, or the service-token registry has no outbound token.
pub async fn build_worker_state(
    config: &WorkerConfig,
) -> anyhow::Result<(WorkerState, Arc<ValuationRunService>)> {
    let pool = connect(&config.database_url, config.pool_connections).await?;
    let repositories = repositories(&pool);
    let service_tokens = Arc::new(PgServiceTokenRegistry::with_outbound(
        pool.clone(),
        &[(
            config.token_service.as_str(),
            config.outbound_token.as_str(),
        )],
    ));
    let adapters = adapters(
        &config.account_url,
        &config.market_data_url,
        &config.ledger_url,
        &config.identity_url,
        &config.notification_url,
        &config.audit_url,
        &config.outbound_token,
    );
    let target_repo: Arc<dyn TargetRepository> = repositories.targets.clone();
    let guardrail_repo: Arc<dyn GuardrailRepository> = repositories.targets.clone();
    let valuation = Arc::new(ValuationService::new(
        repositories.portfolios.clone(),
        repositories.memberships.clone(),
        target_repo.clone(),
        repositories.snapshots.clone(),
        repositories.overlays.clone(),
        adapters.holdings.clone(),
        adapters.market.clone(),
        adapters.clock.clone(),
        adapters.ids.clone(),
    ));
    let guardrails = Arc::new(GuardrailService::new(
        target_repo,
        guardrail_repo,
        adapters.notification.clone(),
        adapters.clock.clone(),
    ));
    let runs: Arc<dyn RunRepository> = repositories.runs.clone();
    let service = Arc::new(ValuationRunService::new(
        repositories.portfolios.clone(),
        repositories.memberships.clone(),
        repositories.overlays.clone(),
        adapters.holdings,
        repositories.trades.clone(),
        runs,
        valuation,
        guardrails,
        adapters.clock,
        adapters.ids,
    ));
    let readiness: Arc<dyn ReadinessProbe> = Arc::new(PgReadinessProbe::new(pool));
    let service_tokens: Arc<dyn ServiceTokenRegistry> = service_tokens;
    let state = WorkerState {
        service_name: WORKER_SERVICE_NAME,
        runs: service.clone(),
        service_tokens,
        readiness,
        metrics: Arc::new(Metrics::default()),
    };
    Ok((state, service))
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
    portfolio_http::server::serve(listener, state).await?;
    tracing::info!(service = SERVICE_NAME, "service stopped");
    Ok(())
}

/// Runs the valuation worker until shutdown.
///
/// # Errors
/// Returns an error when the database is unreachable or the listener cannot
/// bind.
pub async fn run_worker(config: WorkerConfig) -> anyhow::Result<()> {
    init_tracing();
    let (state, runs) = build_worker_state(&config).await?;
    let metrics = state.metrics.clone();
    let listener = TcpListener::bind(("0.0.0.0", config.port)).await?;
    tracing::info!(
        service = WORKER_SERVICE_NAME,
        port = config.port,
        "worker listening"
    );
    let supervisor = tokio::spawn(supervisor_loop(runs, metrics, config.run_interval_seconds));
    portfolio_http::server::serve_worker(listener, state).await?;
    supervisor.abort();
    tracing::info!(service = WORKER_SERVICE_NAME, "worker stopped");
    Ok(())
}

async fn supervisor_loop(
    runs: Arc<ValuationRunService>,
    metrics: Arc<Metrics>,
    run_interval_seconds: u64,
) {
    let owner = format!("{WORKER_SERVICE_NAME}-{}", std::process::id());
    loop {
        match runs.run_scheduled(&owner).await {
            Ok(report) => {
                metrics.record_run(report.failed > 0, report.duration_seconds);
                metrics.record_overlays_expired(report.overlays_expired);
                metrics.record_guardrail_alert_failure(report.guardrail_alerts_failed);
                tracing::info!(
                    portfolios = report.portfolios,
                    snapshots = report.snapshots,
                    failed = report.failed,
                    "valuation run finished"
                );
            }
            Err(error) => tracing::error!(%error, "the valuation run failed"),
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

/// Keeps the wake transport wired for deployments that notify the worker.
#[allow(dead_code)]
fn wake_transport(worker_url: &str, token: &str) -> HttpWake {
    HttpWake::new(worker_url.to_string(), token.to_string())
}
