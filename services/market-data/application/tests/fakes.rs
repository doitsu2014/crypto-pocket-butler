//! Hand-written port fakes for the application suite. Every fake is in-memory
//! and deterministic; no test reads the clock or a provider.

#![allow(dead_code)]

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use market_data_application::ports::{
    AssetRegistry, AuditClient, CanonicalAsset, ClaimOutcome, Clock, ExternalError, FxRepository,
    ObservationRepository, PriceProvider, PriceSource, ProviderError, ProviderQuote,
    ProviderSecrets, RateLimiter, ReadinessProbe, RefreshRunRepository, RegisteredService,
    RepositoryError, ServiceMetrics, ServiceTokenRegistry, Sleeper, TierRepository,
};
use market_data_domain::{
    AssetPriceTier, AuditRecord, ErrorCategory, Freshness, FxRate, Operation, PriceObservation,
    PriceTier, RefreshJob, RefreshRun, RefreshRunId, RunStatus,
};

// ---------------------------------------------------------------------------
// Clock and sleeper
// ---------------------------------------------------------------------------

pub struct MutableClock {
    now: Mutex<DateTime<Utc>>,
}

impl MutableClock {
    pub fn at(now: DateTime<Utc>) -> Self {
        Self {
            now: Mutex::new(now),
        }
    }

    pub fn set(&self, now: DateTime<Utc>) {
        *self.now.lock().unwrap() = now;
    }

    pub fn advance(&self, duration: Duration) {
        let mut now = self.now.lock().unwrap();
        *now += duration;
    }
}

impl Clock for MutableClock {
    fn now(&self) -> DateTime<Utc> {
        *self.now.lock().unwrap()
    }
}

#[derive(Default)]
pub struct RecordingSleeper {
    pauses: Mutex<Vec<Duration>>,
}

impl RecordingSleeper {
    pub fn pauses(&self) -> Vec<Duration> {
        self.pauses.lock().unwrap().clone()
    }
}

#[async_trait]
impl Sleeper for RecordingSleeper {
    async fn sleep(&self, duration: Duration) {
        self.pauses.lock().unwrap().push(duration);
        tokio::task::yield_now().await;
    }
}

// ---------------------------------------------------------------------------
// Observations, FX, tiers, runs
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct FakeObservations {
    rows: Mutex<Vec<PriceObservation>>,
    fail: Mutex<bool>,
}

impl FakeObservations {
    pub fn seed(&self, observation: PriceObservation) {
        self.rows.lock().unwrap().push(observation);
    }

    pub fn set_fail(&self, fail: bool) {
        *self.fail.lock().unwrap() = fail;
    }

