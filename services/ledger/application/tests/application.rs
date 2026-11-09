//! Application use-case tests against port fakes — no database, no network.

mod fakes;

use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};
use ledger_application::{
    decode_cursor, AppError, CorrectionResult, CorrectionsService, Holding, IngestService,
    PnlService, PrivacyService, ReadService, Rebuilder, ReconcileService, ReconcileStatus,
    ServiceCaller, TradeCommand, TradesService, Trail,
};
use ledger_domain::{
    CorrectionField, IncomingTransaction, Kind, PriceSource, Source, Transaction, TransactionId,
};
use uuid::Uuid;

use fakes::{
    dec, FakeAccount, FakeAnonymisations, FakeAnonymiser, FakeAssets, FakeAudit, FakeCorrections,
    FakeLease, FakeLots, FakeMarketData, FakePairs, FakeSnapshots, FakeSubjects, FakeTransactions,
    MutableClock, RecordedPause, SequentialIds,
};

const USER: &str = "11111111-1111-4111-8111-111111111111";
const OTHER: &str = "22222222-2222-4222-8222-222222222222";

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 6, hour, 0, 0).unwrap()
}

fn row(
    asset: &str,
    kind: Kind,
    quantity: &str,
    price: Option<&str>,
    hour: u32,
    external_ref: Option<&str>,
) -> IncomingTransaction {
    IncomingTransaction {
        account_id: None,
        asset_id: asset.to_string(),
        kind: Some(kind),
        quantity: Some(dec(quantity)),
        price: price.map(dec),
        currency: None,
        executed_at: Some(at(hour)),
        external_ref: external_ref.map(str::to_string),
        source: Source::Venue,
        line_error: None,
    }
}

fn csv_row(
    asset: &str,
    kind: Kind,
    quantity: &str,
    price: Option<&str>,
    hour: u32,
    external_ref: &str,
) -> IncomingTransaction {
    IncomingTransaction {
        source: Source::Csv,
        ..row(asset, kind, quantity, price, hour, Some(external_ref))
    }
}

struct Harness {
    transactions: Arc<FakeTransactions>,
    corrections: Arc<FakeCorrections>,
    subjects: Arc<FakeSubjects>,
    lots: Arc<FakeLots>,
    snapshots: Arc<FakeSnapshots>,
    pairs: Arc<FakePairs>,
    account: Arc<FakeAccount>,
    market: Arc<FakeMarketData>,
    assets: Arc<FakeAssets>,
    audit: Arc<FakeAudit>,
    clock: Arc<MutableClock>,
    pause: Arc<RecordedPause>,
    anonymiser: Arc<FakeAnonymiser>,
    lease: Arc<FakeLease>,
    ingest: Arc<IngestService>,
    trades: TradesService,
    corrections_service: CorrectionsService,
    pnl: Arc<PnlService>,
    reads: ReadService,
    privacy: PrivacyService,
    reconcile: ReconcileService,
}

fn harness() -> Harness {
    harness_tuned(&["BTC", "ETH", "SOL"], 0)
}

