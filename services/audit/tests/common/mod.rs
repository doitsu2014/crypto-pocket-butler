//! Shared test harness: an in-memory application state for unit and HTTP tests,
//! and a disposable Postgres schema for the adapter suite.
//!
//! The adapter suite needs `TEST_DATABASE_URL` and is `#[ignore]`d; every other
//! test runs without a database.

#![allow(dead_code)]

pub mod fakes;

use std::sync::Arc;

use audit_application::export::ExportAudit;
use audit_application::ports::{
    AuditEventRepository, Clock, ExportRepository, Principal, PrivacyDeletionRepository,
    RetentionRepository, Role, ServiceTokenRegistry, SupportSessionRepository, TokenValidator,
};
use audit_application::privacy::PrivacyDeletions;
use audit_application::query::QueryEvents;
use audit_application::retention::{RetentionService, SweepConfig};
use audit_application::support::SupportSessions;
use audit_application::{AppendEvent, Trail};
use audit_http::metrics::Metrics;
use audit_http::rate_limit::{Limits, RateLimiter};
use audit_http::AppState;
use chrono::{DateTime, TimeZone, Utc};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

use fakes::{
    FakeEvents, FakeExports, FakePrivacy, FakeRetention, FakeServiceTokens, FakeSupport,
    FakeTokenValidator, MutableClock, NoPause, Ready,
};

/// The admin subject.
pub const ADMIN_SUBJECT: &str = "11111111-1111-4111-8111-111111111111";
/// The support agent subject.
pub const SUPPORT_SUBJECT: &str = "22222222-2222-4222-8222-222222222222";
/// The user whose data is in scope.
pub const SUBJECT_USER: &str = "33333333-3333-4333-8333-333333333333";
/// The independent second approver.
pub const SECOND_APPROVER: &str = "44444444-4444-4444-8444-444444444444";
/// A regular user.
pub const USER_SUBJECT: &str = "55555555-5555-4555-8555-555555555555";
/// A deletion request identifier.
pub const DELETION_REQUEST: &str = "66666666-6666-4666-8666-666666666666";

/// The service token Identity uses.
pub const IDENTITY_TOKEN: &str = "test-service-token-identity";
/// The service token Ledger uses.
pub const LEDGER_TOKEN: &str = "test-service-token-ledger";
/// The service token the gateway uses for session checks.
pub const GATEWAY_TOKEN: &str = "test-service-token-gateway";

/// The fixed instant every test's clock reports.
#[must_use]
pub fn fixed_now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 6, 12, 0, 0).unwrap()
}

/// The in-memory application harness.
pub struct Harness {
    /// The fake trail.
    pub events: Arc<FakeEvents>,
    /// The fake support sessions.
    pub support: Arc<FakeSupport>,
    /// The fake exports.
    pub exports: Arc<FakeExports>,
    /// The fake retention state.
    pub retention: Arc<FakeRetention>,
    /// The fake privacy deletions.
    pub privacy: Arc<FakePrivacy>,
    /// The clock, movable by a test.
    pub mutable_clock: Arc<MutableClock>,
    /// The router state.
    pub state: AppState,
}

/// Builds the default harness.
#[must_use]
pub fn harness() -> Harness {
    harness_at(fixed_now(), Limits::default())
}

/// Builds a harness with explicit rate limits.
#[must_use]
pub fn harness_with_limits(limits: Limits) -> Harness {
    harness_at(fixed_now(), limits)
}