    fn unavailable(&self) -> Result<(), RepositoryError> {
        if *self.fail.lock().unwrap() {
            Err(RepositoryError::Unavailable(
                "observations are down".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    pub fn all(&self) -> Vec<PriceObservation> {
        self.rows.lock().unwrap().clone()
    }

    pub fn accepted(&self) -> Vec<PriceObservation> {
        self.all()
            .into_iter()
            .filter(PriceObservation::is_accepted)
            .collect()
    }
}

#[async_trait]
impl ObservationRepository for FakeObservations {
    async fn insert(&self, observation: &PriceObservation) -> Result<(), RepositoryError> {
        self.unavailable()?;
        self.rows.lock().unwrap().push(observation.clone());
        Ok(())
    }

    async fn latest_accepted_for_source(
        &self,
        source_name: &str,
        asset_ids: &[String],
    ) -> Result<Vec<PriceObservation>, RepositoryError> {
        self.unavailable()?;
        let rows = self.rows.lock().unwrap();
        let mut newest: HashMap<String, PriceObservation> = HashMap::new();
        for row in rows.iter() {
            if !row.is_accepted()
                || row.source_name() != source_name
                || !asset_ids.iter().any(|id| id == row.asset_id())
            {
                continue;
            }
            newest
                .entry(row.asset_id().to_string())
                .and_modify(|current| {
                    if row.observed_at() > current.observed_at() {
                        *current = row.clone();
                    }
                })
                .or_insert_with(|| row.clone());
        }
        Ok(newest.into_values().collect())
    }

    async fn latest_accepted_for_assets(
        &self,
        asset_ids: &[String],
    ) -> Result<Vec<PriceObservation>, RepositoryError> {
        self.unavailable()?;
        let rows = self.rows.lock().unwrap();
        let mut newest: HashMap<String, PriceObservation> = HashMap::new();
        for row in rows.iter() {
            if !row.is_accepted() || !asset_ids.iter().any(|id| id == row.asset_id()) {
                continue;
            }
            newest
                .entry(row.asset_id().to_string())
                .and_modify(|current| {
                    if row.observed_at() > current.observed_at() {
                        *current = row.clone();
                    }
                })
                .or_insert_with(|| row.clone());
        }
        Ok(newest.into_values().collect())
    }

    async fn prune_before(&self, cutoff: DateTime<Utc>) -> Result<u64, RepositoryError> {
        self.unavailable()?;
        let mut rows = self.rows.lock().unwrap();
        let before = rows.len();
        let newest: HashMap<(String, String), (DateTime<Utc>, uuid::Uuid)> = rows
            .iter()
            .map(|row| {
                (
                    (row.asset_id().to_string(), row.source_name().to_string()),
                    (row.observed_at(), row.id().as_uuid()),
                )
            })
            .fold(HashMap::new(), |mut acc, (key, value)| {
                acc.entry(key)
                    .and_modify(|current| {
                        if value > *current {
                            *current = value;
                        }
                    })
                    .or_insert(value);
                acc
            });
        rows.retain(|row| {
            if row.received_at() >= cutoff {
                return true;
            }
            let key = (row.asset_id().to_string(), row.source_name().to_string());
            match newest.get(&key) {
                Some((observed_at, id)) => {
                    !(*observed_at > row.observed_at()
                        || (*observed_at == row.observed_at() && *id > row.id().as_uuid()))
                }
                None => true,
            }
        });
        Ok((before - rows.len()) as u64)
    }
}

#[derive(Default)]
pub struct FakeFx {
    rows: Mutex<Vec<FxRate>>,
    fail: Mutex<bool>,
}

impl FakeFx {
    pub fn seed(&self, rate: FxRate) {
        self.rows.lock().unwrap().push(rate);
    }

    pub fn set_fail(&self, fail: bool) {
        *self.fail.lock().unwrap() = fail;
    }

    fn unavailable(&self) -> Result<(), RepositoryError> {
        if *self.fail.lock().unwrap() {
            Err(RepositoryError::Unavailable(
                "fx readings are down".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    pub fn all(&self) -> Vec<FxRate> {
        self.rows.lock().unwrap().clone()
    }
}

#[async_trait]
impl FxRepository for FakeFx {
    async fn insert(&self, rate: &FxRate) -> Result<(), RepositoryError> {
        self.unavailable()?;
        self.rows.lock().unwrap().push(rate.clone());
        Ok(())
    }

    async fn latest(&self, base: &str, quote: &str) -> Result<Option<FxRate>, RepositoryError> {
        self.unavailable()?;
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .filter(|rate| rate.base() == base && rate.quote() == quote)
            .max_by_key(|rate| rate.observed_at())
            .cloned())
    }

    async fn prune_before(&self, cutoff: DateTime<Utc>) -> Result<u64, RepositoryError> {
        self.unavailable()?;
        let mut rows = self.rows.lock().unwrap();
        let before = rows.len();
        let newest: HashMap<(String, String), (DateTime<Utc>, uuid::Uuid)> = rows
            .iter()
            .map(|rate| {
                (
                    (rate.base().to_string(), rate.quote().to_string()),
                    (rate.observed_at(), rate.id().as_uuid()),
                )
            })
            .fold(HashMap::new(), |mut acc, (key, value)| {
                acc.entry(key)
                    .and_modify(|current| {
                        if value > *current {
                            *current = value;
                        }
                    })
                    .or_insert(value);
                acc
            });
        rows.retain(|rate| {
            if rate.received_at() >= cutoff {
                return true;
            }
            let key = (rate.base().to_string(), rate.quote().to_string());
            match newest.get(&key) {
                Some((observed_at, id)) => {
                    !(*observed_at > rate.observed_at()
                        || (*observed_at == rate.observed_at() && *id > rate.id().as_uuid()))
                }
                None => true,
            }
        });
        Ok((before - rows.len()) as u64)
    }
}

#[derive(Default)]
pub struct FakeTiers {
    rows: Mutex<Vec<AssetPriceTier>>,
}

impl FakeTiers {
    pub fn all(&self) -> Vec<AssetPriceTier> {
        self.rows.lock().unwrap().clone()
    }
}

#[async_trait]
impl TierRepository for FakeTiers {
    async fn upsert(&self, tier: &AssetPriceTier) -> Result<(), RepositoryError> {
        let mut rows = self.rows.lock().unwrap();
        match rows
            .iter_mut()
            .find(|row| row.asset_id() == tier.asset_id())
        {
            Some(existing) => *existing = tier.clone(),
            None => rows.push(tier.clone()),
        }
        Ok(())
    }

    async fn find(&self, asset_id: &str) -> Result<Option<AssetPriceTier>, RepositoryError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .find(|row| row.asset_id() == asset_id)
            .cloned())
    }

    async fn bounds_for(
        &self,
        asset_ids: &[String],
    ) -> Result<Vec<(String, i64)>, RepositoryError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .filter(|row| asset_ids.iter().any(|id| id == row.asset_id()))
            .map(|row| (row.asset_id().to_string(), row.bound_seconds()))
            .collect())
    }
}

pub struct FakeRuns {
    rows: Mutex<Vec<RefreshRun>>,
}

impl Default for FakeRuns {
    fn default() -> Self {
        Self {
            rows: Mutex::new(Vec::new()),
        }
    }
}

impl FakeRuns {
    pub fn all(&self) -> Vec<RefreshRun> {
        self.rows.lock().unwrap().clone()
    }

    pub fn seed(&self, run: RefreshRun) {
        self.rows.lock().unwrap().push(run);
    }
}

#[async_trait]
impl RefreshRunRepository for FakeRuns {
    async fn claim(&self, run: &RefreshRun) -> Result<ClaimOutcome, RepositoryError> {
        let mut rows = self.rows.lock().unwrap();
        for row in rows.iter_mut() {
            let _ = row.expire_if_due(run.started_at());
        }
        if let Some(active) = rows
            .iter()
            .find(|row| row.job() == run.job() && row.status() == RunStatus::Running)
        {
            return Ok(ClaimOutcome::Held(active.clone()));
        }
        rows.push(run.clone());
        Ok(ClaimOutcome::Claimed)
    }

    async fn find_by_id(&self, id: RefreshRunId) -> Result<Option<RefreshRun>, RepositoryError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .find(|row| row.id() == id)
            .cloned())
    }

    async fn active_for_job(
        &self,
        job: RefreshJob,
        now: DateTime<Utc>,
    ) -> Result<Option<RefreshRun>, RepositoryError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .find(|row| row.job() == job && row.is_live(now))
            .cloned())
    }

    async fn renew(
        &self,
        id: RefreshRunId,
        lease_until: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<bool, RepositoryError> {
        let mut rows = self.rows.lock().unwrap();
        match rows.iter_mut().find(|row| row.id() == id) {
            Some(row) if row.is_live(now) => {
                let _ = lease_until;
                Ok(row.renew(now))
            }
            _ => Ok(false),
        }
    }

    async fn finish(&self, run: &RefreshRun) -> Result<(), RepositoryError> {
        let mut rows = self.rows.lock().unwrap();
        match rows.iter_mut().find(|row| row.id() == run.id()) {
            Some(existing) if existing.status() == RunStatus::Running => {
                *existing = run.clone();
                Ok(())
            }
            Some(_) => Err(RepositoryError::Conflict(
                "the run is no longer running".to_string(),
            )),
            None => Err(RepositoryError::Conflict(
                "the run does not exist".to_string(),
            )),
        }
    }

    async fn sweep_expired(&self, now: DateTime<Utc>) -> Result<Vec<RefreshRun>, RepositoryError> {
        let mut rows = self.rows.lock().unwrap();
        let mut swept = Vec::new();
        for row in rows.iter_mut() {
            if row.expire_if_due(now) {
                swept.push(row.clone());
            }
        }
        Ok(swept)
    }
}

