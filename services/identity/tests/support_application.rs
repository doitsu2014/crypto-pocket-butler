//! Support-access use cases against the hand-written port fakes: request,
//! approve, revoke and the metadata read (BR3.1–BR3.13, FR16.1–FR16.6).
//!
//! No test reads the wall clock; the harness clock supplies every instant.

mod common;

use chrono::Duration;
use common::*;
use identity_application::common::{AppError, Caller};
use identity_application::ports::{AccountSummary, ExternalError, PortfolioSummary};
use identity_domain::support::{SupportAccessGrant, SupportGrantId, SupportGrantStatus};
use identity_domain::{
    AuditOutcome, BaseCurrency, Locale, Role, TimeZone, User, UserId, UserStatus,
};

async fn as_support(harness: &Harness) -> Caller {
    let view = harness
        .principal
        .resolve(support_claims(), "corr-support")
        .await
        .expect("the support agent is provisioned");
    let user = harness.users.get(view.user_id()).expect("the user exists");
    caller(user, support_claims())
}

async fn as_admin(harness: &Harness) -> Caller {
    let view = harness
        .principal
        .resolve(admin_claims(), "corr-admin")
        .await
        .expect("the admin is provisioned");
    let user = harness.users.get(view.user_id()).expect("the user exists");
    caller(user, admin_claims())
}

async fn target(harness: &Harness) -> User {
    let view = harness
        .principal
        .resolve(other_claims(), "corr-target")
        .await
        .expect("the target is provisioned");
    harness.users.get(view.user_id()).expect("the user exists")
}

fn pending_target() -> User {
    User::from_stored(
        UserId::generate(),
        Role::User,
        UserStatus::PendingDeletion,
        Some("Leaving".to_string()),
        false,
        Some("leaving@example.com".to_string()),
        false,
        true,
        Locale::En,
        BaseCurrency::parse("USD").expect("USD is valid"),
        TimeZone::parse("UTC").expect("UTC is valid"),
        fixed_now(),
        fixed_now(),
        None,
    )
}

async fn pending_grant(harness: &Harness, target_id: UserId) -> SupportAccessGrant {
    let support = as_support(harness).await;
    harness
        .support
        .request(&support, target_id, "  investigate a sync failure  ")
        .await
        .expect("a support agent requests a grant")
}

async fn approved_grant(harness: &Harness, target_id: UserId) -> SupportAccessGrant {
    let grant = pending_grant(harness, target_id).await;
    let admin = as_admin(harness).await;
    harness
        .support
        .approve(&admin, grant.id())
        .await
        .expect("a distinct admin approves")
}

fn action_of(event: &identity_domain::AuditRecord) -> String {
    event.action().to_string()
}

#[tokio::test]
async fn an_admin_lists_pending_grants_newest_first_within_the_bound() {
    let harness = harness();
    let admin = as_admin(&harness).await;
    let agent = UserId::generate();
    let user = UserId::generate();
    let other = UserId::generate();

    let older = SupportAccessGrant::request(
        SupportGrantId::generate(),
        user,
        agent,
        "investigate the older case",
        fixed_now() - Duration::minutes(5),
    )
    .expect("a valid request");
    let newer = SupportAccessGrant::request(
        SupportGrantId::generate(),
        other,
        agent,
        "investigate the newer case",
        fixed_now(),
    )
    .expect("a valid request");
    let active = SupportAccessGrant::from_stored(
        SupportGrantId::generate(),
        user,
        agent,
        "an approved grant".to_string(),
        SupportGrantStatus::Active,
        fixed_now() - Duration::minutes(10),
        Some(UserId::generate()),
        Some(fixed_now() - Duration::minutes(9)),
        Some(fixed_now() + Duration::minutes(51)),
        None,
    );
    harness
        .grants
        .grants
        .lock()
        .unwrap()
        .extend([older.clone(), active.clone(), newer.clone()]);

    let listed = harness
        .support
        .pending_grants(&admin)
        .await
        .expect("an admin reads the pending list");
    assert_eq!(
        listed
            .iter()
            .map(SupportAccessGrant::id)
            .collect::<Vec<_>>(),
        vec![newer.id(), older.id()],
        "newest first, and only pending grants"
    );
    assert_eq!(
        harness.grants.list_calls.lock().unwrap().as_slice(),
        [(SupportGrantStatus::PendingApproval, 100)],
        "one bounded read of the pending status"
    );
}

