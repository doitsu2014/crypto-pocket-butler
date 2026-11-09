//! In-memory port fakes for the HTTP boundary tests. No database, no network.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ledger_application::ports::{
    AccountClient, AnonymisationRepository, AnonymiserGateway, AssetResolver, AuditClient,
    AuditRecord, Clock, CorrectionRepository, ExternalError, FxQuote, Holding, IdGenerator,
    LotRepository, MarketDataClient, Pause, PriceQuote, ReconcileLease, RepositoryError,
    SnapshotRepository, SubjectRepository, TransactionCursor, TransactionRepository,
    TransferPairRepository,
};
use ledger_domain::{
    Correction, CostBasisLot, IncomingTransaction, LedgerSubject, LedgerSubjectId, PnlSnapshot,
    Transaction, TransactionId, TransferPair,
};
use rust_decimal::Decimal;
use uuid::Uuid;

/// A clock a test moves explicitly.
pub struct MutableClock {
    now: Mutex<DateTime<Utc>>,
}

impl MutableClock {
    pub fn at(now: DateTime<Utc>) -> Self {
        Self {
            now: Mutex::new(now),
        }
    }
}

impl Clock for MutableClock {
    fn now(&self) -> DateTime<Utc> {
        *self.now.lock().unwrap()
    }
}

/// A deterministic identifier source.
#[derive(Default)]
pub struct SequentialIds {
    next: AtomicU64,
}

impl IdGenerator for SequentialIds {
    fn next(&self) -> Uuid {
        Uuid::from_u128(u128::from(self.next.fetch_add(1, Ordering::SeqCst) + 1))
    }
}

/// A pause that never sleeps.
pub struct NoPause;

#[async_trait]
impl Pause for NoPause {
    async fn pause_for(&self, _duration: Duration) {}
}

#[derive(Default)]
pub struct FakeTransactions {
    pub stored: Mutex<Vec<Transaction>>,
}

#[async_trait]
impl TransactionRepository for FakeTransactions {
    async fn insert(&self, transaction: &Transaction) -> Result<(), RepositoryError> {
        let mut stored = self.stored.lock().unwrap();
        if stored
            .iter()
            .any(|row| row.canonical_key() == transaction.canonical_key())
        {
            return Err(RepositoryError::Conflict("duplicate".to_string()));
        }
        if let Some(key) = transaction.external_key() {
            if stored.iter().any(|row| row.external_key() == Some(key)) {
                return Err(RepositoryError::Conflict("duplicate".to_string()));
            }
        }
        stored.push(transaction.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: TransactionId) -> Result<Option<Transaction>, RepositoryError> {
        Ok(self
            .stored
            .lock()
            .unwrap()
            .iter()
            .find(|row| row.id() == id)
            .cloned())
    }

    async fn canonical_exists(&self, key: &str) -> Result<bool, RepositoryError> {
        Ok(self
            .stored
            .lock()
            .unwrap()
            .iter()
            .any(|row| row.canonical_key() == key))
    }

    async fn external_exists(&self, key: &str) -> Result<bool, RepositoryError> {
        Ok(self
            .stored
            .lock()
            .unwrap()
            .iter()
            .any(|row| row.external_key() == Some(key)))
    }

    async fn list_page(
        &self,
        subject: LedgerSubjectId,
        after: Option<TransactionCursor>,
        limit: u32,
    ) -> Result<Vec<Transaction>, RepositoryError> {
        let mut rows: Vec<Transaction> = self
            .stored
            .lock()
            .unwrap()
            .iter()
            .filter(|row| row.subject_id() == subject)
            .cloned()
            .collect();
        rows.sort_by(|left, right| {
            (right.executed_at(), right.id()).cmp(&(left.executed_at(), left.id()))
        });
        if let Some(cursor) = after {
            rows.retain(|row| (row.executed_at(), row.id()) < (cursor.executed_at, cursor.id));
        }
        rows.truncate(limit as usize);
        Ok(rows)
    }

    async fn list_all(
        &self,
        subject: LedgerSubjectId,
    ) -> Result<Vec<Transaction>, RepositoryError> {
        let mut rows: Vec<Transaction> = self
            .stored
            .lock()
            .unwrap()
            .iter()
            .filter(|row| row.subject_id() == subject)
            .cloned()
            .collect();
        rows.sort_by_key(|row| (row.executed_at(), row.id()));
        Ok(rows)
    }

    async fn last_recorded_at(
        &self,
        subject: LedgerSubjectId,
    ) -> Result<Option<DateTime<Utc>>, RepositoryError> {
        Ok(self
            .stored
            .lock()
            .unwrap()
            .iter()
            .filter(|row| row.subject_id() == subject)
            .map(Transaction::recorded_at)
            .max())
    }

    async fn list_subjects(&self) -> Result<Vec<LedgerSubjectId>, RepositoryError> {
        let subjects: BTreeSet<LedgerSubjectId> = self
            .stored
            .lock()
            .unwrap()
            .iter()
            .map(Transaction::subject_id)
            .collect();
        Ok(subjects.into_iter().collect())
    }

    async fn count_for_subject(&self, subject: LedgerSubjectId) -> Result<u64, RepositoryError> {
        Ok(self
            .stored
            .lock()
            .unwrap()
            .iter()
            .filter(|row| row.subject_id() == subject)
            .count() as u64)
    }
}

pub struct FakeCorrections {
    pub stored: Mutex<Vec<Correction>>,
    transactions: Arc<FakeTransactions>,
}

impl FakeCorrections {
    pub fn new(transactions: Arc<FakeTransactions>) -> Self {
        Self {
            stored: Mutex::new(Vec::new()),
            transactions,
        }
    }
}

#[async_trait]
impl CorrectionRepository for FakeCorrections {
    async fn insert(&self, correction: &Correction) -> Result<(), RepositoryError> {
        self.stored.lock().unwrap().push(correction.clone());
        Ok(())
    }

