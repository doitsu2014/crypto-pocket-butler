//! Market Data service package: the composition root for `market-data-api` and
//! `market-data-worker`.
//!
//! It wires infrastructure adapters into application use cases. The four Clean
//! Architecture crates never import each other outward.

pub mod config;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::Context;
use market_data_application::ports::{
    AssetRegistry, AuditClient, Clock, FxRepository, ObservationRepository, PriceProvider,
    ProviderSecrets, RateLimiter, ReadinessProbe, ServiceTokenRegistry, Sleeper, TierRepository,
};
use market_data_application::{FxService, QuoteService, RefreshService, RetentionService};
use market_data_http::metrics::Metrics;
use market_data_http::state::AppState;
use market_data_http::worker::WorkerState;
use market_data_infrastructure::audit_client::HttpAuditClient;
use market_data_infrastructure::clock::SystemClock;
use market_data_infrastructure::connect;
use market_data_infrastructure::provider::HttpPriceProvider;
use market_data_infrastructure::readiness::PgReadinessProbe;
use market_data_infrastructure::registry_client::HttpAssetRegistry;
use market_data_infrastructure::repositories::{
    PgFxRepository, PgObservationRepository, PgRateLimiter, PgRefreshRunRepository,
    PgTierRepository,
};
use market_data_infrastructure::service_token::PgServiceTokenRegistry;
use market_data_infrastructure::sleeper::TokioSleeper;
use market_data_infrastructure::vault::PgProviderSecrets;
use sqlx::PgPool;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

pub use config::{ApiConfig, WorkerConfig};

/// The API service identifier.
pub const SERVICE_NAME: &str = "market-data-api";
/// The refresh worker identifier.
pub const WORKER_SERVICE_NAME: &str = "market-data-worker";

/// The composition of the market-data adapters, shared by both binaries.
struct Components {
    quotes: Arc<QuoteService>,
    fx: Arc<FxService>,
    refresh: Arc<RefreshService>,
    retention: Arc<RetentionService>,
    service_tokens: Arc<PgServiceTokenRegistry>,
    readiness: Arc<dyn ReadinessProbe>,
    metrics: Arc<Metrics>,
}

fn outbound_token(
    registry: &PgServiceTokenRegistry,
    token_service: &str,
) -> anyhow::Result<String> {
    registry.token_for(token_service).with_context(|| {
        format!("the service-token registry has no outbound token for '{token_service}'")
    })
}