#[tokio::test]
async fn a_non_admin_is_refused_on_the_grant_list_and_nothing_is_read() {
    let harness = harness();
    let support = as_support(&harness).await;

    let error = harness
        .support
        .pending_grants(&support)
        .await
        .expect_err("a support operator may not review the queue");
    assert!(matches!(error, AppError::Forbidden(_)), "{error:?}");
    assert!(
        harness.grants.list_calls.lock().unwrap().is_empty(),
        "a refused read never reaches the repository"
    );
}

#[tokio::test]
async fn a_failed_grant_list_read_is_a_503() {
    let harness = harness();
    let admin = as_admin(&harness).await;
    *harness.grants.failure.lock().unwrap() = Some(
        identity_application::ports::RepositoryError::Unavailable("database down".to_string()),
    );

    let error = harness
        .support
        .pending_grants(&admin)
        .await
        .expect_err("a store failure is surfaced");
    assert!(matches!(error, AppError::Unavailable(_)), "{error:?}");
}

#[tokio::test]
async fn a_support_agent_requests_a_pending_grant_and_the_event_joins_the_insert() {
    let harness = harness();
    let target = target(&harness).await;

    let grant = pending_grant(&harness, target.id()).await;

    assert_eq!(grant.status(), SupportGrantStatus::PendingApproval);
    assert_eq!(grant.reason(), "investigate a sync failure");
    assert_eq!(
        grant.support_agent_id(),
        as_support(&harness).await.user_id()
    );
    let events = harness.grants.events.lock().unwrap();
    assert_eq!(events.len(), 1, "the request and its event commit together");
    assert_eq!(action_of(&events[0]), "identity.support_grant.requested");
    assert_eq!(events[0].outcome(), AuditOutcome::Success);
}

#[tokio::test]
async fn a_non_support_caller_is_refused_and_the_refusal_is_audited() {
    let harness = harness();
    let target = target(&harness).await;
    let ordinary = caller(target.clone(), other_claims());

    let error = harness
        .support
        .request(&ordinary, target.id(), "read another user")
        .await
        .expect_err("a regular user cannot request support access");

    assert_eq!(
        error,
        AppError::Forbidden("support-role-required".to_string())
    );
    let events = harness.support_outbox.events.lock().unwrap();
    assert_eq!(
        events.len(),
        1,
        "a refusal is audited before it is returned"
    );
    assert_eq!(action_of(&events[0]), "identity.support_grant.requested");
    assert_eq!(events[0].outcome(), AuditOutcome::Denied);
    assert_eq!(events[0].target(), target.id().to_string());
}

#[tokio::test]
async fn a_pending_deletion_target_is_refused_and_audited() {
    let harness = harness();
    let leaving = pending_target();
    let leaving_id = leaving.id();
    harness
        .users
        .seed(leaving, "55555555-5555-4555-8555-555555555555");
    let support = as_support(&harness).await;

    let error = harness
        .support
        .request(&support, leaving_id, "investigate")
        .await
        .expect_err("a pending-deletion target cannot be read");

    assert_eq!(
        error,
        AppError::Forbidden("target-not-accessible".to_string())
    );
    assert_eq!(harness.support_outbox.events.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn a_second_live_grant_is_a_conflict_and_the_refusal_is_audited() {
    let harness = harness();
    let target = target(&harness).await;
    let first = pending_grant(&harness, target.id()).await;
    let support = as_support(&harness).await;

    let error = harness
        .support
        .request(&support, target.id(), "ask again")
        .await
        .expect_err("a live grant already covers the pair");

    assert_eq!(error, AppError::Conflict("grant-already-live".to_string()));
    assert_eq!(harness.grants.grants.lock().unwrap().len(), 1);
    let events = harness.support_outbox.events.lock().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0]
            .context()
            .get("existingGrantId")
            .map(String::as_str),
        Some(first.id().to_string().as_str())
    );
}