    async fn list_for_transaction(
        &self,
        transaction: TransactionId,
    ) -> Result<Vec<Correction>, RepositoryError> {
        let mut rows: Vec<Correction> = self
            .stored
            .lock()
            .unwrap()
            .iter()
            .filter(|row| row.transaction_id() == transaction)
            .cloned()
            .collect();
        rows.sort_by_key(|row| (row.corrected_at(), row.id().as_uuid()));
        Ok(rows)
    }

    async fn list_for_subject(
        &self,
        subject: LedgerSubjectId,
    ) -> Result<Vec<Correction>, RepositoryError> {
        let ids: BTreeSet<TransactionId> = self
            .transactions
            .stored
            .lock()
            .unwrap()
            .iter()
            .filter(|row| row.subject_id() == subject)
            .map(Transaction::id)
            .collect();
        Ok(self
            .stored
            .lock()
            .unwrap()
            .iter()
            .filter(|row| ids.contains(&row.transaction_id()))
            .cloned()
            .collect())
    }

    async fn last_corrected_at(
        &self,
        subject: LedgerSubjectId,
    ) -> Result<Option<DateTime<Utc>>, RepositoryError> {
        Ok(self
            .list_for_subject(subject)
            .await?
            .iter()
            .map(Correction::corrected_at)
            .max())
    }
}

#[derive(Default)]
pub struct FakeSubjects {
    pub stored: Mutex<Vec<LedgerSubject>>,
}

impl FakeSubjects {
    pub fn anonymise(&self, id: LedgerSubjectId, anonymised_ref: &str) {
        let mut stored = self.stored.lock().unwrap();
        if let Some(index) = stored.iter().position(|subject| subject.id() == id) {
            let created = stored[index].created_at();
            stored[index] =
                LedgerSubject::from_stored(id, None, Some(anonymised_ref.to_string()), created);
        }
    }
}

#[async_trait]
impl SubjectRepository for FakeSubjects {
    async fn find_by_user(&self, user_id: &str) -> Result<Option<LedgerSubject>, RepositoryError> {
        Ok(self
            .stored
            .lock()
            .unwrap()
            .iter()
            .find(|subject| subject.user_id() == Some(user_id))
            .cloned())
    }

    async fn find_by_id(
        &self,
        id: LedgerSubjectId,
    ) -> Result<Option<LedgerSubject>, RepositoryError> {
        Ok(self
            .stored
            .lock()
            .unwrap()
            .iter()
            .find(|subject| subject.id() == id)
            .cloned())
    }

    async fn create(&self, subject: &LedgerSubject) -> Result<(), RepositoryError> {
        let mut stored = self.stored.lock().unwrap();
        if stored.iter().any(|row| row.user_id() == subject.user_id()) {
            return Err(RepositoryError::Conflict("duplicate".to_string()));
        }
        stored.push(subject.clone());
        Ok(())
    }

