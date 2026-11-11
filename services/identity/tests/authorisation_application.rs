//! Application tests for the decision order and the role change, over port
//! fakes (BR2.1–BR2.14).
//!
//! The decision is exercised in-process, exactly as an Identity module would
//! call it; the C4 adapter is tested separately.

mod common;

use std::sync::Arc;

use identity_application::authorisation::catalogue::OperationCatalogue;
use identity_application::authorisation::decision::{AuthorisationDecision, DecisionError};
use identity_application::authorisation::ports::StatusReadError;
use identity_application::authorisation::role_change::RoleChange;
use identity_application::common::AppError;
use identity_application::ports::ExternalError;
use identity_domain::authorisation::{
    CallerIdentity, DenialReason, OperationClass, ResourceOwnership,
};
use identity_domain::{AuditOutcome, Role, UserId, UserStatus};

use common::fakes::{
    FakeAuthorisationAudit, FakeObservation, FakeRoleWriter, FakeStatusReader, FakeUsers,
};

struct Fixture {
    users: Arc<FakeUsers>,
    statuses: Arc<FakeStatusReader>,
    audit: Arc<FakeAuthorisationAudit>,
    roles: Arc<FakeRoleWriter>,
    observation: Arc<FakeObservation>,
    decision: AuthorisationDecision,
    role_change: RoleChange,
}

fn fixture() -> Fixture {
    let users = Arc::new(FakeUsers::default());
    let statuses = Arc::new(FakeStatusReader::default());
    let audit = Arc::new(FakeAuthorisationAudit::default());
    let roles = Arc::new(FakeRoleWriter::default());
    let observation = Arc::new(FakeObservation::default());
    let clock = Arc::new(common::fakes::MutableClock::at(common::fixed_now()));

    let catalogue = OperationCatalogue::new();
    let cached_status: Arc<dyn identity_application::authorisation::ports::StatusReader> =
        Arc::new(identity_infrastructure::StatusCache::new(
            statuses.clone(),
            clock.clone(),
            observation.clone(),
        ));
    let decision = AuthorisationDecision::new(
        catalogue,
        cached_status,
        audit.clone(),
        observation.clone(),
        clock.clone(),
    );
    let role_change = RoleChange::new(
        users.clone(),
        roles.clone(),
        audit.clone(),
        observation.clone(),
        clock.clone(),
    );
    Fixture {
        users,
        statuses,
        audit,
        roles,
        observation,
        decision,
        role_change,
    }
}

fn owner(id: UserId) -> CallerIdentity {
    CallerIdentity::user("owner-subject", id, Role::User)
}

fn admin(id: UserId) -> CallerIdentity {
    CallerIdentity::user("admin-subject", id, Role::Admin)
}

fn support(id: UserId) -> CallerIdentity {
    CallerIdentity::user("support-subject", id, Role::Support)
}

fn system(owner: UserId) -> CallerIdentity {
    CallerIdentity::system("reporting-worker", owner)
}

fn operation(name: &str) -> identity_domain::authorisation::Operation {
    OperationCatalogue::new().lookup(name)
}

#[test]
fn the_catalogue_classes_every_declared_operation_and_marks_unknown_names_undeclared() {
    let catalogue = OperationCatalogue::new();
    for name in [
        "principal:read",
        "profile:read",
        "profile:write",
        "resource:read",
        "resource:write",
    ] {
        assert_eq!(
            catalogue.class_of(name),
            OperationClass::OwnerScoped,
            "{name}"
        );
    }
    for name in ["collection:list", "collection:search", "collection:export"] {
        assert_eq!(
            catalogue.class_of(name),
            OperationClass::Collection,
            "{name}"
        );
    }
    assert_eq!(
        catalogue.class_of("guard:confirm"),
        OperationClass::Confirmation
    );
    assert_eq!(
        catalogue.class_of("support:metadata:read"),
        OperationClass::SupportMetadata
    );
    for name in ["admin:users:list", "admin:role:change", "admin:jobs:run"] {
        assert_eq!(
            catalogue.class_of(name),
            OperationClass::Administrative,
            "{name}"
        );
    }
    assert_eq!(
        catalogue.class_of("principal:delete"),
        OperationClass::Undeclared
    );
    assert_eq!(catalogue.class_of(""), OperationClass::Undeclared);
}