#[tokio::test]
async fn requesting_your_own_record_is_a_validation_refusal() {
    let harness = harness();
    let support = as_support(&harness).await;

    let error = harness
        .support
        .request(&support, support.user_id(), "read myself")
        .await
        .expect_err("a request cannot name the requester");

    assert!(matches!(error, AppError::Validation(_)));
}

#[tokio::test]
async fn a_failed_refusal_append_answers_audit_unavailable() {
    let harness = harness();
    let target = target(&harness).await;
    let ordinary = caller(target.clone(), other_claims());
    *harness.support_outbox.failure.lock().unwrap() = Some(
        identity_application::ports::RepositoryError::Unavailable("audit down".to_string()),
    );

    let error = harness
        .support
        .request(&ordinary, target.id(), "read another user")
        .await
        .expect_err("an unrecorded refusal is surfaced");

    assert_eq!(
        error,
        AppError::Unavailable("audit-unavailable".to_string())
    );
}

#[tokio::test]
async fn an_admin_approves_a_pending_grant_and_the_window_is_sixty_minutes() {
    let harness = harness();
    let target = target(&harness).await;
    let grant = pending_grant(&harness, target.id()).await;
    let admin = as_admin(&harness).await;

    let approved = harness
        .support
        .approve(&admin, grant.id())
        .await
        .expect("a distinct admin approves");

    assert_eq!(approved.status(), SupportGrantStatus::Active);
    assert_eq!(approved.approved_by(), Some(admin.user_id()));
    assert_eq!(
        approved.expires_at(),
        Some(fixed_now() + Duration::minutes(60))
    );
    assert_eq!(harness.grants.events.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn a_non_admin_is_refused_before_the_grant_is_loaded() {
    let harness = harness();
    let support = as_support(&harness).await;
    let unknown = identity_domain::SupportGrantId::generate();

    let error = harness
        .support
        .approve(&support, unknown)
        .await
        .expect_err("a non-admin cannot approve");

    assert_eq!(
        error,
        AppError::Forbidden("admin-role-required".to_string()),
        "the role check runs first, so a non-admin cannot probe grant ids"
    );
}

#[tokio::test]
async fn a_self_approval_is_refused() {
    let harness = harness();
    let target = target(&harness).await;
    let grant = pending_grant(&harness, target.id()).await;
    let support = as_support(&harness).await;
    // The support agent is given the admin role for this one call.
    let mut admin_claims = support_claims();
    admin_claims.role = identity_domain::claims::Role::Admin;
    let support_as_admin = caller(support.user.clone(), admin_claims);

    let error = harness
        .support
        .approve(&support_as_admin, grant.id())
        .await
        .expect_err("the requester cannot approve their own grant");

    assert_eq!(error, AppError::Forbidden("self-approval".to_string()));
}

#[tokio::test]
async fn an_expired_request_is_persisted_as_expired_and_refused() {
    let harness = harness();
    let target = target(&harness).await;
    let grant = pending_grant(&harness, target.id()).await;
    harness.clock.set(fixed_now() + Duration::minutes(61));
    let admin = as_admin(&harness).await;

    let error = harness
        .support
        .approve(&admin, grant.id())
        .await
        .expect_err("a request older than the hour cannot be approved");

    assert_eq!(error, AppError::Conflict("request-expired".to_string()));
    let stored = harness
        .grants
        .grants
        .lock()
        .unwrap()
        .iter()
        .find(|stored| stored.id() == grant.id())
        .cloned()
        .expect("the grant row is kept as evidence");
    assert_eq!(stored.status(), SupportGrantStatus::Expired);
    assert_eq!(stored.approved_at(), None);
}

#[tokio::test]
async fn a_zero_row_conditional_approval_is_a_conflict() {
    let harness = harness();
    let target = target(&harness).await;
    let grant = pending_grant(&harness, target.id()).await;
    harness
        .grants
        .zero_rows
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let admin = as_admin(&harness).await;

    let error = harness
        .support
        .approve(&admin, grant.id())
        .await
        .expect_err("another writer already moved the grant");

    assert_eq!(error, AppError::Conflict("grant-not-pending".to_string()));
}

#[tokio::test]
async fn a_terminal_grant_cannot_be_revoked_again() {
    let harness = harness();
    let target = target(&harness).await;
    let grant = approved_grant(&harness, target.id()).await;
    let admin = as_admin(&harness).await;
    harness.support.revoke(&admin, grant.id()).await.unwrap();

    let error = harness
        .support
        .revoke(&admin, grant.id())
        .await
        .expect_err("a revoked grant is terminal");

    assert_eq!(error, AppError::Conflict("grant-not-pending".to_string()));
}

#[tokio::test]
async fn the_holder_reads_the_projection_and_the_served_event_is_written() {
    let harness = harness();
    let target = target(&harness).await;
    let grant = approved_grant(&harness, target.id()).await;
    let account_id = uuid::Uuid::new_v4();
    let portfolio_id = uuid::Uuid::new_v4();
    harness
        .support_accounts
        .accounts
        .lock()
        .unwrap()
        .push(AccountSummary {
            id: account_id,
            label: "Main exchange".to_string(),
            source_type: "exchange".to_string(),
            sync_status: "active".to_string(),
            last_synced_at: Some(fixed_now()),
        });
    harness
        .support_portfolios
        .portfolios
        .lock()
        .unwrap()
        .push(PortfolioSummary {
            id: portfolio_id,
            label: "Long term".to_string(),
        });
    let support = as_support(&harness).await;

    let metadata = harness
        .support
        .metadata(&support, grant.id())
        .await
        .expect("the holder reads under an active grant");

    assert_eq!(metadata.user_id, target.id());
    assert_eq!(metadata.display_name.as_deref(), Some("Test User"));
    assert!(metadata.email_verified);
    assert_eq!(metadata.accounts.len(), 1);
    assert_eq!(metadata.accounts[0].id, account_id);
    assert_eq!(metadata.portfolios.len(), 1);
    assert_eq!(metadata.portfolios[0].id, portfolio_id);
    let served = harness.support_observation.served.lock().unwrap();
    assert_eq!(
        served.len(),
        1,
        "the served event is written before the response"
    );
    assert_eq!(served[0].0, support.user_id());
}

#[tokio::test]
async fn an_unknown_grant_and_a_foreign_grant_get_the_same_uniform_refusal() {
    let harness = harness();
    let target = target(&harness).await;
    let foreign_grant = SupportAccessGrant::request(
        SupportGrantId::generate(),
        target.id(),
        UserId::generate(),
        "another agent's grant",
        fixed_now(),
    )
    .expect("a grant owned by another agent");
    harness
        .grants
        .grants
        .lock()
        .unwrap()
        .push(foreign_grant.clone());
    let support = as_support(&harness).await;

    let unknown = harness
        .support
        .metadata(&support, SupportGrantId::generate())
        .await
        .expect_err("an unknown grant is refused");
    let foreign = harness.support.metadata(&support, foreign_grant.id()).await;

    assert_eq!(
        unknown,
        AppError::Forbidden("support-access-refused".to_string())
    );
    assert_eq!(
        foreign.expect_err("a foreign holder is refused"),
        AppError::Forbidden("support-access-refused".to_string()),
        "the two answers are indistinguishable"
    );
}

#[tokio::test]
async fn a_non_support_caller_is_refused_on_the_read() {
    let harness = harness();
    let target = target(&harness).await;
    let grant = approved_grant(&harness, target.id()).await;
    let admin = as_admin(&harness).await;

    let error = harness
        .support
        .metadata(&admin, grant.id())
        .await
        .expect_err("an admin is an approver, not a reader");

    assert_eq!(
        error,
        AppError::Forbidden("support-role-required".to_string())
    );
}

#[tokio::test]
async fn a_pending_grant_reads_as_grant_pending_and_a_revoked_one_as_grant_revoked() {
    let harness = harness();
    let target = target(&harness).await;
    let grant = pending_grant(&harness, target.id()).await;
    let support = as_support(&harness).await;

    let pending = harness
        .support
        .metadata(&support, grant.id())
        .await
        .expect_err("a pending grant authorises nothing");
    assert_eq!(pending, AppError::Forbidden("grant-pending".to_string()));

    let admin = as_admin(&harness).await;
    harness.support.revoke(&admin, grant.id()).await.unwrap();
    let revoked = harness
        .support
        .metadata(&support, grant.id())
        .await
        .expect_err("a revoked grant authorises nothing");
    assert_eq!(revoked, AppError::Forbidden("grant-revoked".to_string()));
}

#[tokio::test]
async fn an_expired_grant_is_persisted_before_the_read_refusal() {
    let harness = harness();
    let target = target(&harness).await;
    let grant = approved_grant(&harness, target.id()).await;
    harness.clock.set(fixed_now() + Duration::minutes(61));
    let support = as_support(&harness).await;

    let error = harness
        .support
        .metadata(&support, grant.id())
        .await
        .expect_err("the window has passed");

    assert_eq!(error, AppError::Forbidden("grant-expired".to_string()));
    let stored = harness
        .grants
        .grants
        .lock()
        .unwrap()
        .iter()
        .find(|stored| stored.id() == grant.id())
        .cloned()
        .expect("the grant row is kept");
    assert_eq!(stored.status(), SupportGrantStatus::Expired);
}

#[tokio::test]
async fn a_source_failure_refuses_the_read_and_audits_it_as_failed() {
    let harness = harness();
    let target = target(&harness).await;
    let grant = approved_grant(&harness, target.id()).await;
    *harness.support_portfolios.failure.lock().unwrap() = Some(ExternalError::Unavailable(
        "portfolio timed out".to_string(),
    ));
    let support = as_support(&harness).await;

    let error = harness
        .support
        .metadata(&support, grant.id())
        .await
        .expect_err("half a projection is never returned");

    assert_eq!(
        error,
        AppError::Unavailable("metadata-source-unavailable".to_string())
    );
    assert!(
        harness
            .support_observation
            .served
            .lock()
            .unwrap()
            .is_empty(),
        "no data was served"
    );
    let failed = harness.support_observation.failed.lock().unwrap();
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].2, "metadata-source-unavailable");
}

