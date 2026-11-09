//! Ledger service package: the composition root for `ledger-api`,
//! `ledger-worker`, and `ledger-migrate`.
//!
//! It wires infrastructure adapters into application use cases. The four Clean
//! Architecture crates never import each other outward.

pub mod config;

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use ledger_application::ports::{
    AccountClient, AnonymisationRepository, AnonymiserGateway, AssetResolver, AuditClient, Clock,
    CorrectionRepository, IdGenerator, LotRepository, MarketDataClient, Pause, ReadinessProbe,
    ReconcileLease, ServiceTokenRegistry, SnapshotRepository, SubjectRepository, TokenValidator,
    TransactionRepository, TransferPairRepository,
};
use ledger_application::{
    CorrectionsService, IngestService, PnlService, PrivacyService, ReadService, Rebuilder,
    ReconcileService, TradesService, Trail,
};
use ledger_http::{api_router, worker_router, AppState, Metrics};
use ledger_infrastructure::auth::{HttpJwksSource, JwksValidator};
use ledger_infrastructure::clients::{
    HttpAccountClient, HttpAssetResolver, HttpAuditClient, HttpMarketDataClient,
};
use ledger_infrastructure::clock::SystemClock;
use ledger_infrastructure::connect;
use ledger_infrastructure::ids::UuidGenerator;
use ledger_infrastructure::pause::TokioPause;
use ledger_infrastructure::readiness::PgReadinessProbe;
use ledger_infrastructure::repositories::{
    PgAnonymisationRepository, PgCorrectionRepository, PgLotRepository, PgReconcileLease,
    PgSnapshotRepository, PgSubjectRepository, PgTransactionRepository, PgTransferPairRepository,
};
use ledger_infrastructure::service_token::PgServiceTokenRegistry;
use ledger_infrastructure::PgAnonymiserGateway;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

pub use config::{ApiConfig, MigrateConfig, WorkerConfig};

/// The API service identifier.
pub const SERVICE_NAME: &str = "ledger-api";
/// The worker service identifier.
pub const WORKER_SERVICE_NAME: &str = "ledger-worker";

struct Components {
    ingest: Arc<IngestService>,
    reads: Arc<ReadService>,
    pnl: Arc<PnlService>,
    corrections: Arc<CorrectionsService>,
    trades: Arc<TradesService>,
    privacy: Arc<PrivacyService>,
    reconcile: Arc<ReconcileService>,
    service_tokens: Arc<dyn ServiceTokenRegistry>,
    readiness: Arc<dyn ReadinessProbe>,
    metrics: Arc<Metrics>,
}

