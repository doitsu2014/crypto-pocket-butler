//! Account service package: the composition root for `account-api` and
//! `account-worker`.
//!
//! It wires infrastructure adapters into application use cases. The four Clean
//! Architecture crates never import each other outward.

pub mod config;

use std::sync::Arc;
use std::time::Duration;

use account_application::accounts::AccountService;
use account_application::holdings::HoldingService;
use account_application::imports::ImportService;
use account_application::ports::{
    AuditClient, AuthAdmin, ChainProvider, Clock, ExchangeProvider, NotificationClient,
    ReadinessProbe, ServiceTokenRegistry, TokenValidator, VaultClient, WakeTransport,
};
use account_application::privacy::PrivacyService;
use account_application::sync::SyncService;
use account_application::trail::Trail;
use account_application::verification::VerificationService;
use account_http::metrics::Metrics;
use account_http::state::AppState;
use account_http::worker::WorkerState;
use account_infrastructure::audit_client::HttpAuditClient;
use account_infrastructure::auth::{HttpJwksSource, JwksValidator};
use account_infrastructure::auth_admin::HttpAuthAdmin;
use account_infrastructure::chain::HttpChainProvider;
use account_infrastructure::clock::SystemClock;
use account_infrastructure::connect;
use account_infrastructure::exchange::HttpExchangeProvider;
use account_infrastructure::notification_client::HttpNotificationClient;
use account_infrastructure::readiness::PgReadinessProbe;
use account_infrastructure::registry_client::HttpGetAssetRegistry;
use account_infrastructure::repositories::{
    PgAccountRepository, PgCredentialRepository, PgHoldingRepository, PgImportRepository,
    PgPrivacyRepository, PgSyncRunRepository, PgWalletConnectionRepository,
};
use account_infrastructure::service_token::PgServiceTokenRegistry;
use account_infrastructure::vault::PgVaultClient;
use account_infrastructure::wake::HttpWake;
use anyhow::Context;
use sqlx::PgPool;
use tokio::net::TcpListener;
use tokio::sync::Notify;
use tracing_subscriber::EnvFilter;

pub use config::{ApiConfig, WorkerConfig};

/// The API service identifier.
pub const SERVICE_NAME: &str = "account-api";
/// The sync worker identifier.
pub const WORKER_SERVICE_NAME: &str = "account-worker";
/// The liveness marker the worker container health check reads.
pub const WORKER_HEARTBEAT_PATH: &str = "/tmp/account-worker.heartbeat";

/// Returns the API service identifier.
#[must_use]
pub fn service_name() -> &'static str {
    SERVICE_NAME
}

struct Repositories {
    accounts: Arc<PgAccountRepository>,
    credentials: Arc<PgCredentialRepository>,
    wallets: Arc<PgWalletConnectionRepository>,
    runs: Arc<PgSyncRunRepository>,
    holdings: Arc<PgHoldingRepository>,
    imports: Arc<PgImportRepository>,
    privacy: Arc<PgPrivacyRepository>,
}

fn repositories(pool: &PgPool) -> Repositories {
    Repositories {
        accounts: Arc::new(PgAccountRepository::new(pool.clone())),
        credentials: Arc::new(PgCredentialRepository::new(pool.clone())),
        wallets: Arc::new(PgWalletConnectionRepository::new(pool.clone())),
        runs: Arc::new(PgSyncRunRepository::new(pool.clone())),
        holdings: Arc::new(PgHoldingRepository::new(pool.clone())),
        imports: Arc::new(PgImportRepository::new(pool.clone())),
        privacy: Arc::new(PgPrivacyRepository::new(pool.clone())),
    }
}

struct Adapters {
    audit: Arc<dyn AuditClient>,
    notification: Arc<dyn NotificationClient>,
    exchange: Arc<dyn ExchangeProvider>,
    chain: Arc<dyn ChainProvider>,
    registry: Arc<dyn account_application::ports::AssetRegistry>,
    vault: Arc<dyn VaultClient>,
    auth_admin: Arc<dyn AuthAdmin>,
    clock: Arc<dyn Clock>,
}

