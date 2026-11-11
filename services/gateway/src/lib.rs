//! API Gateway service package: the composition root.

pub mod config;

use std::sync::Arc;

use gateway_application::authorize::AuthorizeAndRoute;
use gateway_application::ports::Clock;
use gateway_domain::RouteTable;
use gateway_http::metrics::Metrics;
use gateway_http::rate_limit::RateLimiter;
use gateway_http::AppState;
use gateway_infrastructure::audit::TracingAuditSink;
use gateway_infrastructure::auth::{HttpJwksSource, JwksValidator};
use gateway_infrastructure::clock::SystemClock;
use gateway_infrastructure::readiness::HttpReadinessProbe;
use gateway_infrastructure::upstream::ReqwestUpstreamClient;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

pub use config::Config;

/// Stable service identifier used in logs, metrics, and health responses.
pub const SERVICE_NAME: &str = "gateway";

/// Returns the service identifier.
#[must_use]
pub fn service_name() -> &'static str {
    SERVICE_NAME
}

/// Builds the application state from configuration.
///
/// # Errors
/// This adapter is infallible; the signature keeps the composition root uniform
/// with future adapters that can fail at startup.
pub fn build_state(config: &Config) -> anyhow::Result<AppState> {
    let validator = Arc::new(JwksValidator::new(
        Arc::new(HttpJwksSource::new(config.jwks_url.clone())),
        config.jwt_issuer.clone(),
        config.jwt_audience.clone(),
    ));
    let mut services = std::collections::BTreeMap::new();
    services.insert(
        gateway_domain::ServiceName::registry().as_str().to_string(),
        config.registry_url.clone(),
    );
    services.insert(
        gateway_domain::ServiceName::identity().as_str().to_string(),
        config.identity_url.clone(),
    );
    let upstream = Arc::new(ReqwestUpstreamClient::with_services(
        services,
        config.upstream_timeout_ms,
    ));
    let audit = Arc::new(TracingAuditSink);
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);

    let use_case = Arc::new(AuthorizeAndRoute::new(
        validator,
        upstream,
        audit,
        clock,
        RouteTable::default_registry_routes(),
    ));

    let readiness = Arc::new(HttpReadinessProbe::new(format!(
        "{}/readyz",
        config.registry_url.trim_end_matches('/')
    )));

    Ok(AppState {
        service_name: SERVICE_NAME,
        use_case,
        readiness,
        metrics: Arc::new(Metrics::default()),
        rate_limiter: Arc::new(RateLimiter::new(
            config.rate_limit_per_minute,
            config.anonymous_rate_limit_per_minute,
        )),
    })
}

/// Runs the service until shutdown.
///
/// # Errors
/// Returns an error when the listener cannot bind.
pub async fn run(config: Config) -> anyhow::Result<()> {
    init_tracing();
    let state = build_state(&config)?;
    let listener = TcpListener::bind(("0.0.0.0", config.port)).await?;
    tracing::info!(
        service = SERVICE_NAME,
        port = config.port,
        "service listening"
    );
    gateway_http::server::serve(listener, state).await?;
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
