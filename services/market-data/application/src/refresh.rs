//! WF3: the two leased refresh jobs and their source calls (C11).
//!
//! At most one run per job is active at a time. Each run reads the enabled
//! sources from C4, calls them in priority order with bounded retries, a shared
//! rate budget, and a per-source circuit breaker, stores one accepted
//! observation per asset, and reconciles its counts (BR4.1–BR4.5).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration, Utc};
use market_data_domain::{
    AssetPriceTier, AuditOutcome, AuditRecord, ErrorCategory, FxRate, FxRateId, ObservationId,
    Operation, PriceObservation, RefreshJob, RefreshRun, RefreshRunId, RENEW_INTERVAL_SECONDS,
};

use crate::common::{AppError, ServiceCaller};
use crate::ports::{
    AssetRegistry, AuditClient, CanonicalAsset, ClaimOutcome, Clock, ExternalError, FxRepository,
    ObservationRepository, PriceProvider, PriceSource, ProviderQuote, RateLimiter,
    RefreshRunRepository, ServiceMetrics, Sleeper, TierRepository,
};

/// The number of consecutive transient failures that open a source's circuit.
pub const CIRCUIT_FAILURE_THRESHOLD: u32 = 3;
/// How long an open circuit skips a source.
pub const CIRCUIT_OPEN_SECONDS: i64 = 300;
/// The bounded retry count for one source call (BR4.2).
pub const MAX_PROVIDER_ATTEMPTS: u32 = 3;
/// How many assets one run keeps in flight. With one source call per asset at a
/// time this also bounds the calls in flight per source at four.
pub const ASSETS_IN_FLIGHT: usize = 4;

/// What one refresh request produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunOutcome {
    /// The run ran to a terminal state.
    Completed(RefreshRun),
    /// A run of this job already held the lease (manual path, 202).
    AlreadyRunning(RefreshRun),
}

impl RunOutcome {
    /// The run the outcome names.
    #[must_use]
    pub fn run(&self) -> &RefreshRun {
        match self {
            Self::Completed(run) | Self::AlreadyRunning(run) => run,
        }
    }
}

#[derive(Debug, Default)]
struct Execution {
    priced: u32,
    unpriced: u32,
    last_category: Option<ErrorCategory>,
    last_message: Option<String>,
    aborted: Option<(ErrorCategory, String)>,
}

/// What one asset produced inside a run.
#[derive(Debug, Clone, PartialEq, Eq)]
enum AssetResult {
    /// One accepted observation was stored.
    Priced,
    /// No source produced an acceptable reading.
    Unpriced {
        category: Option<ErrorCategory>,
        message: Option<String>,
    },
    /// The run cannot continue (a fatal dependency failure).
    Fatal {
        category: ErrorCategory,
        message: String,
    },
}

#[derive(Debug, Default, Clone, Copy)]
struct BreakerState {
    failures: u32,
    open_until: Option<DateTime<Utc>>,
}

/// Runs, sweeps, and finishes the two refresh jobs.
pub struct RefreshService {
    registry: Arc<dyn AssetRegistry>,
    observations: Arc<dyn ObservationRepository>,
    fx: Arc<dyn FxRepository>,
    runs: Arc<dyn RefreshRunRepository>,
    tiers: Arc<dyn TierRepository>,
    provider: Arc<dyn PriceProvider>,
    secrets: Arc<dyn crate::ports::ProviderSecrets>,
    limiter: Arc<dyn RateLimiter>,
    audit: Arc<dyn AuditClient>,
    clock: Arc<dyn Clock>,
    sleeper: Arc<dyn Sleeper>,
    metrics: Arc<dyn ServiceMetrics>,
    fx_base_currencies: Vec<String>,
    breakers: Mutex<HashMap<String, BreakerState>>,
}

