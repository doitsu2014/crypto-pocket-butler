//! Asset Registry service package: the composition root.
//!
//! It wires infrastructure adapters into application use cases and hands the
//! result to the HTTP boundary. The four Clean Architecture crates never import
//! each other outward.

pub mod config;

use std::sync::Arc;

use asset_registry_application::admin::RegistryAdmin;
use asset_registry_application::ports::Clock;
use asset_registry_application::resolve_asset::ResolveAsset;
use asset_registry_application::resolve_price::ResolvePrice;
use asset_registry_http::metrics::Metrics;
use asset_registry_http::AppState;
use asset_registry_infrastructure::audit::TracingAuditSink;
use asset_registry_infrastructure::clock::SystemClock;
use asset_registry_infrastructure::price::NoPriceProvider;
use asset_registry_infrastructure::readiness::PgReadinessProbe;
use asset_registry_infrastructure::repositories::{
    PgAssetRepository, PgChainRepository, PgMetadataRepository, PgObservationRepository,
    PgPriceSourceRepository,
};
use asset_registry_infrastructure::{
    auth::HttpJwksSource, auth::JwksValidator, connect, migrations,
};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

pub use config::Config;

/// Stable service identifier used in logs, metrics, and health responses.
pub const SERVICE_NAME: &str = "asset-registry";

/// Returns the service identifier.
#[must_use]
pub fn service_name() -> &'static str {
    SERVICE_NAME
}

/// Builds the application state from configuration.
///
/// # Errors
/// Returns an error when the database is unreachable, a migration fails, or the
/// listener configuration is invalid.
pub async fn build_state(config: &Config) -> anyhow::Result<AppState> {
    let pool = connect(&config.database_url).await?;
    migrations::run(&pool).await?;

    let assets = Arc::new(PgAssetRepository::new(pool.clone()));
    let chains = Arc::new(PgChainRepository::new(pool.clone()));
    let sources = Arc::new(PgPriceSourceRepository::new(pool.clone()));
    let observations = Arc::new(PgObservationRepository::new(pool.clone()));
    let metadata = Arc::new(PgMetadataRepository::new(pool.clone()));
    let readiness = Arc::new(PgReadinessProbe::new(pool));

    let audit = Arc::new(TracingAuditSink);
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);

    let resolve_asset = Arc::new(ResolveAsset::new(
        assets.clone(),
        chains.clone(),
        observations.clone(),
        clock.clone(),
    ));
    let resolve_price = Arc::new(ResolvePrice::new(
        sources.clone(),
        Arc::new(NoPriceProvider),
        clock.clone(),
    ));
    let admin = Arc::new(RegistryAdmin::new(
        assets.clone(),
        chains,
        sources,
        observations,
        metadata,
        audit.clone(),
        clock.clone(),
    ));

    let jwks_source = Arc::new(HttpJwksSource::new(config.jwks_url.clone()));
    let token_validator = Arc::new(JwksValidator::new(
        jwks_source,
        config.jwt_issuer.clone(),
        config.jwt_audience.clone(),
    ));

    Ok(AppState {
        service_name: SERVICE_NAME,
        resolve_asset,
        resolve_price,
        admin,
        assets,
        token_validator,
        audit,
        clock,
        readiness,
        metrics: Arc::new(Metrics::default()),
    })
}

/// Runs the service until shutdown.
///
/// # Errors
/// Returns an error when state cannot be built or the listener cannot bind.
pub async fn run(config: Config) -> anyhow::Result<()> {
    init_tracing();
    let state = build_state(&config).await?;
    let listener = TcpListener::bind(("0.0.0.0", config.port)).await?;
    tracing::info!(
        service = SERVICE_NAME,
        port = config.port,
        "service listening"
    );
    asset_registry_http::server::serve(listener, state).await?;
    tracing::info!(service = SERVICE_NAME, "service stopped");
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .json()
        .with_env_filter(filter)
        .try_init();
}