    async fn list_ids(&self) -> Result<Vec<LedgerSubjectId>, RepositoryError> {
        Ok(self
            .stored
            .lock()
            .unwrap()
            .iter()
            .map(LedgerSubject::id)
            .collect())
    }
}

#[derive(Default)]
pub struct FakeLots {
    pub stored: Mutex<BTreeMap<LedgerSubjectId, Vec<CostBasisLot>>>,
}

#[async_trait]
impl LotRepository for FakeLots {
    async fn replace_for_subject(
        &self,
        subject: LedgerSubjectId,
        lots: &[CostBasisLot],
    ) -> Result<(), RepositoryError> {
        self.stored.lock().unwrap().insert(subject, lots.to_vec());
        Ok(())
    }

    async fn list_open(
        &self,
        subject: LedgerSubjectId,
    ) -> Result<Vec<CostBasisLot>, RepositoryError> {
        Ok(self
            .stored
            .lock()
            .unwrap()
            .get(&subject)
            .cloned()
            .unwrap_or_default())
    }
}

#[derive(Default)]
pub struct FakeSnapshots {
    pub stored: Mutex<Vec<PnlSnapshot>>,
}

#[async_trait]
impl SnapshotRepository for FakeSnapshots {
    async fn upsert(&self, snapshot: &PnlSnapshot) -> Result<(), RepositoryError> {
        let mut stored = self.stored.lock().unwrap();
        stored.retain(|row| {
            !(row.subject_id() == snapshot.subject_id()
                && row.portfolio_id() == snapshot.portfolio_id())
        });
        stored.push(snapshot.clone());
        Ok(())
    }

    async fn find_latest(
        &self,
        subject: LedgerSubjectId,
        portfolio_id: Option<Uuid>,
    ) -> Result<Option<PnlSnapshot>, RepositoryError> {
        Ok(self
            .stored
            .lock()
            .unwrap()
            .iter()
            .filter(|row| row.subject_id() == subject && row.portfolio_id() == portfolio_id)
            .max_by_key(|row| row.computed_at())
            .cloned())
    }
}

#[derive(Default)]
pub struct FakePairs {
    pub stored: Mutex<Vec<TransferPair>>,
}

#[async_trait]
impl TransferPairRepository for FakePairs {
    async fn insert(&self, pair: &TransferPair) -> Result<(), RepositoryError> {
        self.stored.lock().unwrap().push(pair.clone());
        Ok(())
    }

    async fn list_for_subject(
        &self,
        subject: LedgerSubjectId,
    ) -> Result<Vec<TransferPair>, RepositoryError> {
        Ok(self
            .stored
            .lock()
            .unwrap()
            .iter()
            .filter(|pair| pair.subject_id() == subject)
            .cloned()
            .collect())
    }

    async fn mark_matched(
        &self,
        out_transaction: TransactionId,
        in_transaction: TransactionId,
        at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        if let Some(pair) = self
            .stored
            .lock()
            .unwrap()
            .iter_mut()
            .find(|pair| pair.out_transaction_id() == Some(out_transaction))
        {
            pair.mark_matched(out_transaction, in_transaction, at);
        }
        Ok(())
    }

    async fn match_pending_in(
        &self,
        in_transaction: TransactionId,
        out_transaction: TransactionId,
        at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        if let Some(pair) = self
            .stored
            .lock()
            .unwrap()
            .iter_mut()
            .find(|pair| pair.in_transaction_id() == Some(in_transaction))
        {
            pair.mark_matched(out_transaction, in_transaction, at);
        }
        Ok(())
    }

