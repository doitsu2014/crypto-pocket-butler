//! Hand-written port fakes for the application suite. Every fake is in-memory
//! and deterministic; no test reads the real clock or a provider.

#![allow(dead_code)]

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use portfolio_application::common::{AssuranceLevel, VerifiedClaims};
use portfolio_application::ports::{
    AnonymisationRecord, AuditClient, AuthError, Clock, ExternalError, GuardrailRepository,
    HoldingObservation, HoldingsClient, IdGenerator, IntentRepository, LedgerClient,
    LedgerTradeCommand, MarketDataClient, MembershipRepository, NotificationClient,
    OverlayRepository, PortfolioRepository, PrivacyRepository, ReadinessProbe, RegisteredService,
    RepositoryError, RunRepository, RunStatus, ServiceTokenRegistry, SnapshotRepository,
    TargetRepository, TokenValidator, TradeRepository, UserPreferenceClient, ValuationRun,
    VenueTradeObservation,
};
use portfolio_domain::{
    AccountId, AuditRecord, FxRate, Guardrail, OrderIntent, OrderIntentId, Portfolio,
    PortfolioAccount, PortfolioId, Quote, RecordedTrade, RecordedTradeId, TargetAllocation,
    TradeOverlay, UserId, ValuationSnapshot,
};
use serde_json::Value;
use uuid::Uuid;

pub type Owners = Arc<Mutex<HashMap<PortfolioId, UserId>>>;

#[derive(Clone, Default)]
pub struct MutableClock {
    now: Arc<Mutex<DateTime<Utc>>>,
}

impl MutableClock {
    pub fn new(now: DateTime<Utc>) -> Self {
        Self {
            now: Arc::new(Mutex::new(now)),
        }
    }

    pub fn set(&self, now: DateTime<Utc>) {
        *self.now.lock().unwrap() = now;
    }

    pub fn advance(&self, by: Duration) {
        let mut now = self.now.lock().unwrap();
        *now += by;
    }
}

impl Clock for MutableClock {
    fn now(&self) -> DateTime<Utc> {
        *self.now.lock().unwrap()
    }
}

#[derive(Default)]
pub struct SeqIds {
    counter: AtomicU64,
}

impl IdGenerator for SeqIds {
    fn next(&self) -> Uuid {
        let value = self.counter.fetch_add(1, Ordering::Relaxed) + 1;
        Uuid::from_u128(u128::from(value))
    }
}

// ---------------------------------------------------------------------------
// Portfolios
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct FakePortfolios {
    rows: Mutex<Vec<Portfolio>>,
    owners: Owners,
    fail: Mutex<bool>,
}

impl FakePortfolios {
    pub fn new(owners: Owners) -> Self {
        Self {
            rows: Mutex::new(Vec::new()),
            owners,
            fail: Mutex::new(false),
        }
    }

    pub fn seed(&self, portfolio: Portfolio) {
        self.owners
            .lock()
            .unwrap()
            .insert(portfolio.id(), portfolio.user_id());
        self.rows.lock().unwrap().push(portfolio);
    }

    pub fn fail_writes(&self, fail: bool) {
        *self.fail.lock().unwrap() = fail;
    }

    pub fn all(&self) -> Vec<Portfolio> {
        self.rows.lock().unwrap().clone()
    }

    pub fn get(&self, id: PortfolioId) -> Option<Portfolio> {
        self.rows
            .lock()
            .unwrap()
            .iter()
            .find(|row| row.id() == id)
            .cloned()
    }
}

#[async_trait]
impl PortfolioRepository for FakePortfolios {
    async fn insert(&self, portfolio: &Portfolio) -> Result<(), RepositoryError> {
        if *self.fail.lock().unwrap() {
            return Err(RepositoryError::Unavailable("write failed".to_string()));
        }
        let mut rows = self.rows.lock().unwrap();
        if rows.iter().any(|row| {
            row.user_id() == portfolio.user_id()
                && row.name().eq_ignore_ascii_case(portfolio.name())
                && row.anonymised_at().is_none()
        }) {
            return Err(RepositoryError::Conflict(
                "name is already used".to_string(),
            ));
        }
        rows.push(portfolio.clone());
        drop(rows);
        self.owners
            .lock()
            .unwrap()
            .insert(portfolio.id(), portfolio.user_id());
        Ok(())
    }

    async fn find_by_id(&self, id: PortfolioId) -> Result<Option<Portfolio>, RepositoryError> {
        Ok(self.get(id))
    }