fn harness_tuned(assets: &[&str], audit_failures: usize) -> Harness {
    let transactions = Arc::new(FakeTransactions::default());
    let corrections = Arc::new(FakeCorrections::new(transactions.clone()));
    let subjects = Arc::new(FakeSubjects::default());
    let lots = Arc::new(FakeLots::default());
    let snapshots = Arc::new(FakeSnapshots::default());
    let pairs = Arc::new(FakePairs::default());
    let account = Arc::new(FakeAccount::default());
    let market = Arc::new(FakeMarketData::default());
    let asset_resolver = Arc::new(FakeAssets::knowing(assets));
    let audit = Arc::new(FakeAudit::failing(audit_failures));
    let clock = Arc::new(MutableClock::at(at(23)));
    let ids = Arc::new(SequentialIds::new());
    let pause = Arc::new(RecordedPause::default());
    let anonymiser = Arc::new(FakeAnonymiser::new(subjects.clone()));
    let anonymisations = Arc::new(FakeAnonymisations::default());
    let lease = Arc::new(FakeLease::default());

    let trail = Arc::new(Trail::new(audit.clone(), clock.clone(), pause.clone()));
    let rebuilder = Arc::new(Rebuilder::new(
        transactions.clone(),
        corrections.clone(),
        lots.clone(),
        snapshots.clone(),
        pairs.clone(),
        clock.clone(),
    ));
    let ingest = Arc::new(IngestService::new(
        transactions.clone(),
        subjects.clone(),
        pairs.clone(),
        account.clone(),
        asset_resolver.clone(),
        clock.clone(),
        ids.clone(),
        "USD",
    ));
    let pnl = Arc::new(PnlService::new(
        subjects.clone(),
        rebuilder.clone(),
        account.clone(),
        market.clone(),
        clock.clone(),
        ids.clone(),
        "USD",
    ));
    let trades = TradesService::new(
        transactions.clone(),
        ingest.clone(),
        asset_resolver.clone(),
        rebuilder.clone(),
        pnl.clone(),
        trail.clone(),
        clock.clone(),
        ids.clone(),
        "USD",
    );
    let corrections_service = CorrectionsService::new(
        transactions.clone(),
        corrections.clone(),
        ingest.clone(),
        asset_resolver.clone(),
        pnl.clone(),
        trail.clone(),
        clock.clone(),
        ids.clone(),
    );
    let reads = ReadService::new(transactions.clone(), subjects.clone());
    let privacy = PrivacyService::new(
        transactions.clone(),
        corrections.clone(),
        subjects.clone(),
        anonymiser.clone(),
        anonymisations,
        trail.clone(),
        clock.clone(),
        ids.clone(),
    );
    let reconcile = ReconcileService::new(
        transactions.clone(),
        subjects.clone(),
        lease.clone(),
        rebuilder,
        pnl.clone(),
        clock.clone(),
    );

    Harness {
        transactions,
        corrections,
        subjects,
        lots,
        snapshots,
        pairs,
        account,
        market,
        assets: asset_resolver,
        audit,
        clock,
        pause,
        anonymiser,
        lease,
        ingest,
        trades,
        corrections_service,
        pnl,
        reads,
        privacy,
        reconcile,
    }
}

fn stored_transactions(harness: &Harness) -> Vec<Transaction> {
    harness.transactions.stored.lock().unwrap().clone()
}

fn stored_id(harness: &Harness, index: usize) -> TransactionId {
    stored_transactions(harness)[index].id()
}

fn caller(service: &str, operations: &[&str]) -> ServiceCaller {
    ServiceCaller {
        service: service.to_string(),
        operations: operations.iter().map(|op| (*op).to_string()).collect(),
    }
}

fn hold(harness: &Harness, asset: &str, quantity: &str) {
    harness.account.holdings.lock().unwrap().push(Holding {
        account_id: Uuid::new_v4(),
        asset_id: Some(asset.to_string()),
        quantity: dec(quantity),
    });
}

// --- WF1: ingest and dedupe -----------------------------------------------

#[tokio::test]
async fn ingest_stores_a_venue_row_once() {
    let harness = harness();
    harness.account.rows.lock().unwrap().push(row(
        "BTC",
        Kind::Buy,
        "1",
        Some("100"),
        10,
        Some("v-1"),
    ));
    let report = harness.ingest.ingest_user(USER).await.unwrap();
    assert_eq!(report.stored.len(), 1);
    assert_eq!(report.duplicates, 0);
    assert!(report.refused.is_empty());
    let stored = stored_transactions(&harness);
    assert_eq!(stored[0].price_source(), Some(PriceSource::Venue));
    assert_eq!(stored[0].source(), Source::Venue);
}

#[tokio::test]
async fn ingest_skips_an_exchange_and_csv_row_of_the_same_trade() {
    let harness = harness();
    {
        let mut rows = harness.account.rows.lock().unwrap();
        rows.push(row("BTC", Kind::Buy, "1", Some("100"), 10, Some("v-1")));
        rows.push(csv_row("BTC", Kind::Buy, "1", Some("100"), 10, "c-1"));
    }
    let report = harness.ingest.ingest_user(USER).await.unwrap();
    assert_eq!(report.stored.len(), 1);
    assert_eq!(report.duplicates, 1);
}

#[tokio::test]
async fn ingest_skips_a_second_row_with_the_same_external_key() {
    let harness = harness();
    {
        let mut rows = harness.account.rows.lock().unwrap();
        rows.push(csv_row("BTC", Kind::Buy, "1", Some("100"), 10, "row-1"));
        rows.push(csv_row("BTC", Kind::Buy, "2", Some("100"), 10, "row-1"));
    }
    let report = harness.ingest.ingest_user(USER).await.unwrap();
    assert_eq!(report.stored.len(), 1);
    assert_eq!(report.duplicates, 1);
}

#[tokio::test]
async fn ingest_reports_a_line_error_and_stores_nothing() {
    let harness = harness();
    harness
        .account
        .rows
        .lock()
        .unwrap()
        .push(IncomingTransaction {
            line_error: Some("quantity is not a decimal".to_string()),
            ..row("BTC", Kind::Buy, "1", Some("100"), 10, Some("v-1"))
        });
    let report = harness.ingest.ingest_user(USER).await.unwrap();
    assert!(report.stored.is_empty());
    assert_eq!(report.refused.len(), 1);
    assert_eq!(report.refused[0].external_ref.as_deref(), Some("v-1"));
}