async fn load_run_token(pool: &PgPool, path: Option<&Path>) -> Option<String> {
    if let Some(path) = path {
        return std::fs::read_to_string(path)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
    }
    let secrets = PgProviderSecrets::new(pool.clone());
    match secrets.provider_key("run").await {
        Ok(token) => token,
        Err(error) => {
            tracing::warn!(%error, "the C11 run token could not be read from Vault");
            None
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn components(
    pool: &PgPool,
    asset_registry_url: &str,
    audit_url: &str,
    token_service: &str,
    service_tokens_path: Option<&Path>,
    run_token_path: Option<&Path>,
    provider_urls: &[(String, String)],
    fx_base_currencies: &[String],
) -> anyhow::Result<Components> {
    let mut service_tokens = match service_tokens_path {
        Some(path) => {
            PgServiceTokenRegistry::from_file(pool.clone(), path).map_err(anyhow::Error::msg)?
        }
        None => PgServiceTokenRegistry::with_outbound(pool.clone(), &[]),
    };
    let run_token = load_run_token(pool, run_token_path).await;
    if service_tokens.token_for(token_service).is_none() {
        let vault = PgProviderSecrets::new(pool.clone());
        if let Some(token) = vault
            .provider_key("outbound")
            .await
            .map_err(|error| anyhow::anyhow!("the outbound token could not be read: {error}"))?
        {
            service_tokens = service_tokens.with_outbound_token(token_service, &token);
        }
    }
    let service_tokens = Arc::new(service_tokens.with_run_token(run_token));
    let outbound = outbound_token(&service_tokens, token_service)?;

    let observations: Arc<dyn ObservationRepository> =
        Arc::new(PgObservationRepository::new(pool.clone()));
    let fx: Arc<dyn FxRepository> = Arc::new(PgFxRepository::new(pool.clone()));
    let runs = Arc::new(PgRefreshRunRepository::new(pool.clone()));
    let tiers: Arc<dyn TierRepository> = Arc::new(PgTierRepository::new(pool.clone()));
    let limiter: Arc<dyn RateLimiter> = Arc::new(PgRateLimiter::new(pool.clone()));
    let registry: Arc<dyn AssetRegistry> =
        Arc::new(HttpAssetRegistry::new(asset_registry_url, outbound.clone()));
    let provider: Arc<dyn PriceProvider> = Arc::new(HttpPriceProvider::new(
        provider_urls.iter().cloned().collect::<BTreeMap<_, _>>(),
    ));
    let secrets: Arc<dyn ProviderSecrets> = Arc::new(PgProviderSecrets::new(pool.clone()));
    let audit: Arc<dyn AuditClient> = Arc::new(HttpAuditClient::new(audit_url, outbound));
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let sleeper: Arc<dyn Sleeper> = Arc::new(TokioSleeper);
    let metrics = Arc::new(Metrics::default());

    let quotes = Arc::new(QuoteService::new(
        registry.clone(),
        observations.clone(),
        tiers.clone(),
        metrics.clone(),
        clock.clone(),
    ));
    let fx_service = Arc::new(FxService::new(fx.clone(), metrics.clone(), clock.clone()));
    let refresh = Arc::new(RefreshService::new(
        registry,
        observations.clone(),
        fx.clone(),
        runs,
        tiers,
        provider,
        secrets,
        limiter,
        audit,
        clock.clone(),
        sleeper,
        metrics.clone(),
        fx_base_currencies.to_vec(),
    ));
    let retention = Arc::new(RetentionService::new(observations, fx, clock));
    let readiness: Arc<dyn ReadinessProbe> = Arc::new(PgReadinessProbe::new(pool.clone()));
    Ok(Components {
        quotes,
        fx: fx_service,
        refresh,
        retention,
        service_tokens,
        readiness,
        metrics,
    })
}

/// Builds the API application state from configuration.
///
/// # Errors
/// Returns an error when the database is unreachable, a secret file is
/// unreadable, or the service-token registry has no outbound token.
pub async fn build_api_state(config: &ApiConfig) -> anyhow::Result<AppState> {
    let pool = connect(&config.database_url, config.pool_connections).await?;
    let parts = components(
        &pool,
        &config.asset_registry_url,
        &config.audit_url,
        &config.token_service,
        config.service_tokens_path.as_deref(),
        config.run_token_path.as_deref(),
        &config.provider_urls,
        &config.fx_base_currencies,
    )
    .await?;
    Ok(AppState {
        service_name: SERVICE_NAME,
        quotes: parts.quotes,
        fx: parts.fx,
        refresh: parts.refresh,
        service_tokens: parts.service_tokens,
        readiness: parts.readiness,
        metrics: parts.metrics,
    })
}

/// The worker's state and the supervisors it runs.
pub struct WorkerRuntime {
    /// The listener state.
    pub state: WorkerState,
    refresh: Arc<RefreshService>,
    retention: Arc<RetentionService>,
    sweep_interval_seconds: u64,
}

/// Builds the worker runtime from configuration.
///
/// # Errors
/// Returns an error when the database is unreachable or a secret file is
/// unreadable.
pub async fn build_worker_runtime(config: &WorkerConfig) -> anyhow::Result<WorkerRuntime> {
    let pool = connect(&config.database_url, config.pool_connections).await?;
    let parts = components(
        &pool,
        &config.asset_registry_url,
        &config.audit_url,
        &config.token_service,
        config.service_tokens_path.as_deref(),
        None,
        &config.provider_urls,
        &config.fx_base_currencies,
    )
    .await?;
    Ok(WorkerRuntime {
        state: WorkerState {
            service_name: WORKER_SERVICE_NAME,
            readiness: parts.readiness,
            metrics: parts.metrics,
        },
        refresh: parts.refresh,
        retention: parts.retention,
        sweep_interval_seconds: config.sweep_interval_seconds.max(1),
    })
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
    market_data_http::server::serve(listener, state).await?;
    tracing::info!(service = SERVICE_NAME, "service stopped");
    Ok(())
}

/// Runs the refresh worker's supervisor and probes until shutdown.
///
/// # Errors
/// Returns an error when the runtime cannot be built or the listener cannot bind.
pub async fn run_worker(config: WorkerConfig) -> anyhow::Result<()> {
    init_tracing();
    let runtime = build_worker_runtime(&config).await?;
    let listener = TcpListener::bind(("0.0.0.0", config.port)).await?;
    tracing::info!(
        service = WORKER_SERVICE_NAME,
        port = config.port,
        "worker listening"
    );
    let supervisor = tokio::spawn(supervisor_loop(
        runtime.refresh,
        runtime.retention,
        runtime.sweep_interval_seconds,
    ));
    market_data_http::server::serve_worker(listener, runtime.state).await?;
    supervisor.abort();
    tracing::info!(service = WORKER_SERVICE_NAME, "worker stopped");
    Ok(())
}

async fn supervisor_loop(
    refresh: Arc<RefreshService>,
    retention: Arc<RetentionService>,
    sweep_interval_seconds: u64,
) {
    loop {
        match refresh.sweep().await {
            Ok(swept) if swept > 0 => {
                tracing::warn!(swept, "expired refresh leases were failed")
            }
            Ok(_) => {}
            Err(error) => tracing::error!(%error, "the lease sweep failed"),
        }
        match retention.sweep().await {
            Ok(report) => {
                if report.observations > 0 || report.fx_rates > 0 {
                    tracing::info!(
                        observations = report.observations,
                        fx_rates = report.fx_rates,
                        "retention pruned history"
                    );
                }
            }
            Err(error) => tracing::error!(%error, "the retention sweep failed"),
        }
        tokio::time::sleep(std::time::Duration::from_secs(sweep_interval_seconds)).await;
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .json()
        .with_env_filter(filter)
        .try_init();
}