    async fn find_for_user(&self, user_id: UserId) -> Result<Vec<Portfolio>, RepositoryError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .filter(|row| row.user_id() == user_id)
            .cloned()
            .collect())
    }

    async fn update(&self, portfolio: &Portfolio) -> Result<(), RepositoryError> {
        let mut rows = self.rows.lock().unwrap();
        let Some(existing) = rows.iter_mut().find(|row| row.id() == portfolio.id()) else {
            return Err(RepositoryError::Conflict("portfolio not found".to_string()));
        };
        *existing = portfolio.clone();
        Ok(())
    }

    async fn anonymise_for_user(
        &self,
        user_id: UserId,
        anonymised_ref: UserId,
        at: DateTime<Utc>,
    ) -> Result<u64, RepositoryError> {
        let changed_ids: Vec<PortfolioId> = {
            let mut rows = self.rows.lock().unwrap();
            let mut changed_ids = Vec::new();
            for row in rows.iter_mut().filter(|row| row.user_id() == user_id) {
                if row.anonymised_at().is_none() {
                    *row = Portfolio::from_stored(
                        row.id(),
                        anonymised_ref,
                        format!("Anonymised portfolio {}", row.id()),
                        row.base_currency().to_string(),
                        row.created_at(),
                        at,
                        Some(at),
                    );
                    changed_ids.push(row.id());
                }
            }
            changed_ids
        };
        let changes = changed_ids.len() as u64;
        let mut owners = self.owners.lock().unwrap();
        for id in changed_ids {
            owners.insert(id, anonymised_ref);
        }
        Ok(changes)
    }

    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<Portfolio>, RepositoryError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .skip(offset.max(0) as usize)
            .take(limit.max(0) as usize)
            .cloned()
            .collect())
    }
}

// ---------------------------------------------------------------------------
// Membership
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct FakeMemberships {
    rows: Mutex<Vec<PortfolioAccount>>,
    owners: Owners,
}

impl FakeMemberships {
    pub fn new(owners: Owners) -> Self {
        Self {
            rows: Mutex::new(Vec::new()),
            owners,
        }
    }

    pub fn seed(&self, membership: PortfolioAccount) {
        self.rows.lock().unwrap().push(membership);
    }
}

#[async_trait]
impl MembershipRepository for FakeMemberships {
    async fn assign(&self, membership: &PortfolioAccount) -> Result<(), RepositoryError> {
        let mut rows = self.rows.lock().unwrap();
        rows.retain(|row| row.account_id() != membership.account_id());
        rows.push(membership.clone());
        Ok(())
    }

    async fn list_for_portfolio(
        &self,
        portfolio_id: PortfolioId,
    ) -> Result<Vec<PortfolioAccount>, RepositoryError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .filter(|row| row.portfolio_id() == portfolio_id)
            .cloned()
            .collect())
    }

    async fn list_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<PortfolioAccount>, RepositoryError> {
        let owners = self.owners.lock().unwrap().clone();
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .filter(|row| owners.get(&row.portfolio_id()) == Some(&user_id))
            .cloned()
            .collect())
    }
}

// ---------------------------------------------------------------------------
// Targets and guardrails
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct FakeTargets {
    targets: Mutex<Vec<TargetAllocation>>,
    guardrails: Mutex<Vec<Guardrail>>,
}

impl FakeTargets {
    pub fn seed_target(&self, target: TargetAllocation) {
        self.targets.lock().unwrap().push(target);
    }

    pub fn seed_guardrail(&self, guardrail: Guardrail) {
        self.guardrails.lock().unwrap().push(guardrail);
    }

    pub fn targets(&self) -> Vec<TargetAllocation> {
        self.targets.lock().unwrap().clone()
    }

    pub fn guardrails(&self) -> Vec<Guardrail> {
        self.guardrails.lock().unwrap().clone()
    }
}

#[async_trait]
impl TargetRepository for FakeTargets {
    async fn list_targets(
        &self,
        portfolio_id: PortfolioId,
    ) -> Result<Vec<TargetAllocation>, RepositoryError> {
        Ok(self
            .targets
            .lock()
            .unwrap()
            .iter()
            .filter(|row| row.portfolio_id() == portfolio_id)
            .cloned()
            .collect())
    }