#[tokio::test]
async fn ingest_reports_a_row_missing_its_quantity() {
    let harness = harness();
    harness
        .account
        .rows
        .lock()
        .unwrap()
        .push(IncomingTransaction {
            quantity: None,
            ..row("BTC", Kind::Buy, "1", Some("100"), 10, Some("v-1"))
        });
    let report = harness.ingest.ingest_user(USER).await.unwrap();
    assert!(report.stored.is_empty());
    assert!(report.refused[0].reason.contains("quantity"));
}

#[tokio::test]
async fn ingest_refuses_an_unregistered_asset() {
    let harness = harness();
    harness.account.rows.lock().unwrap().push(row(
        "DOGE",
        Kind::Buy,
        "1",
        Some("1"),
        10,
        Some("v-1"),
    ));
    let report = harness.ingest.ingest_user(USER).await.unwrap();
    assert!(report.stored.is_empty());
    assert!(report.refused[0].reason.contains("assetId"));
}

#[tokio::test]
async fn ingest_fails_when_account_is_unavailable() {
    let harness = harness();
    *harness.account.failing.lock().unwrap() = true;
    let error = harness.ingest.ingest_user(USER).await.unwrap_err();
    assert!(matches!(error, AppError::External(_)));
}

#[tokio::test]
async fn ingest_fails_when_the_asset_registry_is_unavailable() {
    let harness = harness_tuned(&[], 0);
    harness.account.rows.lock().unwrap().push(row(
        "BTC",
        Kind::Buy,
        "1",
        Some("100"),
        10,
        Some("v-1"),
    ));
    *harness.assets.known.lock().unwrap() = std::collections::BTreeSet::new();
    *harness.assets.failing.lock().unwrap() = true;
    let error = harness.ingest.ingest_user(USER).await.unwrap_err();
    assert!(matches!(error, AppError::External(_)));
}

#[tokio::test]
async fn a_transfer_out_then_transfer_in_are_paired() {
    let harness = harness();
    {
        let mut rows = harness.account.rows.lock().unwrap();
        rows.push(row("BTC", Kind::TransferOut, "1", None, 10, None));
        rows.push(row("BTC", Kind::TransferIn, "1", None, 11, None));
    }
    harness.ingest.ingest_user(USER).await.unwrap();
    let pairs = harness.pairs.stored.lock().unwrap();
    assert_eq!(pairs.len(), 1);
    assert!(pairs[0].is_matched());
}

#[tokio::test]
async fn a_transfer_in_then_transfer_out_are_paired() {
    let harness = harness();
    {
        let mut rows = harness.account.rows.lock().unwrap();
        rows.push(row("BTC", Kind::TransferIn, "1", None, 11, None));
        rows.push(row("BTC", Kind::TransferOut, "1", None, 10, None));
    }
    harness.ingest.ingest_user(USER).await.unwrap();
    let pairs = harness.pairs.stored.lock().unwrap();
    assert_eq!(pairs.len(), 1);
    assert!(pairs[0].is_matched());
}

#[tokio::test]
async fn a_lone_transfer_in_gets_a_pending_pair() {
    let harness = harness();
    harness
        .account
        .rows
        .lock()
        .unwrap()
        .push(row("BTC", Kind::TransferIn, "1", None, 11, None));
    harness.ingest.ingest_user(USER).await.unwrap();
    let pairs = harness.pairs.stored.lock().unwrap();
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0].status(), ledger_domain::TransferStatus::Pending);
    assert!(pairs[0].in_transaction_id().is_some());
    assert_eq!(pairs[0].out_transaction_id(), None);
}

#[tokio::test]
async fn an_external_transfer_out_reduces_the_holding_without_a_gain() {
    let harness = harness();
    {
        let mut rows = harness.account.rows.lock().unwrap();
        rows.push(row("BTC", Kind::Buy, "1", Some("100"), 9, Some("v-1")));
        rows.push(row("BTC", Kind::TransferOut, "0.4", None, 10, None));
    }
    harness.ingest.ingest_user(USER).await.unwrap();
    harness.market.set_price("BTC", "100");
    let report = harness.pnl.report(USER, None).await.unwrap();
    assert_eq!(report.realized, Some(dec("0")));
    assert_eq!(report.net_invested, Some(dec("60")));
    assert_eq!(report.label, ledger_domain::CoverageLabel::Complete);
}

// --- WF6: recorded trades -------------------------------------------------

