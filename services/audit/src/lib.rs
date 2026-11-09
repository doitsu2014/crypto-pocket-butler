//! Audit service package: the composition root for both binaries.
//!
//! It wires infrastructure adapters into application use cases. The four Clean
//! Architecture crates never import each other outward.

pub mod config;

use std::sync::Arc;
use std::time::Duration;

use audit_application::export::ExportAudit;
use audit_application::ports::{AuditEventRepository, Clock, ServiceTokenRegistry, TokenValidator};
use audit_application::privacy::PrivacyDeletions;
use audit_application::query::QueryEvents;
use audit_application::retention::{RetentionService, SweepConfig};
use audit_application::support::SupportSessions;
use audit_application::{AppendEvent, Trail};
use audit_http::metrics::Metrics;
use audit_http::rate_limit::{Limits, RateLimiter};
use audit_http::AppState;
use audit_infrastructure::auth::{HttpJwksSource, JwksValidator};
use audit_infrastructure::clock::SystemClock;
use audit_infrastructure::connect;
use audit_infrastructure::pause::TokioPause;
use audit_infrastructure::readiness::PgReadinessProbe;
use audit_infrastructure::repositories::{
    PgAuditEventRepository, PgExportRepository, PgPrivacyDeletionRepository, PgRetentionRepository,
    PgSupportSessionRepository,
};
use audit_infrastructure::service_token::FileServiceTokens;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

pub use config::{ApiConfig, RetentionConfig};

/// The API service identifier.
pub const SERVICE_NAME: &str = "audit-api";
/// The retention worker identifier.
pub const RETENTION_SERVICE_NAME: &str = "audit-retention";

/// The liveness marker the retention container health check reads.
pub const HEARTBEAT_PATH: &str = "/tmp/audit-retention.heartbeat";

/// Returns the API service identifier.
#[must_use]
pub fn service_name() -> &'static str {
    SERVICE_NAME
}

/// Builds the API application state from configuration.
///
/// # Errors
/// Returns an error when the database is unreachable, the service-token file is
/// unreadable, or the listener configuration is invalid.
pub async fn build_state(config: &ApiConfig) -> anyhow::Result<AppState> {
    let pool = connect(&config.database_url).await?;

    let events: Arc<dyn AuditEventRepository> = Arc::new(PgAuditEventRepository::new(pool.clone()));
    let support_sessions = Arc::new(PgSupportSessionRepository::new(pool.clone()));
    let exports = Arc::new(PgExportRepository::new(pool.clone()));
    let retention_repository = Arc::new(PgRetentionRepository::new(pool.clone()));
    let privacy_repository = Arc::new(PgPrivacyDeletionRepository::new(pool.clone()));

    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let trail = Trail::new(events.clone(), clock.clone());

    let append = Arc::new(AppendEvent::new(events.clone(), clock.clone()));
    let query = Arc::new(QueryEvents::new(events.clone(), trail.clone()));
    let export = Arc::new(ExportAudit::new(
        events.clone(),
        exports,
        clock.clone(),
        trail.clone(),
    ));
    let support = Arc::new(SupportSessions::new(
        support_sessions,
        clock.clone(),
        trail.clone(),
    ));
    let privacy = Arc::new(PrivacyDeletions::new(
        privacy_repository,
        clock.clone(),
        trail.clone(),
    ));
    let retention = Arc::new(RetentionService::new(
        retention_repository,
        clock.clone(),
        Arc::new(TokioPause::new(0)),
        trail.clone(),
        SweepConfig::default(),
    ));

    let service_tokens: Arc<dyn ServiceTokenRegistry> = Arc::new(
        FileServiceTokens::from_file(&config.service_tokens_path).map_err(anyhow::Error::msg)?,
    );

    let jwks_source = Arc::new(HttpJwksSource::new(config.jwks_url.clone()));
    let token_validator: Arc<dyn TokenValidator> = Arc::new(JwksValidator::new(
        jwks_source,
        config.jwt_issuer.clone(),
        config.jwt_audience.clone(),
    ));

    Ok(AppState {
        service_name: SERVICE_NAME,
        append,
        query,
        export,
        support,
        privacy,
        retention,
        trail: Arc::new(trail),
        service_tokens,
        token_validator,
        clock,
        readiness: Arc::new(PgReadinessProbe::new(pool)),
        metrics: Arc::new(Metrics::default()),
        limiter: Arc::new(RateLimiter::default()),
        limits: Limits {
            appends_per_minute: config.appends_per_minute,
            queries_per_minute: config.queries_per_minute,
            exports_per_hour: config.exports_per_hour,
        },
    })
}

