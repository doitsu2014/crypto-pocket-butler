//! Shared test harness: a disposable Postgres schema for the adapter suite.
//!
//! The adapter suite needs `TEST_DATABASE_URL` and is `#[ignore]`d; every other
//! test runs without a database.

#![allow(dead_code)]

pub mod fakes;

use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};
use identity_application::common::Caller;
use identity_application::consent::ConsentService;
use identity_application::deletion::{DeletionService, DeletionWorker};
use identity_application::export::{ExportService, ExportWorker};
use identity_application::ports::{ServiceTokenRegistry, TokenValidator};
use identity_application::preferences::PreferencesService;
use identity_application::principal::PrincipalService;
use identity_application::sessions::SessionService;
use identity_application::signup_gate::SignupGateService;
use identity_application::support::SupportService;
use identity_application::trail::Trail;
use identity_application::worker::PrivacyWorker;
use identity_domain::claims::{AssuranceLevel, Role, VerifiedClaims};
use identity_domain::User;
use identity_http::metrics::Metrics;
use identity_http::state::AppState;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

use fakes::{
    FakeAudit, FakeAuthAdmin, FakeConsent, FakeContributors, FakeDeletions, FakeExports,
    FakeGateIssuer, FakeGrants, FakeKeys, FakeRules, FakeServiceTokens, FakeSessions, FakeStore,
    FakeTokenValidator, FakeUsers, FakeWake, MutableClock, Ready,
};

/// The subject of the regular user under test.
pub const USER_SUBJECT: &str = "11111111-1111-4111-8111-111111111111";
/// The subject of an administrator.
pub const ADMIN_SUBJECT: &str = "22222222-2222-4222-8222-222222222222";
/// The subject of a support agent.
pub const SUPPORT_SUBJECT: &str = "33333333-3333-4333-8333-333333333333";
/// The subject of a second user.
pub const OTHER_SUBJECT: &str = "44444444-4444-4444-8444-444444444444";

/// The fixed instant every test's clock reports.
#[must_use]
pub fn fixed_now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, 12, 0, 0).unwrap()
}

/// A verified regular-user claim set with no exchange credential.
#[must_use]
pub fn claims() -> VerifiedClaims {
    VerifiedClaims {
        subject: USER_SUBJECT.to_string(),
        role: Role::User,
        email_confirmed: true,
        email: Some("user@example.com".to_string()),
        assurance: AssuranceLevel::Aal1,
        has_exchange_credentials: false,
        session_id: Some("auth-session-1".to_string()),
        user_agent: Some("Firefox".to_string()),
        location: Some("Hanoi, VN".to_string()),
    }
}

/// Claims for an administrator.
#[must_use]
pub fn admin_claims() -> VerifiedClaims {
    VerifiedClaims {
        subject: ADMIN_SUBJECT.to_string(),
        role: Role::Admin,
        ..claims()
    }
}

/// Claims for a support agent.
#[must_use]
pub fn support_claims() -> VerifiedClaims {
    VerifiedClaims {
        subject: SUPPORT_SUBJECT.to_string(),
        role: Role::Support,
        ..claims()
    }
}

/// Claims for a second user.
#[must_use]
pub fn other_claims() -> VerifiedClaims {
    VerifiedClaims {
        subject: OTHER_SUBJECT.to_string(),
        ..claims()
    }
}

/// Builds a caller around a stored user.
#[must_use]
pub fn caller(user: User, claims: VerifiedClaims) -> Caller {
    Caller {
        user,
        claims,
        correlation_id: "corr-test".to_string(),
    }
}

/// The in-memory application harness.
pub struct Harness {
    /// The fake users.
    pub users: Arc<FakeUsers>,
    /// The fake shredding keys.
    pub keys: Arc<FakeKeys>,
    /// The fake consent history.
    pub consent: Arc<FakeConsent>,
    /// The fake support grants.
    pub grants: Arc<FakeGrants>,
    /// The fake device sessions.
    pub sessions: Arc<FakeSessions>,
    /// The fake export requests.
    pub exports: Arc<FakeExports>,
    /// The fake deletion requests.
    pub deletions: Arc<FakeDeletions>,
    /// The fake jurisdiction rules.
    pub rules: Arc<FakeRules>,
    /// The fake audit sink.
    pub audit: Arc<FakeAudit>,
    /// The fake Supabase Auth admin.
    pub auth_admin: Arc<FakeAuthAdmin>,
    /// The fake privacy contributors.
    pub contributors: Arc<FakeContributors>,
    /// The fake export store.
    pub store: Arc<FakeStore>,
    /// The fake wake transport.
    pub wake: Arc<FakeWake>,
    /// The fake gate-token issuer.
    pub gate: Arc<FakeGateIssuer>,
    /// The movable clock.
    pub clock: Arc<MutableClock>,
    /// Principal resolution.
    pub principal: Arc<PrincipalService>,
    /// Consent use cases.
    pub consent_service: Arc<ConsentService>,
    /// Preference use cases.
    pub preferences: Arc<PreferencesService>,
    /// Session use cases.
    pub sessions_service: Arc<SessionService>,
    /// Support-grant use cases.
    pub support: Arc<SupportService>,
    /// The sign-up gate.
    pub signup_gate: Arc<SignupGateService>,
    /// Export request and status use cases.
    pub export_service: Arc<ExportService>,
    /// The export worker.
    pub export_worker: Arc<ExportWorker>,
    /// Deletion request and status use cases.
    pub deletion_service: Arc<DeletionService>,
    /// The deletion worker.
    pub deletion_worker: Arc<DeletionWorker>,
    /// The combined privacy worker.
    pub worker: Arc<PrivacyWorker>,
    /// The HTTP router state.
    pub state: AppState,
}

