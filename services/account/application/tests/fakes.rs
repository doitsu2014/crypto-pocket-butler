//! Hand-written port fakes for the application suite. Every fake is in-memory
//! and deterministic; no test reads the clock or a provider.

#![allow(dead_code)]

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use account_application::common::VerifiedClaims;
use account_application::ports::{
    AccountRepository, AnonymisationRecord, AssetRegistry, AuditClient, AuthAdmin, AuthError,
    ChainProvider, Clock, CredentialRepository, ExchangeCredentials, ExchangeProvider,
    ExternalError, HoldingRepository, ImportRepository, NotificationClient, Observation,
    ObservedBalance, PrivacyRepository, ReadinessProbe, RegisteredService, RepositoryError,
    ServiceTokenRegistry, SupportedChain, SyncRunRepository, TokenValidator, VaultClient,
    VerifiedPermissions, WakeTransport, WalletConnectionRepository,
};
use account_domain::{
    Account, AccountId, AuditRecord, Credential, CredentialId, HoldingSnapshot, ImportBatch,
    ImportBatchId, ImportRowError, ImportedTransaction, Operation, SyncRun, SyncRunId,
    SyncRunStatus, UserId, WalletConnection,
};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;

// ---------------------------------------------------------------------------
// Accounts
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct FakeAccounts {
    rows: Mutex<Vec<Account>>,
}

impl FakeAccounts {
    pub fn seed(&self, account: Account) {
        self.rows.lock().unwrap().push(account);
    }

    pub fn all(&self) -> Vec<Account> {
        self.rows.lock().unwrap().clone()
    }

    pub fn accounts_of(&self, user_id: UserId) -> Vec<Account> {
        self.rows
            .lock()
            .unwrap()
            .iter()
            .filter(|account| account.user_id() == user_id)
            .cloned()
            .collect()
    }
}

#[async_trait]
impl AccountRepository for FakeAccounts {
    async fn insert(&self, account: &Account) -> Result<(), RepositoryError> {
        let mut rows = self.rows.lock().unwrap();
        if rows.iter().any(|candidate| candidate.id() == account.id()) {
            return Err(RepositoryError::Conflict(
                "duplicate account id".to_string(),
            ));
        }
        if rows.iter().any(|candidate| {
            candidate.user_id() == account.user_id()
                && candidate.anonymised_at().is_none()
                && candidate.label().eq_ignore_ascii_case(account.label())
        }) {
            return Err(RepositoryError::Conflict(
                "the label is already used".to_string(),
            ));
        }
        rows.push(account.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: AccountId) -> Result<Option<Account>, RepositoryError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .find(|account| account.id() == id)
            .cloned())
    }

    async fn find_for_user(&self, user_id: UserId) -> Result<Vec<Account>, RepositoryError> {
        Ok(self.accounts_of(user_id))
    }

    async fn label_exists(
        &self,
        user_id: UserId,
        label: &str,
        except: Option<AccountId>,
    ) -> Result<bool, RepositoryError> {
        Ok(self.rows.lock().unwrap().iter().any(|account| {
            account.user_id() == user_id
                && account.anonymised_at().is_none()
                && account.label().eq_ignore_ascii_case(label)
                && Some(account.id()) != except
        }))
    }

    async fn update(&self, account: &Account) -> Result<(), RepositoryError> {
        let mut rows = self.rows.lock().unwrap();
        match rows
            .iter_mut()
            .find(|candidate| candidate.id() == account.id())
        {
            Some(existing) => {
                *existing = account.clone();
                Ok(())
            }
            None => Err(RepositoryError::Conflict("account not found".to_string())),
        }
    }

    async fn list_syncable(&self) -> Result<Vec<Account>, RepositoryError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .filter(|account| account.is_syncable())
            .cloned()
            .collect())
    }

    async fn anonymise_for_user(
        &self,
        user_id: UserId,
        at: DateTime<Utc>,
    ) -> Result<u64, RepositoryError> {
        let mut rows = self.rows.lock().unwrap();
        let mut changed = 0;
        for account in rows.iter_mut() {
            if account.user_id() == user_id && account.anonymised_at().is_none() {
                account.anonymise(at);
                changed += 1;
            }
        }
        Ok(changed)
    }
}