// ---------------------------------------------------------------------------
// C4 registry
// ---------------------------------------------------------------------------

pub struct FakeRegistry {
    sources: Mutex<Vec<PriceSource>>,
    assets: Mutex<Vec<CanonicalAsset>>,
    fail_sources: Mutex<bool>,
    fail_assets: Mutex<bool>,
    pub source_calls: AtomicUsize,
    pub asset_calls: AtomicUsize,
}

impl Default for FakeRegistry {
    fn default() -> Self {
        Self {
            sources: Mutex::new(Vec::new()),
            assets: Mutex::new(Vec::new()),
            fail_sources: Mutex::new(false),
            fail_assets: Mutex::new(false),
            source_calls: AtomicUsize::new(0),
            asset_calls: AtomicUsize::new(0),
        }
    }
}

impl FakeRegistry {
    pub fn set_sources(&self, sources: Vec<PriceSource>) {
        *self.sources.lock().unwrap() = sources;
    }

    pub fn set_assets(&self, assets: Vec<CanonicalAsset>) {
        *self.assets.lock().unwrap() = assets;
    }

    pub fn set_fail_sources(&self, fail: bool) {
        *self.fail_sources.lock().unwrap() = fail;
    }

    pub fn set_fail_assets(&self, fail: bool) {
        *self.fail_assets.lock().unwrap() = fail;
    }
}