#[tokio::test]
async fn a_source_that_misses_the_deadline_refuses_the_read() {
    let harness = harness();
    let target = target(&harness).await;
    let grant = approved_grant(&harness, target.id()).await;
    harness
        .support_accounts
        .delay_ms
        .store(600, std::sync::atomic::Ordering::SeqCst);
    let support = as_support(&harness).await;

    let error = harness
        .support
        .metadata(&support, grant.id())
        .await
        .expect_err("the 500 ms budget refuses the read");

    assert_eq!(
        error,
        AppError::Unavailable("metadata-source-unavailable".to_string())
    );
}

#[tokio::test]
async fn the_two_sources_are_read_concurrently() {
    let harness = harness();
    let target = target(&harness).await;
    let grant = approved_grant(&harness, target.id()).await;
    harness
        .support_accounts
        .delay_ms
        .store(100, std::sync::atomic::Ordering::SeqCst);
    harness
        .support_portfolios
        .delay_ms
        .store(100, std::sync::atomic::Ordering::SeqCst);
    let support = as_support(&harness).await;

    harness
        .support
        .metadata(&support, grant.id())
        .await
        .expect("both sources answer inside the budget");

    let accounts_start = harness.support_accounts.started.lock().unwrap().unwrap();
    let accounts_end = harness.support_accounts.finished.lock().unwrap().unwrap();
    let portfolios_start = harness.support_portfolios.started.lock().unwrap().unwrap();
    let portfolios_end = harness.support_portfolios.finished.lock().unwrap().unwrap();
    assert!(
        portfolios_start < accounts_end && accounts_start < portfolios_end,
        "the two calls overlap instead of running one after the other"
    );
}