#[tokio::test]
async fn an_owner_scoped_row_is_allowed_for_its_owner_and_refused_for_anyone_else() {
    let fixture = fixture();
    let me = UserId::generate();
    let other = UserId::generate();
    fixture.statuses.set(me, UserStatus::Active);
    let op = operation("principal:read");

    let own = ResourceOwnership::row("account", Some(me));
    let decision = fixture
        .decision
        .decide(&owner(me), &op, Some(&own), "corr")
        .await
        .unwrap();
    assert!(decision.allowed());
    assert_eq!(decision.http_status(), 200);

    let foreign = ResourceOwnership::row("account", Some(other));
    let decision = fixture
        .decision
        .decide(&owner(me), &op, Some(&foreign), "corr")
        .await
        .unwrap();
    assert!(!decision.allowed());
    assert_eq!(decision.reason(), DenialReason::NotOwner);
    assert_eq!(decision.http_status(), 404);

    let absent = ResourceOwnership::row("account", None);
    let decision = fixture
        .decision
        .decide(&owner(me), &op, Some(&absent), "corr")
        .await
        .unwrap();
    assert_eq!(decision.reason(), DenialReason::NotOwner);
}

#[tokio::test]
async fn admin_is_additive_on_its_own_rows_but_not_a_substitute_for_ownership() {
    let fixture = fixture();
    let me = UserId::generate();
    let other = UserId::generate();
    fixture.statuses.set(me, UserStatus::Active);
    let op = operation("profile:write");

    let own = ResourceOwnership::row("preference", Some(me));
    assert!(fixture
        .decision
        .decide(&admin(me), &op, Some(&own), "corr")
        .await
        .unwrap()
        .allowed());

    let foreign = ResourceOwnership::row("preference", Some(other));
    let decision = fixture
        .decision
        .decide(&admin(me), &op, Some(&foreign), "corr")
        .await
        .unwrap();
    assert_eq!(decision.reason(), DenialReason::NotOwner);
}

#[tokio::test]
async fn an_administrative_operation_is_decided_by_the_role_alone() {
    let fixture = fixture();
    let me = UserId::generate();
    let other = UserId::generate();
    fixture.statuses.set(me, UserStatus::Active);
    fixture.statuses.set(other, UserStatus::Active);
    let op = operation("admin:users:list");

    // The admin role allows it with no ownership assertion at all.
    assert!(fixture
        .decision
        .decide(&admin(me), &op, None, "corr")
        .await
        .unwrap()
        .allowed());

    // An owner is refused even when it asserts its own row.
    let own = ResourceOwnership::row("user", Some(me));
    assert_eq!(
        fixture
            .decision
            .decide(&owner(me), &op, Some(&own), "corr")
            .await
            .unwrap()
            .reason(),
        DenialReason::NoRole
    );

    assert_eq!(
        fixture
            .decision
            .decide(&support(other), &op, None, "corr")
            .await
            .unwrap()
            .reason(),
        DenialReason::NoRole
    );
}

#[tokio::test]
async fn status_overrides_every_operation_and_pending_reaches_only_confirmation() {
    let fixture = fixture();
    let me = UserId::generate();
    fixture.statuses.set(me, UserStatus::Disabled);
    let read = operation("principal:read");
    let own = ResourceOwnership::row("account", Some(me));
    assert_eq!(
        fixture
            .decision
            .decide(&owner(me), &read, Some(&own), "corr")
            .await
            .unwrap()
            .reason(),
        DenialReason::StatusDenied
    );

    for status in [UserStatus::PendingDeletion, UserStatus::Deleted] {
        fixture.statuses.set(me, status);
        assert_eq!(
            fixture
                .decision
                .decide(&owner(me), &read, Some(&own), "corr")
                .await
                .unwrap()
                .reason(),
            DenialReason::StatusDenied
        );
    }

    fixture.statuses.set(me, UserStatus::Pending);
    assert_eq!(
        fixture
            .decision
            .decide(&owner(me), &read, Some(&own), "corr")
            .await
            .unwrap()
            .reason(),
        DenialReason::StatusDenied
    );
    assert!(fixture
        .decision
        .decide(&owner(me), &operation("guard:confirm"), None, "corr")
        .await
        .unwrap()
        .allowed());
}

