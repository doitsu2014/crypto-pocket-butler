//! Adapter integration tests against a real Postgres schema.
//!
//! Run with `TEST_DATABASE_URL` pointing at a disposable database:
//! `TEST_DATABASE_URL=postgres://... cargo test -p account-infrastructure --test repositories -- --ignored`

mod common;

use account_application::ports::{
    AccountRepository, AnonymisationRecord, CredentialRepository, HoldingRepository,
    ImportRepository, PrivacyRepository, ServiceTokenRegistry, SyncRunRepository,
    WalletConnectionRepository,
};
use account_domain::{
    Account, AccountId, AccountKind, AccountStatus, Credential, CredentialId, CredentialStatus,
    ErrorCategory, HoldingSnapshot, HoldingSnapshotId, ImportBatch, ImportBatchId, ImportSource,
    ImportedTransaction, ImportedTransactionId, SyncRun, SyncRunId, TransactionKind, UserId,
    WalletConnection, WalletConnectionId,
};
use account_infrastructure::repositories::{
    PgAccountRepository, PgCredentialRepository, PgHoldingRepository, PgImportRepository,
    PgPrivacyRepository, PgSyncRunRepository, PgWalletConnectionRepository,
};
use account_infrastructure::service_token::PgServiceTokenRegistry;
use chrono::{DateTime, Duration, TimeZone, Utc};
use rust_decimal::Decimal;
use std::str::FromStr;

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, hour, 0, 0).unwrap()
}

fn user() -> UserId {
    UserId::parse("11111111-1111-4111-8111-111111111111").unwrap()
}

fn decimal(raw: &str) -> Decimal {
    Decimal::from_str(raw).unwrap()
}

async fn stored_account(
    pool: &sqlx::PgPool,
    kind: AccountKind,
    venue: &str,
    label: &str,
) -> Account {
    let repository = PgAccountRepository::new(pool.clone());
    let account = Account::open(AccountId::generate(), user(), kind, venue, label, at(9)).unwrap();
    repository.insert(&account).await.unwrap();
    account
}

async fn stored_run(pool: &sqlx::PgPool, account: &Account, now: DateTime<Utc>) -> SyncRun {
    let repository = PgSyncRunRepository::new(pool.clone());
    let run = SyncRun::claim(SyncRunId::generate(), account.id(), now, 60);
    repository.claim(&run).await.unwrap();
    run
}

