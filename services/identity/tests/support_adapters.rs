//! Adapter and migration integration tests for support access (BR3.1–BR3.13).
//!
//! Run with a disposable database:
//! `TEST_DATABASE_URL=postgres://... cargo test -p identity --test support_adapters -- --ignored`
//!
//! Every case creates its own schema, runs the real migrations into it, and
//! drops the schema afterwards, so cases run in any order.

mod common;

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::get;
use axum::Router;

use chrono::{DateTime, Duration, TimeZone, Utc};
use identity_application::ports::{
    SupportAccountsSource, SupportDrainDelivery, SupportDrainObservation, SupportGrantRepository,
    SupportOutbox, SupportPortfoliosSource,
};
use identity_domain::support::{SupportAccessGrant, SupportGrantId, SupportGrantStatus};
use identity_domain::{AuditOutcome, AuditRecord, UserId};
use identity_infrastructure::repositories::PgSupportGrantRepository;
use identity_infrastructure::support::backlog::SupportOutboxBacklog;
use identity_infrastructure::support::drainer::SupportOutboxDrainer;
use identity_infrastructure::support::outbox::{SupportDelivery, SupportOutboxStore};
use identity_infrastructure::support::sources::{HttpSupportAccounts, HttpSupportPortfolios};
use sqlx::PgPool;
use uuid::Uuid;

use common::{drop_schema, test_pool};

#[derive(Default)]
struct DrainObservation {
    deliveries: Mutex<Vec<SupportDrainDelivery>>,
    passes: Mutex<Vec<(u64, i64)>>,
}

impl SupportDrainObservation for DrainObservation {
    fn record_delivery(&self, outcome: SupportDrainDelivery) {
        self.deliveries.lock().unwrap().push(outcome);
    }

    fn record_pass(&self, pending: u64, age: i64) {
        self.passes.lock().unwrap().push((pending, age));
    }
}

#[derive(Default)]
struct SourceStub {
    status: AtomicU16,
    body: Mutex<String>,
    requests: Mutex<Vec<(String, String)>>,
}

async fn source_response(
    State(stub): State<Arc<SourceStub>>,
    headers: HeaderMap,
    axum::extract::Path(user_id): axum::extract::Path<String>,
) -> (StatusCode, String) {
    stub.requests.lock().unwrap().push((
        user_id,
        headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string(),
    ));
    (
        StatusCode::from_u16(stub.status.load(Ordering::Relaxed)).unwrap(),
        stub.body.lock().unwrap().clone(),
    )
}

async fn serve_source(stub: Arc<SourceStub>) -> String {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let address = listener.local_addr().unwrap();
    let app = Router::new()
        .route(
            "/v1/internal/support-metadata/{user_id}",
            get(source_response),
        )
        .with_state(stub);
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    format!("http://{address}")
}

#[derive(Default)]
struct AuditStub {
    status: AtomicU16,
    requests: Mutex<Vec<(String, String)>>,
}

async fn audit_response(
    State(stub): State<Arc<AuditStub>>,
    headers: HeaderMap,
    body: String,
) -> StatusCode {
    stub.requests.lock().unwrap().push((
        headers
            .get("idempotency-key")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string(),
        body,
    ));
    StatusCode::from_u16(stub.status.load(Ordering::Relaxed)).unwrap()
}

async fn serve_audit(stub: Arc<AuditStub>) -> String {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let address = listener.local_addr().unwrap();
    let app = Router::new()
        .route("/v1/events", axum::routing::post(audit_response))
        .with_state(stub);
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    format!("http://{address}")
}

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, hour, 0, 0).unwrap()
}

/// Seeds one active user row and returns its id.
async fn seed_user(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, role, status, locale, base_currency, timezone, created_at, updated_at)
         VALUES ($1, 'user', 'active', 'en', 'USD', 'UTC', $2, $2)",
    )
    .bind(id)
    .bind(at(9))
    .execute(pool)
    .await
    .expect("seed a user");
    id
}

/// Inserts one grant row directly, so a constraint case does not depend on the
/// repository under test.
#[allow(clippy::too_many_arguments)]
async fn insert_grant(
    pool: &PgPool,
    user_id: Uuid,
    agent_id: Uuid,
    status: &str,
    approved_by: Option<Uuid>,
    approved_at: Option<DateTime<Utc>>,
    expires_at: Option<DateTime<Utc>>,
    revoked_at: Option<DateTime<Utc>>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO support_access_grants
             (id, user_id, support_agent_id, reason, status, requested_at, approved_by,
              approved_at, expires_at, revoked_at)
         VALUES ($1, $2, $3, 'investigate a reported sync failure', $4, $5, $6, $7, $8, $9)",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(agent_id)
    .bind(status)
    .bind(at(9))
    .bind(approved_by)
    .bind(approved_at)
    .bind(expires_at)
    .bind(revoked_at)
    .execute(pool)
    .await
    .map(|_| ())
}