#[async_trait]
impl AssetRegistry for FakeRegistry {
    async fn sources(&self) -> Result<Vec<PriceSource>, ExternalError> {
        self.source_calls.fetch_add(1, Ordering::Relaxed);
        if *self.fail_sources.lock().unwrap() {
            return Err(ExternalError::Unavailable("C4 is down".to_string()));
        }
        Ok(self.sources.lock().unwrap().clone())
    }

    async fn canonical_assets(&self) -> Result<Vec<CanonicalAsset>, ExternalError> {
        self.asset_calls.fetch_add(1, Ordering::Relaxed);
        if *self.fail_assets.lock().unwrap() {
            return Err(ExternalError::Unavailable("C4 is down".to_string()));
        }
        Ok(self.assets.lock().unwrap().clone())
    }
}

// ---------------------------------------------------------------------------
// Providers and secrets
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct FakeProvider {
    quotes: Mutex<HashMap<String, VecDeque<Result<ProviderQuote, ProviderError>>>>,
    fx: Mutex<HashMap<String, VecDeque<Result<ProviderQuote, ProviderError>>>>,
    calls: Mutex<Vec<(String, String)>>,
    advance: Mutex<Option<(Duration, Arc<MutableClock>)>>,
}

impl FakeProvider {
    pub fn script_quote(
        &self,
        source: &str,
        asset: &str,
        readings: Vec<Result<ProviderQuote, ProviderError>>,
    ) {
        self.quotes
            .lock()
            .unwrap()
            .insert(format!("{source}|{asset}"), readings.into());
    }

    pub fn script_fx(
        &self,
        source: &str,
        base: &str,
        quote: &str,
        readings: Vec<Result<ProviderQuote, ProviderError>>,
    ) {
        self.fx
            .lock()
            .unwrap()
            .insert(format!("{source}|{base}|{quote}"), readings.into());
    }

    pub fn calls(&self) -> Vec<(String, String)> {
        self.calls.lock().unwrap().clone()
    }

    pub fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }

    /// Advances the injected clock after every provider call, so deadline and
    /// lease-renewal paths are deterministic.
    pub fn advance_clock_on_call(&self, duration: Duration, clock: Arc<MutableClock>) {
        *self.advance.lock().unwrap() = Some((duration, clock));
    }

    fn maybe_advance(&self) {
        if let Some((duration, clock)) = self.advance.lock().unwrap().as_ref() {
            clock.advance(*duration);
        }
    }
}

fn take(
    map: &Mutex<HashMap<String, VecDeque<Result<ProviderQuote, ProviderError>>>>,
    key: &str,
) -> Result<ProviderQuote, ProviderError> {
    let mut map = map.lock().unwrap();
    match map.get_mut(key).and_then(VecDeque::pop_front) {
        Some(result) => result,
        None => Err(ProviderError::permanent(
            ErrorCategory::Provider,
            "no scripted reading",
        )),
    }
}