    async fn list_guardrails(
        &self,
        portfolio_id: PortfolioId,
    ) -> Result<Vec<Guardrail>, RepositoryError> {
        Ok(self
            .guardrails
            .lock()
            .unwrap()
            .iter()
            .filter(|row| row.portfolio_id() == portfolio_id)
            .cloned()
            .collect())
    }

    async fn replace_for_portfolio(
        &self,
        portfolio_id: PortfolioId,
        targets: &[TargetAllocation],
        guardrails: &[Guardrail],
    ) -> Result<(), RepositoryError> {
        self.targets
            .lock()
            .unwrap()
            .retain(|row| row.portfolio_id() != portfolio_id);
        self.guardrails
            .lock()
            .unwrap()
            .retain(|row| row.portfolio_id() != portfolio_id);
        self.targets.lock().unwrap().extend_from_slice(targets);
        self.guardrails
            .lock()
            .unwrap()
            .extend_from_slice(guardrails);
        Ok(())
    }
}

#[async_trait]
impl GuardrailRepository for FakeTargets {
    async fn update(&self, guardrail: &Guardrail) -> Result<(), RepositoryError> {
        let mut rows = self.guardrails.lock().unwrap();
        if let Some(existing) = rows.iter_mut().find(|row| row.id() == guardrail.id()) {
            *existing = guardrail.clone();
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Snapshots
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct FakeSnapshots {
    rows: Mutex<Vec<ValuationSnapshot>>,
    owners: Owners,
}

impl FakeSnapshots {
    pub fn new(owners: Owners) -> Self {
        Self {
            rows: Mutex::new(Vec::new()),
            owners,
        }
    }

    pub fn all(&self) -> Vec<ValuationSnapshot> {
        self.rows.lock().unwrap().clone()
    }

    pub fn tamper(&self, index: usize) {
        let mut rows = self.rows.lock().unwrap();
        if let Some(row) = rows.get_mut(index) {
            *row = ValuationSnapshot::from_stored(
                row.id(),
                row.portfolio_id(),
                row.as_of(),
                row.hour_bucket(),
                row.total_value(),
                row.currency().to_string(),
                row.inputs_digest().to_string(),
                row.inputs_json().to_string(),
                "{\"status\":\"tampered\"}".to_string(),
            );
        }
    }
}

#[async_trait]
impl SnapshotRepository for FakeSnapshots {
    async fn insert(&self, snapshot: &ValuationSnapshot) -> Result<(), RepositoryError> {
        let mut rows = self.rows.lock().unwrap();
        if rows.iter().any(|row| {
            row.portfolio_id() == snapshot.portfolio_id()
                && row.hour_bucket() == snapshot.hour_bucket()
        }) {
            return Err(RepositoryError::Conflict("hour bucket taken".to_string()));
        }
        rows.push(snapshot.clone());
        Ok(())
    }

    async fn latest(
        &self,
        portfolio_id: PortfolioId,
    ) -> Result<Option<ValuationSnapshot>, RepositoryError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .filter(|row| row.portfolio_id() == portfolio_id)
            .max_by_key(|row| row.as_of())
            .cloned())
    }

    async fn list(
        &self,
        portfolio_id: PortfolioId,
        from: Option<DateTime<Utc>>,
        limit: i64,
        cursor: Option<DateTime<Utc>>,
    ) -> Result<Vec<ValuationSnapshot>, RepositoryError> {
        let mut rows: Vec<ValuationSnapshot> = self
            .rows
            .lock()
            .unwrap()
            .iter()
            .filter(|row| row.portfolio_id() == portfolio_id)
            .filter(|row| from.is_none_or(|from| row.as_of() >= from))
            .filter(|row| cursor.is_none_or(|cursor| row.as_of() < cursor))
            .cloned()
            .collect();
        rows.sort_by_key(|right| std::cmp::Reverse(right.as_of()));
        rows.truncate(limit.max(0) as usize);
        Ok(rows)
    }

    async fn find_by_id(
        &self,
        id: portfolio_domain::ValuationSnapshotId,
    ) -> Result<Option<ValuationSnapshot>, RepositoryError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .find(|row| row.id() == id)
            .cloned())
    }

    async fn exists_for_hour(
        &self,
        portfolio_id: PortfolioId,
        hour_bucket: DateTime<Utc>,
    ) -> Result<bool, RepositoryError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .any(|row| row.portfolio_id() == portfolio_id && row.hour_bucket() == hour_bucket))
    }

    async fn list_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<ValuationSnapshot>, RepositoryError> {
        let owners = self.owners.lock().unwrap().clone();
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .filter(|row| owners.get(&row.portfolio_id()) == Some(&user_id))
            .cloned()
            .collect())
    }
}