// ---------------------------------------------------------------------------
// Credentials
// ---------------------------------------------------------------------------

pub struct FakeCredentials {
    rows: Mutex<Vec<Credential>>,
    accounts: Arc<FakeAccounts>,
}

impl FakeCredentials {
    pub fn new(accounts: Arc<FakeAccounts>) -> Self {
        Self {
            rows: Mutex::new(Vec::new()),
            accounts,
        }
    }

    pub fn all(&self) -> Vec<Credential> {
        self.rows.lock().unwrap().clone()
    }

    pub fn seed(&self, credential: Credential) {
        self.rows.lock().unwrap().push(credential);
    }
}

#[async_trait]
impl CredentialRepository for FakeCredentials {
    async fn insert(&self, credential: &Credential) -> Result<(), RepositoryError> {
        let mut rows = self.rows.lock().unwrap();
        if rows
            .iter()
            .any(|candidate| candidate.account_id() == credential.account_id())
        {
            return Err(RepositoryError::Conflict(
                "the account already has a credential".to_string(),
            ));
        }
        rows.push(credential.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: CredentialId) -> Result<Option<Credential>, RepositoryError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .find(|credential| credential.id() == id)
            .cloned())
    }

    async fn find_for_account(
        &self,
        account_id: AccountId,
    ) -> Result<Option<Credential>, RepositoryError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .find(|credential| credential.account_id() == account_id)
            .cloned())
    }

    async fn update(&self, credential: &Credential) -> Result<(), RepositoryError> {
        let mut rows = self.rows.lock().unwrap();
        match rows
            .iter_mut()
            .find(|candidate| candidate.id() == credential.id())
        {
            Some(existing) => {
                *existing = credential.clone();
                Ok(())
            }
            None => Err(RepositoryError::Conflict(
                "credential not found".to_string(),
            )),
        }
    }

    async fn list_verified(&self) -> Result<Vec<Credential>, RepositoryError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .filter(|credential| credential.is_usable())
            .cloned()
            .collect())
    }

    async fn list_for_user(&self, user_id: UserId) -> Result<Vec<Credential>, RepositoryError> {
        let accounts: Vec<AccountId> = self
            .accounts
            .accounts_of(user_id)
            .iter()
            .map(Account::id)
            .collect();
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .filter(|credential| accounts.contains(&credential.account_id()))
            .cloned()
            .collect())
    }

    async fn delete_for_user(&self, user_id: UserId) -> Result<u64, RepositoryError> {
        let accounts: Vec<AccountId> = self
            .accounts
            .accounts_of(user_id)
            .iter()
            .map(Account::id)
            .collect();
        let mut rows = self.rows.lock().unwrap();
        let before = rows.len();
        rows.retain(|credential| !accounts.contains(&credential.account_id()));
        Ok((before - rows.len()) as u64)
    }
}

// ---------------------------------------------------------------------------
// Wallet connections
// ---------------------------------------------------------------------------

pub struct FakeWallets {
    rows: Mutex<Vec<WalletConnection>>,
    accounts: Arc<FakeAccounts>,
}

impl FakeWallets {
    pub fn new(accounts: Arc<FakeAccounts>) -> Self {
        Self {
            rows: Mutex::new(Vec::new()),
            accounts,
        }
    }

    pub fn all(&self) -> Vec<WalletConnection> {
        self.rows.lock().unwrap().clone()
    }

    pub fn seed(&self, connection: WalletConnection) {
        self.rows.lock().unwrap().push(connection);
    }
}

#[async_trait]
impl WalletConnectionRepository for FakeWallets {
    async fn insert(&self, connection: &WalletConnection) -> Result<(), RepositoryError> {
        let mut rows = self.rows.lock().unwrap();
        if rows
            .iter()
            .any(|candidate| candidate.account_id() == connection.account_id())
        {
            return Err(RepositoryError::Conflict(
                "the account already has a connection".to_string(),
            ));
        }
        rows.push(connection.clone());
        Ok(())
    }

