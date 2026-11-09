//! Repository integration tests against a real Postgres schema.
//!
//! Run with:
//!   TEST_DATABASE_URL=postgres://... \
//!   cargo test -p ledger-infrastructure --features integration -- --ignored
//!
//! Every test creates a disposable schema, applies the real migrations to it,
//! and drops it afterwards. The connection needs CREATE SCHEMA; role creation
//! and grants are skipped with a notice when the role cannot manage roles.
#![cfg(feature = "integration")]

use std::time::Duration;

use chrono::{DateTime, TimeZone, Utc};
use ledger_application::hashing::sha256_hex;
use ledger_application::ports::{
    AnonymisationRepository, CorrectionRepository, LotRepository, ReadinessProbe, ReconcileLease,
    RepositoryError, ServiceTokenRegistry, SnapshotRepository, SubjectRepository,
    TransactionCursor, TransactionRepository, TransferPairRepository,
};
use ledger_domain::{
    Correction, CorrectionField, CorrectionId, CostBasisLot, CostBasisLotId, Kind, LedgerSubject,
    LedgerSubjectId, NewTransaction, PnlReport, PnlSnapshot, PnlSnapshotId, PriceSource, Source,
    Transaction, TransactionId, TransferPair, TransferPairId,
};
use ledger_infrastructure::repositories::{
    PgAnonymisationRepository, PgCorrectionRepository, PgLotRepository, PgReconcileLease,
    PgSnapshotRepository, PgSubjectRepository, PgTransactionRepository, PgTransferPairRepository,
};
use ledger_infrastructure::service_token::PgServiceTokenRegistry;
use ledger_infrastructure::PgReadinessProbe;
use rust_decimal::Decimal;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 6, hour, 0, 0).unwrap()
}

fn dec(raw: &str) -> Decimal {
    raw.parse().unwrap()
}