// ---------------------------------------------------------------------------
// Overlays
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct FakeOverlays {
    rows: Mutex<Vec<TradeOverlay>>,
    owners: Owners,
}

impl FakeOverlays {
    pub fn new(owners: Owners) -> Self {
        Self {
            rows: Mutex::new(Vec::new()),
            owners,
        }
    }

    pub fn all(&self) -> Vec<TradeOverlay> {
        self.rows.lock().unwrap().clone()
    }
}

#[async_trait]
impl OverlayRepository for FakeOverlays {
    async fn insert(&self, overlay: &TradeOverlay) -> Result<(), RepositoryError> {
        self.rows.lock().unwrap().push(overlay.clone());
        Ok(())
    }

    async fn list_for_portfolio(
        &self,
        portfolio_id: PortfolioId,
    ) -> Result<Vec<TradeOverlay>, RepositoryError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .filter(|row| row.portfolio_id() == portfolio_id)
            .cloned()
            .collect())
    }

    async fn delete_for_recorded_trade(
        &self,
        recorded_trade_id: RecordedTradeId,
    ) -> Result<u64, RepositoryError> {
        let mut rows = self.rows.lock().unwrap();
        let before = rows.len();
        rows.retain(|row| row.recorded_trade_id() != recorded_trade_id);
        Ok((before - rows.len()) as u64)
    }

    async fn expire_before(&self, cutoff: DateTime<Utc>) -> Result<u64, RepositoryError> {
        let mut rows = self.rows.lock().unwrap();
        let before = rows.len();
        rows.retain(|row| row.applied_at() >= cutoff);
        Ok((before - rows.len()) as u64)
    }

    async fn list_for_user(&self, user_id: UserId) -> Result<Vec<TradeOverlay>, RepositoryError> {
        let owners = self.owners.lock().unwrap().clone();
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .filter(|row| owners.get(&row.portfolio_id()) == Some(&user_id))
            .cloned()
            .collect())
    }
}

// ---------------------------------------------------------------------------
// Intents
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct FakeIntents {
    rows: Mutex<Vec<OrderIntent>>,
    owners: Owners,
}

impl FakeIntents {
    pub fn new(owners: Owners) -> Self {
        Self {
            rows: Mutex::new(Vec::new()),
            owners,
        }
    }

    pub fn all(&self) -> Vec<OrderIntent> {
        self.rows.lock().unwrap().clone()
    }
}

#[async_trait]
impl IntentRepository for FakeIntents {
    async fn insert(&self, intent: &OrderIntent) -> Result<(), RepositoryError> {
        self.rows.lock().unwrap().push(intent.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: OrderIntentId) -> Result<Option<OrderIntent>, RepositoryError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .find(|row| row.id() == id)
            .cloned())
    }

    async fn list_for_portfolio(
        &self,
        portfolio_id: PortfolioId,
    ) -> Result<Vec<OrderIntent>, RepositoryError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .filter(|row| row.portfolio_id() == portfolio_id)
            .cloned()
            .collect())
    }

    async fn update(&self, intent: &OrderIntent) -> Result<(), RepositoryError> {
        let mut rows = self.rows.lock().unwrap();
        if let Some(existing) = rows.iter_mut().find(|row| row.id() == intent.id()) {
            *existing = intent.clone();
        }
        Ok(())
    }

    async fn list_for_user(&self, user_id: UserId) -> Result<Vec<OrderIntent>, RepositoryError> {
        let owners = self.owners.lock().unwrap().clone();
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .filter(|row| owners.get(&row.portfolio_id()) == Some(&user_id))
            .cloned()
            .collect())
    }
}

// ---------------------------------------------------------------------------
// Trades
// ---------------------------------------------------------------------------

pub struct FakeTrades {
    rows: Mutex<Vec<RecordedTrade>>,
    owners: Owners,
    overlays: Arc<FakeOverlays>,
}

impl FakeTrades {
    pub fn new(owners: Owners, overlays: Arc<FakeOverlays>) -> Self {
        Self {
            rows: Mutex::new(Vec::new()),
            owners,
            overlays,
        }
    }

    pub fn all(&self) -> Vec<RecordedTrade> {
        self.rows.lock().unwrap().clone()
    }
}