#[allow(clippy::too_many_arguments)]
fn adapters(
    audit_url: &str,
    notification_url: &str,
    asset_registry_url: &str,
    supabase_auth_url: &str,
    service_role_key: &str,
    evm_rpc_url: &str,
    solana_rpc_url: &str,
    binance_base_url: &str,
    okx_base_url: &str,
    pool: &PgPool,
    outbound_token: &str,
) -> Adapters {
    let audit = Arc::new(HttpAuditClient::new(
        audit_url.to_string(),
        outbound_token.to_string(),
    ));
    let notification = Arc::new(HttpNotificationClient::new(
        notification_url.to_string(),
        outbound_token.to_string(),
    ));
    let exchange = Arc::new(HttpExchangeProvider::with_bases(
        binance_base_url.to_string(),
        okx_base_url.to_string(),
    ));
    let chain = Arc::new(HttpChainProvider::new(
        evm_rpc_url.to_string(),
        solana_rpc_url.to_string(),
    ));
    let registry = Arc::new(HttpGetAssetRegistry::new(
        asset_registry_url.to_string(),
        outbound_token.to_string(),
    ));
    let vault = Arc::new(PgVaultClient::new(pool.clone()));
    let auth_admin = Arc::new(HttpAuthAdmin::new(
        supabase_auth_url.to_string(),
        service_role_key.to_string(),
    ));
    Adapters {
        audit,
        notification,
        exchange,
        chain,
        registry,
        vault,
        auth_admin,
        clock: Arc::new(SystemClock),
    }
}

fn outbound_token(
    registry: &PgServiceTokenRegistry,
    token_service: &str,
) -> anyhow::Result<String> {
    registry.token_for(token_service).with_context(|| {
        format!("the service-token registry has no outbound token for '{token_service}'")
    })
}