async fn pool_with_schema() -> (PgPool, String) {
    let url = std::env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL must point at a disposable Postgres database");
    let schema = format!("ledger_it_{}", Uuid::new_v4().simple());
    let admin = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .expect("the test database is reachable");
    sqlx::query(&format!("CREATE SCHEMA \"{schema}\""))
        .execute(&admin)
        .await
        .expect("the test role can create a schema");
    admin.close().await;

    let scoped = schema.clone();
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(5))
        .after_connect(move |connection, _meta| {
            let scoped = scoped.clone();
            Box::pin(async move {
                sqlx::query(&format!("SET search_path TO \"{scoped}\""))
                    .execute(&mut *connection)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
        .expect("the disposable schema is reachable");
    ledger_infrastructure::migrations::run(&pool)
        .await
        .expect("the migrations apply");
    (pool, schema)
}

async fn drop_schema(pool: &PgPool, schema: &str) {
    let _ = sqlx::query(&format!("DROP SCHEMA \"{schema}\" CASCADE"))
        .execute(pool)
        .await;
}

async fn subject(pool: &PgPool) -> LedgerSubject {
    let repo = PgSubjectRepository::new(pool.clone());
    let user = Uuid::new_v4().to_string();
    let subject =
        LedgerSubject::new(LedgerSubjectId::new(Uuid::new_v4()), user.clone(), at(9)).unwrap();
    repo.create(&subject).await.unwrap();
    assert_eq!(
        repo.find_by_user(&user).await.unwrap().unwrap().id(),
        subject.id()
    );
    subject
}

fn transaction(
    subject: &LedgerSubject,
    value: u128,
    asset: &str,
    kind: Kind,
    quantity: &str,
    price: Option<&str>,
    hour: u32,
) -> Transaction {
    Transaction::record(
        TransactionId::new(Uuid::from_u128(value)),
        NewTransaction {
            subject_id: subject.id(),
            user_id: subject.user_id().unwrap().to_string(),
            account_id: None,
            asset_id: asset.to_string(),
            kind,
            quantity: dec(quantity),
            price: price.map(dec),
            price_source: price.map(|_| PriceSource::Venue),
            currency: "USD".to_string(),
            source: Source::Venue,
            external_ref: Some(format!("ref-{value}")),
            executed_at: at(hour),
            recorded_at: at(23),
        },
    )
    .unwrap()
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL and the integration feature"]
async fn subjects_round_trip_and_anonymise() {
    let (pool, schema) = pool_with_schema().await;
    let repo = PgSubjectRepository::new(pool.clone());
    let subject = subject(&pool).await;
    assert!(repo.find_by_id(subject.id()).await.unwrap().is_some());
    assert_eq!(repo.list_ids().await.unwrap(), vec![subject.id()]);
    drop_schema(&pool, &schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL and the integration feature"]
async fn transactions_round_trip_with_both_keys() {
    let (pool, schema) = pool_with_schema().await;
    let subject = subject(&pool).await;
    let repo = PgTransactionRepository::new(pool.clone());
    let row = transaction(&subject, 1, "BTC", Kind::Buy, "1", Some("100"), 10);
    repo.insert(&row).await.unwrap();
    let stored = repo.find_by_id(row.id()).await.unwrap().unwrap();
    assert_eq!(stored.canonical_key(), row.canonical_key());
    assert!(repo.canonical_exists(row.canonical_key()).await.unwrap());
    assert!(repo
        .external_exists(row.external_key().unwrap())
        .await
        .unwrap());
    drop_schema(&pool, &schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL and the integration feature"]
async fn a_duplicate_canonical_key_is_a_conflict() {
    let (pool, schema) = pool_with_schema().await;
    let subject = subject(&pool).await;
    let repo = PgTransactionRepository::new(pool.clone());
    let row = transaction(&subject, 1, "BTC", Kind::Buy, "1", Some("100"), 10);
    repo.insert(&row).await.unwrap();
    let duplicate = Transaction::from_stored(
        TransactionId::new(Uuid::from_u128(2)),
        row.subject_id(),
        None,
        row.asset_id().to_string(),
        row.kind(),
        row.quantity(),
        row.price(),
        row.price_source(),
        row.currency().to_string(),
        row.source(),
        Some("different-ref".to_string()),
        row.canonical_key().to_string(),
        Some("user|venue|different-ref".to_string()),
        row.executed_at(),
        row.recorded_at(),
    );
    assert!(matches!(
        repo.insert(&duplicate).await,
        Err(RepositoryError::Conflict(_))
    ));
    drop_schema(&pool, &schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL and the integration feature"]
async fn the_transaction_list_pages_and_orders() {
    let (pool, schema) = pool_with_schema().await;
    let subject = subject(&pool).await;
    let repo = PgTransactionRepository::new(pool.clone());
    for (index, hour) in [9_u32, 10, 11].iter().enumerate() {
        repo.insert(&transaction(
            &subject,
            index as u128 + 1,
            "BTC",
            Kind::Buy,
            "1",
            Some("100"),
            *hour,
        ))
        .await
        .unwrap();
    }
    let first = repo.list_page(subject.id(), None, 2).await.unwrap();
    assert_eq!(first.len(), 2);
    assert_eq!(first[0].executed_at(), at(11));
    let cursor = TransactionCursor {
        executed_at: first[1].executed_at(),
        id: first[1].id(),
    };
    let second = repo.list_page(subject.id(), Some(cursor), 2).await.unwrap();
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].executed_at(), at(9));
    assert_eq!(repo.count_for_subject(subject.id()).await.unwrap(), 3);
    assert_eq!(repo.list_subjects().await.unwrap(), vec![subject.id()]);
    drop_schema(&pool, &schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL and the integration feature"]
async fn corrections_round_trip_and_find_the_last_time() {
    let (pool, schema) = pool_with_schema().await;
    let subject = subject(&pool).await;
    let transactions = PgTransactionRepository::new(pool.clone());
    let row = transaction(&subject, 1, "BTC", Kind::Buy, "1", Some("100"), 10);
    transactions.insert(&row).await.unwrap();
    let repo = PgCorrectionRepository::new(pool.clone());
    let correction = Correction::record(
        CorrectionId::new(Uuid::from_u128(50)),
        row.id(),
        CorrectionField::Quantity,
        "1",
        "2",
        "a typo",
        "user-1",
        at(12),
    )
    .unwrap();
    repo.insert(&correction).await.unwrap();
    assert_eq!(repo.list_for_transaction(row.id()).await.unwrap().len(), 1);
    assert_eq!(repo.list_for_subject(subject.id()).await.unwrap().len(), 1);
    assert_eq!(
        repo.last_corrected_at(subject.id()).await.unwrap(),
        Some(at(12))
    );
    drop_schema(&pool, &schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL and the integration feature"]
async fn lots_are_replaced_atomically() {
    let (pool, schema) = pool_with_schema().await;
    let subject = subject(&pool).await;
    let transactions = PgTransactionRepository::new(pool.clone());
    let row = transaction(&subject, 1, "BTC", Kind::Buy, "1", Some("100"), 10);
    transactions.insert(&row).await.unwrap();
    let repo = PgLotRepository::new(pool.clone());
    let lot = CostBasisLot::acquire(
        CostBasisLotId::new(Uuid::from_u128(60)),
        subject.id(),
        "BTC",
        at(10),
        dec("1"),
        dec("100"),
        "USD",
        row.id(),
    )
    .unwrap();
    repo.replace_for_subject(subject.id(), &[lot])
        .await
        .unwrap();
    assert_eq!(repo.list_open(subject.id()).await.unwrap().len(), 1);
    repo.replace_for_subject(subject.id(), &[]).await.unwrap();
    assert!(repo.list_open(subject.id()).await.unwrap().is_empty());
    drop_schema(&pool, &schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL and the integration feature"]
async fn snapshots_upsert_by_scope() {
    let (pool, schema) = pool_with_schema().await;
    let subject = subject(&pool).await;
    let repo = PgSnapshotRepository::new(pool.clone());
    let report = PnlReport {
        as_of: at(12),
        currency: "USD".to_string(),
        fx_as_of: None,
        fx_estimated: false,
        net_invested: Some(dec("100")),
        realized: Some(dec("10")),
        unrealized: Some(dec("5")),
        total_return: Some(dec("15")),
        total_acquisition_cost: Some(dec("200")),
        marks_unavailable: false,
        holdings_unavailable: false,
        fx_unavailable: false,
        coverage: Decimal::ONE,
        label: ledger_domain::CoverageLabel::Complete,
        coverage_gap: Vec::new(),
        assets: Vec::new(),
    };
    let first = PnlSnapshot::record(
        PnlSnapshotId::new(Uuid::from_u128(70)),
        subject.id(),
        None,
        &report,
        at(12),
    );
    repo.upsert(&first).await.unwrap();
    let second = PnlSnapshot::record(
        PnlSnapshotId::new(Uuid::from_u128(71)),
        subject.id(),
        None,
        &report,
        at(13),
    );
    repo.upsert(&second).await.unwrap();
    let latest = repo.find_latest(subject.id(), None).await.unwrap().unwrap();
    assert_eq!(latest.computed_at(), at(13));
    assert_eq!(latest.net_invested(), Some(dec("100")));
    drop_schema(&pool, &schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL and the integration feature"]
async fn transfer_pairs_match_and_reject() {
    let (pool, schema) = pool_with_schema().await;
    let subject = subject(&pool).await;
    let transactions = PgTransactionRepository::new(pool.clone());
    let out = transaction(&subject, 1, "BTC", Kind::TransferOut, "1", None, 10);
    let incoming = transaction(&subject, 2, "BTC", Kind::TransferIn, "1", None, 10);
    transactions.insert(&out).await.unwrap();
    transactions.insert(&incoming).await.unwrap();
    let repo = PgTransferPairRepository::new(pool.clone());
    let pair = TransferPair::pending(
        TransferPairId::new(Uuid::from_u128(80)),
        subject.id(),
        out.id(),
        "BTC",
    );
    repo.insert(&pair).await.unwrap();
    repo.mark_matched(out.id(), incoming.id(), at(10))
        .await
        .unwrap();
    let matched = repo.list_for_subject(subject.id()).await.unwrap();
    assert!(matched[0].is_matched());
    assert_eq!(matched[0].in_transaction_id(), Some(incoming.id()));
    drop_schema(&pool, &schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL and the integration feature"]
async fn anonymisation_requests_are_idempotent() {
    let (pool, schema) = pool_with_schema().await;
    let repo = PgAnonymisationRepository::new(pool.clone());
    assert_eq!(repo.find("idem-1").await.unwrap(), None);
    repo.record("idem-1", "anon-1", at(12)).await.unwrap();
    repo.record("idem-1", "anon-2", at(13)).await.unwrap();
    assert_eq!(
        repo.find("idem-1").await.unwrap(),
        Some("anon-1".to_string())
    );
    drop_schema(&pool, &schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL and the integration feature"]
async fn the_reconcile_lease_is_exclusive() {
    let (pool, schema) = pool_with_schema().await;
    let repo = PgReconcileLease::new(pool.clone());
    assert!(repo
        .acquire(at(10), at(10) + chrono::Duration::minutes(5))
        .await
        .unwrap());
    assert!(!repo
        .acquire(at(10), at(10) + chrono::Duration::minutes(5))
        .await
        .unwrap());
    repo.release(at(11)).await.unwrap();
    assert_eq!(repo.last_completed().await.unwrap(), Some(at(11)));
    assert!(repo
        .acquire(at(11), at(11) + chrono::Duration::minutes(5))
        .await
        .unwrap());
    drop_schema(&pool, &schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL and the integration feature"]
async fn service_tokens_resolve_by_hash() {
    let (pool, schema) = pool_with_schema().await;
    sqlx::query(
        "INSERT INTO service_token_registry (service, token_sha256, allowed_operations)
         VALUES ($1, $2, $3)",
    )
    .bind("portfolio")
    .bind(sha256_hex("portfolio-token"))
    .bind(vec!["trades.write".to_string()])
    .execute(&pool)
    .await
    .unwrap();
    let registry = PgServiceTokenRegistry::new(pool.clone());
    let caller = registry.resolve("portfolio-token").await.unwrap().unwrap();
    assert_eq!(caller.service, "portfolio");
    assert!(caller.allows("trades.write"));
    assert!(registry.resolve("nope").await.unwrap().is_none());
    drop_schema(&pool, &schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL and the integration feature"]
async fn readiness_reports_the_migrated_database() {
    let (pool, schema) = pool_with_schema().await;
    let probe = PgReadinessProbe::new(pool.clone());
    probe.check().await.unwrap();
    drop_schema(&pool, &schema).await;
}