#[async_trait]
impl TradeRepository for FakeTrades {
    async fn insert_with_overlay(
        &self,
        trade: &RecordedTrade,
        overlay: &TradeOverlay,
    ) -> Result<(), RepositoryError> {
        {
            let mut rows = self.rows.lock().unwrap();
            if rows.iter().any(|row| {
                row.portfolio_id() == trade.portfolio_id()
                    && row.idempotency_key() == trade.idempotency_key()
            }) {
                return Err(RepositoryError::Conflict(
                    "idempotency key used".to_string(),
                ));
            }
            rows.push(trade.clone());
        }
        self.overlays.insert(overlay).await?;
        Ok(())
    }

    async fn find_by_idempotency_key(
        &self,
        portfolio_id: PortfolioId,
        idempotency_key: &str,
    ) -> Result<Option<RecordedTrade>, RepositoryError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .find(|row| {
                row.portfolio_id() == portfolio_id && row.idempotency_key() == idempotency_key
            })
            .cloned())
    }

    async fn find_by_id(
        &self,
        id: RecordedTradeId,
    ) -> Result<Option<RecordedTrade>, RepositoryError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .find(|row| row.id() == id)
            .cloned())
    }

    async fn list_for_portfolio(
        &self,
        portfolio_id: PortfolioId,
    ) -> Result<Vec<RecordedTrade>, RepositoryError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .filter(|row| row.portfolio_id() == portfolio_id)
            .cloned()
            .collect())
    }

    async fn list_for_user(&self, user_id: UserId) -> Result<Vec<RecordedTrade>, RepositoryError> {
        let owners = self.owners.lock().unwrap().clone();
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .filter(|row| owners.get(&row.portfolio_id()) == Some(&user_id))
            .cloned()
            .collect())
    }

    async fn anonymise_for_user(
        &self,
        user_id: UserId,
        anonymised_ref: UserId,
    ) -> Result<u64, RepositoryError> {
        let mut rows = self.rows.lock().unwrap();
        let mut changed = 0;
        for row in rows.iter_mut().filter(|row| row.user_id() == user_id) {
            *row = RecordedTrade::from_stored(
                row.id(),
                row.portfolio_id(),
                anonymised_ref,
                row.asset_id().to_string(),
                row.side(),
                row.quantity(),
                row.price(),
                row.price_source(),
                row.executed_at(),
                row.created_at(),
                row.idempotency_key().to_string(),
                row.ledger_transaction_id().map(ToString::to_string),
            );
            changed += 1;
        }
        Ok(changed)
    }
}

// ---------------------------------------------------------------------------
// Runs
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct FakeRuns {
    rows: Mutex<Vec<ValuationRun>>,
}

impl FakeRuns {
    pub fn all(&self) -> Vec<ValuationRun> {
        self.rows.lock().unwrap().clone()
    }
}

#[async_trait]
impl RunRepository for FakeRuns {
    async fn claim(&self, run: &ValuationRun) -> Result<bool, RepositoryError> {
        let mut rows = self.rows.lock().unwrap();
        if rows
            .iter()
            .any(|row| row.job == run.job && row.status == RunStatus::Running)
        {
            return Ok(false);
        }
        rows.push(run.clone());
        Ok(true)
    }