#[async_trait]
impl PriceProvider for FakeProvider {
    async fn quote(
        &self,
        source: &PriceSource,
        asset_id: &str,
        _api_key: Option<&str>,
    ) -> Result<ProviderQuote, ProviderError> {
        self.calls
            .lock()
            .unwrap()
            .push((source.name.clone(), asset_id.to_string()));
        let result = take(&self.quotes, &format!("{}|{asset_id}", source.name));
        self.maybe_advance();
        result
    }

    async fn fx_rate(
        &self,
        source: &PriceSource,
        base: &str,
        quote: &str,
        _api_key: Option<&str>,
    ) -> Result<ProviderQuote, ProviderError> {
        self.calls
            .lock()
            .unwrap()
            .push((source.name.clone(), format!("{base}/{quote}")));
        let result = take(&self.fx, &format!("{}|{base}|{quote}", source.name));
        self.maybe_advance();
        result
    }
}

#[derive(Default)]
pub struct FakeSecrets {
    keys: Mutex<HashMap<String, String>>,
    fail: Mutex<bool>,
    refused: Mutex<bool>,
}

impl FakeSecrets {
    pub fn set_key(&self, source: &str, key: &str) {
        self.keys
            .lock()
            .unwrap()
            .insert(source.to_string(), key.to_string());
    }

    pub fn set_fail(&self, fail: bool) {
        *self.fail.lock().unwrap() = fail;
    }

    pub fn set_refused(&self, refused: bool) {
        *self.refused.lock().unwrap() = refused;
    }
}

#[async_trait]
impl ProviderSecrets for FakeSecrets {
    async fn provider_key(&self, source: &str) -> Result<Option<String>, ExternalError> {
        if *self.refused.lock().unwrap() {
            return Err(ExternalError::Refused(
                "the provider secret function was refused".to_string(),
            ));
        }
        if *self.fail.lock().unwrap() {
            return Err(ExternalError::Unavailable("Vault is down".to_string()));
        }
        Ok(self.keys.lock().unwrap().get(source).cloned())
    }
}

// ---------------------------------------------------------------------------
// Rate limiter
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct FakeRateLimiter {
    blocked: Mutex<bool>,
    blocked_sources: Mutex<std::collections::HashSet<String>>,
    buckets: Mutex<HashMap<String, (i64, DateTime<Utc>)>>,
}

impl FakeRateLimiter {
    pub fn block(&self, blocked: bool) {
        *self.blocked.lock().unwrap() = blocked;
    }

    pub fn block_source(&self, source: &str) {
        self.blocked_sources
            .lock()
            .unwrap()
            .insert(source.to_string());
    }
}

#[async_trait]
impl RateLimiter for FakeRateLimiter {
    async fn try_acquire(
        &self,
        source_name: &str,
        rate_limit_per_minute: u32,
        now: DateTime<Utc>,
    ) -> Result<bool, RepositoryError> {
        if *self.blocked.lock().unwrap()
            || self.blocked_sources.lock().unwrap().contains(source_name)
        {
            return Ok(false);
        }
        let mut buckets = self.buckets.lock().unwrap();
        let (tokens, updated_at) = buckets
            .get(source_name)
            .copied()
            .unwrap_or((i64::from(rate_limit_per_minute), now));
        let elapsed = (now - updated_at).num_seconds();
        let (taken, remaining) = market_data_domain::rate::bucket_try_take(
            tokens,
            i64::from(rate_limit_per_minute),
            elapsed,
        );
        buckets.insert(source_name.to_string(), (remaining, now));
        Ok(taken)
    }
}

// ---------------------------------------------------------------------------
// Audit, service tokens, readiness, metrics
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct FakeAudit {
    events: Mutex<Vec<AuditRecord>>,
    fail: Mutex<bool>,
}

impl FakeAudit {
    pub fn events(&self) -> Vec<AuditRecord> {
        self.events.lock().unwrap().clone()
    }

    pub fn set_fail(&self, fail: bool) {
        *self.fail.lock().unwrap() = fail;
    }
}

#[async_trait]
impl AuditClient for FakeAudit {
    async fn append(
        &self,
        record: &AuditRecord,
        _correlation_id: &str,
    ) -> Result<(), RepositoryError> {
        if *self.fail.lock().unwrap() {
            return Err(RepositoryError::Unavailable(
                "the audit service is unavailable".to_string(),
            ));
        }
        self.events.lock().unwrap().push(record.clone());
        Ok(())
    }
}

