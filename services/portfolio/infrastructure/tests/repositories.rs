//! Repository integration tests. They need a real Postgres schema and are
//! ignored by default; run them with
//! `TEST_DATABASE_URL=... cargo test -p portfolio-infrastructure --features integration -- --ignored`.
//!
//! Each test points the migration at a disposable schema so the assertions do
//! not depend on shared state. The runtime role's insert-only grant on snapshots
//! is part of what these tests prove.

use chrono::{DateTime, TimeZone, Utc};
use portfolio_domain::{Portfolio, PortfolioId, UserId};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool() -> PgPool {
    let url = std::env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL must point at a disposable Postgres database");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("the test database must be reachable");
    portfolio_infrastructure::migrations::run(&pool)
        .await
        .expect("the migrations must apply");
    pool
}

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, hour, 0, 0).unwrap()
}

fn portfolio() -> Portfolio {
    let name = format!("Portfolio {}", Uuid::new_v4());
    Portfolio::new(
        PortfolioId::new(Uuid::new_v4()),
        UserId::new(Uuid::new_v4()),
        &name,
        "USD",
        at(0),
    )
    .unwrap()
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL"]
async fn portfolio_round_trips() {
    use portfolio_application::ports::PortfolioRepository;
    use portfolio_infrastructure::repositories::PgPortfolioRepository;

    let pool = pool().await;
    let repository = PgPortfolioRepository::new(pool);
    let value = portfolio();
    repository.insert(&value).await.unwrap();
    let found = repository.find_by_id(value.id()).await.unwrap().unwrap();
    assert_eq!(found.name(), value.name());
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL"]
async fn a_duplicate_name_conflicts() {
    use portfolio_application::ports::{PortfolioRepository, RepositoryError};
    use portfolio_infrastructure::repositories::PgPortfolioRepository;

    let pool = pool().await;
    let repository = PgPortfolioRepository::new(pool);
    let value = portfolio();
    repository.insert(&value).await.unwrap();
    let duplicate = Portfolio::new(
        PortfolioId::new(Uuid::new_v4()),
        value.user_id(),
        &value.name().to_ascii_uppercase(),
        "USD",
        at(0),
    )
    .unwrap();
    assert!(matches!(
        repository.insert(&duplicate).await.unwrap_err(),
        RepositoryError::Conflict(_)
    ));
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL"]
async fn a_membership_move_updates_the_row() {
    use portfolio_application::ports::{MembershipRepository, PortfolioRepository};
    use portfolio_domain::{AccountId, PortfolioAccount, PortfolioAccountId};
    use portfolio_infrastructure::repositories::{PgMembershipRepository, PgPortfolioRepository};

    let pool = pool().await;
    let portfolios = PgPortfolioRepository::new(pool.clone());
    let memberships = PgMembershipRepository::new(pool);
    let first = portfolio();
    let second = portfolio();
    portfolios.insert(&first).await.unwrap();
    portfolios.insert(&second).await.unwrap();
    let account = AccountId::new(Uuid::new_v4());
    memberships
        .assign(&PortfolioAccount::new(
            PortfolioAccountId::new(Uuid::new_v4()),
            first.id(),
            account,
            at(0),
        ))
        .await
        .unwrap();
    memberships
        .assign(&PortfolioAccount::new(
            PortfolioAccountId::new(Uuid::new_v4()),
            second.id(),
            account,
            at(1),
        ))
        .await
        .unwrap();
    assert!(memberships
        .list_for_portfolio(first.id())
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        memberships
            .list_for_portfolio(second.id())
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL"]
async fn a_duplicate_hour_snapshot_conflicts() {
    use portfolio_application::ports::{PortfolioRepository, RepositoryError, SnapshotRepository};
    use portfolio_domain::ValuationSnapshot;
    use portfolio_infrastructure::repositories::{PgPortfolioRepository, PgSnapshotRepository};

    let pool = pool().await;
    let portfolios = PgPortfolioRepository::new(pool.clone());
    let snapshots = PgSnapshotRepository::new(pool);
    let value = portfolio();
    portfolios.insert(&value).await.unwrap();
    let snapshot = ValuationSnapshot::new(
        portfolio_domain::ValuationSnapshotId::new(Uuid::new_v4()),
        value.id(),
        at(0),
        rust_decimal::Decimal::from(100),
        "USD",
        "a".repeat(64),
        "{}".to_string(),
        "{}".to_string(),
    )
    .unwrap();
    snapshots.insert(&snapshot).await.unwrap();
    let duplicate = ValuationSnapshot::new(
        portfolio_domain::ValuationSnapshotId::new(Uuid::new_v4()),
        value.id(),
        at(0),
        rust_decimal::Decimal::from(100),
        "USD",
        "b".repeat(64),
        "{}".to_string(),
        "{}".to_string(),
    )
    .unwrap();
    assert!(matches!(
        snapshots.insert(&duplicate).await.unwrap_err(),
        RepositoryError::Conflict(_)
    ));
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL"]
async fn the_run_lease_is_single_flight() {
    use portfolio_application::ports::{RunRepository, RunStatus, ValuationRun};
    use portfolio_infrastructure::repositories::PgRunRepository;

    let pool = pool().await;
    let runs = PgRunRepository::new(pool);
    // Every test shares one schema, so the job name must be unique per run:
    // the two claims below then contend only with each other.
    let job = format!("portfolio-valuation-{}", Uuid::new_v4());
    let run = ValuationRun {
        id: Uuid::new_v4(),
        job,
        owner: "test".to_string(),
        started_at: at(0),
        lease_until: at(0) + chrono::Duration::seconds(900),
        finished_at: None,
        status: RunStatus::Running,
        detail: None,
    };
    assert!(runs.claim(&run).await.unwrap());
    let later = ValuationRun {
        id: Uuid::new_v4(),
        ..run.clone()
    };
    assert!(!runs.claim(&later).await.unwrap());
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL"]
async fn the_prune_removes_only_old_snapshots() {
    use portfolio_application::ports::{PortfolioRepository, SnapshotRepository};
    use portfolio_domain::ValuationSnapshot;
    use portfolio_infrastructure::repositories::{PgPortfolioRepository, PgSnapshotRepository};

    let pool = pool().await;
    let portfolios = PgPortfolioRepository::new(pool.clone());
    let snapshots = PgSnapshotRepository::new(pool.clone());
    let value = portfolio();
    portfolios.insert(&value).await.unwrap();

    let old_at = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
    let new_at = Utc.with_ymd_and_hms(2026, 11, 7, 0, 0, 0).unwrap();
    for (id, as_of) in [(Uuid::new_v4(), old_at), (Uuid::new_v4(), new_at)] {
        snapshots
            .insert(
                &ValuationSnapshot::new(
                    portfolio_domain::ValuationSnapshotId::new(id),
                    value.id(),
                    as_of,
                    rust_decimal::Decimal::from(100),
                    "USD",
                    "a".repeat(64),
                    "{}".to_string(),
                    "{}".to_string(),
                )
                .unwrap(),
            )
            .await
            .unwrap();
    }

    let removed = portfolio_infrastructure::migrations::prune_snapshots(&pool, 730)
        .await
        .unwrap();
    assert_eq!(removed, 1);
    let remaining = snapshots.list(value.id(), None, 10, None).await.unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].as_of(), new_at);
}