/// Builds the API application state from configuration.
///
/// # Errors
/// Returns an error when the database is unreachable, a secret file is
/// unreadable, or the service-token registry has no outbound token.
pub async fn build_api_state(config: &ApiConfig) -> anyhow::Result<AppState> {
    let pool = connect(&config.database_url, config.pool_connections).await?;
    let repositories = repositories(&pool);
    let service_tokens = Arc::new(
        PgServiceTokenRegistry::from_file(pool.clone(), &config.service_tokens_path)
            .map_err(anyhow::Error::msg)?,
    );
    let outbound = outbound_token(&service_tokens, &config.token_service)?;
    let adapters = adapters(
        &config.audit_url,
        &config.notification_url,
        &config.asset_registry_url,
        &config.supabase_auth_url,
        &config.service_role_key,
        &config.evm_rpc_url,
        &config.solana_rpc_url,
        &config.binance_base_url,
        &config.okx_base_url,
        &pool,
        &outbound,
    );
    let trail = Arc::new(Trail::new(adapters.audit.clone(), adapters.clock.clone()));
    let wake: Arc<dyn WakeTransport> = Arc::new(HttpWake::new(config.worker_url.clone(), outbound));

    let accounts = Arc::new(AccountService::new(
        repositories.accounts.clone(),
        repositories.credentials.clone(),
        repositories.wallets.clone(),
        repositories.runs.clone(),
        adapters.exchange.clone(),
        adapters.registry.clone(),
        adapters.vault.clone(),
        adapters.auth_admin.clone(),
        wake,
        trail.clone(),
        adapters.clock.clone(),
    ));
    let sync = Arc::new(SyncService::new(
        repositories.accounts.clone(),
        repositories.credentials.clone(),
        repositories.wallets.clone(),
        repositories.runs.clone(),
        adapters.exchange.clone(),
        adapters.chain.clone(),
        adapters.registry.clone(),
        adapters.vault.clone(),
        adapters.notification.clone(),
        trail.clone(),
        adapters.clock.clone(),
    ));
    let imports = Arc::new(ImportService::new(
        repositories.accounts.clone(),
        repositories.imports.clone(),
        trail.clone(),
        adapters.clock.clone(),
    ));
    let holdings = Arc::new(HoldingService::new(
        repositories.accounts.clone(),
        repositories.holdings.clone(),
        repositories.imports.clone(),
    ));
    let privacy = Arc::new(PrivacyService::new(
        repositories.accounts.clone(),
        repositories.credentials.clone(),
        repositories.wallets.clone(),
        repositories.holdings.clone(),
        repositories.imports.clone(),
        repositories.privacy.clone(),
        adapters.vault.clone(),
        trail,
        adapters.clock,
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
        accounts,
        sync,
        imports,
        holdings,
        privacy,
        token_validator,
        service_tokens,
        readiness,
        metrics: Arc::new(Metrics::default()),
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
    account_http::server::serve(listener, state).await?;
    tracing::info!(service = SERVICE_NAME, "service stopped");
    Ok(())
}

/// Runs the sync worker until shutdown.
///
/// # Errors
/// Returns an error when the database is unreachable or the listener cannot bind.
pub async fn run_worker(config: WorkerConfig) -> anyhow::Result<()> {
    init_tracing();
    let pool = connect(&config.database_url, config.pool_connections).await?;
    let repositories = repositories(&pool);
    let service_tokens = Arc::new(
        PgServiceTokenRegistry::from_file(pool.clone(), &config.service_tokens_path)
            .map_err(anyhow::Error::msg)?,
    );
    let outbound = outbound_token(&service_tokens, &config.token_service)?;
    let adapters = adapters(
        &config.audit_url,
        &config.notification_url,
        &config.asset_registry_url,
        &config.supabase_auth_url,
        &config.service_role_key,
        &config.evm_rpc_url,
        &config.solana_rpc_url,
        &config.binance_base_url,
        &config.okx_base_url,
        &pool,
        &outbound,
    );
    let trail = Arc::new(Trail::new(adapters.audit.clone(), adapters.clock.clone()));

    let sync = Arc::new(SyncService::new(
        repositories.accounts.clone(),
        repositories.credentials.clone(),
        repositories.wallets.clone(),
        repositories.runs.clone(),
        adapters.exchange.clone(),
        adapters.chain,
        adapters.registry,
        adapters.vault.clone(),
        adapters.notification.clone(),
        trail.clone(),
        adapters.clock.clone(),
    ));
    let imports = Arc::new(ImportService::new(
        repositories.accounts,
        repositories.imports,
        trail.clone(),
        adapters.clock.clone(),
    ));
    let verification = Arc::new(VerificationService::new(
        repositories.credentials,
        Arc::new(PgAccountRepository::new(pool.clone())),
        adapters.exchange,
        adapters.vault,
        adapters.auth_admin,
        adapters.notification,
        trail,
        adapters.clock,
    ));

    let notifier = Arc::new(Notify::new());
    let readiness: Arc<dyn ReadinessProbe> = Arc::new(PgReadinessProbe::new(pool));
    let metrics = Arc::new(Metrics::default());
    let state = WorkerState {
        service_name: WORKER_SERVICE_NAME,
        notifier: notifier.clone(),
        service_tokens,
        readiness,
        metrics: metrics.clone(),
    };

    let listener = TcpListener::bind(("0.0.0.0", config.port)).await?;
    tracing::info!(
        service = WORKER_SERVICE_NAME,
        port = config.port,
        "worker listening"
    );
    let supervisor = tokio::spawn(supervisor_loop(
        sync,
        imports,
        verification,
        notifier,
        metrics,
        config.sync_interval_seconds,
        config.verify_interval_seconds,
    ));
    account_http::server::serve_worker(listener, state).await?;
    supervisor.abort();
    tracing::info!(service = WORKER_SERVICE_NAME, "worker stopped");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn supervisor_loop(
    sync: Arc<SyncService>,
    imports: Arc<ImportService>,
    verification: Arc<VerificationService>,
    notifier: Arc<Notify>,
    metrics: Arc<Metrics>,
    sync_interval_seconds: u64,
    verify_interval_seconds: u64,
) {
    let worker_id = format!("{WORKER_SERVICE_NAME}-{}", std::process::id());
    let verify_every = if sync_interval_seconds == 0 {
        1
    } else {
        (verify_interval_seconds / sync_interval_seconds).max(1)
    };
    let mut ticks: u64 = 0;
    loop {
        let correlation = format!("{worker_id}-{ticks}");
        if let Ok(swept) = sync.sweep().await {
            metrics.record_lease_expiry(swept);
        }
        match sync.process_pending(&correlation).await {
            Ok(report) => {
                for _ in 0..report.processed.saturating_sub(report.failed) {
                    metrics.record_sync(true);
                }
                for _ in 0..report.failed {
                    metrics.record_sync(false);
                }
            }
            Err(error) => tracing::error!(%error, "the sync sweep failed"),
        }
        match imports.process_pending(&correlation).await {
            Ok(Some(summary)) => {
                metrics.record_import(summary.imported, summary.skipped, summary.failed);
            }
            Ok(None) => {}
            Err(error) => tracing::error!(%error, "the import sweep failed"),
        }
        if ticks % verify_every == 0 {
            match verification.reverify_all(&correlation).await {
                Ok(report) => tracing::info!(
                    verified = report.verified,
                    rejected = report.rejected,
                    stale = report.stale,
                    "credential re-verification finished"
                ),
                Err(error) => tracing::error!(%error, "the credential re-verification failed"),
            }
        }
        ticks = ticks.wrapping_add(1);
        write_heartbeat();
        tokio::select! {
            () = tokio::time::sleep(Duration::from_secs(sync_interval_seconds.max(1))) => {}
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