#[allow(clippy::too_many_arguments)]
async fn components(
    database_url: &str,
    anonymiser_database_url: &str,
    pool_connections: u32,
    account_url: &str,
    asset_registry_url: &str,
    market_data_url: &str,
    base_currency: &str,
    audit_url: &str,
    outbound_token: &str,
    smoke_token_path: Option<&Path>,
) -> anyhow::Result<Components> {
    let pool = connect(database_url, pool_connections).await?;

    let transactions: Arc<dyn TransactionRepository> =
        Arc::new(PgTransactionRepository::new(pool.clone()));
    let corrections: Arc<dyn CorrectionRepository> =
        Arc::new(PgCorrectionRepository::new(pool.clone()));
    let subjects: Arc<dyn SubjectRepository> = Arc::new(PgSubjectRepository::new(pool.clone()));
    let lots: Arc<dyn LotRepository> = Arc::new(PgLotRepository::new(pool.clone()));
    let snapshots: Arc<dyn SnapshotRepository> = Arc::new(PgSnapshotRepository::new(pool.clone()));
    let pairs: Arc<dyn TransferPairRepository> =
        Arc::new(PgTransferPairRepository::new(pool.clone()));
    let anonymisations: Arc<dyn AnonymisationRepository> =
        Arc::new(PgAnonymisationRepository::new(pool.clone()));
    let lease: Arc<dyn ReconcileLease> = Arc::new(PgReconcileLease::new(pool.clone()));

    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let ids: Arc<dyn IdGenerator> = Arc::new(UuidGenerator);
    let pause: Arc<dyn Pause> = Arc::new(TokioPause::new());

    let audit_client: Arc<dyn AuditClient> =
        Arc::new(HttpAuditClient::new(audit_url, outbound_token));
    let trail = Arc::new(Trail::new(audit_client, clock.clone(), pause));
    let account: Arc<dyn AccountClient> =
        Arc::new(HttpAccountClient::new(account_url, outbound_token));
    let assets: Arc<dyn AssetResolver> =
        Arc::new(HttpAssetResolver::new(asset_registry_url, outbound_token));
    let trade_assets: Arc<dyn AssetResolver> =
        Arc::new(HttpAssetResolver::new(asset_registry_url, outbound_token));
    let market: Arc<dyn MarketDataClient> =
        Arc::new(HttpMarketDataClient::new(market_data_url, outbound_token));

    let ingest = Arc::new(IngestService::new(
        transactions.clone(),
        subjects.clone(),
        pairs.clone(),
        account.clone(),
        assets.clone(),
        clock.clone(),
        ids.clone(),
        base_currency,
    ));
    let rebuilder = Arc::new(Rebuilder::new(
        transactions.clone(),
        corrections.clone(),
        lots.clone(),
        snapshots.clone(),
        pairs.clone(),
        clock.clone(),
    ));
    let reads = Arc::new(ReadService::new(transactions.clone(), subjects.clone()));
    let pnl = Arc::new(PnlService::new(
        subjects.clone(),
        rebuilder.clone(),
        account,
        market,
        clock.clone(),
        ids.clone(),
        base_currency,
    ));
    let corrections_service = Arc::new(CorrectionsService::new(
        transactions.clone(),
        corrections.clone(),
        ingest.clone(),
        assets,
        pnl.clone(),
        trail.clone(),
        clock.clone(),
        ids.clone(),
    ));
    let trades = Arc::new(TradesService::new(
        transactions.clone(),
        ingest.clone(),
        trade_assets,
        rebuilder.clone(),
        pnl.clone(),
        trail.clone(),
        clock.clone(),
        ids.clone(),
        base_currency,
    ));
    let anonymiser: Arc<dyn AnonymiserGateway> =
        Arc::new(PgAnonymiserGateway::new(anonymiser_database_url));
    let privacy = Arc::new(PrivacyService::new(
        transactions.clone(),
        corrections,
        subjects.clone(),
        anonymiser,
        anonymisations,
        trail,
        clock.clone(),
        ids.clone(),
    ));
    let reconcile = Arc::new(ReconcileService::new(
        transactions,
        subjects,
        lease,
        rebuilder,
        pnl.clone(),
        clock,
    ));

    let mut registry = PgServiceTokenRegistry::new(pool.clone());
    if let Some(path) = smoke_token_path {
        let token = std::fs::read_to_string(path)
            .map_err(|error| anyhow::anyhow!("{} is unreadable: {error}", path.display()))?;
        registry = registry.with_extra_token(token.trim(), "smoke-reader", &["smoke"]);
    }

    Ok(Components {
        ingest,
        reads,
        pnl,
        corrections: corrections_service,
        trades,
        privacy,
        reconcile,
        service_tokens: Arc::new(registry),
        readiness: Arc::new(PgReadinessProbe::new(pool)),
        metrics: Arc::new(Metrics::default()),
    })
}

/// Builds the API state from configuration.
///
/// # Errors
/// Returns an error when the database is unreachable or any secret file is
/// missing.
pub async fn build_state(config: &ApiConfig) -> anyhow::Result<AppState> {
    let components = components(
        &config.database_url,
        &config.anonymiser_database_url,
        config.pool_connections,
        &config.account_url,
        &config.asset_registry_url,
        &config.market_data_url,
        &config.base_currency,
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
        reads: components.reads,
        pnl: components.pnl,
        corrections: components.corrections,
        trades: components.trades,
        privacy: components.privacy,
        reconcile: components.reconcile,
        service_tokens: components.service_tokens,
        token_validator,
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
    ) -> Result<ledger_application::Principal, ledger_application::AuthError> {
        Err(ledger_application::AuthError::Missing)
    }
}

/// Builds the worker state from configuration.
///
/// # Errors
/// Returns an error when the database is unreachable or any secret file is
/// missing.
pub async fn build_worker_state(config: &WorkerConfig) -> anyhow::Result<AppState> {
    let components = components(
        &config.database_url,
        &config.anonymiser_database_url,
        config.pool_connections,
        &config.account_url,
        &config.asset_registry_url,
        &config.market_data_url,
        &config.base_currency,
        &config.audit_url,
        &config.outbound_token,
        config.smoke_token_path.as_deref(),
    )
    .await?;

    Ok(AppState {
        service_name: WORKER_SERVICE_NAME,
        ingest: components.ingest,
        reads: components.reads,
        pnl: components.pnl,
        corrections: components.corrections,
        trades: components.trades,
        privacy: components.privacy,
        reconcile: components.reconcile,
        service_tokens: components.service_tokens,
        token_validator: Arc::new(WorkerTokenValidator),
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
    ledger_http::server::serve(listener, api_router(state)).await?;
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
    ledger_http::server::serve(listener, worker_router(state)).await?;
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
    ledger_infrastructure::migrations::run(&pool).await?;
    tracing::info!("ledger migrations applied");
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .json()
        .with_env_filter(filter)
        .try_init();
}