#[tokio::test]
async fn support_never_owns_but_is_allowed_its_own_metadata_class() {
    let fixture = fixture();
    let agent = UserId::generate();
    let other = UserId::generate();
    fixture.statuses.set(agent, UserStatus::Active);
    let own = ResourceOwnership::row("account", Some(other));

    for name in [
        "principal:read",
        "collection:list",
        "admin:users:list",
        "guard:confirm",
    ] {
        assert_eq!(
            fixture
                .decision
                .decide(&support(agent), &operation(name), Some(&own), "corr")
                .await
                .unwrap()
                .reason(),
            DenialReason::NoRole,
            "{name}"
        );
    }

    // The one exception: the grant check is u3's, not an ownership comparison.
    assert!(fixture
        .decision
        .decide(
            &support(agent),
            &operation("support:metadata:read"),
            None,
            "corr",
        )
        .await
        .unwrap()
        .allowed());
}

#[tokio::test]
async fn a_collection_returns_the_callers_own_owner_filter() {
    let fixture = fixture();
    let me = UserId::generate();
    fixture.statuses.set(me, UserStatus::Active);
    let collection = ResourceOwnership::collection("transaction");

    let decision = fixture
        .decision
        .decide(
            &owner(me),
            &operation("collection:list"),
            Some(&collection),
            "corr",
        )
        .await
        .unwrap();
    assert!(decision.allowed());
    assert_eq!(decision.filter_owner_id(), Some(me));

    // An admin's collection is still filtered to the admin's own rows.
    let decision = fixture
        .decision
        .decide(
            &admin(me),
            &operation("collection:export"),
            Some(&collection),
            "corr",
        )
        .await
        .unwrap();
    assert_eq!(decision.filter_owner_id(), Some(me));
}

#[tokio::test]
async fn a_system_job_is_never_an_admin_and_acts_only_for_its_stated_owner() {
    let fixture = fixture();
    let iterated = UserId::generate();
    let other = UserId::generate();
    let op = operation("resource:read");

    assert_eq!(
        fixture
            .decision
            .decide(
                &system(iterated),
                &operation("admin:jobs:run"),
                None,
                "corr"
            )
            .await
            .unwrap()
            .reason(),
        DenialReason::NoRole
    );
    assert_eq!(
        fixture
            .decision
            .decide(&system(iterated), &operation("guard:confirm"), None, "corr")
            .await
            .unwrap()
            .reason(),
        DenialReason::NoRole
    );

    // It reads the row it iterates for.
    let stated = ResourceOwnership::row("snapshot", Some(iterated));
    assert!(fixture
        .decision
        .decide(&system(iterated), &op, Some(&stated), "corr")
        .await
        .unwrap()
        .allowed());

    // It cannot reach another owner's row, even though it is a job.
    let foreign = ResourceOwnership::row("snapshot", Some(other));
    assert_eq!(
        fixture
            .decision
            .decide(&system(iterated), &op, Some(&foreign), "corr")
            .await
            .unwrap()
            .reason(),
        DenialReason::NotOwner
    );

    // A row with no stated owner is refused.
    let unstated = ResourceOwnership::row("snapshot", None);
    assert_eq!(
        fixture
            .decision
            .decide(&system(iterated), &op, Some(&unstated), "corr")
            .await
            .unwrap()
            .reason(),
        DenialReason::NotOwner
    );

    // A collection carries the job's own iterated owner as its filter.
    let collection = ResourceOwnership::collection("transaction");
    let decision = fixture
        .decision
        .decide(
            &system(iterated),
            &operation("collection:list"),
            Some(&collection),
            "corr",
        )
        .await
        .unwrap();
    assert!(decision.allowed());
    assert_eq!(decision.filter_owner_id(), Some(iterated));

    // A collection with no ownership assertion at all still filters to the
    // job's stated owner, never to "everyone".
    let decision = fixture
        .decision
        .decide(
            &system(iterated),
            &operation("collection:list"),
            None,
            "corr",
        )
        .await
        .unwrap();
    assert_eq!(decision.filter_owner_id(), Some(iterated));
}

#[tokio::test]
async fn an_undeclared_operation_is_denied_before_the_status_read() {
    let fixture = fixture();
    let me = UserId::generate();
    fixture.statuses.fail_next(StatusReadError::Unavailable(
        "should not be read".to_string(),
    ));

    let decision = fixture
        .decision
        .decide(&owner(me), &operation("principal:delete"), None, "corr")
        .await
        .unwrap();
    assert_eq!(decision.reason(), DenialReason::NotDeclared);
    assert_eq!(decision.http_status(), 403);
    assert_eq!(fixture.statuses.calls(), 0, "the catalogue decides first");
}