    async fn mark_rejected(&self, out_transaction: TransactionId) -> Result<(), RepositoryError> {
        if let Some(pair) = self
            .stored
            .lock()
            .unwrap()
            .iter_mut()
            .find(|pair| pair.out_transaction_id() == Some(out_transaction))
        {
            pair.reject();
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct FakeAnonymisations {
    pub stored: Mutex<BTreeMap<String, String>>,
}

#[async_trait]
impl AnonymisationRepository for FakeAnonymisations {
    async fn find(&self, idempotency_key: &str) -> Result<Option<String>, RepositoryError> {
        Ok(self.stored.lock().unwrap().get(idempotency_key).cloned())
    }

    async fn record(
        &self,
        idempotency_key: &str,
        anonymised_ref: &str,
        _at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        self.stored
            .lock()
            .unwrap()
            .insert(idempotency_key.to_string(), anonymised_ref.to_string());
        Ok(())
    }
}

#[derive(Default)]
pub struct FakeLease {
    pub leased_until: Mutex<Option<DateTime<Utc>>>,
    pub last_completed: Mutex<Option<DateTime<Utc>>>,
}

#[async_trait]
impl ReconcileLease for FakeLease {
    async fn acquire(
        &self,
        now: DateTime<Utc>,
        until: DateTime<Utc>,
    ) -> Result<bool, RepositoryError> {
        let mut leased = self.leased_until.lock().unwrap();
        if leased.is_some_and(|existing| existing > now) {
            return Ok(false);
        }
        *leased = Some(until);
        Ok(true)
    }

    async fn release(&self, now: DateTime<Utc>) -> Result<(), RepositoryError> {
        *self.leased_until.lock().unwrap() = None;
        *self.last_completed.lock().unwrap() = Some(now);
        Ok(())
    }

    async fn last_completed(&self) -> Result<Option<DateTime<Utc>>, RepositoryError> {
        Ok(*self.last_completed.lock().unwrap())
    }
}

#[derive(Default)]
pub struct FakeAccount {
    pub rows: Mutex<Vec<IncomingTransaction>>,
    pub holdings: Mutex<Vec<Holding>>,
}

#[async_trait]
impl AccountClient for FakeAccount {
    async fn transactions_for_user(
        &self,
        _user_id: &str,
    ) -> Result<Vec<IncomingTransaction>, ExternalError> {
        Ok(self.rows.lock().unwrap().clone())
    }

    async fn holdings_for_user(&self, _user_id: &str) -> Result<Vec<Holding>, ExternalError> {
        Ok(self.holdings.lock().unwrap().clone())
    }
}

/// A C6 client with configured prices and FX.
#[derive(Default)]
pub struct FakeMarketData {
    pub prices: Mutex<BTreeMap<String, PriceQuote>>,
    pub fx: Mutex<BTreeMap<(String, String), FxQuote>>,
    pub failing: Mutex<bool>,
}

impl FakeMarketData {
    /// Sets one USD price.
    pub fn set_price(&self, asset_id: &str, price: &str) {
        self.prices.lock().unwrap().insert(
            asset_id.to_string(),
            PriceQuote {
                asset_id: asset_id.to_string(),
                price: Some(dec(price)),
                currency: Some("USD".to_string()),
                as_of: Some(Utc::now()),
                freshness: "live".to_string(),
            },
        );
    }
}

#[async_trait]
impl MarketDataClient for FakeMarketData {
    async fn prices(&self, asset_ids: &[String]) -> Result<Vec<PriceQuote>, ExternalError> {
        if *self.failing.lock().unwrap() {
            return Err(ExternalError::Unavailable("C6 is down".to_string()));
        }
        let prices = self.prices.lock().unwrap();
        Ok(asset_ids
            .iter()
            .filter_map(|asset| prices.get(asset).cloned())
            .collect())
    }

    async fn fx(&self, base: &str, quote: &str) -> Result<FxQuote, ExternalError> {
        if *self.failing.lock().unwrap() {
            return Err(ExternalError::Unavailable("C6 is down".to_string()));
        }
        self.fx
            .lock()
            .unwrap()
            .get(&(base.to_string(), quote.to_string()))
            .cloned()
            .ok_or_else(|| ExternalError::Refused(format!("no FX rate from {quote} to {base}")))
    }
}

#[derive(Default)]
pub struct FakeAssets {
    pub known: Mutex<BTreeSet<String>>,
}

impl FakeAssets {
    pub fn knowing(assets: &[&str]) -> Self {
        Self {
            known: Mutex::new(assets.iter().map(|asset| (*asset).to_string()).collect()),
        }
    }
}

#[async_trait]
impl AssetResolver for FakeAssets {
    async fn exists(&self, asset_id: &str) -> Result<bool, ExternalError> {
        Ok(self.known.lock().unwrap().contains(asset_id))
    }
}

#[derive(Default)]
pub struct FakeAudit {
    pub records: Mutex<Vec<AuditRecord>>,
}

#[async_trait]
impl AuditClient for FakeAudit {
    async fn append(&self, record: &AuditRecord) -> Result<(), ExternalError> {
        self.records.lock().unwrap().push(record.clone());
        Ok(())
    }
}

pub struct FakeAnonymiser {
    pub subjects: Arc<FakeSubjects>,
}

#[async_trait]
impl AnonymiserGateway for FakeAnonymiser {
    async fn anonymise(
        &self,
        subject: LedgerSubjectId,
        anonymised_ref: &str,
    ) -> Result<(), RepositoryError> {
        self.subjects.anonymise(subject, anonymised_ref);
        Ok(())
    }
}

pub fn dec(raw: &str) -> Decimal {
    raw.parse().unwrap()
}
