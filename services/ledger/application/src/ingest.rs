//! WF1 and WF2: ingest from C5 with canonical dedupe, and transfer pairing
//! (BR1.1, BR1.2, BR1.4, BR2.4, BR6.1).

use std::collections::BTreeSet;
use std::sync::Arc;

use ledger_domain::{
    find_match, IncomingTransaction, Kind, LedgerSubject, LedgerSubjectId, NewTransaction,
    PriceSource, Source, Transaction, TransactionId, TransferPair, TransferPairId, TransferStatus,
};

use crate::common::AppError;
use crate::ports::{
    AccountClient, AssetResolver, Clock, IdGenerator, RepositoryError, SubjectRepository,
    TransactionRepository, TransferPairRepository,
};

/// One row that could not be stored, with the reason only (never an amount).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowRefusal {
    /// The source row identifier, when present.
    pub external_ref: Option<String>,
    /// Why the row was refused.
    pub reason: String,
}

/// The outcome of one ingest run.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IngestReport {
    /// The stored transaction identifiers.
    pub stored: Vec<TransactionId>,
    /// How many rows were skipped as duplicates (BR1.2).
    pub duplicates: u64,
    /// The rows refused for a line or validation error (BR1.4).
    pub refused: Vec<RowRefusal>,
}

/// What happened to one row.
enum StoreOutcome {
    Stored(TransactionId),
    Duplicate,
    Refused(String),
}

/// Reads C5, dedupes, stores, and pairs transfers.
pub struct IngestService {
    transactions: Arc<dyn TransactionRepository>,
    subjects: Arc<dyn SubjectRepository>,
    pairs: Arc<dyn TransferPairRepository>,
    account: Arc<dyn AccountClient>,
    assets: Arc<dyn AssetResolver>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
    base_currency: String,
}