    async fn find_for_account(
        &self,
        account_id: AccountId,
    ) -> Result<Option<WalletConnection>, RepositoryError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .find(|connection| connection.account_id() == account_id)
            .cloned())
    }

    async fn list_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<WalletConnection>, RepositoryError> {
        let accounts: Vec<AccountId> = self
            .accounts
            .accounts_of(user_id)
            .iter()
            .map(Account::id)
            .collect();
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .filter(|connection| accounts.contains(&connection.account_id()))
            .cloned()
            .collect())
    }

    async fn delete_for_user(&self, user_id: UserId) -> Result<u64, RepositoryError> {
        let accounts: Vec<AccountId> = self
            .accounts
            .accounts_of(user_id)
            .iter()
            .map(Account::id)
            .collect();
        let mut rows = self.rows.lock().unwrap();
        let before = rows.len();
        rows.retain(|connection| !accounts.contains(&connection.account_id()));
        Ok((before - rows.len()) as u64)
    }
}

// ---------------------------------------------------------------------------
// Holdings and sync runs
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct FakeHoldingStore {
    pub snapshots: Mutex<Vec<HoldingSnapshot>>,
}

pub struct FakeRuns {
    rows: Mutex<Vec<SyncRun>>,
    store: Arc<FakeHoldingStore>,
}

impl FakeRuns {
    pub fn new(store: Arc<FakeHoldingStore>) -> Self {
        Self {
            rows: Mutex::new(Vec::new()),
            store,
        }
    }

    pub fn all(&self) -> Vec<SyncRun> {
        self.rows.lock().unwrap().clone()
    }

    pub fn seed(&self, run: SyncRun) {
        self.rows.lock().unwrap().push(run);
    }

    pub fn snapshots(&self) -> Vec<HoldingSnapshot> {
        self.store.snapshots.lock().unwrap().clone()
    }
}

#[async_trait]
impl SyncRunRepository for FakeRuns {
    async fn claim(&self, run: &SyncRun) -> Result<(), RepositoryError> {
        let mut rows = self.rows.lock().unwrap();
        if rows.iter().any(|candidate| {
            candidate.account_id() == run.account_id()
                && candidate.status() == SyncRunStatus::Running
        }) {
            return Err(RepositoryError::Conflict(
                "a run already owns the account".to_string(),
            ));
        }
        rows.push(run.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: SyncRunId) -> Result<Option<SyncRun>, RepositoryError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .find(|run| run.id() == id)
            .cloned())
    }

    async fn running_for_account(
        &self,
        account_id: AccountId,
    ) -> Result<Option<SyncRun>, RepositoryError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .find(|run| run.account_id() == account_id && run.status() == SyncRunStatus::Running)
            .cloned())
    }

    async fn list_running(&self, now: DateTime<Utc>) -> Result<Vec<SyncRun>, RepositoryError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .filter(|run| run.is_live(now))
            .cloned()
            .collect())
    }

    async fn renew(
        &self,
        id: SyncRunId,
        lease_until: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<bool, RepositoryError> {
        let mut rows = self.rows.lock().unwrap();
        match rows.iter_mut().find(|run| run.id() == id) {
            Some(run) if run.is_live(now) => {
                let seconds = (lease_until - now).num_seconds();
                let renewed = run.renew(now, seconds);
                Ok(renewed)
            }
            _ => Ok(false),
        }
    }

    async fn finish(&self, run: &SyncRun) -> Result<(), RepositoryError> {
        let mut rows = self.rows.lock().unwrap();
        if let Some(existing) = rows.iter_mut().find(|candidate| candidate.id() == run.id()) {
            if existing.status() == SyncRunStatus::Running {
                *existing = run.clone();
            }
        }
        Ok(())
    }

    async fn write_snapshot(
        &self,
        run: &SyncRun,
        holdings: &[HoldingSnapshot],
        _last_synced_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        let rows = self.rows.lock().unwrap();
        let live = rows.iter().any(|candidate| {
            candidate.id() == run.id()
                && candidate.status() == SyncRunStatus::Running
                && candidate.lease_until() > now
        });
        if !live {
            return Err(RepositoryError::Conflict(
                "the run no longer owns the lease".to_string(),
            ));
        }
        self.store
            .snapshots
            .lock()
            .unwrap()
            .extend(holdings.iter().cloned());
        Ok(())
    }

    async fn sweep_expired(&self, now: DateTime<Utc>) -> Result<Vec<SyncRun>, RepositoryError> {
        let mut rows = self.rows.lock().unwrap();
        let mut changed = Vec::new();
        for run in rows.iter_mut() {
            if run.expire_if_due(now) {
                changed.push(run.clone());
            }
        }
        Ok(changed)
    }
}