/// Inserts one grant row whose ordering timestamp is chosen by the caller.
/// Only the two shapes that admit NULL approval fields are used here.
async fn insert_grant_at(
    pool: &PgPool,
    user_id: Uuid,
    agent_id: Uuid,
    status: &str,
    requested_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO support_access_grants
             (id, user_id, support_agent_id, reason, status, requested_at)
         VALUES ($1, $2, $3, 'investigate a reported sync failure', $4, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(agent_id)
    .bind(status)
    .bind(requested_at)
    .execute(pool)
    .await
    .map(|_| ())
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn the_support_outbox_has_the_state_machine_columns_and_partial_indexes() {
    let (pool, schema) = test_pool().await;

    let columns: Vec<(String, String)> = sqlx::query_as(
        "SELECT column_name, data_type
         FROM information_schema.columns
         WHERE table_schema = current_schema() AND table_name = 'support_audit_outbox'
         ORDER BY column_name",
    )
    .fetch_all(&pool)
    .await
    .expect("read the outbox columns");
    let names: Vec<&str> = columns.iter().map(|(name, _)| name.as_str()).collect();
    for required in [
        "attempts",
        "created_at",
        "delivered_at",
        "id",
        "idempotency_key",
        "last_error",
        "next_attempt_at",
        "payload",
        "state",
    ] {
        assert!(
            names.contains(&required),
            "support_audit_outbox is missing the {required} column; found {names:?}"
        );
    }

    let indexes: Vec<String> = sqlx::query_scalar(
        "SELECT indexname FROM pg_indexes
         WHERE schemaname = current_schema() AND tablename = 'support_audit_outbox'",
    )
    .fetch_all(&pool)
    .await
    .expect("read the outbox indexes");
    assert!(indexes
        .iter()
        .any(|name| name == "support_audit_outbox_due_idx"));
    assert!(indexes
        .iter()
        .any(|name| name == "support_audit_outbox_delivered_idx"));

    // The state vocabulary is closed: a fourth value is refused.
    let refused = sqlx::query(
        "INSERT INTO support_audit_outbox (payload, idempotency_key, state, next_attempt_at, created_at)
         VALUES ('{}'::jsonb, $1, 'failed', $2, $2)",
    )
    .bind(format!("support:{}", Uuid::new_v4()))
    .bind(at(9))
    .execute(&pool)
    .await;
    assert!(refused.is_err(), "'failed' is not an outbox state");

    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn the_approval_shape_admits_an_unapproved_expired_row_and_nothing_else() {
    let (pool, schema) = test_pool().await;
    let user = seed_user(&pool).await;
    let agent = Uuid::new_v4();

    insert_grant(&pool, user, agent, "expired", None, None, None, None)
        .await
        .expect("a request that expired before approval has no approval fields");

    let refused = insert_grant(
        &pool,
        user,
        agent,
        "active",
        Some(Uuid::new_v4()),
        None,
        Some(at(10)),
        None,
    )
    .await;
    assert!(
        refused.is_err(),
        "an active grant still needs approved_at; the relaxation is exactly one clause"
    );

    let refused = insert_grant(
        &pool,
        user,
        agent,
        "expired",
        Some(Uuid::new_v4()),
        None,
        None,
        None,
    )
    .await;
    assert!(
        refused.is_err(),
        "an expired row cannot carry an approver without the approval timestamps"
    );

    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn a_second_live_grant_for_the_same_pair_is_rejected_by_the_partial_index() {
    let (pool, schema) = test_pool().await;
    let user = seed_user(&pool).await;
    let agent = Uuid::new_v4();

    insert_grant(
        &pool,
        user,
        agent,
        "pending_approval",
        None,
        None,
        None,
        None,
    )
    .await
    .expect("the first live grant");

    let refused = insert_grant(
        &pool,
        user,
        agent,
        "pending_approval",
        None,
        None,
        None,
        None,
    )
    .await;
    assert!(
        refused
            .as_ref()
            .err()
            .and_then(sqlx::Error::as_database_error)
            .is_some_and(sqlx::error::DatabaseError::is_unique_violation),
        "the (agent, target) partial unique index refuses a second live grant: {refused:?}"
    );

    // A terminal grant does not occupy the slot.
    let other_agent = Uuid::new_v4();
    insert_grant(
        &pool,
        user,
        other_agent,
        "revoked",
        None,
        None,
        None,
        Some(at(10)),
    )
    .await
    .expect("a revoked grant is not live");
    insert_grant(
        &pool,
        user,
        other_agent,
        "pending_approval",
        None,
        None,
        None,
        None,
    )
    .await
    .expect("a new request is allowed once the old grant is terminal");

    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn the_outbox_grants_match_what_the_drainer_needs() {
    let (pool, schema) = test_pool().await;

    let privilege = |role: &'static str, privilege: &'static str| {
        let pool = pool.clone();
        async move {
            sqlx::query_scalar::<_, bool>(
                "SELECT has_table_privilege($1, 'support_audit_outbox', $2)",
            )
            .bind(role)
            .bind(privilege)
            .fetch_one(&pool)
            .await
            .expect("read the privilege")
        }
    };

    for wanted in ["SELECT", "INSERT", "UPDATE", "DELETE"] {
        assert!(
            privilege("identity_app", wanted).await,
            "identity_app needs {wanted} for the drainer's cleanup"
        );
    }
    assert!(
        privilege("identity_readonly", "SELECT").await,
        "identity_readonly reads the backlog"
    );
    assert!(
        !privilege("identity_readonly", "INSERT").await,
        "identity_readonly never writes"
    );
    assert!(
        privilege("identity_migrator", "TRUNCATE").await,
        "the migration role owns the table's DDL"
    );

    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn the_down_migration_guards_the_outbox_and_restores_the_old_shape() {
    let (pool, schema) = test_pool().await;

    // 0008 sits above the pair under test; reverse it so `revert` lands on the
    // 0007 down migration this case exercises.
    identity_infrastructure::migrations::revert(&pool)
        .await
        .expect("revert the newer 0008 index migration");

    sqlx::query(
        "INSERT INTO support_audit_outbox (payload, idempotency_key, next_attempt_at, created_at)
         VALUES ('{}'::jsonb, $1, $2, $2)",
    )
    .bind(format!("support:{}", Uuid::new_v4()))
    .bind(at(9))
    .execute(&pool)
    .await
    .expect("seed a pending row");

    let refused = identity_infrastructure::migrations::revert(&pool).await;
    assert!(
        refused.is_err(),
        "a rollback must not drop a pending audit intent"
    );

    sqlx::query("DELETE FROM support_audit_outbox")
        .execute(&pool)
        .await
        .expect("drain the outbox");
    identity_infrastructure::migrations::revert(&pool)
        .await
        .expect("the down migration applies once the outbox is empty");

    let restored: bool = sqlx::query_scalar(
        "SELECT convalidated FROM pg_constraint
         WHERE conname = 'support_grants_approval_shape'
           AND conrelid = 'support_access_grants'::regclass",
    )
    .fetch_one(&pool)
    .await
    .expect("the approval-shape constraint is restored");
    assert!(
        !restored,
        "the rollback restores the constraint NOT VALID, so a big table is not rescanned"
    );

    let user = seed_user(&pool).await;
    let refused = insert_grant(
        &pool,
        user,
        Uuid::new_v4(),
        "expired",
        None,
        None,
        None,
        None,
    )
    .await;
    assert!(
        refused.is_err(),
        "after the rollback an unapproved expired row is refused again"
    );

    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn the_migration_pair_is_reversible_once_more() {
    let (pool, schema) = test_pool().await;

    // Reverse the newer 0008 index migration first, so the pair this case
    // exercises is the 0007 one.
    identity_infrastructure::migrations::revert(&pool)
        .await
        .expect("revert 0008");
    identity_infrastructure::migrations::revert(&pool)
        .await
        .expect("revert 0007");
    identity_infrastructure::migrations::run(&pool)
        .await
        .expect("re-apply 0007 and 0008");

    let present: bool =
        sqlx::query_scalar("SELECT to_regclass('support_audit_outbox') IS NOT NULL")
            .fetch_one(&pool)
            .await
            .expect("read the table's presence");
    assert!(present, "re-applying the pair recreates the outbox");

    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn the_mine_index_is_created_reversed_and_recreated() {
    let (pool, schema) = test_pool().await;

    let definition: Option<String> = sqlx::query_scalar(
        "SELECT indexdef FROM pg_indexes
         WHERE schemaname = current_schema() AND indexname = 'support_grants_mine_idx'",
    )
    .fetch_optional(&pool)
    .await
    .expect("read the own-grant index");
    let definition = definition.expect("0008 creates the own-grant recency index");
    assert!(
        definition.contains("(support_agent_id, requested_at DESC)"),
        "the index serves the self-scoped recency read: {definition}"
    );

    identity_infrastructure::migrations::revert(&pool)
        .await
        .expect("revert 0008");
    let present: bool =
        sqlx::query_scalar("SELECT to_regclass('support_grants_mine_idx') IS NOT NULL")
            .fetch_one(&pool)
            .await
            .expect("read the index's presence");
    assert!(!present, "reversing 0008 drops the index");

    identity_infrastructure::migrations::run(&pool)
        .await
        .expect("re-apply 0008");
    let present: bool =
        sqlx::query_scalar("SELECT to_regclass('support_grants_mine_idx') IS NOT NULL")
            .fetch_one(&pool)
            .await
            .expect("read the index's presence");
    assert!(present, "re-applying 0008 recreates the index");

    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn the_outbox_accepts_a_payload_that_is_a_json_object_only() {
    let (pool, schema) = test_pool().await;

    let refused = sqlx::query(
        "INSERT INTO support_audit_outbox (payload, idempotency_key, next_attempt_at, created_at)
         VALUES ('[]'::jsonb, $1, $2, $2)",
    )
    .bind(format!("support:{}", Uuid::new_v4()))
    .bind(at(9))
    .execute(&pool)
    .await;
    assert!(refused.is_err(), "an array is not an audit event payload");

    let refused = sqlx::query(
        "INSERT INTO support_audit_outbox (payload, idempotency_key, next_attempt_at, created_at)
         VALUES ('{}'::jsonb, $1, $2, $2)",
    )
    .bind("")
    .bind(at(9))
    .execute(&pool)
    .await;
    assert!(refused.is_err(), "an empty idempotency key is refused");

    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn the_delivery_shape_requires_a_timestamp_when_delivered() {
    let (pool, schema) = test_pool().await;

    let refused = sqlx::query(
        "INSERT INTO support_audit_outbox (payload, idempotency_key, state, next_attempt_at, created_at)
         VALUES ('{}'::jsonb, $1, 'delivered', $2, $2)",
    )
    .bind(format!("support:{}", Uuid::new_v4()))
    .bind(at(9))
    .execute(&pool)
    .await;
    assert!(
        refused.is_err(),
        "a delivered row without delivered_at is refused"
    );

    sqlx::query(
        "INSERT INTO support_audit_outbox
             (payload, idempotency_key, state, next_attempt_at, created_at, delivered_at)
         VALUES ('{}'::jsonb, $1, 'delivered', $2, $2, $2)",
    )
    .bind(format!("support:{}", Uuid::new_v4()))
    .bind(at(9))
    .execute(&pool)
    .await
    .expect("a delivered row keeps the acknowledgement time");

    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn the_app_role_cannot_delete_a_grant() {
    let (pool, schema) = test_pool().await;
    let user = seed_user(&pool).await;
    let agent = Uuid::new_v4();
    insert_grant(
        &pool,
        user,
        agent,
        "pending_approval",
        None,
        None,
        None,
        None,
    )
    .await
    .expect("seed a grant");

    let app = common::role_pool(&schema, "identity_app").await;
    let refused = sqlx::query("DELETE FROM support_access_grants")
        .execute(&app)
        .await;
    assert!(
        refused.is_err(),
        "grant rows are evidence and identity_app holds no DELETE"
    );
    app.close().await;

    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn a_grant_window_over_sixty_minutes_is_refused_by_the_database() {
    let (pool, schema) = test_pool().await;
    let user = seed_user(&pool).await;

    let approved = at(9);
    let refused = insert_grant(
        &pool,
        user,
        Uuid::new_v4(),
        "active",
        Some(Uuid::new_v4()),
        Some(approved),
        Some(approved + Duration::minutes(61)),
        None,
    )
    .await;
    assert!(
        refused.is_err(),
        "the storage-level window bound stays in force"
    );

    drop_schema(&schema).await;
}
// ---------------------------------------------------------------------------
// Repository and outbox behaviour (Step 6)
// ---------------------------------------------------------------------------

fn grant(user: UserId, agent: UserId) -> SupportAccessGrant {
    SupportAccessGrant::request(
        SupportGrantId::generate(),
        user,
        agent,
        "investigate a reported sync failure",
        at(9),
    )
    .expect("a valid request")
}

fn record(actor: UserId, action: &str, target: UserId) -> AuditRecord {
    AuditRecord::new(
        Some(actor),
        action,
        target.to_string(),
        AuditOutcome::Success,
        at(9),
        BTreeMap::new(),
    )
    .expect("a valid audit record")
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn the_repository_lists_one_status_newest_first_and_bounded() {
    let (pool, schema) = test_pool().await;
    let repo = PgSupportGrantRepository::new(pool.clone());
    let agent = UserId::generate();

    // Distinct instants: `insert_grant` always writes at(9), so this case seeds
    // its own rows to make the ordering observable. Each row names its own
    // target, because the live-pair index allows one live grant per
    // (agent, target), and the active row carries its approval shape.
    for (hour, status) in [
        (8, "pending_approval"),
        (10, "pending_approval"),
        (11, "active"),
    ] {
        let row_user = seed_user(&pool).await;
        let (approved_by, approved_at, expires_at) = if status == "active" {
            (Some(Uuid::new_v4()), Some(at(11)), Some(at(hour + 1)))
        } else {
            (None, None, None)
        };
        sqlx::query(
            "INSERT INTO support_access_grants
                 (id, user_id, support_agent_id, reason, status, requested_at,
                  approved_by, approved_at, expires_at)
             VALUES ($1, $2, $3, 'a reason', $4, $5, $6, $7, $8)",
        )
        .bind(Uuid::new_v4())
        .bind(row_user)
        .bind(agent.as_uuid())
        .bind(status)
        .bind(at(hour))
        .bind(approved_by)
        .bind(approved_at)
        .bind(expires_at)
        .execute(&pool)
        .await
        .expect("seed a grant");
    }

    let listed = repo
        .list_by_status(SupportGrantStatus::PendingApproval, 100)
        .await
        .expect("list the pending grants");
    assert_eq!(listed.len(), 2, "only the requested status is returned");
    assert_eq!(listed[0].requested_at(), at(10), "newest first");
    assert_eq!(listed[1].requested_at(), at(8));

    let bounded = repo
        .list_by_status(SupportGrantStatus::PendingApproval, 1)
        .await
        .expect("list with the bound");
    assert_eq!(bounded.len(), 1, "the caller's limit bounds the read");
    assert_eq!(bounded[0].requested_at(), at(10));

    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn the_repository_finds_only_a_live_grant_for_a_pair() {
    let (pool, schema) = test_pool().await;
    let repo = PgSupportGrantRepository::new(pool.clone());
    let user = UserId::new(seed_user(&pool).await);
    let agent = UserId::generate();
    let other_agent = UserId::generate();

    let requested = grant(user, agent);
    repo.insert(&requested).await.expect("insert the request");

    let live = repo
        .find_live_by_agent_and_user(agent, user)
        .await
        .expect("read the live pair")
        .expect("the pending request is live");
    assert_eq!(live.id(), requested.id());
    assert!(repo
        .find_live_by_agent_and_user(other_agent, user)
        .await
        .expect("read another pair")
        .is_none());

    let mut revoked = requested.clone();
    revoked.revoke(at(9)).expect("revoke the request");
    repo.update(&revoked).await.expect("persist the revoke");
    assert!(
        repo.find_live_by_agent_and_user(agent, user)
            .await
            .expect("read after the revoke")
            .is_none(),
        "a revoked grant is history, not a live pair"
    );

    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn the_conditional_approval_returns_one_row_and_then_zero() {
    let (pool, schema) = test_pool().await;
    let repo = PgSupportGrantRepository::new(pool.clone());
    let user = UserId::new(seed_user(&pool).await);
    let agent = UserId::generate();
    let admin = UserId::generate();
    let requested = grant(user, agent);
    let event = record(admin, "identity.support_grant.approved", user);

    let created = repo
        .insert_with_event(&requested, &event, "corr-approve")
        .await;
    // The requested event is `requested`, but the helper only needs any valid
    // action; the approval below is the case under test.
    created.expect("insert the grant and its event together");

    let affected = repo
        .approve_with_event(
            requested.id(),
            admin,
            at(10),
            at(10) + Duration::minutes(60),
            &event,
            "corr-approve",
        )
        .await
        .expect("approve conditionally");
    assert_eq!(affected, 1, "the first approval moves the row");

    let again = repo
        .approve_with_event(
            requested.id(),
            admin,
            at(11),
            at(11) + Duration::minutes(60),
            &event,
            "corr-approve-2",
        )
        .await
        .expect("a conditional update does not error on zero rows");
    assert_eq!(again, 0, "a second approval moves nothing");

    let stored = repo
        .find_by_id(requested.id())
        .await
        .unwrap()
        .expect("the grant is stored");
    assert_eq!(stored.status(), SupportGrantStatus::Active);
    assert_eq!(stored.approved_by(), Some(admin));

    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn the_conditional_revocation_refuses_a_terminal_grant() {
    let (pool, schema) = test_pool().await;
    let repo = PgSupportGrantRepository::new(pool.clone());
    let user = UserId::new(seed_user(&pool).await);
    let agent = UserId::generate();
    let admin = UserId::generate();
    let requested = grant(user, agent);
    repo.insert(&requested).await.expect("insert the request");
    let event = record(admin, "identity.support_grant.revoked", user);

    let first = repo
        .revoke_with_event(requested.id(), at(10), &event, "corr-revoke")
        .await
        .expect("revoke conditionally");
    assert_eq!(first, 1);

    let stored = repo.find_by_id(requested.id()).await.unwrap().unwrap();
    assert_eq!(stored.status(), SupportGrantStatus::Revoked);
    assert_eq!(stored.revoked_at(), Some(at(10)));

    let second = repo
        .revoke_with_event(requested.id(), at(11), &event, "corr-revoke-2")
        .await
        .expect("a conditional update does not error on zero rows");
    assert_eq!(second, 0, "a terminal grant cannot be revoked again");

    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn the_grant_and_its_outbox_row_commit_together() {
    let (pool, schema) = test_pool().await;
    let repo = PgSupportGrantRepository::new(pool.clone());
    let user = UserId::new(seed_user(&pool).await);
    let agent = UserId::generate();
    let event = record(agent, "identity.support_grant.requested", user);

    let first = grant(user, agent);
    repo.insert_with_event(&first, &event, "corr-1")
        .await
        .expect("the first write commits both rows");

    // The second insert breaks the live-pair index after its event row was
    // written inside the transaction, so the rollback must take the event too.
    let second = grant(user, agent);
    let refused = repo.insert_with_event(&second, &event, "corr-2").await;
    assert!(refused.is_err(), "a second live grant is refused");

    let grants: i64 = sqlx::query_scalar("SELECT count(*) FROM support_access_grants")
        .fetch_one(&pool)
        .await
        .unwrap();
    let events: i64 = sqlx::query_scalar("SELECT count(*) FROM support_audit_outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(grants, 1, "the failed request stored no grant");
    assert_eq!(
        events, 1,
        "the failed request stored no orphan audit row: {events} events remain"
    );

    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn two_concurrent_claims_never_take_the_same_row() {
    let (pool, schema) = test_pool().await;
    let store = SupportOutboxStore::new(pool.clone());
    let actor = UserId::generate();
    let target = UserId::generate();
    for index in 0..4 {
        store
            .append(
                &record(actor, "identity.support_read.served", target),
                &format!("corr-{index}"),
                at(9),
            )
            .await
            .expect("append an event");
    }

    let (first, second) = tokio::join!(store.claim(at(9), 5), store.claim(at(9), 5));
    let mut ids: Vec<Uuid> = first
        .expect("first claim")
        .into_iter()
        .chain(second.expect("second claim"))
        .map(|row| row.id)
        .collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(
        ids.len(),
        4,
        "a claimed row is hidden from the other drainer by SKIP LOCKED"
    );

    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn a_transient_failure_backs_off_and_the_third_attempt_is_dead() {
    let (pool, schema) = test_pool().await;
    let store = SupportOutboxStore::new(pool.clone());
    let actor = UserId::generate();
    let target = UserId::generate();
    store
        .append(
            &record(actor, "identity.support_grant.requested", target),
            "corr-retry",
            at(9),
        )
        .await
        .expect("append an event");

    let row = store
        .claim(at(9), 1)
        .await
        .expect("claim")
        .pop()
        .expect("one row");
    assert_eq!(row.attempts, 1);

    let outcome = store
        .record_failure(&row, at(9), "the audit service is unreachable", true)
        .await
        .expect("record the first failure");
    assert_eq!(outcome, SupportDelivery::Retryable);

    let next_attempt: DateTime<Utc> =
        sqlx::query_scalar("SELECT next_attempt_at FROM support_audit_outbox WHERE id = $1")
            .bind(row.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        next_attempt >= at(9) + Duration::seconds(30),
        "the retry backs off by at least 30 seconds"
    );

    // Second and third attempts: the third is dead-lettered.
    let row2 = store
        .claim(at(10), 1)
        .await
        .expect("claim after backoff")
        .pop()
        .expect("the retried row");
    assert_eq!(row2.attempts, 2);
    assert_eq!(
        store
            .record_failure(&row2, at(10), "the audit service is unreachable", true)
            .await
            .expect("second failure"),
        SupportDelivery::Retryable
    );

    let row3 = store
        .claim(at(12), 1)
        .await
        .expect("claim after the second backoff")
        .pop()
        .expect("the row");
    assert_eq!(row3.attempts, 3);
    assert_eq!(
        store
            .record_failure(&row3, at(12), "the audit service is unreachable", true)
            .await
            .expect("third failure"),
        SupportDelivery::Dead
    );

    let state: String = sqlx::query_scalar("SELECT state FROM support_audit_outbox WHERE id = $1")
        .bind(row.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(state, "dead");

    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn a_permanent_failure_is_dead_on_the_first_attempt() {
    let (pool, schema) = test_pool().await;
    let store = SupportOutboxStore::new(pool.clone());
    store
        .append(
            &record(
                UserId::generate(),
                "identity.support_read.denied",
                UserId::generate(),
            ),
            "corr-permanent",
            at(9),
        )
        .await
        .expect("append an event");
    let row = store.claim(at(9), 1).await.unwrap().pop().unwrap();

    assert_eq!(
        store
            .record_failure(&row, at(9), "the audit service refused the event", false)
            .await
            .unwrap(),
        SupportDelivery::Dead
    );

    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn the_cleanup_removes_only_delivered_rows_older_than_a_day() {
    let (pool, schema) = test_pool().await;
    let store = SupportOutboxStore::new(pool.clone());
    let actor = UserId::generate();
    let target = UserId::generate();

    store
        .append(
            &record(actor, "identity.support_read.served", target),
            "corr-old",
            at(9),
        )
        .await
        .expect("append the old row");
    let old_row = store
        .claim(at(9), 1)
        .await
        .unwrap()
        .pop()
        .expect("the old row");
    store.mark_delivered(old_row.id, at(9)).await.unwrap();

    store
        .append(
            &record(actor, "identity.support_read.served", target),
            "corr-new",
            at(9),
        )
        .await
        .expect("append the fresh row");
    let fresh = store.claim(at(9), 1).await.unwrap().pop().unwrap();
    store
        .mark_delivered(fresh.id, at(9) + Duration::hours(20))
        .await
        .unwrap();

    // A pending row must never be swept.
    store
        .append(
            &record(actor, "identity.support_read.denied", target),
            "corr-pending",
            at(9),
        )
        .await
        .expect("append a pending row");

    let removed = store
        .cleanup(at(9) + Duration::hours(25))
        .await
        .expect("run the cleanup");
    assert_eq!(removed, 1, "only the delivered row past 24 h is removed");

    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM support_audit_outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        remaining, 2,
        "the fresh delivered row and the pending row stay"
    );

    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn the_backlog_reads_the_pending_count_and_the_oldest_age() {
    let (pool, schema) = test_pool().await;
    let store = SupportOutboxStore::new(pool.clone());
    let backlog = SupportOutboxBacklog::new(pool.clone());
    let actor = UserId::generate();
    let target = UserId::generate();

    assert_eq!(
        backlog.read(at(12)).await.expect("an empty backlog"),
        (0, 0)
    );

    store
        .append(
            &record(actor, "identity.support_read.served", target),
            "corr-a",
            at(5),
        )
        .await
        .expect("append one");
    store
        .append(
            &record(actor, "identity.support_read.denied", target),
            "corr-b",
            at(5),
        )
        .await
        .expect("append another");

    // Both rows were created at 05:00, so the oldest age read at 12:00 is 7 h.
    let (pending, age) = backlog.read(at(12)).await.expect("read the backlog");
    assert_eq!(pending, 2);
    assert_eq!(age, 7 * 60 * 60, "the age is measured from the oldest row");

    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn the_refusal_append_writes_one_row_in_its_own_transaction() {
    let (pool, schema) = test_pool().await;
    let store = SupportOutboxStore::new(pool.clone());
    let actor = UserId::generate();
    let target = UserId::generate();

    SupportOutbox::append(
        &store,
        &record(actor, "identity.support_read.denied", target),
        "corr-refusal",
        at(9),
    )
    .await
    .expect("append the refusal");

    let (state, key): (String, String) =
        sqlx::query_as("SELECT state, idempotency_key FROM support_audit_outbox")
            .fetch_one(&pool)
            .await
            .expect("read the row");
    assert_eq!(state, "pending");
    assert!(key.starts_with("support:"));

    drop_schema(&schema).await;
}

// ---------------------------------------------------------------------------
// S2 — the support agent's own grants, against the real table
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn the_mine_read_returns_only_the_agents_rows_newest_first() {
    let (pool, schema) = test_pool().await;
    let user = seed_user(&pool).await;
    let agent = Uuid::new_v4();
    let other_agent = Uuid::new_v4();

    insert_grant_at(&pool, user, agent, "expired", at(9))
        .await
        .expect("an older own grant");
    insert_grant_at(&pool, user, agent, "expired", at(10))
        .await
        .expect("a newer own grant");
    insert_grant_at(&pool, user, other_agent, "expired", at(11))
        .await
        .expect("another agent's newer grant");

    let repository = PgSupportGrantRepository::new(pool.clone());
    let mine = repository
        .list_by_support_agent(UserId::new(agent), 20)
        .await
        .expect("read the agent's grants");

    assert_eq!(mine.len(), 2, "only the caller's two grants");
    assert!(
        mine.iter()
            .all(|grant| grant.support_agent_id() == UserId::new(agent)),
        "another agent's row is never returned"
    );
    assert_eq!(mine[0].requested_at(), at(10), "newest first");
    assert_eq!(mine[1].requested_at(), at(9));

    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn the_mine_read_drops_the_twenty_first_row() {
    let (pool, schema) = test_pool().await;
    let user = seed_user(&pool).await;
    let agent = Uuid::new_v4();

    for hour in 0..21 {
        insert_grant_at(&pool, user, agent, "expired", at(hour))
            .await
            .expect("seed one grant per hour");
    }

    let repository = PgSupportGrantRepository::new(pool.clone());
    let mine = repository
        .list_by_support_agent(UserId::new(agent), 20)
        .await
        .expect("read the agent's grants");

    assert_eq!(mine.len(), 20, "the bound drops the twenty-first row");
    assert_eq!(mine[0].requested_at(), at(20), "newest first");
    assert_eq!(
        mine[19].requested_at(),
        at(1),
        "the oldest row is the one dropped"
    );

    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn an_agent_with_no_grants_reads_an_empty_list() {
    let (pool, schema) = test_pool().await;

    let repository = PgSupportGrantRepository::new(pool.clone());
    let mine = repository
        .list_by_support_agent(UserId::new(Uuid::new_v4()), 20)
        .await
        .expect("an empty read is not an error");

    assert!(mine.is_empty());

    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn support_drainer_delivers_and_records_observation() {
    let (pool, schema) = test_pool().await;
    let store = SupportOutboxStore::new(pool.clone());
    let actor = UserId::generate();
    let target = UserId::generate();
    store
        .append(
            &record(actor, "identity.support_read.served", target),
            "corr-drain",
            at(9),
        )
        .await
        .unwrap();
    let stub = Arc::new(AuditStub {
        status: AtomicU16::new(200),
        ..Default::default()
    });
    let observation = Arc::new(DrainObservation::default());
    let audit_url = serve_audit(stub.clone()).await;
    let drainer = SupportOutboxDrainer::new(
        pool.clone(),
        audit_url,
        "audit-token",
        observation.clone(),
        std::time::Duration::from_secs(60),
    );
    let report = drainer.run_once(at(9)).await;
    assert_eq!(report.delivered, 1);
    assert_eq!(
        observation.deliveries.lock().unwrap().as_slice(),
        &[SupportDrainDelivery::Delivered]
    );
    assert_eq!(stub.requests.lock().unwrap().len(), 1);
    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn support_sources_cover_success_and_error_projection_paths() {
    let user_id = UserId::generate();
    let stub = Arc::new(SourceStub {
        status: AtomicU16::new(200),
        body: Mutex::new(
            serde_json::json!({
                "accounts": [{
                    "id": Uuid::new_v4(), "label": "Ledger", "sourceType": "manual",
                    "syncStatus": "ready", "lastSyncedAt": null
                }],
                "portfolios": [{ "id": Uuid::new_v4(), "label": "Long term" }]
            })
            .to_string(),
        ),
        ..Default::default()
    });
    let base = serve_source(stub.clone()).await;
    assert_eq!(
        HttpSupportAccounts::new(&base, "account-token")
            .accounts(user_id)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        HttpSupportPortfolios::new(&base, "portfolio-token")
            .portfolios(user_id)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(stub.requests.lock().unwrap().len(), 2);

    stub.status.store(503, Ordering::Relaxed);
    assert!(HttpSupportAccounts::new(&base, "token")
        .accounts(user_id)
        .await
        .is_err());
    stub.status.store(200, Ordering::Relaxed);
    *stub.body.lock().unwrap() =
        serde_json::json!({"accounts": [{"id": "not-a-uuid"}]}).to_string();
    assert!(HttpSupportAccounts::new(&base, "token")
        .accounts(user_id)
        .await
        .is_err());
    *stub.body.lock().unwrap() =
        serde_json::json!({"portfolios": [{"id": Uuid::new_v4()}]}).to_string();
    assert!(HttpSupportPortfolios::new(&base, "token")
        .portfolios(user_id)
        .await
        .is_err());
}