#[tokio::test]
async fn a_recorded_trade_is_stored_labelled_user_and_audited() {
    let harness = harness();
    let trade = harness
        .trades
        .record(
            USER,
            TradeCommand {
                asset_id: "BTC".to_string(),
                kind: Kind::Buy,
                quantity: dec("1"),
                price: Some(dec("100")),
                currency: None,
                executed_at: at(10),
            },
        )
        .await
        .unwrap();
    assert_eq!(trade.transaction.price_source(), Some(PriceSource::User));
    assert_eq!(trade.transaction.source(), Source::RecordedTrade);
    assert!(!trade.audit_failed);
    assert_eq!(harness.audit.records.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn a_recorded_trade_without_a_price_is_refused() {
    let harness = harness();
    let error = harness
        .trades
        .record(
            USER,
            TradeCommand {
                asset_id: "BTC".to_string(),
                kind: Kind::Buy,
                quantity: dec("1"),
                price: None,
                currency: None,
                executed_at: at(10),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Rejected(ref errors) if errors[0].field == "price"));
}

#[tokio::test]
async fn a_recorded_sell_beyond_holdings_is_refused() {
    let harness = harness();
    harness.account.rows.lock().unwrap().push(row(
        "BTC",
        Kind::Buy,
        "1",
        Some("100"),
        9,
        Some("v-1"),
    ));
    harness.ingest.ingest_user(USER).await.unwrap();
    let error = harness
        .trades
        .record(
            USER,
            TradeCommand {
                asset_id: "BTC".to_string(),
                kind: Kind::Sell,
                quantity: dec("2"),
                price: Some(dec("200")),
                currency: None,
                executed_at: at(10),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Rejected(ref errors) if errors[0].field == "quantity"));
}

#[tokio::test]
async fn a_recorded_sell_within_holdings_is_stored() {
    let harness = harness();
    harness.account.rows.lock().unwrap().push(row(
        "BTC",
        Kind::Buy,
        "2",
        Some("100"),
        9,
        Some("v-1"),
    ));
    harness.ingest.ingest_user(USER).await.unwrap();
    let trade = harness
        .trades
        .record(
            USER,
            TradeCommand {
                asset_id: "BTC".to_string(),
                kind: Kind::Sell,
                quantity: dec("1"),
                price: Some(dec("200")),
                currency: None,
                executed_at: at(10),
            },
        )
        .await
        .unwrap();
    assert_eq!(trade.transaction.kind(), Kind::Sell);
}

#[tokio::test]
async fn a_recorded_trade_of_a_non_trade_kind_is_refused() {
    let harness = harness();
    let error = harness
        .trades
        .record(
            USER,
            TradeCommand {
                asset_id: "BTC".to_string(),
                kind: Kind::TransferIn,
                quantity: dec("1"),
                price: Some(dec("100")),
                currency: None,
                executed_at: at(10),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Rejected(ref errors) if errors[0].field == "kind"));
}

#[tokio::test]
async fn a_recorded_trade_of_an_unknown_asset_is_refused() {
    let harness = harness();
    let error = harness
        .trades
        .record(
            USER,
            TradeCommand {
                asset_id: "DOGE".to_string(),
                kind: Kind::Buy,
                quantity: dec("1"),
                price: Some(dec("100")),
                currency: None,
                executed_at: at(10),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Rejected(ref errors) if errors[0].field == "assetId"));
}

#[tokio::test]
async fn a_duplicate_recorded_trade_is_refused() {
    let harness = harness();
    let command = TradeCommand {
        asset_id: "BTC".to_string(),
        kind: Kind::Buy,
        quantity: dec("1"),
        price: Some(dec("100")),
        currency: None,
        executed_at: at(10),
    };
    harness.trades.record(USER, command.clone()).await.unwrap();
    let error = harness.trades.record(USER, command).await.unwrap_err();
    assert!(matches!(error, AppError::Rejected(ref errors) if errors[0].field == "transaction"));
}

#[tokio::test]
async fn an_audit_failure_is_surfaced_after_the_retries_but_the_trade_stays() {
    let harness = harness_tuned(&["BTC"], 3);
    let trade = harness
        .trades
        .record(
            USER,
            TradeCommand {
                asset_id: "BTC".to_string(),
                kind: Kind::Buy,
                quantity: dec("1"),
                price: Some(dec("100")),
                currency: None,
                executed_at: at(10),
            },
        )
        .await
        .unwrap();
    assert!(trade.audit_failed);
    assert_eq!(stored_transactions(&harness).len(), 1);
    assert_eq!(harness.pause.pauses.lock().unwrap().len(), 3);
}

// --- WF5: corrections -----------------------------------------------------

async fn ingested_buy(harness: &Harness) -> TransactionId {
    harness.account.rows.lock().unwrap().push(row(
        "BTC",
        Kind::Buy,
        "1",
        Some("100"),
        10,
        Some("v-1"),
    ));
    harness.ingest.ingest_user(USER).await.unwrap();
    stored_id(harness, 0)
}

#[tokio::test]
async fn a_correction_records_the_old_value_it_replaces() {
    let harness = harness();
    let id = ingested_buy(&harness).await;
    let result = harness
        .corrections_service
        .correct(USER, id, "quantity", "2", "a typo")
        .await
        .unwrap();
    assert_eq!(result.correction.field(), CorrectionField::Quantity);
    assert_eq!(result.correction.old_value(), "1");
    assert_eq!(result.correction.new_value(), "2");
    assert!(!result.audit_failed);
}

#[tokio::test]
async fn a_correction_with_a_negative_price_is_refused() {
    let harness = harness();
    let id = ingested_buy(&harness).await;
    let error = harness
        .corrections_service
        .correct(USER, id, "price", "-1", "a typo")
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Rejected(ref errors) if errors[0].field == "newValue"));
}