impl IngestService {
    /// Creates the ingest use case.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        transactions: Arc<dyn TransactionRepository>,
        subjects: Arc<dyn SubjectRepository>,
        pairs: Arc<dyn TransferPairRepository>,
        account: Arc<dyn AccountClient>,
        assets: Arc<dyn AssetResolver>,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
        base_currency: impl Into<String>,
    ) -> Self {
        Self {
            transactions,
            subjects,
            pairs,
            account,
            assets,
            clock,
            ids,
            base_currency: ledger_domain::normalise_currency(&base_currency.into()),
        }
    }

    /// Ensures the mutable subject for a user exists.
    ///
    /// # Errors
    /// Returns [`AppError`] when the read, the insert, or the domain validation
    /// fails.
    pub async fn ensure_subject(&self, user_id: &str) -> Result<LedgerSubject, AppError> {
        if let Some(subject) = self.subjects.find_by_user(user_id).await? {
            return Ok(subject);
        }
        let subject = LedgerSubject::new(
            LedgerSubjectId::new(self.ids.next()),
            user_id,
            self.clock.now(),
        )?;
        match self.subjects.create(&subject).await {
            Ok(()) => Ok(subject),
            Err(RepositoryError::Conflict(_)) => {
                self.subjects.find_by_user(user_id).await?.ok_or_else(|| {
                    AppError::Conflict("the subject could not be created".to_string())
                })
            }
            Err(error) => Err(AppError::Repository(error)),
        }
    }

    /// Ingests every C5 row for a user. A line error is reported and skipped;
    /// a canonical or external duplicate is skipped and counted.
    ///
    /// # Errors
    /// Returns [`AppError`] when C5, C4, or the store fails.
    pub async fn ingest_user(&self, user_id: &str) -> Result<IngestReport, AppError> {
        let subject = self.ensure_subject(user_id).await?;
        let rows = self.account.transactions_for_user(user_id).await?;
        let mut report = IngestReport::default();
        for row in rows {
            let external_ref = row.external_ref.clone();
            match self.store_row(&subject, user_id, row).await? {
                StoreOutcome::Stored(id) => report.stored.push(id),
                StoreOutcome::Duplicate => report.duplicates += 1,
                StoreOutcome::Refused(reason) => report.refused.push(RowRefusal {
                    external_ref,
                    reason,
                }),
            }
        }
        Ok(report)
    }

    async fn store_row(
        &self,
        subject: &LedgerSubject,
        user_id: &str,
        row: IncomingTransaction,
    ) -> Result<StoreOutcome, AppError> {
        if let Some(reason) = row.refusal() {
            return Ok(StoreOutcome::Refused(reason));
        }
        let kind = row.kind.expect("a refused row has no kind");
        let quantity = row.quantity.expect("a refused row has no quantity");
        let executed_at = row.executed_at.expect("a refused row has no time");
        if !self
            .assets
            .exists(&row.asset_id)
            .await
            .map_err(AppError::External)?
        {
            return Ok(StoreOutcome::Refused(
                "assetId is not registered".to_string(),
            ));
        }
        let price_source = match (row.source, row.price) {
            (Source::Venue, Some(_)) => Some(PriceSource::Venue),
            (Source::Csv, Some(_)) => Some(PriceSource::Csv),
            (Source::RecordedTrade, Some(_)) => Some(PriceSource::User),
            (_, None) => None,
        };
        let transaction = match Transaction::record(
            TransactionId::new(self.ids.next()),
            NewTransaction {
                subject_id: subject.id(),
                user_id: user_id.to_string(),
                account_id: row.account_id,
                asset_id: row.asset_id,
                kind,
                quantity,
                price: row.price,
                price_source,
                currency: row.currency.unwrap_or_else(|| self.base_currency.clone()),
                source: row.source,
                external_ref: row.external_ref,
                executed_at,
                recorded_at: self.clock.now(),
            },
        ) {
            Ok(transaction) => transaction,
            Err(error) => return Ok(StoreOutcome::Refused(error.to_string())),
        };
        if self
            .transactions
            .canonical_exists(transaction.canonical_key())
            .await?
        {
            return Ok(StoreOutcome::Duplicate);
        }
        if let Some(external_key) = transaction.external_key() {
            if self.transactions.external_exists(external_key).await? {
                return Ok(StoreOutcome::Duplicate);
            }
        }
        match self.transactions.insert(&transaction).await {
            Ok(()) => {
                if transaction.kind().is_transfer() {
                    self.pair_transfer(subject.id(), &transaction).await?;
                }
                Ok(StoreOutcome::Stored(transaction.id()))
            }
            Err(RepositoryError::Conflict(_)) => Ok(StoreOutcome::Duplicate),
            Err(error) => Err(AppError::Repository(error)),
        }
    }

    /// Pairs a freshly stored transfer leg with the nearest unmatched
    /// counter-entry (BR6.1). A lone leg gets a pending pair row so a later
    /// counter-entry can match it in either ingest order.
    async fn pair_transfer(
        &self,
        subject: LedgerSubjectId,
        transaction: &Transaction,
    ) -> Result<(), AppError> {
        match transaction.kind() {
            Kind::TransferOut => {
                let existing = self.transactions.list_all(subject).await?;
                let pairs = self.pairs.list_for_subject(subject).await?;
                let pending_ins: BTreeSet<TransactionId> = pairs
                    .iter()
                    .filter(|pair| pair.status() == TransferStatus::Pending)
                    .filter_map(TransferPair::in_transaction_id)
                    .collect();
                let candidates: Vec<Transaction> = existing
                    .iter()
                    .filter(|candidate| pending_ins.contains(&candidate.id()))
                    .cloned()
                    .collect();
                if let Some(in_id) = find_match(transaction, &candidates) {
                    self.pairs
                        .match_pending_in(in_id, transaction.id(), self.clock.now())
                        .await?;
                    return Ok(());
                }
                let pair = TransferPair::pending(
                    TransferPairId::new(self.ids.next()),
                    subject,
                    transaction.id(),
                    transaction.asset_id(),
                );
                self.pairs.insert(&pair).await?;
                // A legacy lone transfer_in may predate pending-in pairs.
                let paired_ins: BTreeSet<TransactionId> = pairs
                    .iter()
                    .filter_map(TransferPair::in_transaction_id)
                    .collect();
                let legacy: Vec<Transaction> = existing
                    .into_iter()
                    .filter(|candidate| candidate.kind() == Kind::TransferIn)
                    .filter(|candidate| !paired_ins.contains(&candidate.id()))
                    .collect();
                if let Some(in_id) = find_match(transaction, &legacy) {
                    self.pairs
                        .mark_matched(transaction.id(), in_id, self.clock.now())
                        .await?;
                }
            }
            Kind::TransferIn => {
                let existing = self.transactions.list_all(subject).await?;
                let pairs = self.pairs.list_for_subject(subject).await?;
                let pending_out: BTreeSet<TransactionId> = pairs
                    .iter()
                    .filter(|pair| pair.status() == TransferStatus::Pending)
                    .filter_map(TransferPair::out_transaction_id)
                    .collect();
                let candidates: Vec<Transaction> = existing
                    .into_iter()
                    .filter(|candidate| candidate.kind() == Kind::TransferOut)
                    .filter(|candidate| pending_out.contains(&candidate.id()))
                    .collect();
                if let Some(out_id) = find_match(transaction, &candidates) {
                    self.pairs
                        .mark_matched(out_id, transaction.id(), self.clock.now())
                        .await?;
                } else {
                    let pair = TransferPair::pending_in(
                        TransferPairId::new(self.ids.next()),
                        subject,
                        transaction.id(),
                        transaction.asset_id(),
                    );
                    self.pairs.insert(&pair).await?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