impl RefreshService {
    /// Creates the use case.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        registry: Arc<dyn AssetRegistry>,
        observations: Arc<dyn ObservationRepository>,
        fx: Arc<dyn FxRepository>,
        runs: Arc<dyn RefreshRunRepository>,
        tiers: Arc<dyn TierRepository>,
        provider: Arc<dyn PriceProvider>,
        secrets: Arc<dyn crate::ports::ProviderSecrets>,
        limiter: Arc<dyn RateLimiter>,
        audit: Arc<dyn AuditClient>,
        clock: Arc<dyn Clock>,
        sleeper: Arc<dyn Sleeper>,
        metrics: Arc<dyn ServiceMetrics>,
        fx_base_currencies: Vec<String>,
    ) -> Self {
        Self {
            registry,
            observations,
            fx,
            runs,
            tiers,
            provider,
            secrets,
            limiter,
            audit,
            clock,
            sleeper,
            metrics,
            fx_base_currencies,
            breakers: Mutex::new(HashMap::new()),
        }
    }

    /// Claims and runs one job.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for a caller without the run operation,
    /// [`AppError::Conflict`] when a scheduled call meets a held lease, and a
    /// repository refusal when the claim cannot be made.
    pub async fn run(
        &self,
        caller: &ServiceCaller,
        job: RefreshJob,
        trigger: market_data_domain::Trigger,
        correlation_id: &str,
    ) -> Result<RunOutcome, AppError> {
        caller.require_operation(Operation::RunRefresh)?;
        let now = self.clock.now();
        let mut run = RefreshRun::claim(RefreshRunId::generate(), job, now);
        match self
            .runs
            .claim(&run)
            .await
            .map_err(AppError::from_repository)?
        {
            ClaimOutcome::Held(active) => match trigger {
                market_data_domain::Trigger::Scheduled => Err(AppError::held_run(
                    format!("another {} run holds the single-flight lease", job.as_str()),
                    active.id(),
                )),
                market_data_domain::Trigger::Manual => Ok(RunOutcome::AlreadyRunning(active)),
            },
            ClaimOutcome::Claimed => {
                let execution = match self.execute(job, &mut run).await {
                    Ok(execution) => execution,
                    Err(error) => {
                        // An early repository/domain refusal still records the run
                        // failed and writes the C10 event before the error surfaces.
                        let failure = Execution {
                            aborted: Some((ErrorCategory::Data, error.to_string())),
                            ..Execution::default()
                        };
                        if let Err(finish_error) = self
                            .finish_run(&mut run, failure, trigger, correlation_id)
                            .await
                        {
                            tracing::error!(
                                %finish_error,
                                "the failed run could not be recorded"
                            );
                        }
                        return Err(error);
                    }
                };
                self.finish_run(&mut run, execution, trigger, correlation_id)
                    .await?;
                Ok(RunOutcome::Completed(run))
            }
        }
    }

    /// Marks every expired run failed (the worker calls this every minute).
    ///
    /// # Errors
    /// Returns a repository refusal when the sweep cannot run.
    pub async fn sweep(&self) -> Result<usize, AppError> {
        let swept = self
            .runs
            .sweep_expired(self.clock.now())
            .await
            .map_err(AppError::from_repository)?;
        Ok(swept.len())
    }

    async fn execute(&self, job: RefreshJob, run: &mut RefreshRun) -> Result<Execution, AppError> {
        let mut execution = Execution::default();
        let assets = match self.registry.canonical_assets().await {
            Ok(assets) => assets,
            Err(error) => {
                execution.aborted = Some((ErrorCategory::Network, error.to_string()));
                return Ok(execution);
            }
        };
        if job == RefreshJob::Top100 {
            self.refresh_tiers(&assets).await?;
        }
        let sources = match self.registry.sources().await {
            Ok(sources) => sources,
            Err(error) => {
                execution.aborted = Some((ErrorCategory::Network, error.to_string()));
                return Ok(execution);
            }
        };
        let mut enabled: Vec<PriceSource> = sources
            .into_iter()
            .filter(|source| source.enabled)
            .collect();
        enabled.sort_by_key(|source| source.priority);

        let slice: Vec<&CanonicalAsset> = assets
            .iter()
            .filter(|asset| job.selects_rank(asset.rank))
            .collect();
        let deadline = run.started_at() + Duration::seconds(job.deadline_seconds());
        let mut last_renew = self.clock.now();
        let mut index = 0;

        while index < slice.len() {
            if self.clock.now() >= deadline {
                break;
            }
            let now = self.clock.now();
            if (now - last_renew).num_seconds() >= RENEW_INTERVAL_SECONDS {
                if !run.renew(now) {
                    execution.aborted =
                        Some((ErrorCategory::Data, "the run lease expired".to_string()));
                    break;
                }
                let renewed = self
                    .runs
                    .renew(run.id(), run.lease_until(), now)
                    .await
                    .map_err(AppError::from_repository)?;
                if !renewed {
                    execution.aborted = Some((
                        ErrorCategory::Data,
                        "the run lease was superseded".to_string(),
                    ));
                    break;
                }
                last_renew = now;
            }

            let end = (index + ASSETS_IN_FLIGHT).min(slice.len());
            let batch = &slice[index..end];
            let futures = batch
                .iter()
                .map(|asset| self.process_asset(asset, &enabled));
            let results = futures::future::join_all(futures).await;
            let mut fatal = None;
            for result in results {
                match result {
                    Ok(AssetResult::Priced) => execution.priced += 1,
                    Ok(AssetResult::Unpriced { category, message }) => {
                        execution.unpriced += 1;
                        if category.is_some() {
                            execution.last_category = category;
                        }
                        if message.is_some() {
                            execution.last_message = message;
                        }
                    }
                    Ok(AssetResult::Fatal { category, message }) => {
                        fatal = Some((category, message));
                    }
                    Err(error) => {
                        fatal = Some((ErrorCategory::Data, error.to_string()));
                    }
                }
            }
            if let Some(aborted) = fatal {
                execution.aborted = Some(aborted);
                break;
            }
            index = end;
        }

        if job == RefreshJob::Top100 && execution.aborted.is_none() {
            self.refresh_fx(&enabled).await;
        }
        Ok(execution)
    }

    /// Prices one asset by trying the enabled sources in priority order.
    async fn process_asset(
        &self,
        asset: &CanonicalAsset,
        sources: &[PriceSource],
    ) -> Result<AssetResult, AppError> {
        let mut last_category = None;
        let mut last_message = None;
        for source in sources {
            let now = self.clock.now();
            if !self.circuit_allows(&source.name, now) {
                continue;
            }
            let acquired = self
                .limiter
                .try_acquire(&source.name, source.rate_limit_per_minute, now)
                .await
                .map_err(AppError::from_repository)?;
            if !acquired {
                self.metrics
                    .record_source_error(&source.name, ErrorCategory::RateLimit);
                last_category = Some(ErrorCategory::RateLimit);
                continue;
            }
            let key = match self.secrets.provider_key(&source.name).await {
                Ok(key) => key,
                Err(ExternalError::Refused(message)) => {
                    // A refused secret function or grant is a fatal misconfiguration:
                    // the run stops, is recorded failed, and is audited.
                    return Ok(AssetResult::Fatal {
                        category: ErrorCategory::Network,
                        message,
                    });
                }
                Err(error) => {
                    tracing::warn!(%error, source = %source.name, "the provider key could not be read");
                    self.metrics
                        .record_source_error(&source.name, ErrorCategory::Network);
                    last_category = Some(ErrorCategory::Network);
                    last_message = Some(error.to_string());
                    continue;
                }
            };
            match self.call_provider(source, &asset.id, key.as_deref()).await {
                Ok(reading) => {
                    self.circuit_success(&source.name);
                    match self.store_reading(&asset.id, source, reading).await? {
                        true => return Ok(AssetResult::Priced),
                        false => {
                            self.metrics
                                .record_source_error(&source.name, ErrorCategory::Data);
                            last_category = Some(ErrorCategory::Data);
                            last_message = Some("reading rejected".to_string());
                        }
                    }
                }
                Err(error) => {
                    self.metrics
                        .record_source_error(&source.name, error.category);
                    last_category = Some(error.category);
                    last_message = Some(error.message);
                }
            }
        }
        Ok(AssetResult::Unpriced {
            category: last_category,
            message: last_message,
        })
    }

    async fn refresh_tiers(&self, assets: &[crate::ports::CanonicalAsset]) -> Result<(), AppError> {
        let now = self.clock.now();
        for asset in assets {
            let tier = AssetPriceTier::for_rank(&asset.id, asset.rank, now)
                .map_err(AppError::from_domain)?;
            self.tiers
                .upsert(&tier)
                .await
                .map_err(AppError::from_repository)?;
        }
        Ok(())
    }

    async fn store_reading(
        &self,
        asset_id: &str,
        source: &PriceSource,
        reading: ProviderQuote,
    ) -> Result<bool, AppError> {
        let received = self.clock.now();
        let observation = PriceObservation::evaluate(
            ObservationId::generate(),
            asset_id,
            &source.name,
            &reading.price,
            &reading.currency,
            reading.observed_at,
            received,
        )
        .map_err(AppError::from_domain)?;
        let accepted = observation.is_accepted();
        self.observations
            .insert(&observation)
            .await
            .map_err(AppError::from_repository)?;
        tracing::debug!(
            asset = asset_id,
            source = %source.name,
            accepted,
            "stored one price observation"
        );
        Ok(accepted)
    }

    async fn call_provider(
        &self,
        source: &PriceSource,
        asset_id: &str,
        api_key: Option<&str>,
    ) -> Result<ProviderQuote, crate::ports::ProviderError> {
        let mut last: Option<crate::ports::ProviderError> = None;
        for attempt in 0..MAX_PROVIDER_ATTEMPTS {
            match self.provider.quote(source, asset_id, api_key).await {
                Ok(reading) => return Ok(reading),
                Err(error) if error.retryable && attempt + 1 < MAX_PROVIDER_ATTEMPTS => {
                    // Each transient failure counts toward the circuit; three in a
                    // row open it for five minutes.
                    self.circuit_failure(&source.name, self.clock.now());
                    self.sleeper.sleep(backoff(attempt)).await;
                    last = Some(error);
                }
                Err(error) => {
                    if error.retryable {
                        self.circuit_failure(&source.name, self.clock.now());
                    }
                    return Err(error);
                }
            }
        }
        Err(last.unwrap_or_else(|| {
            crate::ports::ProviderError::permanent(
                ErrorCategory::Provider,
                "the provider did not answer",
            )
        }))
    }

    async fn refresh_fx(&self, sources: &[PriceSource]) {
        let now = self.clock.now();
        for base in self.refresh_pairs(sources) {
            for source in sources {
                if !self.circuit_allows(&source.name, now) {
                    continue;
                }
                let acquired = match self
                    .limiter
                    .try_acquire(&source.name, source.rate_limit_per_minute, now)
                    .await
                {
                    Ok(acquired) => acquired,
                    Err(error) => {
                        tracing::warn!(%error, "the FX rate bucket could not be read");
                        continue;
                    }
                };
                if !acquired {
                    self.metrics
                        .record_source_error(&source.name, ErrorCategory::RateLimit);
                    continue;
                }
                let key = match self.secrets.provider_key(&source.name).await {
                    Ok(key) => key,
                    Err(error) => {
                        tracing::warn!(%error, "the FX provider key could not be read");
                        continue;
                    }
                };
                match self
                    .provider
                    .fx_rate(source, &base, "USD", key.as_deref())
                    .await
                {
                    Ok(reading) => {
                        self.circuit_success(&source.name);
                        let received = self.clock.now();
                        let parsed = market_data_domain::parse_price(&reading.price);
                        let stored = parsed.ok().and_then(|rate| {
                            FxRate::record(
                                FxRateId::generate(),
                                &base,
                                "USD",
                                rate,
                                &source.name,
                                reading.observed_at,
                                received,
                            )
                            .ok()
                        });
                        match stored {
                            Some(rate) => {
                                if let Err(error) = self.fx.insert(&rate).await {
                                    tracing::warn!(%error, pair = %format!("{base}/USD"), "the FX reading could not be stored");
                                }
                                break;
                            }
                            None => {
                                tracing::warn!(source = %source.name, pair = %format!("{base}/USD"), "the FX reading was malformed");
                            }
                        }
                    }
                    Err(error) => {
                        self.circuit_failure(&source.name, now);
                        self.metrics
                            .record_source_error(&source.name, error.category);
                    }
                }
            }
        }
    }

    /// The USD legs to refresh: every enabled source's own quote currency plus
    /// every configured launch base. `USD → USD` is skipped; a USD-quoted source
    /// still lets the other pairs and the other sources' quotes be refreshed.
    fn refresh_pairs(&self, sources: &[PriceSource]) -> Vec<String> {
        let mut bases: BTreeSet<String> = sources
            .iter()
            .map(|source| source.quote_currency.clone())
            .collect();
        bases.extend(self.fx_base_currencies.iter().cloned());
        bases.into_iter().filter(|base| base != "USD").collect()
    }

    async fn finish_run(
        &self,
        run: &mut RefreshRun,
        execution: Execution,
        trigger: market_data_domain::Trigger,
        correlation_id: &str,
    ) -> Result<(), AppError> {
        let requested = execution.priced + execution.unpriced;
        run.record_counts(requested, execution.priced, execution.unpriced)
            .map_err(AppError::from_domain)?;
        let now = self.clock.now();
        let failure = if let Some((category, message)) = execution.aborted.clone() {
            Some((category, message))
        } else if requested > 0 && execution.priced == 0 {
            Some((
                execution.last_category.unwrap_or(ErrorCategory::Data),
                execution
                    .last_message
                    .clone()
                    .unwrap_or_else(|| "no source produced an accepted observation".to_string()),
            ))
        } else {
            None
        };

        match failure {
            Some((category, message)) => {
                run.fail(now, category, &message)
                    .map_err(AppError::from_domain)?;
                let finished = self.runs.finish(run).await;
                self.metrics.record_run(run.job(), "failed");
                self.metrics
                    .record_unpriced(run.job(), run.unpriced_count());
                // The audit is attempted even when the terminal write failed, so
                // a failed run is never silent.
                self.audit_failure(run, trigger, correlation_id).await;
                finished.map_err(AppError::from_repository)?;
            }
            None => {
                run.succeed(now).map_err(AppError::from_domain)?;
                self.runs
                    .finish(run)
                    .await
                    .map_err(AppError::from_repository)?;
                self.metrics.record_run(run.job(), "succeeded");
                self.metrics
                    .record_unpriced(run.job(), run.unpriced_count());
            }
        }
        Ok(())
    }

    async fn audit_failure(
        &self,
        run: &RefreshRun,
        trigger: market_data_domain::Trigger,
        correlation_id: &str,
    ) {
        let mut context = BTreeMap::new();
        context.insert("job".to_string(), run.job().as_str().to_string());
        context.insert("trigger".to_string(), trigger.as_str().to_string());
        if let Some(category) = run.error_category() {
            context.insert("category".to_string(), category.as_str().to_string());
        }
        let actor_service =
            matches!(trigger, market_data_domain::Trigger::Manual).then_some("market-data");
        let record = match AuditRecord::new(
            "system",
            actor_service,
            "price_refresh_failed",
            run.id().to_string(),
            AuditOutcome::Failed,
            self.clock.now(),
            context,
        ) {
            Ok(record) => record,
            Err(error) => {
                tracing::error!(%error, "the refresh-failure audit event shape was refused");
                self.metrics.record_audit_failure();
                return;
            }
        };
        if let Err(error) = self.audit.append(&record, correlation_id).await {
            tracing::error!(%error, "the refresh-failure audit event could not be recorded");
            self.metrics.record_audit_failure();
        }
    }

    fn circuit_allows(&self, source: &str, now: DateTime<Utc>) -> bool {
        let breakers = self.breakers.lock().unwrap();
        match breakers.get(source).and_then(|state| state.open_until) {
            Some(open_until) => open_until <= now,
            None => true,
        }
    }

    fn circuit_failure(&self, source: &str, now: DateTime<Utc>) {
        let mut breakers = self.breakers.lock().unwrap();
        let state = breakers.entry(source.to_string()).or_default();
        state.failures += 1;
        if state.failures >= CIRCUIT_FAILURE_THRESHOLD {
            state.failures = 0;
            state.open_until = Some(now + Duration::seconds(CIRCUIT_OPEN_SECONDS));
        }
    }

    fn circuit_success(&self, source: &str) {
        let mut breakers = self.breakers.lock().unwrap();
        breakers.insert(source.to_string(), BreakerState::default());
    }
}

fn backoff(attempt: u32) -> Duration {
    let base = 100_i64.saturating_mul(1_i64 << attempt.min(5));
    let jitter = i64::from(attempt) * 37 % 50;
    Duration::milliseconds(base + jitter)
}
