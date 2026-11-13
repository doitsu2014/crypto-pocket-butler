//! Shared test harness: a disposable Postgres schema for the adapter suite.
//!
//! The adapter suite needs `TEST_DATABASE_URL` and is `#[ignore]`d; every other
//! test runs without a database.

#![allow(dead_code)]

pub mod fakes;

use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};
use identity_application::account_lifecycle::AccountLifecycleService;
use identity_application::authorisation::catalogue::OperationCatalogue;
use identity_application::authorisation::decision::AuthorisationDecision;
use identity_application::authorisation::role_change::RoleChange;
use identity_application::common::Caller;
use identity_application::consent::{
    ConsentGrant, ConsentHistory, ConsentService, ConsentStateQuery, ConsentWithdraw,
};
use identity_application::deletion::{DeletionService, DeletionWorker};
use identity_application::export::{ExportService, ExportWorker};
use identity_application::jurisdiction::{CheckJurisdiction, PublishList};
use identity_application::ports::{ServiceTokenRegistry, TokenValidator};
use identity_application::preferences::read::ReadUserPreferences;
use identity_application::preferences::write::WriteUserPreferences;
use identity_application::preferences::PreferencesService;
use identity_application::principal::PrincipalService;
use identity_application::profile::ProfileService;
use identity_application::sessions::SessionService;
use identity_application::signup_gate::SignupGateService;
use identity_application::support::SupportService;
use identity_application::trail::Trail;
use identity_application::user_registry::AdminLookup;
use identity_application::worker::PrivacyWorker;
use identity_domain::claims::{AssuranceLevel, Role, VerifiedClaims};
use identity_domain::jurisdiction::{
    CountryCode, JurisdictionListEntry, JurisdictionListVersion, JurisdictionStatus,
};
use identity_domain::User;
use identity_domain::UserStatus;
use identity_http::metrics::Metrics;
use identity_http::state::AppState;
use identity_infrastructure::authorisation::status_cache::StatusCache;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