#[tokio::test]
async fn every_denial_is_audited_before_it_returns() {
    let fixture = fixture();
    let me = UserId::generate();
    let other = UserId::generate();
    fixture.statuses.set(me, UserStatus::Active);
    let foreign = ResourceOwnership::row("account", Some(other));

    let decision = fixture
        .decision
        .decide(
            &owner(me),
            &operation("principal:read"),
            Some(&foreign),
            "corr-9",
        )
        .await
        .unwrap();
    assert_eq!(decision.reason(), DenialReason::NotOwner);

    let events = fixture.audit.events();
    assert_eq!(events.len(), 1, "one denial writes one event");
    let (event, correlation) = &events[0];
    assert_eq!(correlation, "corr-9");
    assert_eq!(event.actor(), me.to_string());
    assert_eq!(event.action(), "principal:read");
    assert_eq!(event.outcome(), AuditOutcome::Denied);
    assert_eq!(
        event.context().get("reason").map(String::as_str),
        Some("not_owner")
    );
    // The target names the resource kind and the caller's own id, never the
    // other user's id.
    assert!(event.target().contains("account"));
    assert!(event.target().contains(&me.to_string()));
    assert!(!event.target().contains(&other.to_string()));
}

#[tokio::test]
async fn a_denial_that_cannot_be_audited_fails_closed_with_503() {
    let fixture = fixture();
    let me = UserId::generate();
    fixture.statuses.set(me, UserStatus::Active);
    fixture
        .audit
        .fail_next(identity_application::ports::RepositoryError::Unavailable(
            "audit down".to_string(),
        ));

    let decision = fixture
        .decision
        .decide(&owner(me), &operation("admin:users:list"), None, "corr")
        .await
        .unwrap();
    assert_eq!(decision.reason(), DenialReason::AuditUnavailable);
    assert_eq!(decision.http_status(), 503);
    assert_eq!(
        fixture
            .observation
            .audit_failures
            .lock()
            .unwrap()
            .as_slice(),
        ["denial".to_string()]
    );
}

#[tokio::test]
async fn a_status_read_failure_is_an_error_not_a_decision() {
    let fixture = fixture();
    let me = UserId::generate();
    fixture.statuses.fail_next(StatusReadError::Timeout);

    let error = fixture
        .decision
        .decide(&owner(me), &operation("principal:read"), None, "corr")
        .await
        .unwrap_err();
    assert!(matches!(error, DecisionError::StatusUnavailable(_)));
    // Nothing was decided, so nothing is audited as a denial.
    assert!(fixture.audit.events().is_empty());
    assert_eq!(
        fixture
            .observation
            .status_failures
            .lock()
            .unwrap()
            .as_slice(),
        ["timeout".to_string()]
    );
}

#[tokio::test]
async fn a_role_change_writes_the_intent_the_provider_then_the_outcome() {
    let fixture = fixture();
    let admin_id = UserId::generate();
    let target = common::active_user();
    let target_id = target.id();
    fixture.statuses.set(admin_id, UserStatus::Active);
    fixture.users.seed(target, "target-subject");

    let view = fixture
        .role_change
        .change(
            &admin(admin_id),
            target_id,
            Role::Admin,
            "promotion approved by the team",
            "corr-role",
        )
        .await
        .unwrap();
    assert_eq!(view.user_id(), target_id);
    assert_eq!(view.previous_role(), Role::User);
    assert_eq!(view.new_role(), Role::Admin);

    assert_eq!(
        fixture.roles.writes(),
        vec![("target-subject".to_string(), Role::Admin)]
    );

    let events = fixture.audit.events();
    assert_eq!(
        events.len(),
        2,
        "the intent and the outcome are both recorded"
    );
    assert_eq!(events[0].0.action(), "identity.role.change.requested");
    assert_eq!(events[0].0.outcome(), AuditOutcome::Success);
    assert_eq!(
        events[0]
            .0
            .context()
            .get("previousRole")
            .map(String::as_str),
        Some("owner")
    );
    assert_eq!(
        events[0].0.context().get("newRole").map(String::as_str),
        Some("admin")
    );
    assert_eq!(events[1].0.action(), "identity.role.changed");
    assert_eq!(events[1].0.outcome(), AuditOutcome::Success);
    assert_eq!(
        fixture.observation.role_changes.lock().unwrap().as_slice(),
        ["success".to_string()]
    );
}