#[tokio::test]
async fn an_unrecordable_served_read_is_refused() {
    let harness = harness();
    let target = target(&harness).await;
    let grant = approved_grant(&harness, target.id()).await;
    *harness.support_observation.failure.lock().unwrap() = Some(
        identity_application::ports::RepositoryError::Unavailable("audit down".to_string()),
    );
    let support = as_support(&harness).await;

    let error = harness
        .support
        .metadata(&support, grant.id())
        .await
        .expect_err("no data leaves without a record");

    assert_eq!(
        error,
        AppError::Unavailable("audit-unavailable".to_string())
    );
}

// ---------------------------------------------------------------------------
// S2 — the support agent's own grants (GET /v1/support-grants/mine)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_support_agent_reads_only_their_own_grants_newest_first_within_the_bound() {
    let harness = harness();
    let support = as_support(&harness).await;
    let agent = support.user_id();
    let other_agent = UserId::generate();

    // Twenty-one of the caller's grants, one minute apart, plus one held by a
    // different agent that is newer than all of them.
    let mut seeded = Vec::new();
    for index in 0..21 {
        seeded.push(
            SupportAccessGrant::request(
                SupportGrantId::generate(),
                UserId::generate(),
                agent,
                "investigate a reported sync failure",
                fixed_now() - Duration::minutes(30 - index),
            )
            .expect("a valid request"),
        );
    }
    let foreign = SupportAccessGrant::request(
        SupportGrantId::generate(),
        UserId::generate(),
        other_agent,
        "another agent's grant",
        fixed_now(),
    )
    .expect("a valid request");
    let mut stored = seeded.clone();
    stored.push(foreign.clone());
    harness.grants.grants.lock().unwrap().extend(stored);

    let mine = harness
        .support
        .my_grants(&support)
        .await
        .expect("a support agent reads their own grants");

    assert_eq!(mine.len(), 20, "the read is bounded by MY_GRANTS_LIMIT");
    assert!(
        mine.iter().all(|grant| grant.support_agent_id() == agent),
        "only the caller's grants are returned"
    );
    assert!(
        !mine.iter().any(|grant| grant.id() == foreign.id()),
        "another agent's grant is absent even though it is the newest row"
    );
    let newest_first: Vec<_> = mine.iter().map(SupportAccessGrant::requested_at).collect();
    let mut sorted = newest_first.clone();
    sorted.sort_by(|left, right| right.cmp(left));
    assert_eq!(newest_first, sorted, "newest first");
    assert_eq!(
        mine.first().map(SupportAccessGrant::id),
        Some(seeded[20].id()),
        "the newest own grant is first"
    );
    assert_eq!(
        mine.last().map(SupportAccessGrant::id),
        Some(seeded[1].id()),
        "the 21st newest own grant is the one dropped"
    );
    assert_eq!(
        harness.grants.mine_calls.lock().unwrap().as_slice(),
        [(agent, 20)],
        "one bounded read scoped to the caller"
    );
}

#[tokio::test]
async fn a_support_agent_with_no_grants_gets_an_empty_list_not_an_error() {
    let harness = harness();
    let support = as_support(&harness).await;

    let mine = harness
        .support
        .my_grants(&support)
        .await
        .expect("an empty read is not an error");

    assert!(mine.is_empty());
    assert_eq!(
        harness.grants.mine_calls.lock().unwrap().as_slice(),
        [(support.user_id(), 20)],
        "the empty answer is still one bounded, caller-scoped read"
    );
}

#[tokio::test]
async fn a_caller_without_the_support_role_is_refused_on_the_mine_read() {
    let harness = harness();
    let user = target(&harness).await;
    let non_support = caller(user, other_claims());

    let error = harness
        .support
        .my_grants(&non_support)
        .await
        .expect_err("only a support agent may read their grants");

    assert!(matches!(error, AppError::Forbidden(_)));
    assert!(
        harness.grants.mine_calls.lock().unwrap().is_empty(),
        "a refused caller causes no read"
    );
}
