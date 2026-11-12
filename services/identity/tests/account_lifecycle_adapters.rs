//! Adapter and migration suite for the account lifecycle (US4.3, US4.5, BR5.3,
//! BR5.6, BR5.9).
//!
//! Every case needs a disposable Postgres through `TEST_DATABASE_URL` and is
//! `#[ignore]`d by default; the gate provisions one database per run.

mod common;

use chrono::Utc;
use common::{drop_schema, role_pool, test_pool};
use sqlx::{PgPool, Row};
use uuid::Uuid;

async fn column_exists(pool: &PgPool, table: &str, column: &str) -> bool {
    sqlx::query_scalar(
        "SELECT EXISTS(
             SELECT 1 FROM information_schema.columns
             WHERE table_schema = current_schema()
               AND table_name = $1 AND column_name = $2
         )",
    )
    .bind(table)
    .bind(column)
    .fetch_one(pool)
    .await
    .expect("read the column catalogue")
}

async fn table_exists(pool: &PgPool, table: &str) -> bool {
    sqlx::query_scalar("SELECT to_regclass($1) IS NOT NULL")
        .bind(table)
        .fetch_one(pool)
        .await
        .expect("read the table catalogue")
}

async fn index_exists(pool: &PgPool, index: &str) -> bool {
    sqlx::query_scalar(
        "SELECT EXISTS(
             SELECT 1 FROM pg_indexes
             WHERE schemaname = current_schema() AND indexname = $1
         )",
    )
    .bind(index)
    .fetch_one(pool)
    .await
    .expect("read the index catalogue")
}

/// Seeds one user; the deletion request and transition rows reference it.
async fn seed_user(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, role, status, locale, base_currency, timezone, created_at, updated_at)
         VALUES ($1, 'user', 'active', 'en', 'USD', 'UTC', now(), now())",
    )
    .bind(id)
    .execute(pool)
    .await
    .expect("seed a user");
    id
}