fn snapshot(run: &SyncRun, token: &str, asset: Option<&str>, quantity: &str) -> HoldingSnapshot {
    HoldingSnapshot::record(
        HoldingSnapshotId::generate(),
        run.id(),
        run.account_id(),
        token,
        asset,
        decimal(quantity),
        at(10),
    )
    .unwrap()
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn the_migration_applies_reverts_and_reapplies_on_a_fresh_schema() {
    let (pool, schema) = common::test_pool().await;
    account_infrastructure::migrations::revert(&pool)
        .await
        .expect("down migration on a fresh schema");
    account_infrastructure::migrations::run(&pool)
        .await
        .expect("re-run migration");
    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn accounts_are_found_and_labels_are_unique_ignoring_case() {
    let (pool, schema) = common::test_pool().await;
    let repository = PgAccountRepository::new(pool.clone());
    let account = stored_account(&pool, AccountKind::Exchange, "binance", "Main").await;

    assert_eq!(
        repository
            .find_by_id(account.id())
            .await
            .unwrap()
            .unwrap()
            .id(),
        account.id()
    );
    assert_eq!(repository.find_for_user(user()).await.unwrap().len(), 1);
    assert!(repository.label_exists(user(), "main", None).await.unwrap());
    assert!(!repository
        .label_exists(user(), "main", Some(account.id()))
        .await
        .unwrap());

    let duplicate = Account::open(
        AccountId::generate(),
        user(),
        AccountKind::Exchange,
        "okx",
        "MAIN",
        at(9),
    )
    .unwrap();
    assert!(repository.insert(&duplicate).await.is_err());
    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn an_account_update_and_anonymisation_round_trip() {
    let (pool, schema) = common::test_pool().await;
    let repository = PgAccountRepository::new(pool.clone());
    let mut account = stored_account(&pool, AccountKind::Exchange, "binance", "Main").await;
    account.begin_sync(at(10)).unwrap();
    account.finish_sync_success(at(10)).unwrap();
    repository.update(&account).await.unwrap();
    let stored = repository.find_by_id(account.id()).await.unwrap().unwrap();
    assert_eq!(stored.status(), AccountStatus::Active);
    assert_eq!(stored.last_synced_at(), Some(at(10)));

    assert_eq!(
        repository.anonymise_for_user(user(), at(11)).await.unwrap(),
        1
    );
    let anonymised = repository.find_by_id(account.id()).await.unwrap().unwrap();
    assert!(anonymised.label().starts_with("anonymised-"));
    assert_eq!(anonymised.anonymised_at(), Some(at(11)));
    assert_eq!(
        repository.anonymise_for_user(user(), at(12)).await.unwrap(),
        0
    );
    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn credentials_round_trip_and_list_only_verified_rows() {
    let (pool, schema) = common::test_pool().await;
    let account = stored_account(&pool, AccountKind::Exchange, "binance", "Main").await;
    let repository = PgCredentialRepository::new(pool.clone());
    let mut credential = Credential::pending(
        CredentialId::generate(),
        account.id(),
        "vault-reference-1",
        at(9),
    )
    .unwrap();
    repository.insert(&credential).await.unwrap();
    assert!(repository.list_verified().await.unwrap().is_empty());

    credential.verify(at(10));
    repository.update(&credential).await.unwrap();
    assert_eq!(repository.list_verified().await.unwrap().len(), 1);
    assert_eq!(
        repository
            .find_for_account(account.id())
            .await
            .unwrap()
            .unwrap()
            .verification_status(),
        CredentialStatus::Verified
    );
    assert_eq!(repository.list_for_user(user()).await.unwrap().len(), 1);
    assert_eq!(repository.delete_for_user(user()).await.unwrap(), 1);
    assert!(repository
        .find_by_id(credential.id())
        .await
        .unwrap()
        .is_none());
    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn wallet_connections_round_trip_and_are_deleted_for_the_user() {
    let (pool, schema) = common::test_pool().await;
    let account = stored_account(&pool, AccountKind::Wallet, "evm:1", "Wallet").await;
    let repository = PgWalletConnectionRepository::new(pool.clone());
    let connection = WalletConnection::new_address(
        WalletConnectionId::generate(),
        account.id(),
        "evm:1",
        "0x52908400098527886E0F7030069857D2E4169EE7",
        at(9),
    )
    .unwrap();
    repository.insert(&connection).await.unwrap();
    assert_eq!(repository.list_for_user(user()).await.unwrap().len(), 1);
    assert!(repository
        .find_for_account(account.id())
        .await
        .unwrap()
        .is_some());
    assert_eq!(repository.delete_for_user(user()).await.unwrap(), 1);
    assert!(repository
        .find_for_account(account.id())
        .await
        .unwrap()
        .is_none());
    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn a_live_lease_blocks_a_second_claim_and_an_expired_one_does_not() {
    let (pool, schema) = common::test_pool().await;
    let account = stored_account(&pool, AccountKind::Exchange, "binance", "Main").await;
    let repository = PgSyncRunRepository::new(pool.clone());
    let first = SyncRun::claim(SyncRunId::generate(), account.id(), at(10), 60);
    repository.claim(&first).await.unwrap();

    let second = SyncRun::claim(SyncRunId::generate(), account.id(), at(10), 60);
    assert!(repository.claim(&second).await.is_err());

    let third = SyncRun::claim(SyncRunId::generate(), account.id(), at(11), 60);
    repository.claim(&third).await.unwrap();
    let first_after = repository.find_by_id(first.id()).await.unwrap().unwrap();
    assert_eq!(first_after.status(), account_domain::SyncRunStatus::Failed);
    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn renewal_and_the_sweep_both_track_the_lease() {
    let (pool, schema) = common::test_pool().await;
    let account = stored_account(&pool, AccountKind::Exchange, "binance", "Main").await;
    let repository = PgSyncRunRepository::new(pool.clone());
    let run = stored_run(&pool, &account, at(10)).await;

    assert!(repository.renew(run.id(), at(12), at(10)).await.unwrap());
    assert_eq!(repository.sweep_expired(at(11)).await.unwrap().len(), 0);
    assert_eq!(repository.sweep_expired(at(12)).await.unwrap().len(), 1);
    let stored = repository.find_by_id(run.id()).await.unwrap().unwrap();
    assert_eq!(stored.status(), account_domain::SyncRunStatus::Failed);
    assert_eq!(stored.error_category(), Some(ErrorCategory::Data));
    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn the_snapshot_write_is_fenced_on_a_live_lease() {
    let (pool, schema) = common::test_pool().await;
    let account = stored_account(&pool, AccountKind::Exchange, "binance", "Main").await;
    let repository = PgSyncRunRepository::new(pool.clone());
    let mut run = stored_run(&pool, &account, at(10)).await;

    repository
        .write_snapshot(
            &run,
            &[snapshot(&run, "BTC", Some("asset-btc"), "1.5")],
            at(10),
            at(10),
        )
        .await
        .unwrap();

    run.succeed(at(10)).unwrap();
    repository.finish(&run).await.unwrap();

    let stored_holdings = PgHoldingRepository::new(pool.clone())
        .latest_for_account(account.id())
        .await
        .unwrap();
    assert_eq!(stored_holdings.len(), 1);

    let second = stored_run(&pool, &account, at(10) + Duration::seconds(80)).await;
    assert!(repository
        .write_snapshot(&second, &[], at(11), at(11) + Duration::seconds(100))
        .await
        .is_err());
    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn the_latest_holdings_come_from_the_latest_succeeded_run() {
    let (pool, schema) = common::test_pool().await;
    let account = stored_account(&pool, AccountKind::Exchange, "binance", "Main").await;
    let runs = PgSyncRunRepository::new(pool.clone());
    let first = stored_run(&pool, &account, at(10)).await;
    runs.write_snapshot(
        &first,
        &[snapshot(&first, "BTC", Some("asset-btc"), "1")],
        at(10),
        at(10),
    )
    .await
    .unwrap();
    let mut finished = runs.find_by_id(first.id()).await.unwrap().unwrap();
    finished.succeed(at(10)).unwrap();
    runs.finish(&finished).await.unwrap();

    let second = stored_run(&pool, &account, at(11)).await;
    runs.write_snapshot(
        &second,
        &[snapshot(&second, "ETH", Some("asset-eth"), "2")],
        at(11),
        at(11),
    )
    .await
    .unwrap();
    let mut finished = runs.find_by_id(second.id()).await.unwrap().unwrap();
    finished.succeed(at(11)).unwrap();
    runs.finish(&finished).await.unwrap();

    let latest = PgHoldingRepository::new(pool.clone())
        .latest_for_account(account.id())
        .await
        .unwrap();
    assert_eq!(latest.len(), 1);
    assert_eq!(latest[0].asset_id(), Some("asset-eth"));
    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn an_import_batch_is_staged_claimed_and_read_back() {
    let (pool, schema) = common::test_pool().await;
    let account = stored_account(&pool, AccountKind::Csv, "csv:summary", "History").await;
    let repository = PgImportRepository::new(pool.clone());
    let bytes = b"external_id,kind,asset,quantity,price,executed_at\n";
    let batch = ImportBatch::new(
        ImportBatchId::generate(),
        account.id(),
        ImportSource::Generic,
        "trades.csv",
        at(9),
    )
    .unwrap();
    repository.create_batch(&batch, bytes).await.unwrap();
    assert_eq!(
        repository
            .find_batch(batch.id())
            .await
            .unwrap()
            .unwrap()
            .status(),
        account_domain::ImportBatchStatus::Pending
    );
    let claimed = repository.claim_pending().await.unwrap().unwrap();
    assert_eq!(
        claimed.status(),
        account_domain::ImportBatchStatus::Processing
    );
    assert_eq!(
        repository.staged_bytes(batch.id()).await.unwrap().unwrap(),
        bytes
    );
    repository.delete_staging(batch.id()).await.unwrap();
    assert!(repository.staged_bytes(batch.id()).await.unwrap().is_none());
    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn imported_transactions_dedupe_and_filter_by_time() {
    let (pool, schema) = common::test_pool().await;
    let account = stored_account(&pool, AccountKind::Csv, "csv:summary", "History").await;
    let repository = PgImportRepository::new(pool.clone());
    let batch = ImportBatch::new(
        ImportBatchId::generate(),
        account.id(),
        ImportSource::Generic,
        "trades.csv",
        at(9),
    )
    .unwrap();
    repository.create_batch(&batch, b"header\n").await.unwrap();

    let parsed = account_domain::ParsedTransaction {
        external_id: Some("t-1".to_string()),
        asset_id: "asset-btc".to_string(),
        kind: TransactionKind::Buy,
        quantity: decimal("1.5"),
        price: Some(decimal("30000")),
        executed_at: at(9),
    };
    let row = ImportedTransaction::record(
        ImportedTransactionId::generate(),
        batch.id(),
        account.id(),
        &parsed,
    )
    .unwrap();
    assert_eq!(
        repository
            .insert_transactions(std::slice::from_ref(&row))
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        repository
            .insert_transactions(std::slice::from_ref(&row))
            .await
            .unwrap(),
        0
    );
    assert!(repository
        .dedupe_exists(
            account.id(),
            &format!("{account_id}|ext|t-1", account_id = account.id())
        )
        .await
        .unwrap());
    assert_eq!(
        repository
            .transactions_for_account(account.id(), Some(at(9)))
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        repository
            .transactions_for_user(user())
            .await
            .unwrap()
            .len(),
        1
    );
    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn the_service_token_registry_resolves_callers_and_operations() {
    let (pool, schema) = common::test_pool().await;
    sqlx::query(
        "INSERT INTO service_token_registry (id, token_hash, caller, operations, active, created_at)
         VALUES ($1, $2, $3, $4, true, $5)",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(account_infrastructure::service_token::hash_token("in-token"))
    .bind("identity")
    .bind(vec!["privacy_export".to_string(), "privacy_anonymise".to_string()])
    .bind(at(9))
    .execute(&pool)
    .await
    .unwrap();

    let registry = PgServiceTokenRegistry::with_outbound(pool.clone(), &[("account", "out-token")]);
    let service = registry.service_for("in-token").await.unwrap().unwrap();
    assert_eq!(service.name, "identity");
    assert!(service.allows(account_domain::Operation::PrivacyExport));
    assert!(!service.allows(account_domain::Operation::HoldingsRead));
    assert!(registry.service_for("wrong").await.unwrap().is_none());
    assert_eq!(registry.token_for("account").as_deref(), Some("out-token"));
    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn the_privacy_ledger_records_one_anonymisation_per_user() {
    let (pool, schema) = common::test_pool().await;
    let repository = PgPrivacyRepository::new(pool.clone());
    let record = AnonymisationRecord {
        user_id: user(),
        idempotency_key: "idem-1".to_string(),
        retained_categories: vec!["holdings".to_string(), "transactions".to_string()],
        completed_at: at(10),
    };
    repository.record_anonymisation(&record).await.unwrap();
    let found = repository
        .find_anonymisation(user())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.retained_categories, record.retained_categories);

    let second = AnonymisationRecord {
        idempotency_key: "idem-2".to_string(),
        ..record
    };
    repository.record_anonymisation(&second).await.unwrap();
    assert_eq!(
        repository
            .find_anonymisation(user())
            .await
            .unwrap()
            .unwrap()
            .idempotency_key,
        "idem-1"
    );
    common::drop_schema(&schema).await;
}