/// Builds a harness at a fixed instant.
#[must_use]
pub fn harness_at(now: DateTime<Utc>, limits: Limits) -> Harness {
    let fake_events = Arc::new(FakeEvents::default());
    let fake_support = Arc::new(FakeSupport::default());
    let fake_exports = Arc::new(FakeExports::default());
    let fake_retention = Arc::new(FakeRetention::default());
    let fake_privacy = Arc::new(FakePrivacy::default());

    let events: Arc<dyn AuditEventRepository> = fake_events.clone();
    let support_sessions: Arc<dyn SupportSessionRepository> = fake_support.clone();
    let exports: Arc<dyn ExportRepository> = fake_exports.clone();
    let retention_repository: Arc<dyn RetentionRepository> = fake_retention.clone();
    let privacy_repository: Arc<dyn PrivacyDeletionRepository> = fake_privacy.clone();

    let mutable_clock = Arc::new(MutableClock::at(now));
    let clock: Arc<dyn Clock> = mutable_clock.clone();
    let trail = Trail::new(events.clone(), clock.clone());

    let append = Arc::new(AppendEvent::new(events.clone(), clock.clone()));
    let query = Arc::new(QueryEvents::new(events.clone(), trail.clone()));
    let export = Arc::new(ExportAudit::new(
        events.clone(),
        exports,
        clock.clone(),
        trail.clone(),
    ));
    let support = Arc::new(SupportSessions::new(
        support_sessions,
        clock.clone(),
        trail.clone(),
    ));
    let privacy = Arc::new(PrivacyDeletions::new(
        privacy_repository,
        clock.clone(),
        trail.clone(),
    ));
    let retention = Arc::new(RetentionService::new(
        retention_repository,
        clock.clone(),
        Arc::new(NoPause),
        trail.clone(),
        SweepConfig {
            batch_size: 2,
            max_batches: 5,
        },
    ));

    let service_tokens: Arc<dyn ServiceTokenRegistry> = Arc::new(FakeServiceTokens::with(&[
        (IDENTITY_TOKEN, "identity"),
        (LEDGER_TOKEN, "ledger"),
        (GATEWAY_TOKEN, "gateway"),
    ]));

    let token_validator: Arc<dyn TokenValidator> = Arc::new(FakeTokenValidator::with(vec![
        (
            "admin-token",
            Ok(Principal {
                subject: ADMIN_SUBJECT.to_string(),
                role: Role::Admin,
            }),
        ),
        (
            "support-token",
            Ok(Principal {
                subject: SUPPORT_SUBJECT.to_string(),
                role: Role::Support,
            }),
        ),
        (
            "approver-token",
            Ok(Principal {
                subject: SECOND_APPROVER.to_string(),
                role: Role::Support,
            }),
        ),
        (
            "user-token",
            Ok(Principal {
                subject: USER_SUBJECT.to_string(),
                role: Role::User,
            }),
        ),
    ]));

    let state = AppState {
        service_name: "audit-api",
        append,
        query,
        export,
        support,
        privacy,
        retention,
        trail: Arc::new(trail),
        service_tokens,
        token_validator,
        clock: clock.clone(),
        readiness: Arc::new(Ready(true)),
        metrics: Arc::new(Metrics::default()),
        limiter: Arc::new(RateLimiter::default()),
        limits,
    };

    Harness {
        events: fake_events,
        support: fake_support,
        exports: fake_exports,
        retention: fake_retention,
        privacy: fake_privacy,
        mutable_clock,
        state,
    }
}

/// Connects to a disposable Postgres database, creates a unique schema, and runs
/// the real migrations into it.
///
/// # Panics
/// Panics when `TEST_DATABASE_URL` is unset or the migrations fail.
pub async fn test_pool() -> (PgPool, String) {
    let url = std::env::var("TEST_DATABASE_URL").expect(
        "TEST_DATABASE_URL must point at a disposable Postgres database for integration tests",
    );
    let schema = format!("test_audit_{}", Uuid::new_v4().simple());

    let admin = PgPool::connect(&url)
        .await
        .expect("connect to test database");
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&admin)
        .await
        .expect("create disposable schema");
    admin.close().await;

    let schema_for_connection = schema.clone();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .after_connect(move |connection, _meta| {
            let schema = schema_for_connection.clone();
            Box::pin(async move {
                sqlx::query(&format!("SET search_path TO {schema}"))
                    .execute(&mut *connection)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
        .expect("connect disposable pool");

    // Role creation is cluster-wide, so migrations are serialised across test
    // schemas with an advisory lock.
    let mut lock = pool.acquire().await.expect("acquire migration lock");
    sqlx::query("SELECT pg_advisory_lock(9812460)")
        .execute(&mut *lock)
        .await
        .expect("take migration advisory lock");
    let result = audit_infrastructure::migrations::run(&pool).await;
    let _ = sqlx::query("SELECT pg_advisory_unlock(9812460)")
        .execute(&mut *lock)
        .await;
    drop(lock);
    result.expect("run migrations in the disposable schema");

    (pool, schema)
}

/// Connects to the disposable database, pins `search_path`, and assumes `role`
/// on every pooled connection so a test exercises the real production grants
/// instead of the database superuser's.
///
/// # Panics
/// Panics when `TEST_DATABASE_URL` is unset or the role cannot be assumed.
pub async fn role_pool(schema: &str, role: &str) -> PgPool {
    let url = std::env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL must point at a disposable Postgres database");
    let schema_for_connection = schema.to_string();
    let role_for_connection = role.to_string();
    PgPoolOptions::new()
        .max_connections(1)
        .after_connect(move |connection, _meta| {
            let schema = schema_for_connection.clone();
            let role = role_for_connection.clone();
            Box::pin(async move {
                sqlx::query(&format!("SET search_path TO {schema}"))
                    .execute(&mut *connection)
                    .await?;
                sqlx::query(&format!("SET ROLE {role}"))
                    .execute(&mut *connection)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
        .expect("connect a role-scoped pool")
}

/// Drops a disposable schema.
pub async fn drop_schema(schema: &str) {
    let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
        return;
    };
    let admin = PgPool::connect(&url)
        .await
        .expect("connect to test database");
    sqlx::query(&format!("DROP SCHEMA IF EXISTS {schema} CASCADE"))
        .execute(&admin)
        .await
        .expect("drop disposable schema");
    admin.close().await;
}