/// Runs the API until shutdown.
///
/// # Errors
/// Returns an error when state cannot be built or the listener cannot bind.
pub async fn run(config: ApiConfig) -> anyhow::Result<()> {
    init_tracing();
    let state = build_state(&config).await?;
    match state.export.recover_stale().await {
        Ok(0) => {}
        Ok(recovered) => {
            tracing::warn!(recovered, "marked stale audit exports as failed on start");
        }
        Err(error) => tracing::error!(%error, "failed to recover stale audit exports"),
    }
    let listener = TcpListener::bind(("0.0.0.0", config.port)).await?;
    tracing::info!(
        service = SERVICE_NAME,
        port = config.port,
        "service listening"
    );
    audit_http::server::serve(listener, state).await?;
    tracing::info!(service = SERVICE_NAME, "service stopped");
    Ok(())
}

/// Runs the retention supervisor until shutdown.
///
/// # Errors
/// Returns an error when the database is unreachable.
pub async fn run_retention(config: RetentionConfig) -> anyhow::Result<()> {
    init_tracing();
    let pool = connect(&config.database_url).await?;
    let events: Arc<dyn AuditEventRepository> = Arc::new(PgAuditEventRepository::new(pool.clone()));
    let retention_repository = Arc::new(PgRetentionRepository::new(pool.clone()));
    let privacy_repository = Arc::new(PgPrivacyDeletionRepository::new(pool.clone()));
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let trail = Trail::new(events, clock.clone());

    let retention = RetentionService::new(
        retention_repository,
        clock.clone(),
        Arc::new(TokioPause::new(config.pause_millis)),
        trail.clone(),
        SweepConfig {
            batch_size: config.batch_size,
            max_batches: config.max_batches,
        },
    );
    let privacy = PrivacyDeletions::new(privacy_repository, clock, trail);

    tracing::info!(service = RETENTION_SERVICE_NAME, "retention worker started");
    loop {
        run_retention_tick(&retention, &privacy, &config).await;
        write_heartbeat();
        tokio::select! {
            () = tokio::time::sleep(Duration::from_secs(config.sweep_interval_seconds)) => {}
            () = shutdown_signal() => break,
        }
    }
    tracing::info!(service = RETENTION_SERVICE_NAME, "retention worker stopped");
    Ok(())
}

async fn run_retention_tick(
    retention: &RetentionService,
    privacy: &PrivacyDeletions,
    config: &RetentionConfig,
) {
    match retention
        .ensure_partitions(config.partition_horizon_months)
        .await
    {
        Ok(partitions) => {
            if !partitions.is_empty() {
                tracing::info!(count = partitions.len(), "audit partitions ensured");
            }
        }
        Err(error) => tracing::error!(%error, "failed to ensure audit partitions"),
    }

    match retention.sweep().await {
        Ok(run) => tracing::info!(
            run_id = %run.id(),
            deleted = run.deleted_count(),
            "audit retention sweep completed"
        ),
        Err(error) => tracing::error!(%error, "audit retention sweep failed"),
    }

    match privacy.execute_pending(config.deletion_batch).await {
        Ok(done) if !done.is_empty() => {
            tracing::info!(count = done.len(), "privacy deletions executed")
        }
        Ok(_) => {}
        Err(error) => tracing::error!(%error, "privacy deletion execution failed"),
    }
}

/// The worker has no inbound listener, so its liveness is a heartbeat file the
/// container health check reads. The file lives on the container's tmpfs and
/// holds only a timestamp, never a credential.
fn write_heartbeat() {
    let now = chrono::Utc::now().to_rfc3339();
    if let Err(error) = std::fs::write(HEARTBEAT_PATH, now) {
        tracing::warn!(%error, "failed to write the retention heartbeat");
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