pub struct FakeHoldings {
    store: Arc<FakeHoldingStore>,
    accounts: Arc<FakeAccounts>,
}

impl FakeHoldings {
    pub fn new(store: Arc<FakeHoldingStore>, accounts: Arc<FakeAccounts>) -> Self {
        Self { store, accounts }
    }
}

#[async_trait]
impl HoldingRepository for FakeHoldings {
    async fn latest_for_account(
        &self,
        account_id: AccountId,
    ) -> Result<Vec<HoldingSnapshot>, RepositoryError> {
        let snapshots = self.store.snapshots.lock().unwrap();
        let latest_run = snapshots
            .iter()
            .rev()
            .find(|snapshot| snapshot.account_id() == account_id)
            .map(HoldingSnapshot::sync_run_id);
        Ok(match latest_run {
            Some(run_id) => snapshots
                .iter()
                .filter(|snapshot| snapshot.sync_run_id() == run_id)
                .cloned()
                .collect(),
            None => Vec::new(),
        })
    }

    async fn snapshots_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<HoldingSnapshot>, RepositoryError> {
        let accounts: Vec<AccountId> = self
            .accounts
            .accounts_of(user_id)
            .iter()
            .map(Account::id)
            .collect();
        Ok(self
            .store
            .snapshots
            .lock()
            .unwrap()
            .iter()
            .filter(|snapshot| accounts.contains(&snapshot.account_id()))
            .cloned()
            .collect())
    }
}

// ---------------------------------------------------------------------------
// Imports
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct FakeImports {
    batches: Mutex<Vec<ImportBatch>>,
    staging: Mutex<HashMap<ImportBatchId, Vec<u8>>>,
    transactions: Mutex<Vec<ImportedTransaction>>,
    errors: Mutex<Vec<ImportRowError>>,
    accounts: Mutex<Arc<FakeAccounts>>,
}

impl FakeImports {
    pub fn new(accounts: Arc<FakeAccounts>) -> Self {
        Self {
            batches: Mutex::new(Vec::new()),
            staging: Mutex::new(HashMap::new()),
            transactions: Mutex::new(Vec::new()),
            errors: Mutex::new(Vec::new()),
            accounts: Mutex::new(accounts),
        }
    }

    pub fn batches(&self) -> Vec<ImportBatch> {
        self.batches.lock().unwrap().clone()
    }

    pub fn transactions(&self) -> Vec<ImportedTransaction> {
        self.transactions.lock().unwrap().clone()
    }

    pub fn errors(&self) -> Vec<ImportRowError> {
        self.errors.lock().unwrap().clone()
    }

    pub fn has_staging(&self, id: ImportBatchId) -> bool {
        self.staging.lock().unwrap().contains_key(&id)
    }
}

#[async_trait]
impl ImportRepository for FakeImports {
    async fn create_batch(&self, batch: &ImportBatch, bytes: &[u8]) -> Result<(), RepositoryError> {
        self.batches.lock().unwrap().push(batch.clone());
        self.staging
            .lock()
            .unwrap()
            .insert(batch.id(), bytes.to_vec());
        Ok(())
    }

    async fn find_batch(&self, id: ImportBatchId) -> Result<Option<ImportBatch>, RepositoryError> {
        Ok(self
            .batches
            .lock()
            .unwrap()
            .iter()
            .find(|batch| batch.id() == id)
            .cloned())
    }

    async fn update_batch(&self, batch: &ImportBatch) -> Result<(), RepositoryError> {
        let mut rows = self.batches.lock().unwrap();
        match rows
            .iter_mut()
            .find(|candidate| candidate.id() == batch.id())
        {
            Some(existing) => {
                *existing = batch.clone();
                Ok(())
            }
            None => Err(RepositoryError::Conflict("batch not found".to_string())),
        }
    }