    async fn renew(
        &self,
        id: Uuid,
        lease_until: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<bool, RepositoryError> {
        let mut rows = self.rows.lock().unwrap();
        let Some(run) = rows
            .iter_mut()
            .find(|row| row.id == id && row.status == RunStatus::Running && row.lease_until > now)
        else {
            return Ok(false);
        };
        run.lease_until = lease_until;
        Ok(true)
    }

    async fn finish(&self, run: &ValuationRun) -> Result<(), RepositoryError> {
        let mut rows = self.rows.lock().unwrap();
        if let Some(existing) = rows.iter_mut().find(|row| row.id == run.id) {
            *existing = run.clone();
        }
        Ok(())
    }

    async fn sweep_expired(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<ValuationRun>, RepositoryError> {
        let mut rows = self.rows.lock().unwrap();
        let mut swept = Vec::new();
        for run in rows.iter_mut() {
            if run.status == RunStatus::Running && run.lease_until <= now {
                run.status = RunStatus::Failed;
                run.finished_at = Some(now);
                run.detail = Some("lease expired".to_string());
                swept.push(run.clone());
            }
        }
        Ok(swept)
    }
}

// ---------------------------------------------------------------------------
// Privacy
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct FakePrivacy {
    rows: Mutex<HashMap<String, AnonymisationRecord>>,
}

#[async_trait]
impl PrivacyRepository for FakePrivacy {
    async fn find_anonymisation(
        &self,
        idempotency_key: &str,
    ) -> Result<Option<AnonymisationRecord>, RepositoryError> {
        Ok(self.rows.lock().unwrap().get(idempotency_key).cloned())
    }

    async fn record_anonymisation(
        &self,
        record: &AnonymisationRecord,
    ) -> Result<(), RepositoryError> {
        self.rows
            .lock()
            .unwrap()
            .entry(record.idempotency_key.clone())
            .or_insert_with(|| record.clone());
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// C5 and C6
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct FakeHoldings {
    holdings: Mutex<Vec<HoldingObservation>>,
    transactions: Mutex<HashMap<AccountId, Vec<VenueTradeObservation>>>,
    unavailable: Mutex<bool>,
    unavailable_users: Mutex<HashSet<UserId>>,
}

impl FakeHoldings {
    pub fn seed(&self, holding: HoldingObservation) {
        self.holdings.lock().unwrap().push(holding);
    }

    pub fn seed_transaction(&self, account: AccountId, transaction: VenueTradeObservation) {
        self.transactions
            .lock()
            .unwrap()
            .entry(account)
            .or_default()
            .push(transaction);
    }

    pub fn set_unavailable(&self, unavailable: bool) {
        *self.unavailable.lock().unwrap() = unavailable;
    }

    pub fn clear_holdings(&self) {
        self.holdings.lock().unwrap().clear();
    }

    pub fn set_user_unavailable(&self, user_id: UserId, unavailable: bool) {
        let mut users = self.unavailable_users.lock().unwrap();
        if unavailable {
            users.insert(user_id);
        } else {
            users.remove(&user_id);
        }
    }
}

#[async_trait]
impl HoldingsClient for FakeHoldings {
    async fn holdings_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<HoldingObservation>, ExternalError> {
        if *self.unavailable.lock().unwrap()
            || self.unavailable_users.lock().unwrap().contains(&user_id)
        {
            return Err(ExternalError::Unavailable("C5 down".to_string()));
        }
        Ok(self.holdings.lock().unwrap().clone())
    }

    async fn transactions_for_account(
        &self,
        account_id: AccountId,
        _since: Option<DateTime<Utc>>,
    ) -> Result<Vec<VenueTradeObservation>, ExternalError> {
        if *self.unavailable.lock().unwrap() {
            return Err(ExternalError::Unavailable("C5 down".to_string()));
        }
        Ok(self
            .transactions
            .lock()
            .unwrap()
            .get(&account_id)
            .cloned()
            .unwrap_or_default())
    }
}

#[derive(Default)]
pub struct FakeMarket {
    quotes: Mutex<HashMap<String, Quote>>,
    fx: Mutex<BTreeMap<(String, String), FxRate>>,
    unavailable: Mutex<bool>,
}

impl FakeMarket {
    pub fn seed_quote(&self, quote: Quote) {
        self.quotes
            .lock()
            .unwrap()
            .insert(quote.asset_id().to_string(), quote);
    }

    pub fn seed_fx(&self, rate: FxRate) {
        self.fx
            .lock()
            .unwrap()
            .insert((rate.base().to_string(), rate.quote().to_string()), rate);
    }

    pub fn set_unavailable(&self, unavailable: bool) {
        *self.unavailable.lock().unwrap() = unavailable;
    }
}

#[async_trait]
impl MarketDataClient for FakeMarket {
    async fn prices(&self, asset_ids: &[String]) -> Result<Vec<Quote>, ExternalError> {
        if *self.unavailable.lock().unwrap() {
            return Err(ExternalError::Unavailable("C6 down".to_string()));
        }
        let quotes = self.quotes.lock().unwrap();
        Ok(asset_ids
            .iter()
            .filter_map(|asset| quotes.get(asset).cloned())
            .collect())
    }

    async fn fx(&self, base: &str, quote: &str) -> Result<FxRate, ExternalError> {
        if *self.unavailable.lock().unwrap() {
            return Err(ExternalError::Unavailable("C6 down".to_string()));
        }
        self.fx
            .lock()
            .unwrap()
            .get(&(base.to_string(), quote.to_string()))
            .cloned()
            .ok_or_else(|| ExternalError::Refused("no FX pair".to_string()))
    }
}

// ---------------------------------------------------------------------------
// C7, C9, C10, Identity, auth
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct FakeLedger {
    calls: Mutex<Vec<(UserId, LedgerTradeCommand, String)>>,
    fail: Mutex<bool>,
}

impl FakeLedger {
    pub fn calls(&self) -> Vec<(UserId, LedgerTradeCommand, String)> {
        self.calls.lock().unwrap().clone()
    }

    pub fn fail(&self, fail: bool) {
        *self.fail.lock().unwrap() = fail;
    }
}

#[async_trait]
impl LedgerClient for FakeLedger {
    async fn record_trade(
        &self,
        user_id: UserId,
        command: &LedgerTradeCommand,
        idempotency_key: &str,
    ) -> Result<String, ExternalError> {
        if *self.fail.lock().unwrap() {
            return Err(ExternalError::Unavailable("C7 down".to_string()));
        }
        self.calls
            .lock()
            .unwrap()
            .push((user_id, command.clone(), idempotency_key.to_string()));
        Ok(format!("ledger-{idempotency_key}"))
    }
}

#[derive(Default)]
pub struct FakeNotifications {
    alerts: Mutex<Vec<(UserId, String, String, Value)>>,
    fail: Mutex<bool>,
}

impl FakeNotifications {
    pub fn alerts(&self) -> Vec<(UserId, String, String, Value)> {
        self.alerts.lock().unwrap().clone()
    }

    pub fn fail(&self, fail: bool) {
        *self.fail.lock().unwrap() = fail;
    }
}

#[async_trait]
impl NotificationClient for FakeNotifications {
    async fn notify(
        &self,
        user_id: UserId,
        alert_type: &str,
        idempotency_key: &str,
        payload: Value,
    ) -> Result<(), ExternalError> {
        if *self.fail.lock().unwrap() {
            return Err(ExternalError::Unavailable("C9 down".to_string()));
        }
        self.alerts.lock().unwrap().push((
            user_id,
            alert_type.to_string(),
            idempotency_key.to_string(),
            payload,
        ));
        Ok(())
    }
}

#[derive(Default)]
pub struct FakeAudit {
    records: Mutex<Vec<AuditRecord>>,
    fail: Mutex<bool>,
}

impl FakeAudit {
    pub fn records(&self) -> Vec<AuditRecord> {
        self.records.lock().unwrap().clone()
    }

    pub fn fail(&self, fail: bool) {
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
            return Err(RepositoryError::Unavailable("C10 down".to_string()));
        }
        self.records.lock().unwrap().push(record.clone());
        Ok(())
    }
}

pub struct FakePreferences {
    currency: Mutex<Option<String>>,
}

impl FakePreferences {
    pub fn new(currency: Option<&str>) -> Self {
        Self {
            currency: Mutex::new(currency.map(ToString::to_string)),
        }
    }
}

#[async_trait]
impl UserPreferenceClient for FakePreferences {
    async fn base_currency(&self, _user_id: UserId) -> Result<Option<String>, ExternalError> {
        Ok(self.currency.lock().unwrap().clone())
    }
}

pub struct StaticValidator;

#[async_trait]
impl TokenValidator for StaticValidator {
    async fn validate(&self, token: &str) -> Result<VerifiedClaims, AuthError> {
        if token == "valid" {
            Ok(VerifiedClaims {
                user_id: UserId::parse("11111111-1111-4111-8111-111111111111").unwrap(),
                assurance: AssuranceLevel::Aal2,
            })
        } else {
            Err(AuthError::Invalid("bad token".to_string()))
        }
    }
}

pub struct StaticServiceTokens;

#[async_trait]
impl ServiceTokenRegistry for StaticServiceTokens {
    async fn service_for(&self, token: &str) -> Result<Option<RegisteredService>, RepositoryError> {
        Ok(if token == "service" {
            Some(RegisteredService {
                name: "identity".to_string(),
                operations: vec![
                    portfolio_domain::Operation::PrivacyExport,
                    portfolio_domain::Operation::PrivacyAnonymise,
                ],
            })
        } else {
            None
        })
    }

    fn token_for(&self, _service: &str) -> Option<String> {
        None
    }
}

pub struct ReadyProbe;

#[async_trait]
impl ReadinessProbe for ReadyProbe {
    async fn check(&self) -> Result<(), String> {
        Ok(())
    }
}