/// Builds the default harness.
#[must_use]
pub fn harness() -> Harness {
    let users = Arc::new(FakeUsers::default());
    let keys = Arc::new(FakeKeys::default());
    let consent = Arc::new(FakeConsent::default());
    let grants = Arc::new(FakeGrants::default());
    let sessions = Arc::new(FakeSessions::default());
    let exports = Arc::new(FakeExports::default());
    let deletions = Arc::new(FakeDeletions::default());
    let rules = Arc::new(FakeRules::default());
    let audit = Arc::new(FakeAudit::default());
    let auth_admin = Arc::new(FakeAuthAdmin::default());
    let contributors = Arc::new(FakeContributors::default());
    let store = Arc::new(FakeStore::default());
    let wake = Arc::new(FakeWake::default());
    let gate = Arc::new(FakeGateIssuer::default());
    let clock = Arc::new(MutableClock::at(fixed_now()));

    let trail = Arc::new(Trail::new(audit.clone(), clock.clone()));

    let principal = Arc::new(PrincipalService::new(
        users.clone(),
        keys.clone(),
        sessions.clone(),
        trail.clone(),
        clock.clone(),
    ));
    let consent_service = Arc::new(ConsentService::new(
        consent.clone(),
        trail.clone(),
        clock.clone(),
    ));
    let preferences = Arc::new(PreferencesService::new(
        users.clone(),
        trail.clone(),
        clock.clone(),
    ));
    let sessions_service = Arc::new(SessionService::new(
        sessions.clone(),
        users.clone(),
        auth_admin.clone(),
        trail.clone(),
        clock.clone(),
    ));
    let support = Arc::new(SupportService::new(
        grants.clone(),
        users.clone(),
        trail.clone(),
        clock.clone(),
    ));
    let signup_gate = Arc::new(SignupGateService::new(
        rules.clone(),
        gate.clone(),
        trail.clone(),
        clock.clone(),
    ));
    let export_service = Arc::new(ExportService::new(
        exports.clone(),
        store.clone(),
        trail.clone(),
        wake.clone(),
        clock.clone(),
    ));
    let export_worker = Arc::new(ExportWorker::new(
        exports.clone(),
        contributors.clone(),
        store.clone(),
        trail.clone(),
    ));
    let deletion_service = Arc::new(DeletionService::new(
        deletions.clone(),
        trail.clone(),
        wake.clone(),
        clock.clone(),
    ));
    let deletion_worker = Arc::new(DeletionWorker::new(
        deletions.clone(),
        users.clone(),
        keys.clone(),
        consent.clone(),
        contributors.clone(),
        auth_admin.clone(),
        trail.clone(),
    ));
    let worker = Arc::new(PrivacyWorker::new(
        export_worker.clone(),
        deletion_worker.clone(),
    ));

    let mut unconfirmed = claims();
    unconfirmed.email_confirmed = false;
    let token_validator: Arc<dyn TokenValidator> = Arc::new(FakeTokenValidator::with(vec![
        ("user-token", Ok(claims())),
        ("unconfirmed-token", Ok(unconfirmed)),
        ("admin-token", Ok(admin_claims())),
        ("support-token", Ok(support_claims())),
        ("other-token", Ok(other_claims())),
    ]));
    let service_tokens: Arc<dyn ServiceTokenRegistry> = Arc::new(FakeServiceTokens::with(&[
        ("service-token", "identity"),
        ("api-token", "identity-api"),
        ("notification-token", "notification"),
        ("gateway-token", "gateway"),
        ("run-token", "job-orchestration"),
        ("reporting-token", "reporting"),
    ]));
    let state = AppState {
        service_name: "identity-api",
        principal: principal.clone(),
        consent: consent_service.clone(),
        preferences: preferences.clone(),
        sessions: sessions_service.clone(),
        support: support.clone(),
        signup_gate: signup_gate.clone(),
        exports: export_service.clone(),
        deletions: deletion_service.clone(),
        due_users: Arc::new(identity_application::due_users::DueUsersService::new(
            users.clone(),
        )),
        token_validator,
        service_tokens,
        wake: wake.clone(),
        readiness: Arc::new(Ready(true)),
        metrics: Arc::new(Metrics::default()),
    };

    Harness {
        users,
        keys,
        consent,
        grants,
        sessions,
        exports,
        deletions,
        rules,
        audit,
        auth_admin,
        contributors,
        store,
        wake,
        gate,
        clock,
        principal,
        consent_service,
        preferences,
        sessions_service,
        support,
        signup_gate,
        export_service,
        export_worker,
        deletion_service,
        deletion_worker,
        worker,
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
    let schema = format!("test_identity_{}", Uuid::new_v4().simple());

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
    sqlx::query("SELECT pg_advisory_lock(9812461)")
        .execute(&mut *lock)
        .await
        .expect("take migration advisory lock");
    let result = identity_infrastructure::migrations::run(&pool).await;
    let _ = sqlx::query("SELECT pg_advisory_unlock(9812461)")
        .execute(&mut *lock)
        .await;
    drop(lock);
    result.expect("run migrations in the disposable schema");

    (pool, schema)
}

/// Connects to the disposable database, pins `search_path`, and assumes `role`
/// on every pooled connection so a test exercises the real production grants.
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