    async fn claim_pending(&self) -> Result<Option<ImportBatch>, RepositoryError> {
        let mut rows = self.batches.lock().unwrap();
        let pending = rows
            .iter_mut()
            .find(|batch| batch.status() == account_domain::ImportBatchStatus::Pending);
        Ok(match pending {
            Some(batch) => {
                batch.begin_processing();
                Some(batch.clone())
            }
            None => None,
        })
    }

    async fn staged_bytes(&self, id: ImportBatchId) -> Result<Option<Vec<u8>>, RepositoryError> {
        Ok(self.staging.lock().unwrap().get(&id).cloned())
    }

    async fn delete_staging(&self, id: ImportBatchId) -> Result<(), RepositoryError> {
        self.staging.lock().unwrap().remove(&id);
        Ok(())
    }

    async fn dedupe_exists(
        &self,
        account_id: AccountId,
        dedupe_key: &str,
    ) -> Result<bool, RepositoryError> {
        Ok(self
            .transactions
            .lock()
            .unwrap()
            .iter()
            .any(|row| row.account_id() == account_id && row.dedupe_key() == dedupe_key))
    }

    async fn insert_transactions(
        &self,
        rows: &[ImportedTransaction],
    ) -> Result<u64, RepositoryError> {
        let mut stored = self.transactions.lock().unwrap();
        let mut inserted = 0;
        for row in rows {
            let duplicate = stored.iter().any(|existing| {
                existing.account_id() == row.account_id()
                    && existing.dedupe_key() == row.dedupe_key()
            });
            if !duplicate {
                stored.push(row.clone());
                inserted += 1;
            }
        }
        Ok(inserted)
    }

    async fn insert_row_errors(&self, errors: &[ImportRowError]) -> Result<(), RepositoryError> {
        self.errors.lock().unwrap().extend(errors.iter().cloned());
        Ok(())
    }

    async fn transactions_for_account(
        &self,
        account_id: AccountId,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<ImportedTransaction>, RepositoryError> {
        Ok(self
            .transactions
            .lock()
            .unwrap()
            .iter()
            .filter(|row| {
                row.account_id() == account_id
                    && since.is_none_or(|since| row.executed_at() >= since)
            })
            .cloned()
            .collect())
    }

    async fn transactions_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<ImportedTransaction>, RepositoryError> {
        let accounts: Vec<AccountId> = self
            .accounts
            .lock()
            .unwrap()
            .accounts_of(user_id)
            .iter()
            .map(Account::id)
            .collect();
        Ok(self
            .transactions
            .lock()
            .unwrap()
            .iter()
            .filter(|row| accounts.contains(&row.account_id()))
            .cloned()
            .collect())
    }
}

// ---------------------------------------------------------------------------
// Privacy
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct FakePrivacy {
    rows: Mutex<Vec<AnonymisationRecord>>,
}

#[async_trait]
impl PrivacyRepository for FakePrivacy {
    async fn find_anonymisation(
        &self,
        user_id: UserId,
    ) -> Result<Option<AnonymisationRecord>, RepositoryError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .find(|record| record.user_id == user_id)
            .cloned())
    }

    async fn record_anonymisation(
        &self,
        record: &AnonymisationRecord,
    ) -> Result<(), RepositoryError> {
        self.rows.lock().unwrap().push(record.clone());
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Providers and external clients
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct ExchangeBehaviour {
    pub verification: Result<VerifiedPermissions, ExternalError>,
    pub balances: Result<Vec<ObservedBalance>, ExternalError>,
}

impl Default for ExchangeBehaviour {
    fn default() -> Self {
        Self {
            verification: Ok(VerifiedPermissions { read_only: true }),
            balances: Ok(Vec::new()),
        }
    }
}

pub struct FakeExchange {
    behaviour: Mutex<ExchangeBehaviour>,
    pub verify_calls: AtomicUsize,
    pub balance_calls: AtomicUsize,
}

impl Default for FakeExchange {
    fn default() -> Self {
        Self {
            behaviour: Mutex::new(ExchangeBehaviour::default()),
            verify_calls: AtomicUsize::new(0),
            balance_calls: AtomicUsize::new(0),
        }
    }
}

impl FakeExchange {
    pub fn set_verification(&self, verification: Result<VerifiedPermissions, ExternalError>) {
        self.behaviour.lock().unwrap().verification = verification;
    }

    pub fn set_balances(&self, balances: Result<Vec<ObservedBalance>, ExternalError>) {
        self.behaviour.lock().unwrap().balances = balances;
    }
}

#[async_trait]
impl ExchangeProvider for FakeExchange {
    async fn verify_read_only(
        &self,
        _venue: &str,
        _credentials: &ExchangeCredentials,
    ) -> Result<VerifiedPermissions, ExternalError> {
        self.verify_calls.fetch_add(1, Ordering::Relaxed);
        self.behaviour.lock().unwrap().verification.clone()
    }

    async fn balances(
        &self,
        _venue: &str,
        _credentials: &ExchangeCredentials,
    ) -> Result<Vec<ObservedBalance>, ExternalError> {
        self.balance_calls.fetch_add(1, Ordering::Relaxed);
        self.behaviour.lock().unwrap().balances.clone()
    }
}

pub struct FakeChain {
    balances: Mutex<Result<Vec<ObservedBalance>, ExternalError>>,
    pub calls: AtomicUsize,
}

impl Default for FakeChain {
    fn default() -> Self {
        Self {
            balances: Mutex::new(Ok(Vec::new())),
            calls: AtomicUsize::new(0),
        }
    }
}

impl FakeChain {
    pub fn set_balances(&self, balances: Result<Vec<ObservedBalance>, ExternalError>) {
        *self.balances.lock().unwrap() = balances;
    }
}

#[async_trait]
impl ChainProvider for FakeChain {
    async fn balances(
        &self,
        _chain: &str,
        _address: &str,
    ) -> Result<Vec<ObservedBalance>, ExternalError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        self.balances.lock().unwrap().clone()
    }
}

