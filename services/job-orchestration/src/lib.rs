//! Job Orchestration service wiring: configuration, state and the tick supervisor.

pub mod config {
    use anyhow::{Context, Result};

    /// The service configuration.
    #[derive(Debug, Clone)]
    pub struct Config {
        /// HTTP port.
        pub port: u16,
        /// Database URL (secret).
        pub database_url: String,
        /// Outbound service token (secret).
        pub outbound_token: String,
        /// JWKS URL.
        pub jwks_url: String,
        /// Expected issuer.
        pub jwt_issuer: String,
        /// Expected audience.
        pub jwt_audience: String,
        /// Audit base URL.
        pub audit_url: String,
        /// Provider base URLs by service key.
        pub provider_urls: std::collections::BTreeMap<String, String>,
        /// Tick interval seconds.
        pub tick_seconds: u64,
        /// Pool size.
        pub pool_connections: u32,
    }

    impl Config {
        /// Reads the environment.
        ///
        /// # Errors
        /// Returns an error naming a missing required value.
        pub fn from_env() -> Result<Self> {
            let mut provider_urls = std::collections::BTreeMap::new();
            for (key, variable, fallback) in [
                ("account", "ACCOUNT_URL", "http://account-api:8091"),
                (
                    "market-data",
                    "MARKET_DATA_URL",
                    "http://market-data-api:8093",
                ),
                ("portfolio", "PORTFOLIO_URL", "http://portfolio-api:8101"),
                ("ledger", "LEDGER_URL", "http://ledger-api:8097"),
                ("analytics", "ANALYTICS_URL", "http://analytics-api:8095"),
                (
                    "notification",
                    "NOTIFICATION_URL",
                    "http://notification-api:8099",
                ),
                ("reporting", "REPORTING_URL", "http://reporting-api:8103"),
                ("identity", "IDENTITY_URL", "http://identity-api:8089"),
                ("audit", "AUDIT_URL", "http://audit-api:8084"),
            ] {
                provider_urls.insert(
                    key.to_string(),
                    std::env::var(variable).unwrap_or_else(|_| fallback.to_string()),
                );
            }
            Ok(Self {
                port: std::env::var("JOB_ORCHESTRATION_PORT")
                    .unwrap_or_else(|_| "8105".to_string())
                    .parse()
                    .context("JOB_ORCHESTRATION_PORT must be a port")?,
                database_url: secret("DATABASE_URL")?,
                outbound_token: secret("OUTBOUND_TOKEN")?,
                jwks_url: required("JWT_JWKS_URL")?,
                jwt_issuer: required("JWT_ISSUER")?,
                jwt_audience: required("JWT_AUDIENCE")?,
                audit_url: std::env::var("AUDIT_URL")
                    .unwrap_or_else(|_| "http://audit-api:8084".to_string()),
                provider_urls,
                tick_seconds: std::env::var("JOB_ORCHESTRATION_TICK_SECONDS")
                    .unwrap_or_else(|_| "60".to_string())
                    .parse()
                    .context("JOB_ORCHESTRATION_TICK_SECONDS must be seconds")?,
                pool_connections: std::env::var("JOB_ORCHESTRATION_POOL")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()
                    .context("JOB_ORCHESTRATION_POOL must be a number")?,
            })
        }
    }

    fn required(name: &str) -> Result<String> {
        std::env::var(name).with_context(|| format!("{name} must be set"))
    }

    fn secret(name: &str) -> Result<String> {
        let file_variable = format!("{name}_FILE");
        if let Ok(path) = std::env::var(&file_variable) {
            let value = std::fs::read_to_string(path)?;
            let value = value.trim().to_string();
            if value.is_empty() {
                anyhow::bail!("{file_variable} is empty");
            }
            return Ok(value);
        }
        required(name)
    }
}

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use job_orchestration_application::operator::OperatorService;
use job_orchestration_application::ports::{
    AuditClient, Clock, HeartbeatRepository, IdGenerator, LeaseRepository, ReadinessProbe,
    RunRepository, ServiceTokenRegistry, TargetClient, TokenValidator,
};
use job_orchestration_application::scheduler::Scheduler;
use job_orchestration_http::{AppState, Metrics};
use job_orchestration_infrastructure::{
    connect, HttpAuditClient, HttpTargetClient, JwksValidator, PgHeartbeatRepository,
    PgLeaseRepository, PgReadinessProbe, PgRunRepository, PgServiceTokenRegistry, SystemClock,
    UuidGenerator,
};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