use fakes::{
    FakeAccountObservation, FakeAccountsSource, FakeAudit, FakeAuthAdmin, FakeAuthorisationAudit,
    FakeConsent, FakeConsentRepository, FakeContributors, FakeDecisionAudit, FakeDeletions,
    FakeErasureStore, FakeExports, FakeGate, FakeGateIssuer, FakeGeoLocator, FakeGrants, FakeKeys,
    FakeListStore, FakeObservationSink, FakePortfoliosSource, FakePreferencesRepository,
    FakeRoleReader, FakeRoleWriter, FakeRules, FakeServiceTokens, FakeSessions, FakeStatusReader,
    FakeStatusTransitions, FakeStore, FakeSupportObservation, FakeSupportOutbox,
    FakeTokenValidator, FakeUserLookup, FakeUsers, FakeWake, MutableClock, Ready,
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

/// The three-entry jurisdiction list every gate test uses (U6).
#[must_use]
pub fn jurisdiction_list() -> JurisdictionListVersion {
    let entry = |code: &str, status: JurisdictionStatus| JurisdictionListEntry {
        code: CountryCode::parse(code).expect("a fixture country code is valid"),
        status,
        source: "compliance".to_string(),
        updated_at: fixed_now(),
    };
    JurisdictionListVersion::new(
        "2026-11-10.1",
        fixed_now(),
        vec![
            entry("VN", JurisdictionStatus::Allowed),
            entry("US", JurisdictionStatus::Allowed),
            entry("KP", JurisdictionStatus::Restricted),
        ],
    )
    .expect("a complete list is valid")
}

/// A verified regular-user claim set with no exchange credential.
#[must_use]
pub fn claims() -> VerifiedClaims {
    VerifiedClaims {
        subject: USER_SUBJECT.to_string(),
        role: Role::User,
        email_confirmed: true,
        email: Some("user@example.com".to_string()),
        display_name: Some("Test User".to_string()),
        assurance: AssuranceLevel::Aal1,
        has_exchange_credentials: false,
        session_id: Some("auth-session-1".to_string()),
        user_agent: Some("Firefox".to_string()),
        location: Some("Hanoi, VN".to_string()),
        country_code: Some("VN".to_string()),
        client_ip: Some("192.0.2.10".to_string()),
    }
}

/// Claims for an administrator.
#[must_use]
pub fn admin_claims() -> VerifiedClaims {
    VerifiedClaims {
        subject: ADMIN_SUBJECT.to_string(),
        role: Role::Admin,
        email: Some("admin@example.com".to_string()),
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

/// A stored user fixture in the active state.
#[must_use]
pub fn active_user() -> User {
    User::from_stored(
        identity_domain::UserId::generate(),
        Role::User,
        identity_domain::UserStatus::Active,
        Some("Test User".to_string()),
        false,
        Some("user@example.com".to_string()),
        false,
        true,
        identity_domain::Locale::En,
        identity_domain::BaseCurrency::parse("USD").expect("USD is valid"),
        identity_domain::TimeZone::parse("UTC").expect("UTC is valid"),
        fixed_now(),
        fixed_now(),
        None,
    )
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
    /// U4: the fake consent event store.
    pub consent_events: Arc<FakeConsentRepository>,
    /// U4: the fake preferences store.
    pub preferences_repository: Arc<FakePreferencesRepository>,
    /// U4: the fake provider-profile lookup.
    pub user_lookup: Arc<FakeUserLookup>,
    /// U4: the fake consent observation sink.
    pub consent_observation: Arc<FakeObservationSink>,
    /// U4: the fake erasure role.
    pub erasure: Arc<FakeErasureStore>,
    /// The fake support grants.
    pub grants: Arc<FakeGrants>,
    /// The fake support audit outbox (refusals and failures).
    pub support_outbox: Arc<FakeSupportOutbox>,
    /// The fake Account metadata source.
    pub support_accounts: Arc<FakeAccountsSource>,
    /// The fake Portfolio metadata source.
    pub support_portfolios: Arc<FakePortfoliosSource>,
    /// The fake inline read-audit sink.
    pub support_observation: Arc<FakeSupportObservation>,
    /// The fake device sessions.
    pub sessions: Arc<FakeSessions>,
    /// The fake export requests.
    pub exports: Arc<FakeExports>,
    /// The fake deletion requests.
    pub deletions: Arc<FakeDeletions>,
    /// U5: the fake status transitions and audit intents.
    pub transitions: Arc<FakeStatusTransitions>,
    /// U5: the fake lifecycle metrics sink.
    pub account_observation: Arc<FakeAccountObservation>,
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
    /// The in-process jurisdiction gate.
    pub gate_clearance: Arc<FakeGate>,
    /// The fake gate-token issuer.
    pub gate: Arc<FakeGateIssuer>,
    /// The fake geolocation provider (U6).
    pub geo: Arc<FakeGeoLocator>,
    /// The fake jurisdiction list store (U6).
    pub jurisdiction_lists: Arc<FakeListStore>,
    /// The fake decision audit sink (U6).
    pub decision_audit: Arc<FakeDecisionAudit>,
    /// The authorisation status reader fake (U2).
    pub status_reader: Arc<FakeStatusReader>,
    /// The authorisation audit sink fake (U2).
    pub authorisation_audit: Arc<FakeAuthorisationAudit>,
    /// The provider role writer fake (U2).
    pub role_writer: Arc<FakeRoleWriter>,
    /// The provider role reader fake (U10 admin lookup).
    pub role_reader: Arc<FakeRoleReader>,
    /// The movable clock.
    pub clock: Arc<MutableClock>,
    /// Principal resolution.
    pub principal: Arc<PrincipalService>,
    /// Consent use cases.
    pub consent_service: Arc<ConsentService>,
    /// Preference use cases.
    pub preferences: Arc<PreferencesService>,
    /// U7: the profile read and write use cases.
    pub profile: Arc<ProfileService>,
    /// Session use cases.
    pub sessions_service: Arc<SessionService>,
    /// Support-grant use cases.
    pub support: Arc<SupportService>,
    /// The sign-up gate.
    pub signup_gate: Arc<SignupGateService>,
    /// The runtime jurisdiction check (U6).
    pub jurisdiction_check: Arc<CheckJurisdiction>,
    /// The jurisdiction list publish (U6).
    pub jurisdiction_publish: Arc<PublishList>,
    /// Export request and status use cases.
    pub export_service: Arc<ExportService>,
    /// The export worker.
    pub export_worker: Arc<ExportWorker>,
    /// Deletion request and status use cases.
    pub deletion_service: Arc<DeletionService>,
    /// U5: the account status machine and the recovery window.
    pub account_lifecycle: Arc<AccountLifecycleService>,
    /// The deletion worker.
    pub deletion_worker: Arc<DeletionWorker>,
    /// The combined privacy worker.
    pub worker: Arc<PrivacyWorker>,
    /// The admin and owner user lookup.
    pub user_registry: Arc<AdminLookup>,
    /// The HTTP router state.
    pub state: AppState,
}

/// Builds the default harness.
#[must_use]
pub fn harness() -> Harness {
    let users = Arc::new(FakeUsers::default());
    let keys = Arc::new(FakeKeys::default());
    users.attach_keys(keys.clone());
    let gate_clearance = Arc::new(FakeGate::default());
    let consent = Arc::new(FakeConsent::default());
    let grants = Arc::new(FakeGrants::default());
    let support_outbox = Arc::new(FakeSupportOutbox::default());
    let support_accounts = Arc::new(FakeAccountsSource::default());
    let support_portfolios = Arc::new(FakePortfoliosSource::default());
    let support_observation = Arc::new(FakeSupportObservation::default());
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
    let geo = Arc::new(FakeGeoLocator::default());
    geo.set("192.0.2.10", "VN");
    geo.set("192.0.2.20", "KP");
    geo.set("192.0.2.30", "US");
    let jurisdiction_lists = Arc::new(FakeListStore::default());
    jurisdiction_lists.seed(jurisdiction_list());
    let decision_audit = Arc::new(FakeDecisionAudit::default());
    let clock = Arc::new(MutableClock::at(fixed_now()));

    let consent_events = Arc::new(FakeConsentRepository::default());
    let erasure = Arc::new(FakeErasureStore::default());
    let transitions = Arc::new(FakeStatusTransitions::default());
    transitions.attach_users(users.clone());
    transitions.attach_deletions(deletions.clone());
    let account_observation = Arc::new(FakeAccountObservation::default());
    let preferences_repository = Arc::new(FakePreferencesRepository::default());
    let user_lookup = Arc::new(FakeUserLookup::default());
    let consent_observation = Arc::new(FakeObservationSink::default());
    let consent_state = Arc::new(ConsentStateQuery::new(
        consent_events.clone(),
        consent_observation.clone(),
    ));
    let consent_grant = Arc::new(ConsentGrant::new(
        consent_events.clone(),
        consent_observation.clone(),
    ));
    let consent_withdraw = Arc::new(ConsentWithdraw::new(
        consent_events.clone(),
        consent_observation.clone(),
    ));
    let consent_history = Arc::new(ConsentHistory::new(consent_events.clone()));
    let preferences_read = Arc::new(ReadUserPreferences::new(
        preferences_repository.clone(),
        user_lookup.clone(),
    ));
    let preferences_write = Arc::new(WriteUserPreferences::new(
        preferences_repository.clone(),
        user_lookup.clone(),
        clock.clone(),
        consent_observation.clone(),
    ));

    let status_reader = Arc::new(FakeStatusReader::default());
    let authorisation_audit = Arc::new(FakeAuthorisationAudit::default());
    let role_writer = Arc::new(FakeRoleWriter::default());
    let role_reader = Arc::new(FakeRoleReader::default());
    let metrics = Arc::new(Metrics::default());
    let authorisation = Arc::new(AuthorisationDecision::new(
        OperationCatalogue::new(),
        Arc::new(StatusCache::new(
            status_reader.clone(),
            clock.clone(),
            metrics.clone(),
        )),
        authorisation_audit.clone(),
        metrics.clone(),
        clock.clone(),
    ));
    let role_change = Arc::new(RoleChange::new(
        users.clone(),
        role_writer.clone(),
        authorisation_audit.clone(),
        metrics.clone(),
        clock.clone(),
    ));

    let trail = Arc::new(Trail::new(audit.clone(), clock.clone()));

    let principal = Arc::new(PrincipalService::new(
        users.clone(),
        sessions.clone(),
        trail.clone(),
        gate_clearance.clone(),
        clock.clone(),
    ));
    let user_registry = Arc::new(AdminLookup::new(
        users.clone(),
        role_reader.clone(),
        trail.clone(),
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
    let profile = Arc::new(ProfileService::new(
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
        support_outbox.clone(),
        support_accounts.clone(),
        support_portfolios.clone(),
        support_observation.clone(),
        clock.clone(),
    ));
    let signup_gate = Arc::new(SignupGateService::new(
        rules.clone(),
        gate.clone(),
        trail.clone(),
        clock.clone(),
    ));
    let jurisdiction_check = Arc::new(CheckJurisdiction::new(
        jurisdiction_lists.clone(),
        geo.clone(),
        decision_audit.clone(),
        clock.clone(),
    ));
    let jurisdiction_publish = Arc::new(PublishList::new(
        jurisdiction_lists.clone(),
        decision_audit.clone(),
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
    let deletion_service = Arc::new(DeletionService::new(deletions.clone(), trail.clone()));
    let account_lifecycle = Arc::new(AccountLifecycleService::new(
        users.clone(),
        transitions.clone(),
        deletions.clone(),
        wake.clone(),
        account_observation.clone(),
        clock.clone(),
    ));
    let deletion_worker = Arc::new(DeletionWorker::new(
        deletions.clone(),
        users.clone(),
        keys.clone(),
        erasure.clone(),
        contributors.clone(),
        auth_admin.clone(),
        transitions.clone(),
        account_observation.clone(),
        trail.clone(),
    ));
    let worker = Arc::new(PrivacyWorker::new(
        export_worker.clone(),
        deletion_worker.clone(),
    ));

    let mut unconfirmed = claims();
    unconfirmed.email_confirmed = false;
    let mut nameless = claims();
    nameless.display_name = None;
    let mut undeclared = claims();
    undeclared.country_code = None;
    undeclared.client_ip = None;
    let token_validator: Arc<dyn TokenValidator> = Arc::new(FakeTokenValidator::with(vec![
        ("user-token", Ok(claims())),
        ("unconfirmed-token", Ok(unconfirmed)),
        ("nameless-token", Ok(nameless)),
        ("undeclared-token", Ok(undeclared)),
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
        ("account-token", "account-api"),
        ("portfolio-token", "portfolio"),
        ("smoke-token", "identity-smoke"),
    ]));
    let state = AppState {
        service_name: "identity-api",
        principal: principal.clone(),
        consent: consent_service.clone(),
        consent_state,
        consent_grant,
        consent_withdraw,
        consent_history,
        consent_services: Arc::new(vec!["account-api".to_string()]),
        consent_observation: consent_observation.clone(),
        preferences: preferences.clone(),
        profile: profile.clone(),
        preferences_read,
        preferences_write,
        sessions: sessions_service.clone(),
        support: support.clone(),
        signup_gate: signup_gate.clone(),
        jurisdiction_check: jurisdiction_check.clone(),
        jurisdiction_publish: jurisdiction_publish.clone(),
        exports: export_service.clone(),
        deletions: deletion_service.clone(),
        account_lifecycle: account_lifecycle.clone(),
        due_users: Arc::new(identity_application::due_users::DueUsersService::new(
            users.clone(),
        )),
        user_registry: user_registry.clone(),
        users: users.clone(),
        authorisation,
        role_change,
        authorize_services: Arc::new(vec![
            "account-api".to_string(),
            "portfolio".to_string(),
            "identity-smoke".to_string(),
        ]),
        token_validator,
        service_tokens,
        wake: wake.clone(),
        readiness: Arc::new(Ready(true)),
        metrics,
    };

    Harness {
        users,
        keys,
        consent,
        consent_events,
        preferences_repository,
        user_lookup,
        consent_observation,
        erasure,
        grants,
        support_outbox,
        support_accounts,
        support_portfolios,
        support_observation,
        sessions,
        exports,
        deletions,
        transitions,
        account_observation,
        rules,
        audit,
        auth_admin,
        contributors,
        store,
        wake,
        gate_clearance,
        gate,
        geo,
        jurisdiction_lists,
        decision_audit,
        status_reader,
        authorisation_audit,
        role_writer,
        role_reader,
        clock,
        principal,
        consent_service,
        preferences,
        profile,
        sessions_service,
        support,
        signup_gate,
        jurisdiction_check,
        jurisdiction_publish,
        export_service,
        export_worker,
        deletion_service,
        account_lifecycle,
        deletion_worker,
        worker,
        user_registry,
        state,
    }
}

/// The authorisation HTTP harness: the default harness plus the four users the
/// C4 tests resolve by subject.
pub struct AuthorisationHarness {
    /// The shared harness.
    pub harness: Harness,
    /// The regular user's internal id.
    pub user_id: identity_domain::UserId,
    /// The administrator's internal id.
    pub admin_id: identity_domain::UserId,
    /// The support agent's internal id.
    pub support_id: identity_domain::UserId,
    /// A second user's internal id.
    pub other_id: identity_domain::UserId,
}

/// Builds the authorisation harness and seeds its users with an `active` status.
#[must_use]
pub fn authorisation_harness() -> AuthorisationHarness {
    let harness = harness();
    let user_id = seed_authorisation_user(&harness, USER_SUBJECT);
    let admin_id = seed_authorisation_user(&harness, ADMIN_SUBJECT);
    let support_id = seed_authorisation_user(&harness, SUPPORT_SUBJECT);
    let other_id = seed_authorisation_user(&harness, OTHER_SUBJECT);
    AuthorisationHarness {
        harness,
        user_id,
        admin_id,
        support_id,
        other_id,
    }
}

fn seed_authorisation_user(harness: &Harness, subject: &str) -> identity_domain::UserId {
    let user = active_user();
    let id = user.id();
    harness.users.seed(user, subject);
    harness.status_reader.set(id, UserStatus::Active);
    id
}

/// Connects to a disposable Postgres database, creates a unique schema, and runs
/// the real migrations into it.
///
/// # Panics
/// Panics when `TEST_DATABASE_URL` is unset or the migrations fail.
pub async fn test_pool() -> (PgPool, String) {
    let (pool, schema) = disposable_pool().await;
    run_migrations(&pool, None).await;
    (pool, schema)
}

/// Creates the schema and runs only the migrations below `version`. Used to seed
/// legacy rows at the 0004 shape before the live-table 0005 applies.
///
/// # Panics
/// Panics when `TEST_DATABASE_URL` is unset or the subset fails.
pub async fn test_pool_before(version: i64) -> (PgPool, String) {
    let (pool, schema) = disposable_pool().await;
    run_migrations(&pool, Some(version)).await;
    (pool, schema)
}

async fn disposable_pool() -> (PgPool, String) {
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

    (pool, schema)
}

/// The cluster-wide roles, created once under a short advisory lock. Holding
/// that lock across `0005` would deadlock: `CREATE INDEX CONCURRENTLY` waits for
/// every open snapshot, including the sessions blocked on the lock.
const ROLE_BOOTSTRAP: &str = r#"
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'identity_app') THEN
        EXECUTE 'CREATE ROLE identity_app NOLOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'identity_readonly') THEN
        EXECUTE 'CREATE ROLE identity_readonly NOLOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'identity_migrator') THEN
        EXECUTE 'CREATE ROLE identity_migrator NOLOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'identity_retention') THEN
        EXECUTE 'CREATE ROLE identity_retention LOGIN';
    END IF;
END
$$;
"#;

async fn run_migrations(pool: &PgPool, before: Option<i64>) {
    // sqlx's migrator takes a database-wide advisory lock, and a session blocked
    // on that lock still holds a virtual xid that 0005's
    // `CREATE INDEX CONCURRENTLY` waits for. Two cases migrating at once
    // therefore deadlock, so the binary runs one migration set at a time.
    static MIGRATION_GATE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _gate = MIGRATION_GATE.lock().await;

    // The lock is transaction-scoped so a panicking case can never return the
    // connection to the pool still holding it (which would wedge every later
    // case in the same test binary). It is released at commit, before the
    // migrations run, because 0005's `CREATE INDEX CONCURRENTLY` waits for every
    // open snapshot.
    let mut transaction = pool.begin().await.expect("acquire migration lock");
    sqlx::query("SELECT pg_advisory_xact_lock(9812461)")
        .execute(&mut *transaction)
        .await
        .expect("take migration advisory lock");
    sqlx::query(ROLE_BOOTSTRAP)
        .execute(&mut *transaction)
        .await
        .expect("bootstrap the cluster-wide roles");
    transaction
        .commit()
        .await
        .expect("release the migration advisory lock");

    let result = match before {
        Some(version) => {
            identity_infrastructure::migrations::migrator_before(version)
                .run(pool)
                .await
        }
        None => identity_infrastructure::migrations::run(pool).await,
    };
    result.expect("run migrations in the disposable schema");
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