pub struct FakeRegistry {
    chains: Mutex<Vec<SupportedChain>>,
    resolutions: Mutex<HashMap<String, Option<String>>>,
    fail: Mutex<bool>,
    pub resolve_calls: AtomicUsize,
}

impl Default for FakeRegistry {
    fn default() -> Self {
        Self {
            chains: Mutex::new(vec![
                SupportedChain {
                    id: "evm:1".to_string(),
                    native_symbol: "ETH".to_string(),
                },
                SupportedChain {
                    id: "evm:137".to_string(),
                    native_symbol: "POL".to_string(),
                },
                SupportedChain {
                    id: "solana".to_string(),
                    native_symbol: "SOL".to_string(),
                },
            ]),
            resolutions: Mutex::new(HashMap::new()),
            fail: Mutex::new(false),
            resolve_calls: AtomicUsize::new(0),
        }
    }
}

impl FakeRegistry {
    pub fn set_resolution(&self, token: &str, asset: Option<&str>) {
        self.resolutions
            .lock()
            .unwrap()
            .insert(token.to_string(), asset.map(ToString::to_string));
    }

    pub fn set_fail(&self, fail: bool) {
        *self.fail.lock().unwrap() = fail;
    }

    pub fn supported(&self) -> Vec<SupportedChain> {
        self.chains.lock().unwrap().clone()
    }
}

#[async_trait]
impl AssetRegistry for FakeRegistry {
    async fn resolve(&self, observation: &Observation) -> Result<Option<String>, ExternalError> {
        self.resolve_calls.fetch_add(1, Ordering::Relaxed);
        if *self.fail.lock().unwrap() {
            return Err(ExternalError::Unavailable("registry is down".to_string()));
        }
        let key = observation
            .symbol
            .clone()
            .or_else(|| observation.contract_address.clone())
            .unwrap_or_default();
        Ok(self
            .resolutions
            .lock()
            .unwrap()
            .get(&key)
            .cloned()
            .unwrap_or(None))
    }

    async fn supported_chains(&self) -> Result<Vec<SupportedChain>, ExternalError> {
        Ok(self.chains.lock().unwrap().clone())
    }
}

#[derive(Default)]
pub struct FakeVault {
    secrets: Mutex<HashMap<String, String>>,
    next: AtomicUsize,
    destroyed: Mutex<Vec<String>>,
    pub store_calls: AtomicUsize,
}