pub use config::Config;

/// The service name.
pub const SERVICE_NAME: &str = "job-orchestration";

/// Runs the API and the tick supervisor until shutdown.
///
/// # Errors
/// Returns an error when configuration, the database or the listener fails.
pub async fn run(config: Config) -> anyhow::Result<()> {
    init_tracing();
    let pool = connect(&config.database_url, config.pool_connections).await?;
    let runs: Arc<dyn RunRepository> = Arc::new(PgRunRepository::new(pool.clone()));
    let lease: Arc<dyn LeaseRepository> = Arc::new(PgLeaseRepository::new(pool.clone()));
    let heartbeat: Arc<dyn HeartbeatRepository> =
        Arc::new(PgHeartbeatRepository::new(pool.clone()));
    let target: Arc<dyn TargetClient> = Arc::new(HttpTargetClient::new(
        config.provider_urls.clone(),
        config.outbound_token.clone(),
    ));
    let audit: Arc<dyn AuditClient> = Arc::new(HttpAuditClient::new(
        config.audit_url.clone(),
        config.outbound_token.clone(),
    ));
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let ids: Arc<dyn IdGenerator> = Arc::new(UuidGenerator);
    let scheduler = Arc::new(Scheduler::new(
        runs.clone(),
        lease,
        heartbeat.clone(),
        target,
        audit.clone(),
        clock.clone(),
        ids.clone(),
    ));
    let operator = Arc::new(OperatorService::new(
        runs.clone(),
        audit,
        clock.clone(),
        ids,
    ));
    let schedules = Arc::new(
        runs.definitions()
            .await
            .map_err(|error| anyhow::anyhow!(error))?
            .into_iter()
            .map(|definition| (definition.name, definition.schedule))
            .collect::<BTreeMap<_, _>>(),
    );
    let token_validator: Arc<dyn TokenValidator> = Arc::new(JwksValidator::new(
        config.jwks_url.clone(),
        config.jwt_issuer.clone(),
        config.jwt_audience.clone(),
    ));
    let service_tokens: Arc<dyn ServiceTokenRegistry> =
        Arc::new(PgServiceTokenRegistry::new(pool.clone()));
    let readiness: Arc<dyn ReadinessProbe> = Arc::new(PgReadinessProbe::new(pool));
    let metrics = Arc::new(Metrics::default());
    let state = AppState {
        service_name: SERVICE_NAME,
        operator,
        scheduler: scheduler.clone(),
        schedules,
        token_validator,
        service_tokens,
        heartbeat: heartbeat.clone(),
        readiness,
        metrics: metrics.clone(),
    };

    let owner = format!("job-orchestration-{}", std::process::id());
    let tick_metrics = metrics.clone();
    let tick_heartbeat = heartbeat.clone();
    let tick_clock = clock.clone();
    let supervisor = tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(Duration::from_secs(config.tick_seconds.clamp(30, 90)));
        loop {
            interval.tick().await;
            let started = Instant::now();
            match scheduler.tick(&owner).await {
                Ok(report) => tick_metrics.record_tick(
                    report.succeeded,
                    report.failed,
                    report.skipped,
                    report.abandoned,
                    started.elapsed().as_millis() as u64,
                ),
                Err(error) => tracing::error!(%error, "the tick failed"),
            }
            if let Ok(age) = tick_heartbeat.age_seconds(tick_clock.now()).await {
                tick_metrics.record_heartbeat_age(age);
            }
        }
    });

    let listener = TcpListener::bind(("0.0.0.0", config.port)).await?;
    tracing::info!(
        service = SERVICE_NAME,
        port = config.port,
        "service listening"
    );
    axum::serve(listener, job_orchestration_http::router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    supervisor.abort();
    Ok(())
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
