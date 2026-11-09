//! Notification service package: the composition root for both binaries.
//!
//! It wires infrastructure adapters into application use cases. The four Clean
//! Architecture crates never import each other outward.

pub mod config;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use notification_application::ports::{
    AlertRepository, AttemptRepository, AuditClient, Clock, ContactDirectory, DeliveryControl,
    EmailSender, IdGenerator, Pause, PreferenceRepository, ReadinessProbe, ServiceTokenRegistry,
    TokenValidator,
};
use notification_application::{
    DeliveryService, EraseService, FeedService, PreferenceService, RetentionService, SubmitAlert,
    Trail,
};
use notification_http::metrics::Metrics;
use notification_http::AppState;
use notification_infrastructure::auth::{HttpJwksSource, JwksValidator};
use notification_infrastructure::clients::{
    HttpAuditClient, HttpContactDirectory, HttpEmailSender,
};
use notification_infrastructure::{
    connect, NotificationReadinessProbe, PgAlertRepository, PgAttemptRepository, PgDeliveryControl,
    PgPreferenceRepository, PgServiceTokenRegistry, SystemClock, TokioPause, UuidGenerator,
};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

pub use config::{ApiConfig, MigrateConfig, WorkerConfig};

/// The API service identifier.
pub const SERVICE_NAME: &str = "notification-api";
/// The worker service identifier.
pub const WORKER_SERVICE_NAME: &str = "notification-worker";

struct Components {
    submit: Arc<SubmitAlert>,
    preferences: Arc<PreferenceService>,
    feed: Arc<FeedService>,
    delivery: Arc<DeliveryService>,
    retention: Arc<RetentionService>,
    erase: Arc<EraseService>,
    trail: Arc<Trail>,
    service_tokens: Arc<dyn ServiceTokenRegistry>,
    clock: Arc<dyn Clock>,
    readiness: Arc<dyn ReadinessProbe>,
    metrics: Arc<Metrics>,
}

#[allow(clippy::too_many_arguments)]
async fn components(
    database_url: &str,
    pool_connections: u32,
    identity_url: &str,
    audit_url: &str,
    outbound_token: &str,
    email_provider_url: &str,
    email_key: &str,
    email_from: &str,
    smoke_token_path: Option<&Path>,
) -> anyhow::Result<Components> {
    let pool = connect(database_url, pool_connections).await?;

    let alerts: Arc<dyn AlertRepository> = Arc::new(PgAlertRepository::new(pool.clone()));
    let preferences: Arc<dyn PreferenceRepository> =
        Arc::new(PgPreferenceRepository::new(pool.clone()));
    let attempts: Arc<dyn AttemptRepository> = Arc::new(PgAttemptRepository::new(pool.clone()));
    let control: Arc<dyn DeliveryControl> = Arc::new(PgDeliveryControl::new(pool.clone()));

    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let ids: Arc<dyn IdGenerator> = Arc::new(UuidGenerator);
    let pause: Arc<dyn Pause> = Arc::new(TokioPause);
    let contacts: Arc<dyn ContactDirectory> = Arc::new(HttpContactDirectory::new(
        identity_url,
        outbound_token.to_string(),
    ));
    let email: Arc<dyn EmailSender> = Arc::new(HttpEmailSender::new(
        email_provider_url,
        email_key.to_string(),
        email_from,
    ));
    let audit: Arc<dyn AuditClient> =
        Arc::new(HttpAuditClient::new(audit_url, outbound_token.to_string()));
    let trail = Arc::new(Trail::new(audit, clock.clone(), pause.clone()));

    let submit = Arc::new(SubmitAlert::new(
        alerts.clone(),
        control.clone(),
        ids.clone(),
        clock.clone(),
    ));
    let preferences_service = Arc::new(PreferenceService::new(
        preferences.clone(),
        ids.clone(),
        clock.clone(),
        trail.clone(),
    ));
    let feed = Arc::new(FeedService::new(alerts.clone()));
    let delivery = Arc::new(DeliveryService::new(
        alerts.clone(),
        preferences.clone(),
        attempts.clone(),
        control,
        contacts,
        email,
        trail.clone(),
        clock.clone(),
        ids.clone(),
    ));
    let retention = Arc::new(RetentionService::new(
        alerts.clone(),
        attempts,
        clock.clone(),
        pause,
    ));
    let erase = Arc::new(EraseService::new(alerts, ids, trail.clone()));

    let mut registry = PgServiceTokenRegistry::new(pool.clone());
    if let Some(path) = smoke_token_path {
        let token = std::fs::read_to_string(path)
            .map_err(|error| anyhow::anyhow!("{} is unreadable: {error}", path.display()))?;
        registry = registry.with_extra_token(token.trim(), "smoke-reader", &["smoke"]);
    }
    let service_tokens: Arc<dyn ServiceTokenRegistry> = Arc::new(registry);
    let readiness = Arc::new(NotificationReadinessProbe::new(pool));

    Ok(Components {
        submit,
        preferences: preferences_service,
        feed,
        delivery,
        retention,
        erase,
        trail,
        service_tokens,
        clock,
        readiness,
        metrics: Arc::new(Metrics::default()),
    })
}