#[tokio::test]
async fn a_correction_of_an_unknown_field_is_refused() {
    let harness = harness();
    let id = ingested_buy(&harness).await;
    let error = harness
        .corrections_service
        .correct(USER, id, "notes", "x", "a typo")
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Rejected(ref errors) if errors[0].field == "field"));
}

#[tokio::test]
async fn a_correction_without_a_reason_is_refused() {
    let harness = harness();
    let id = ingested_buy(&harness).await;
    let error = harness
        .corrections_service
        .correct(USER, id, "quantity", "2", "   ")
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Rejected(ref errors) if errors[0].field == "reason"));
}

#[tokio::test]
async fn a_correction_for_another_users_transaction_is_forbidden() {
    let harness = harness();
    let id = ingested_buy(&harness).await;
    let error = harness
        .corrections_service
        .correct(OTHER, id, "quantity", "2", "a typo")
        .await
        .unwrap_err();
    assert_eq!(error, AppError::Forbidden);
}

#[tokio::test]
async fn a_correction_for_an_unknown_transaction_is_not_found() {
    let harness = harness();
    let error = harness
        .corrections_service
        .correct(
            USER,
            TransactionId::new(Uuid::from_u128(999)),
            "quantity",
            "2",
            "a typo",
        )
        .await
        .unwrap_err();
    assert_eq!(error, AppError::NotFound);
}

#[tokio::test]
async fn a_second_correction_uses_the_value_in_force() {
    let harness = harness();
    let id = ingested_buy(&harness).await;
    harness
        .corrections_service
        .correct(USER, id, "quantity", "2", "first")
        .await
        .unwrap();
    harness.clock.advance(chrono::Duration::minutes(1));
    let second = harness
        .corrections_service
        .correct(USER, id, "quantity", "3", "second")
        .await
        .unwrap();
    assert_eq!(second.correction.old_value(), "2");
}

#[tokio::test]
async fn a_correction_to_an_unregistered_asset_is_refused() {
    let harness = harness();
    let id = ingested_buy(&harness).await;
    let error = harness
        .corrections_service
        .correct(USER, id, "assetId", "DOGE", "wrong asset")
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Rejected(ref errors) if errors[0].field == "newValue"));
}

#[tokio::test]
async fn a_correction_audit_failure_is_flagged_but_the_row_is_committed() {
    let harness = harness_tuned(&["BTC"], 3);
    let id = ingested_buy(&harness).await;
    let result: CorrectionResult = harness
        .corrections_service
        .correct(USER, id, "quantity", "2", "a typo")
        .await
        .unwrap();
    assert!(result.audit_failed);
    assert_eq!(harness.corrections.stored.lock().unwrap().len(), 1);
    assert_eq!(harness.pause.pauses.lock().unwrap().len(), 3);
}

// --- WF4: P&L and reads ---------------------------------------------------

async fn ingest_buy_sell(harness: &Harness) {
    {
        let mut rows = harness.account.rows.lock().unwrap();
        rows.push(row("BTC", Kind::Buy, "1", Some("100"), 9, Some("v-1")));
        rows.push(row("BTC", Kind::Sell, "0.5", Some("300"), 10, Some("v-2")));
    }
    harness.ingest.ingest_user(USER).await.unwrap();
    hold(harness, "BTC", "0.5");
    harness.market.set_price("BTC", "300");
}