#[derive(Default)]
pub struct FakeServiceTokens {
    inbound: HashMap<String, RegisteredService>,
    outbound: HashMap<String, String>,
}

impl FakeServiceTokens {
    pub fn with(entries: &[(&str, &str, Vec<Operation>)]) -> Self {
        let mut inbound = HashMap::new();
        let mut outbound = HashMap::new();
        for (token, name, operations) in entries {
            inbound.insert(
                (*token).to_string(),
                RegisteredService {
                    name: (*name).to_string(),
                    operations: operations.clone(),
                },
            );
            outbound.insert((*name).to_string(), (*token).to_string());
        }
        Self { inbound, outbound }
    }
}

#[async_trait]
impl ServiceTokenRegistry for FakeServiceTokens {
    async fn service_for(&self, token: &str) -> Result<Option<RegisteredService>, RepositoryError> {
        Ok(self.inbound.get(token).cloned())
    }

    fn token_for(&self, service: &str) -> Option<String> {
        self.outbound.get(service).cloned()
    }
}

pub struct Ready(pub bool);

#[async_trait]
impl ReadinessProbe for Ready {
    async fn check(&self) -> Result<(), String> {
        if self.0 {
            Ok(())
        } else {
            Err("not ready".to_string())
        }
    }
}

#[derive(Default)]
pub struct FakeMetrics {
    quotes: Mutex<Vec<(Freshness, PriceTier, i64)>>,
    fx: Mutex<Vec<(String, i64)>>,
    source_errors: Mutex<Vec<(String, ErrorCategory)>>,
    runs: Mutex<Vec<(RefreshJob, String)>>,
    unpriced: Mutex<Vec<(RefreshJob, u32)>>,
    audit_failures: AtomicUsize,
}

impl FakeMetrics {
    pub fn quotes(&self) -> Vec<(Freshness, PriceTier, i64)> {
        self.quotes.lock().unwrap().clone()
    }

    pub fn fx(&self) -> Vec<(String, i64)> {
        self.fx.lock().unwrap().clone()
    }

    pub fn source_errors(&self) -> Vec<(String, ErrorCategory)> {
        self.source_errors.lock().unwrap().clone()
    }

    pub fn runs(&self) -> Vec<(RefreshJob, String)> {
        self.runs.lock().unwrap().clone()
    }

    pub fn unpriced(&self) -> Vec<(RefreshJob, u32)> {
        self.unpriced.lock().unwrap().clone()
    }

    pub fn audit_failures(&self) -> usize {
        self.audit_failures.load(Ordering::Relaxed)
    }
}

impl ServiceMetrics for FakeMetrics {
    fn record_quote(&self, freshness: Freshness, tier: PriceTier, age_seconds: i64) {
        self.quotes
            .lock()
            .unwrap()
            .push((freshness, tier, age_seconds));
    }

    fn record_fx(&self, pair: &str, age_seconds: i64) {
        self.fx
            .lock()
            .unwrap()
            .push((pair.to_string(), age_seconds));
    }

    fn record_source_error(&self, source: &str, category: ErrorCategory) {
        self.source_errors
            .lock()
            .unwrap()
            .push((source.to_string(), category));
    }

    fn record_run(&self, job: RefreshJob, status: &str) {
        self.runs.lock().unwrap().push((job, status.to_string()));
    }

    fn record_unpriced(&self, job: RefreshJob, count: u32) {
        self.unpriced.lock().unwrap().push((job, count));
    }

    fn record_audit_failure(&self) {
        self.audit_failures.fetch_add(1, Ordering::Relaxed);
    }
}

/// A helper for building a source fixture.
pub fn source(name: &str, priority: u32, delayed: bool) -> PriceSource {
    PriceSource::new(name, priority, "USD", 120, delayed, true).unwrap()
}

/// A helper for building a canonical asset fixture.
pub fn asset(id: &str, rank: u32) -> CanonicalAsset {
    CanonicalAsset {
        id: id.to_string(),
        rank,
    }
}

/// Shared harness pieces are re-exported so the HTTP suite can reuse them.
pub type SharedMetrics = Arc<FakeMetrics>;