/// Builds the API state from configuration.
///
/// # Errors
/// Returns an error when the database is unreachable or a secret file is missing.
pub async fn build_state(config: &ApiConfig) -> anyhow::Result<AppState> {
    let components = components(
        &config.database_url,
        config.pool_connections,
        &config.identity_url,
        &config.audit_url,
        &config.outbound_token,
        &config.email_provider_url,
        &config.email_key,
        &config.email_from,
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
        submit: components.submit,
        preferences: components.preferences,
        feed: components.feed,
        delivery: components.delivery,
        retention: components.retention,
        erase: components.erase,
        trail: components.trail,
        service_tokens: components.service_tokens,
        token_validator,
        readiness: components.readiness,
        clock: components.clock,
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
    ) -> Result<notification_application::Principal, notification_application::AuthError> {
        Err(notification_application::AuthError::Missing)
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
        &config.email_provider_url,
        &config.email_key,
        &config.email_from,
        config.smoke_token_path.as_deref(),
    )
    .await?;

    Ok(AppState {
        service_name: WORKER_SERVICE_NAME,
        submit: components.submit,
        preferences: components.preferences,
        feed: components.feed,
        delivery: components.delivery,
        retention: components.retention,
        erase: components.erase,
        trail: components.trail,
        service_tokens: components.service_tokens,
        token_validator: Arc::new(WorkerTokenValidator),
        readiness: components.readiness,
        clock: components.clock,
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
    notification_http::server::serve(listener, notification_http::api_router(state)).await?;
    tracing::info!(service = SERVICE_NAME, "service stopped");
    Ok(())
}

/// Runs the worker until shutdown. It polls the durable wake flag and the queue
/// every five seconds (NFR2.5) and serves the C11 run paths.
///
/// # Errors
/// Returns an error when state cannot be built or the listener cannot bind.
pub async fn run_worker(config: WorkerConfig) -> anyhow::Result<()> {
    init_tracing();
    let state = build_worker_state(&config).await?;
    spawn_delivery_poll(state.clone());
    let listener = TcpListener::bind(("0.0.0.0", config.port)).await?;
    tracing::info!(
        service = WORKER_SERVICE_NAME,
        port = config.port,
        "worker listening"
    );
    notification_http::server::serve(listener, notification_http::worker_router(state)).await?;
    tracing::info!(service = WORKER_SERVICE_NAME, "worker stopped");
    Ok(())
}

/// The five-second poll. It runs only when the wake flag is set or the queue has
/// due work, so an idle worker performs one cheap read per tick.
fn spawn_delivery_poll(state: AppState) {
    tokio::spawn(async move {
        let mut ticker =
            tokio::time::interval(Duration::from_secs(notification_domain::WAKE_POLL_SECONDS));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            match state.delivery.poll().await {
                Ok(Some(report)) => {
                    // Meter normal operation, not only the C11 path.
                    notification_http::metrics::record_delivery_report(
                        state.metrics.as_ref(),
                        &report,
                    );
                    tracing::info!(
                        processed = report.processed,
                        delivered = report.delivered,
                        failed = report.failed,
                        deferred = report.deferred,
                        "delivery run finished"
                    );
                }
                Ok(None) => {}
                Err(error) => tracing::warn!(error = %error, "delivery poll failed"),
            }
        }
    });
}

/// Applies the migrations and exits.
///
/// # Errors
/// Returns an error when the database is unreachable or a migration fails.
pub async fn run_migrate(config: MigrateConfig) -> anyhow::Result<()> {
    init_tracing();
    let pool = connect(&config.database_url, 2).await?;
    notification_infrastructure::migrations::run(&pool).await?;
    tracing::info!("notification migrations applied");
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .json()
        .with_env_filter(filter)
        .try_init();
}