#[tokio::test]
async fn pnl_is_complete_after_a_buy_and_a_partial_sell() {
    let harness = harness();
    ingest_buy_sell(&harness).await;
    let report = harness.pnl.report(USER, None).await.unwrap();
    assert_eq!(report.label, ledger_domain::CoverageLabel::Complete);
    assert_eq!(report.net_invested, Some(dec("50")));
    assert_eq!(report.realized, Some(dec("100")));
    assert_eq!(report.unrealized, Some(dec("100")));
}

#[tokio::test]
async fn pnl_names_an_uncovered_asset_and_excludes_it() {
    let harness = harness();
    {
        let mut rows = harness.account.rows.lock().unwrap();
        rows.push(row("BTC", Kind::Buy, "1", Some("100"), 9, Some("v-1")));
        rows.push(row("ETH", Kind::Sell, "2", Some("50"), 10, Some("v-2")));
    }
    harness.ingest.ingest_user(USER).await.unwrap();
    hold(&harness, "ETH", "2");
    harness.market.set_price("BTC", "100");
    harness.market.set_price("ETH", "50");
    let report = harness.pnl.report(USER, None).await.unwrap();
    assert_eq!(report.label, ledger_domain::CoverageLabel::Partial);
    assert_eq!(report.coverage_gap, vec!["ETH".to_string()]);
}

#[tokio::test]
async fn pnl_for_a_user_with_no_ledger_is_a_complete_zero() {
    let harness = harness();
    let report = harness.pnl.report(USER, None).await.unwrap();
    assert_eq!(report.label, ledger_domain::CoverageLabel::Complete);
    assert_eq!(report.net_invested, Some(dec("0")));
}

#[tokio::test]
async fn the_transaction_list_pages_newest_first_with_a_cursor() {
    let harness = harness();
    {
        let mut rows = harness.account.rows.lock().unwrap();
        rows.push(row("BTC", Kind::Buy, "1", Some("100"), 9, Some("v-1")));
        rows.push(row("ETH", Kind::Buy, "1", Some("50"), 10, Some("v-2")));
        rows.push(row("SOL", Kind::Buy, "1", Some("20"), 11, Some("v-3")));
    }
    harness.ingest.ingest_user(USER).await.unwrap();
    let first = harness
        .reads
        .transactions(USER, None, Some(2))
        .await
        .unwrap();
    assert_eq!(first.transactions.len(), 2);
    assert_eq!(first.transactions[0].asset_id(), "SOL");
    let cursor = first.next_cursor.expect("a next cursor");
    let second = harness
        .reads
        .transactions(USER, Some(&cursor), Some(2))
        .await
        .unwrap();
    assert_eq!(second.transactions.len(), 1);
    assert_eq!(second.transactions[0].asset_id(), "BTC");
}

#[tokio::test]
async fn a_malformed_cursor_is_refused() {
    let harness = harness();
    let error = harness
        .reads
        .transactions(USER, Some("not-a-cursor"), None)
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Rejected(_)));
    assert!(decode_cursor("nope").is_err());
}

// --- C7 privacy -----------------------------------------------------------

#[tokio::test]
async fn export_requires_the_privacy_export_operation() {
    let harness = harness();
    let error = harness
        .privacy
        .export(&caller("reporting", &[]), USER)
        .await
        .unwrap_err();
    assert_eq!(error, AppError::Forbidden);
}

#[tokio::test]
async fn export_returns_the_transactions_and_corrections() {
    let harness = harness();
    let id = ingested_buy(&harness).await;
    harness
        .corrections_service
        .correct(USER, id, "quantity", "2", "a typo")
        .await
        .unwrap();
    let export = harness
        .privacy
        .export(&caller("identity", &["privacy.export"]), USER)
        .await
        .unwrap();
    assert_eq!(export.transactions.len(), 1);
    assert_eq!(export.corrections.len(), 1);
}

#[tokio::test]
async fn anonymise_is_restricted_to_identity_and_is_idempotent() {
    let harness = harness();
    ingested_buy(&harness).await;
    let read_only = caller("reporting", &["privacy.anonymise"]);
    assert_eq!(
        harness
            .privacy
            .anonymise(&read_only, USER, "idem-1")
            .await
            .unwrap_err(),
        AppError::Forbidden
    );
    let identity = caller("identity", &["privacy.anonymise"]);
    let first = harness
        .privacy
        .anonymise(&identity, USER, "idem-1")
        .await
        .unwrap();
    assert!(!first.already_applied);
    assert!(first.anonymised_ref.is_some());
    let second = harness
        .privacy
        .anonymise(&identity, USER, "idem-1")
        .await
        .unwrap();
    assert!(second.already_applied);
    assert_eq!(first.anonymised_ref, second.anonymised_ref);
    assert_eq!(harness.anonymiser.calls.lock().unwrap().len(), 1);
    assert!(harness.subjects.stored.lock().unwrap()[0].is_anonymised());
}