impl FakeVault {
    pub fn secret(&self, reference: &str) -> Option<String> {
        self.secrets.lock().unwrap().get(reference).cloned()
    }

    pub fn destroyed(&self) -> Vec<String> {
        self.destroyed.lock().unwrap().clone()
    }

    pub fn seed(&self, reference: &str, secret: &str) {
        self.secrets
            .lock()
            .unwrap()
            .insert(reference.to_string(), secret.to_string());
    }
}

#[async_trait]
impl VaultClient for FakeVault {
    async fn store(&self, _owner: &str, secret: &str) -> Result<String, ExternalError> {
        self.store_calls.fetch_add(1, Ordering::Relaxed);
        let reference = format!(
            "vault://account/{}",
            self.next.fetch_add(1, Ordering::Relaxed)
        );
        self.secrets
            .lock()
            .unwrap()
            .insert(reference.clone(), secret.to_string());
        Ok(reference)
    }

    async fn reveal(&self, reference: &str) -> Result<String, ExternalError> {
        self.secrets
            .lock()
            .unwrap()
            .get(reference)
            .cloned()
            .ok_or_else(|| ExternalError::Refused("the secret was destroyed".to_string()))
    }

    async fn destroy(&self, reference: &str) -> Result<(), ExternalError> {
        self.secrets.lock().unwrap().remove(reference);
        self.destroyed.lock().unwrap().push(reference.to_string());
        Ok(())
    }
}

#[derive(Default)]
pub struct FakeAuthAdmin {
    calls: Mutex<Vec<(UserId, bool)>>,
}

impl FakeAuthAdmin {
    pub fn calls(&self) -> Vec<(UserId, bool)> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl AuthAdmin for FakeAuthAdmin {
    async fn set_has_exchange_credentials(
        &self,
        user_id: UserId,
        value: bool,
    ) -> Result<(), ExternalError> {
        self.calls.lock().unwrap().push((user_id, value));
        Ok(())
    }
}

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

    pub fn actions(&self) -> Vec<String> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .map(|event| event.action().to_string())
            .collect()
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
pub struct FakeNotifications {
    sent: Mutex<Vec<(UserId, String, String)>>,
}

impl FakeNotifications {
    pub fn sent(&self) -> Vec<(UserId, String, String)> {
        self.sent.lock().unwrap().clone()
    }
}

#[async_trait]
impl NotificationClient for FakeNotifications {
    async fn notify(
        &self,
        user_id: UserId,
        alert_type: &str,
        idempotency_key: &str,
        _payload: Value,
    ) -> Result<(), ExternalError> {
        self.sent.lock().unwrap().push((
            user_id,
            alert_type.to_string(),
            idempotency_key.to_string(),
        ));
        Ok(())
    }
}

#[derive(Default)]
pub struct FakeWake {
    pub calls: AtomicUsize,
    fail: Mutex<bool>,
}

impl FakeWake {
    pub fn set_fail(&self, fail: bool) {
        *self.fail.lock().unwrap() = fail;
    }
}

#[async_trait]
impl WakeTransport for FakeWake {
    async fn wake(&self) -> Result<(), String> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        if *self.fail.lock().unwrap() {
            Err("the worker is unreachable".to_string())
        } else {
            Ok(())
        }
    }
}

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

pub struct FakeTokenValidator {
    tokens: HashMap<String, Result<VerifiedClaims, AuthError>>,
}

impl FakeTokenValidator {
    pub fn with(entries: Vec<(&str, Result<VerifiedClaims, AuthError>)>) -> Self {
        Self {
            tokens: entries
                .into_iter()
                .map(|(token, claims)| (token.to_string(), claims))
                .collect(),
        }
    }
}

#[async_trait]
impl TokenValidator for FakeTokenValidator {
    async fn validate(&self, token: &str) -> Result<VerifiedClaims, AuthError> {
        self.tokens
            .get(token)
            .cloned()
            .unwrap_or(Err(AuthError::Invalid("unknown token".to_string())))
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

/// A helper for assertions over an audit event's context.
pub fn context_of(record: &AuditRecord) -> BTreeMap<String, String> {
    record.context().clone()
}