#[tokio::test]
async fn a_role_change_is_refused_for_a_non_admin_and_the_refusal_is_audited() {
    let fixture = fixture();
    let caller = UserId::generate();
    let target = common::active_user();
    let target_id = target.id();
    fixture.statuses.set(caller, UserStatus::Active);
    fixture.users.seed(target, "target-subject");

    let error = fixture
        .role_change
        .change(&owner(caller), target_id, Role::Admin, "reason", "corr")
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Forbidden(_)));
    assert!(fixture.roles.writes().is_empty());

    let events = fixture.audit.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].0.outcome(), AuditOutcome::Denied);
}

#[tokio::test]
async fn a_role_change_without_a_reason_writes_nothing() {
    let fixture = fixture();
    let admin_id = UserId::generate();
    let target = common::active_user();
    let target_id = target.id();
    fixture.statuses.set(admin_id, UserStatus::Active);
    fixture.users.seed(target, "target-subject");

    let error = fixture
        .role_change
        .change(&admin(admin_id), target_id, Role::Admin, "   ", "corr")
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Validation(_)));
    assert!(fixture.roles.writes().is_empty());
    assert!(fixture.audit.events().is_empty());
}

#[tokio::test]
async fn a_role_change_that_cannot_record_its_intent_aborts_before_the_provider_write() {
    let fixture = fixture();
    let admin_id = UserId::generate();
    let target = common::active_user();
    let target_id = target.id();
    fixture.statuses.set(admin_id, UserStatus::Active);
    fixture.users.seed(target, "target-subject");
    fixture
        .audit
        .fail_next(identity_application::ports::RepositoryError::Unavailable(
            "audit down".to_string(),
        ));

    let error = fixture
        .role_change
        .change(&admin(admin_id), target_id, Role::Admin, "reason", "corr")
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Unavailable(_)));
    assert!(fixture.roles.writes().is_empty());
}

#[tokio::test]
async fn every_role_change_audit_failure_is_a_503_not_a_validation_refusal() {
    // The audit adapter refuses a redaction-shaped value with a validation
    // error; on the privileged path that must still be "unavailable" (503), not
    // a 422 that would read as a client mistake.
    let fixture = fixture();
    let admin_id = UserId::generate();
    let target = common::active_user();
    let target_id = target.id();
    fixture.statuses.set(admin_id, UserStatus::Active);
    fixture.users.seed(target, "target-subject");
    fixture
        .audit
        .fail_next(identity_application::ports::RepositoryError::Validation(
            "the audit target field would carry a secret".to_string(),
        ));

    let error = fixture
        .role_change
        .change(&admin(admin_id), target_id, Role::Admin, "reason", "corr")
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Unavailable(_)), "got {error:?}");
    assert!(fixture.roles.writes().is_empty());
}

#[tokio::test]
async fn a_failed_provider_write_records_outcome_failed_and_surfaces_the_error() {
    let fixture = fixture();
    let admin_id = UserId::generate();
    let target = common::active_user();
    let target_id = target.id();
    fixture.statuses.set(admin_id, UserStatus::Active);
    fixture.users.seed(target, "target-subject");
    fixture
        .roles
        .fail_next(ExternalError::Refused("403".to_string()));

    let error = fixture
        .role_change
        .change(&admin(admin_id), target_id, Role::Admin, "reason", "corr")
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Unavailable(_)));

    let events = fixture.audit.events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].0.action(), "identity.role.changed");
    assert_eq!(events[1].0.outcome(), AuditOutcome::Failed);
    assert!(events[1].0.context().contains_key("failureCause"));
    assert_eq!(
        fixture.observation.role_changes.lock().unwrap().as_slice(),
        ["failed".to_string()]
    );
}

#[tokio::test]
async fn an_unknown_role_or_a_missing_target_is_refused() {
    let fixture = fixture();
    let admin_id = UserId::generate();
    fixture.statuses.set(admin_id, UserStatus::Active);

    let error = fixture
        .role_change
        .change(
            &admin(admin_id),
            UserId::generate(),
            Role::System,
            "reason",
            "corr",
        )
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Validation(_)));

    let error = fixture
        .role_change
        .change(
            &admin(admin_id),
            UserId::generate(),
            Role::Admin,
            "reason",
            "corr",
        )
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::NotFound(_)));
}