#[tokio::test]
async fn anonymise_requires_an_idempotency_key() {
    let harness = harness();
    let error = harness
        .privacy
        .anonymise(&caller("identity", &["privacy.anonymise"]), USER, " ")
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Rejected(ref errors) if errors[0].field == "idempotencyKey"));
}

#[tokio::test]
async fn anonymise_with_no_ledger_subject_is_already_applied() {
    let harness = harness();
    let result = harness
        .privacy
        .anonymise(&caller("identity", &["privacy.anonymise"]), USER, "idem-1")
        .await
        .unwrap();
    assert!(result.already_applied);
    assert_eq!(result.anonymised_ref, None);
}

// --- C11 reconcile --------------------------------------------------------

#[tokio::test]
async fn reconcile_rebuilds_every_subject() {
    let harness = harness();
    {
        let mut rows = harness.account.rows.lock().unwrap();
        rows.push(row("BTC", Kind::Buy, "1", Some("100"), 9, Some("v-1")));
    }
    harness.ingest.ingest_user(USER).await.unwrap();
    harness.ingest.ingest_user(OTHER).await.unwrap();
    let report = harness.reconcile.run().await.unwrap();
    assert_eq!(report.status, ReconcileStatus::Completed);
    assert_eq!(report.subjects, 2);
    assert_eq!(report.batches, 1);
    assert_eq!(harness.snapshots.stored.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn reconcile_is_skipped_while_the_lease_is_held() {
    let harness = harness();
    *harness.lease.leased_until.lock().unwrap() = Some(at(23) + chrono::Duration::minutes(5));
    let report = harness.reconcile.run().await.unwrap();
    assert_eq!(report.status, ReconcileStatus::Skipped);
    assert_eq!(report.subjects, 0);
}

#[tokio::test]
async fn reconcile_releases_the_lease_when_it_completes() {
    let harness = harness();
    harness.reconcile.run().await.unwrap();
    assert_eq!(*harness.lease.leased_until.lock().unwrap(), None);
    assert!(harness.reconcile.last_completed().await.unwrap().is_some());
}

// --- Further edge cases ---------------------------------------------------

#[tokio::test]
async fn replaying_the_same_c5_page_stores_nothing_new() {
    let harness = harness();
    harness.account.rows.lock().unwrap().push(row(
        "BTC",
        Kind::Buy,
        "1",
        Some("100"),
        10,
        Some("v-1"),
    ));
    let first = harness.ingest.ingest_user(USER).await.unwrap();
    let second = harness.ingest.ingest_user(USER).await.unwrap();
    assert_eq!(first.stored.len(), 1);
    assert!(second.stored.is_empty());
    assert_eq!(second.duplicates, 1);
}

#[tokio::test]
async fn ingest_stores_a_fee_row() {
    let harness = harness();
    harness.account.rows.lock().unwrap().push(row(
        "BTC",
        Kind::Fee,
        "0.01",
        Some("100"),
        10,
        Some("v-1"),
    ));
    let report = harness.ingest.ingest_user(USER).await.unwrap();
    assert_eq!(report.stored.len(), 1);
    assert_eq!(stored_transactions(&harness)[0].kind(), Kind::Fee);
}

#[tokio::test]
async fn ingest_reports_a_row_missing_its_kind() {
    let harness = harness();
    harness
        .account
        .rows
        .lock()
        .unwrap()
        .push(IncomingTransaction {
            kind: None,
            ..row("BTC", Kind::Buy, "1", Some("100"), 10, Some("v-1"))
        });
    let report = harness.ingest.ingest_user(USER).await.unwrap();
    assert!(report.stored.is_empty());
    assert!(report.refused[0].reason.contains("kind"));
}

#[tokio::test]
async fn a_correction_rebuilds_the_lots() {
    let harness = harness();
    let id = ingested_buy(&harness).await;
    harness
        .corrections_service
        .correct(USER, id, "quantity", "2", "a typo")
        .await
        .unwrap();
    let subject = harness.subjects.stored.lock().unwrap()[0].id();
    let lots = harness.lots.stored.lock().unwrap();
    let stored = lots.get(&subject).unwrap();
    assert_eq!(stored[0].quantity(), dec("2"));
    assert_eq!(stored[0].remaining(), dec("2"));
}

#[tokio::test]
async fn a_pnl_read_writes_a_snapshot() {
    let harness = harness();
    ingest_buy_sell(&harness).await;
    harness.pnl.report(USER, None).await.unwrap();
    assert_eq!(harness.snapshots.stored.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn the_transaction_limit_is_clamped() {
    let harness = harness();
    let page = harness
        .reads
        .transactions(USER, None, Some(0))
        .await
        .unwrap();
    assert!(page.transactions.is_empty());
    assert_eq!(page.next_cursor, None);
}

#[tokio::test]
async fn exporting_a_user_with_no_ledger_rows_is_empty() {
    let harness = harness();
    let export = harness
        .privacy
        .export(&caller("identity", &["privacy.export"]), USER)
        .await
        .unwrap();
    assert!(export.transactions.is_empty());
    assert!(export.corrections.is_empty());
}

#[tokio::test]
async fn a_fresh_reconcile_has_no_completed_run() {
    let harness = harness();
    assert_eq!(harness.reconcile.last_completed().await.unwrap(), None);
}

#[tokio::test]
async fn an_audit_failure_still_rebuilds_the_trade() {
    let harness = harness_tuned(&["BTC"], 3);
    let trade = harness
        .trades
        .record(
            USER,
            TradeCommand {
                asset_id: "BTC".to_string(),
                kind: Kind::Buy,
                quantity: dec("1"),
                price: Some(dec("100")),
                currency: None,
                executed_at: at(10),
            },
        )
        .await
        .unwrap();
    assert!(trade.audit_failed);
    assert!(trade.report.is_some());
    assert_eq!(harness.lots.stored.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn correcting_a_price_changes_the_pnl() {
    let harness = harness();
    let id = ingested_buy(&harness).await;
    harness.market.set_price("BTC", "100");
    let before = harness.pnl.report(USER, None).await.unwrap();
    harness
        .corrections_service
        .correct(USER, id, "price", "200", "the venue price was wrong")
        .await
        .unwrap();
    let after = harness.pnl.report(USER, None).await.unwrap();
    assert_eq!(before.net_invested, Some(dec("100")));
    assert_eq!(after.net_invested, Some(dec("200")));
}

#[tokio::test]
async fn a_non_base_trade_is_converted_through_c6_fx() {
    let harness = harness();
    harness
        .trades
        .record(
            USER,
            TradeCommand {
                asset_id: "BTC".to_string(),
                kind: Kind::Buy,
                quantity: dec("1"),
                price: Some(dec("100")),
                currency: Some("EUR".to_string()),
                executed_at: at(10),
            },
        )
        .await
        .unwrap();
    hold(&harness, "BTC", "1");
    harness.market.set_price("BTC", "300");
    harness.market.set_fx("USD", "EUR", "1.1");
    let report = harness.pnl.report(USER, None).await.unwrap();
    assert_eq!(report.currency, "USD");
    assert!(report.fx_estimated);
    assert_eq!(report.net_invested, Some(dec("110")));
    assert_eq!(report.unrealized, Some(dec("190")));
}

#[tokio::test]
async fn a_c6_outage_returns_a_degraded_partial_pnl() {
    let harness = harness();
    ingested_buy(&harness).await;
    *harness.market.failing.lock().unwrap() = true;
    let report = harness.pnl.report(USER, None).await.unwrap();
    assert!(report.marks_unavailable);
    assert_eq!(report.label, ledger_domain::CoverageLabel::Partial);
    assert_eq!(report.coverage_gap, vec!["BTC".to_string()]);
    assert_eq!(report.net_invested, None);
}

#[tokio::test]
async fn a_c5_outage_returns_a_degraded_pnl() {
    let harness = harness();
    ingested_buy(&harness).await;
    *harness.account.failing.lock().unwrap() = true;
    let report = harness.pnl.report(USER, None).await.unwrap();
    assert!(report.holdings_unavailable);
    assert_eq!(report.label, ledger_domain::CoverageLabel::Partial);
    assert_eq!(report.net_invested, None);
}

#[tokio::test]
async fn a_c6_fx_outage_degrades_rather_than_mixing_currencies() {
    let harness = harness();
    harness
        .trades
        .record(
            USER,
            TradeCommand {
                asset_id: "BTC".to_string(),
                kind: Kind::Buy,
                quantity: dec("1"),
                price: Some(dec("100")),
                currency: Some("EUR".to_string()),
                executed_at: at(10),
            },
        )
        .await
        .unwrap();
    hold(&harness, "BTC", "1");
    harness.market.set_price("BTC", "300");
    let report = harness.pnl.report(USER, None).await.unwrap();
    assert!(report.fx_unavailable);
    assert_eq!(report.net_invested, None);
    assert_eq!(report.label, ledger_domain::CoverageLabel::Partial);
}