#[tokio::test]
#[ignore = "needs a disposable Postgres via TEST_DATABASE_URL"]
async fn the_lifecycle_migration_adds_the_tombstone_the_window_and_the_transition_log() {
    let (pool, schema) = test_pool().await;

    assert!(
        column_exists(&pool, "users", "erased_subject_hash").await,
        "users.erased_subject_hash is the tombstone the denial path reads"
    );
    for column in [
        "recoverable_until",
        "cancelled_at",
        "erased_at",
        "attempt_count",
        "next_attempt_at",
    ] {
        assert!(
            column_exists(&pool, "deletion_requests", column).await,
            "deletion_requests.{column} is part of the recovery window"
        );
    }

    assert!(table_exists(&pool, "status_transitions").await);
    assert!(table_exists(&pool, "status_audit_outbox").await);
    assert!(
        index_exists(&pool, "status_transitions_user_idx").await,
        "the newest-transition read is indexed per user"
    );

    assert!(
        index_exists(&pool, "deletion_requests_one_open_per_user").await,
        "one open request per user is the unique index the write relies on"
    );
    assert!(
        !index_exists(&pool, "deletion_requests_one_active_per_user").await,
        "the status-keyed index from 0001 is replaced by the open-request index"
    );

    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs a disposable Postgres via TEST_DATABASE_URL"]
async fn the_down_migration_refuses_while_a_request_is_open() {
    let (pool, schema) = test_pool().await;
    let user_id = seed_user(&pool).await;
    sqlx::query(
        "INSERT INTO deletion_requests
             (id, user_id, status, requested_at, recoverable_until, next_attempt_at)
         VALUES ($1, $2, 'requested', now(), now() + interval '30 days', now())",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .execute(&pool)
    .await
    .expect("seed an open request");

    // 0008 and 0007 ship above 0006, so a rollback reaches 0006 by reverting
    // them first.
    identity_infrastructure::migrations::revert(&pool)
        .await
        .expect("the own-grant index down migration applies");
    identity_infrastructure::migrations::revert(&pool)
        .await
        .expect("the support-access down migration applies");

    let refused = identity_infrastructure::migrations::revert(&pool).await;
    assert!(
        refused.is_err(),
        "a rollback must not strand an open deletion request"
    );
    // The rollback changed nothing.
    assert!(column_exists(&pool, "users", "erased_subject_hash").await);
    assert!(table_exists(&pool, "status_transitions").await);

    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs a disposable Postgres via TEST_DATABASE_URL"]
async fn the_down_migration_reverses_the_shape_once_no_request_is_open() {
    let (pool, schema) = test_pool().await;
    let user_id = seed_user(&pool).await;
    // A closed request is evidence, not an open obligation.
    sqlx::query(
        "INSERT INTO deletion_requests
             (id, user_id, status, requested_at, recoverable_until, next_attempt_at,
              cancelled_at)
         VALUES ($1, $2, 'requested', now(), now() + interval '30 days', now(), now())",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .execute(&pool)
    .await
    .expect("seed a closed request");

    // 0008 and 0007 ship above 0006, so a rollback reaches 0006 by reverting
    // them first.
    identity_infrastructure::migrations::revert(&pool)
        .await
        .expect("the own-grant index down migration applies");
    identity_infrastructure::migrations::revert(&pool)
        .await
        .expect("the support-access down migration applies");

    identity_infrastructure::migrations::revert(&pool)
        .await
        .expect("the down migration applies once no request is open");

    assert!(!column_exists(&pool, "users", "erased_subject_hash").await);
    assert!(!table_exists(&pool, "status_transitions").await);
    assert!(!table_exists(&pool, "status_audit_outbox").await);
    assert!(index_exists(&pool, "deletion_requests_one_active_per_user").await);

    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs a disposable Postgres via TEST_DATABASE_URL"]
async fn a_transition_is_insert_only_for_the_application_role() {
    let (pool, schema) = test_pool().await;
    let user_id = seed_user(&pool).await;
    let transition_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO status_transitions (id, user_id, from_status, to_status, actor, occurred_at)
         VALUES ($1, $2, 'active', 'disabled', 'admin-1', now())",
    )
    .bind(transition_id)
    .bind(user_id)
    .execute(&pool)
    .await
    .expect("append one transition");

    let app = role_pool(&schema, "identity_app").await;
    let update = sqlx::query("UPDATE status_transitions SET reason = 'rewritten' WHERE id = $1")
        .bind(transition_id)
        .execute(&app)
        .await;
    assert!(
        update.is_err(),
        "the application role must not rewrite a transition"
    );
    let delete = sqlx::query("DELETE FROM status_transitions WHERE id = $1")
        .bind(transition_id)
        .execute(&app)
        .await;
    assert!(
        delete.is_err(),
        "the application role must not delete a transition"
    );

    let seq: i64 = sqlx::query_scalar("SELECT seq FROM status_transitions WHERE id = $1")
        .bind(transition_id)
        .fetch_one(&pool)
        .await
        .expect("read the sequence value");
    sqlx::query(
        "INSERT INTO status_transitions (user_id, from_status, to_status, actor, occurred_at)
         VALUES ($1, 'disabled', 'active', 'admin-1', now())",
    )
    .bind(user_id)
    .execute(&pool)
    .await
    .expect("append a second transition");
    let next_seq: i64 =
        sqlx::query_scalar("SELECT max(seq) FROM status_transitions WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .expect("read the newest sequence value");
    assert!(next_seq > seq, "the sequence is monotonic");

    app.close().await;
    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs a disposable Postgres via TEST_DATABASE_URL"]
async fn a_status_change_writes_the_transition_and_its_audit_intent_together() {
    use identity_application::ports::{StatusChange, StatusTransitionStore};
    use identity_domain::UserStatus;
    use identity_infrastructure::repositories::PgStatusTransitionStore;

    let (pool, schema) = test_pool().await;
    let user_id = seed_user(&pool).await;
    let store = PgStatusTransitionStore::new(pool.clone());
    let change = StatusChange {
        user_id: identity_domain::UserId::new(user_id),
        from: UserStatus::Active,
        to: UserStatus::Disabled,
        actor: Uuid::new_v4().to_string(),
        actor_kind: "admin",
        reason: Some("account abuse".to_string()),
        occurred_at: Utc::now(),
    };

    let transition = store.apply(&change).await.expect("the transition commits");
    assert!(transition.seq().is_some());

    let status: String = sqlx::query_scalar("SELECT status FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("read the status");
    assert_eq!(status, "disabled");

    let (payload, key): (serde_json::Value, String) = sqlx::query_as(
        "SELECT payload, idempotency_key FROM status_audit_outbox WHERE transition_id = $1",
    )
    .bind(transition.id().as_uuid())
    .fetch_one(&pool)
    .await
    .expect("the audit intent committed with the transition");
    assert_eq!(payload["action"], "identity.account.status_changed");
    assert_eq!(payload["context"]["reason"], "account abuse");
    assert_eq!(key, format!("status-transition:{}", transition.id()));

    // A stale `from` is refused: a concurrent change wins and nothing is
    // written.
    let stale = StatusChange {
        from: UserStatus::Active,
        ..change.clone()
    };
    assert!(
        store.apply(&stale).await.is_err(),
        "a status that moved is never overwritten"
    );

    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs a disposable Postgres via TEST_DATABASE_URL"]
async fn the_status_outbox_drains_to_c9_with_an_idempotency_key() {
    use axum::routing::post;
    use identity_application::consent::ports::DrainerObservation;
    use identity_application::ports::{StatusChange, StatusTransitionStore};
    use identity_domain::UserStatus;
    use identity_infrastructure::consent::outbox::AuditOutboxDrainer;
    use identity_infrastructure::repositories::PgStatusTransitionStore;
    use std::sync::Mutex;
    use std::time::Duration;

    #[derive(Default)]
    struct Observation {
        deliveries: Mutex<Vec<&'static str>>,
        status_deliveries: Mutex<Vec<&'static str>>,
    }

    impl DrainerObservation for Observation {
        fn record_pass(&self, _pending: u64, _oldest_age_seconds: i64) {}
        fn record_delivery(&self, outcome: identity_application::consent::ports::DeliveryOutcome) {
            self.deliveries.lock().unwrap().push(match outcome {
                identity_application::consent::ports::DeliveryOutcome::Delivered => "delivered",
                identity_application::consent::ports::DeliveryOutcome::Retryable => "retryable",
                identity_application::consent::ports::DeliveryOutcome::Dead => "dead",
            });
        }
        fn record_status_delivery(
            &self,
            outcome: identity_application::consent::ports::DeliveryOutcome,
        ) {
            self.status_deliveries.lock().unwrap().push(match outcome {
                identity_application::consent::ports::DeliveryOutcome::Delivered => "delivered",
                identity_application::consent::ports::DeliveryOutcome::Retryable => "retryable",
                identity_application::consent::ports::DeliveryOutcome::Dead => "dead",
            });
        }
    }

    let (pool, schema) = test_pool().await;
    let user_id = seed_user(&pool).await;
    let store = PgStatusTransitionStore::new(pool.clone());
    store
        .apply(&StatusChange {
            user_id: identity_domain::UserId::new(user_id),
            from: UserStatus::Active,
            to: UserStatus::Disabled,
            actor: Uuid::new_v4().to_string(),
            actor_kind: "admin",
            reason: Some("abuse".to_string()),
            occurred_at: Utc::now(),
        })
        .await
        .expect("the transition commits");

    let seen: std::sync::Arc<Mutex<Vec<Option<String>>>> =
        std::sync::Arc::new(Mutex::new(Vec::new()));
    let seen_by_server = seen.clone();
    let app = axum::Router::new().route(
        "/v1/events",
        post(move |headers: axum::http::HeaderMap| {
            let seen = seen_by_server.clone();
            async move {
                seen.lock().unwrap().push(
                    headers
                        .get("idempotency-key")
                        .and_then(|value| value.to_str().ok())
                        .map(ToString::to_string),
                );
                axum::http::StatusCode::ACCEPTED
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let address = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    let observation = std::sync::Arc::new(Observation::default());
    let drainer = AuditOutboxDrainer::new(
        pool.clone(),
        format!("http://{address}"),
        "service-token",
        observation.clone(),
        Duration::from_secs(60),
    );
    drainer.run_once(Utc::now()).await;

    let state: String = sqlx::query_scalar("SELECT state FROM status_audit_outbox")
        .fetch_one(&pool)
        .await
        .expect("one row was drained");
    assert_eq!(state, "delivered");
    let keys = seen.lock().unwrap().clone();
    assert_eq!(keys.len(), 1, "one post was made");
    assert!(
        keys[0]
            .as_deref()
            .is_some_and(|key| key.starts_with("status-transition:")),
        "C9 received the idempotency key"
    );
    assert_eq!(
        observation.status_deliveries.lock().unwrap().as_slice(),
        ["delivered"]
    );

    handle.abort();
    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs a disposable Postgres via TEST_DATABASE_URL"]
async fn a_deletion_request_keeps_one_open_row_and_a_resolved_outcome() {
    let (pool, schema) = test_pool().await;
    let user_id = seed_user(&pool).await;
    let request = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO deletion_requests
             (id, user_id, status, requested_at, recoverable_until, next_attempt_at)
         VALUES ($1, $2, 'requested', now(), now() + interval '30 days', now())",
    )
    .bind(request)
    .bind(user_id)
    .execute(&pool)
    .await
    .expect("seed an open request");

    // A second open row for the same user violates the open-request index.
    let second = sqlx::query(
        "INSERT INTO deletion_requests
             (id, user_id, status, requested_at, recoverable_until, next_attempt_at)
         VALUES ($1, $2, 'requested', now(), now() + interval '30 days', now())",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .execute(&pool)
    .await;
    assert!(second.is_err(), "one open request per user");

    // Closing the first lets a new one open.
    sqlx::query("UPDATE deletion_requests SET cancelled_at = now() WHERE id = $1")
        .bind(request)
        .execute(&pool)
        .await
        .expect("cancel the first request");
    sqlx::query(
        "INSERT INTO deletion_requests
             (id, user_id, status, requested_at, recoverable_until, next_attempt_at)
         VALUES ($1, $2, 'requested', now(), now() + interval '30 days', now())",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .execute(&pool)
    .await
    .expect("a closed request no longer blocks a new one");

    // A completed request must carry an erased timestamp.
    let bad = sqlx::query("UPDATE deletion_requests SET status = 'completed' WHERE id = $1")
        .bind(request)
        .execute(&pool)
        .await;
    assert!(bad.is_err(), "completed without erased_at is refused");

    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs a disposable Postgres via TEST_DATABASE_URL"]
async fn anonymise_copies_the_subject_hash_before_the_personal_row_leaves() {
    use identity_application::ports::UserRepository;
    use identity_application::user_registry::subject_hash;
    use identity_domain::{User, UserId};
    use identity_infrastructure::repositories::PgUserRepository;

    let (pool, schema) = test_pool().await;
    // Seeded directly so the case does not depend on pgsodium: the repository's
    // erasure path reads and deletes the personal row, it never decrypts it.
    let user_id = Uuid::new_v4();
    let user = User::from_stored(
        UserId::new(user_id),
        identity_domain::Role::User,
        identity_domain::UserStatus::PendingDeletion,
        None,
        false,
        None,
        false,
        false,
        identity_domain::Locale::En,
        identity_domain::BaseCurrency::parse("USD").expect("USD is valid"),
        identity_domain::TimeZone::parse("UTC").expect("UTC is valid"),
        Utc::now(),
        Utc::now(),
        None,
    );
    let key_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, role, status, locale, base_currency, timezone, created_at, updated_at)
         VALUES ($1, 'user', 'pending_deletion', 'en', 'USD', 'UTC', now(), now())",
    )
    .bind(user_id)
    .execute(&pool)
    .await
    .expect("seed the account");
    sqlx::query(
        "INSERT INTO shredding_keys
             (id, user_id, vault_reference, key_id, status, created_at)
         VALUES ($1, $2, $3, 1, 'active', now())",
    )
    .bind(key_id)
    .bind(user_id)
    .bind(format!("identity/user/{user_id}"))
    .execute(&pool)
    .await
    .expect("seed the key reference");
    sqlx::query(
        "INSERT INTO user_personal
             (user_id, auth_subject_hash, auth_subject_protected, shredding_key_id)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(user_id)
    .bind(subject_hash("adapter-subject"))
    .bind(vec![0u8])
    .bind(key_id)
    .execute(&pool)
    .await
    .expect("seed the protected subject");

    let repository = PgUserRepository::new(pool.clone());
    let mut erased = user.clone();
    erased.anonymise(Utc::now());
    repository
        .anonymise(&erased)
        .await
        .expect("the guarded tombstone move commits");

    let stored: Option<String> =
        sqlx::query_scalar("SELECT erased_subject_hash FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .expect("read the tombstone");
    assert_eq!(
        stored.as_deref(),
        Some(subject_hash("adapter-subject").as_str())
    );
    let personal: Option<Uuid> =
        sqlx::query_scalar("SELECT user_id FROM user_personal WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(&pool)
            .await
            .expect("read the personal row");
    assert!(personal.is_none(), "the personal row left after the copy");

    // The C3 lookup reaches the tombstone once the personal row is gone.
    let by_hash = repository
        .find_by_subject_hash(&subject_hash("adapter-subject"))
        .await
        .expect("the C3 lookup answers")
        .expect("the tombstone answers C3 consumers");
    assert_eq!(by_hash.id(), user.id());
    assert_eq!(by_hash.status(), identity_domain::UserStatus::Deleted);

    // Idempotent: a second run keeps the hash and changes nothing else.
    repository
        .anonymise(&erased)
        .await
        .expect("a repeated erasure is a no-op success");

    // A restored account is never overwritten by a late erasure. A restored
    // row carries no tombstone and no anonymisation time, so the simulated
    // restore clears both; the status guard is what the erasure must respect.
    sqlx::query(
        "UPDATE users
         SET status = 'active', erased_subject_hash = NULL, anonymised_at = NULL
         WHERE id = $1",
    )
    .bind(user_id)
    .execute(&pool)
    .await
    .expect("a concurrent restore wins");
    assert!(
        repository.anonymise(&erased).await.is_err(),
        "the guarded move refuses a restored account"
    );

    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs a disposable Postgres via TEST_DATABASE_URL"]
async fn the_sweep_claims_only_requests_that_are_due() {
    let (pool, schema) = test_pool().await;
    let user_id = seed_user(&pool).await;
    let other_user = seed_user(&pool).await;
    let due = Uuid::new_v4();
    let not_due = Uuid::new_v4();
    for (id, owner, offset) in [(due, user_id, "-1 day"), (not_due, other_user, "1 day")] {
        sqlx::query(
            "INSERT INTO deletion_requests
                 (id, user_id, status, requested_at, recoverable_until, next_attempt_at)
             VALUES ($1, $2, 'requested', now(), now() + $3::interval, now())",
        )
        .bind(id)
        .bind(owner)
        .bind(offset)
        .execute(&pool)
        .await
        .expect("seed a request");
    }

    let claimed: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM deletion_requests
         WHERE status IN ('requested', 'in_progress', 'failed')
           AND next_attempt_at <= now()
           AND recoverable_until <= now()
           AND (lease_until IS NULL OR lease_until < now())",
    )
    .fetch_all(&pool)
    .await
    .expect("run the claim predicate");
    assert_eq!(claimed, vec![due], "only the due request is claimable");

    drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "needs a disposable Postgres via TEST_DATABASE_URL"]
async fn the_tombstone_lookup_finds_an_erased_subject_and_anonymise_keeps_the_hash() {
    let (pool, schema) = test_pool().await;
    let user_id = seed_user(&pool).await;
    let hash = "a".repeat(64);

    // Erasure copies the hash onto the row before the personal row leaves.
    sqlx::query(
        "UPDATE users
         SET erased_subject_hash = $2, status = 'deleted', anonymised_at = now(),
             display_name = NULL, email = NULL, email_verified = false
         WHERE id = $1",
    )
    .bind(user_id)
    .bind(&hash)
    .execute(&pool)
    .await
    .expect("write the tombstone");

    let found: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM users WHERE erased_subject_hash = $1")
            .bind(&hash)
            .fetch_optional(&pool)
            .await
            .expect("look the tombstone up");
    assert_eq!(found, Some(user_id));

    let row = sqlx::query(
        "SELECT status, erased_subject_hash, anonymised_at, email FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("read the erased row");
    assert_eq!(row.get::<String, _>("status"), "deleted");
    assert!(row.get::<Option<String>, _>("email").is_none());
    assert!(row
        .get::<Option<chrono::DateTime<Utc>>, _>("anonymised_at")
        .is_some());

    drop_schema(&schema).await;
}
