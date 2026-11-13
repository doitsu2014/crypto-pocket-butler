//! Domain tests for the authorisation value objects (BR2.1–BR2.9, BR2.14).
//!
//! The role mapping, the operation classes, the denial reasons, and the
//! decision's status mapping are business rules, so they are tested here without
//! a database, a transport, or a fake.

use identity_domain::authorisation::{
    CallerIdentity, Decision, DenialReason, Operation, OperationClass, ResourceOwnership,
    ResourceScope, Role,
};
use identity_domain::UserId;

#[test]
fn the_role_claim_maps_admin_and_support_and_everything_else_to_owner() {
    assert_eq!(Role::from_claim(Some("admin")), Role::Admin);
    assert_eq!(Role::from_claim(Some("support")), Role::Support);
    assert_eq!(Role::from_claim(Some("owner")), Role::User);
    assert_eq!(Role::from_claim(Some("user")), Role::User);
    assert_eq!(Role::from_claim(Some("root")), Role::User);
    assert_eq!(Role::from_claim(Some("")), Role::User);
    assert_eq!(Role::from_claim(None), Role::User);
    // A claim can never grant the job-only system role (BR2.2).
    assert_eq!(Role::from_claim(Some("system")), Role::User);
}

#[test]
fn the_provider_role_value_is_owner_admin_support_and_never_system() {
    assert_eq!(Role::User.provider_value(), "owner");
    assert_eq!(Role::Admin.provider_value(), "admin");
    assert_eq!(Role::Support.provider_value(), "support");
    assert_eq!(Role::System.provider_value(), "system");
    assert!(Role::System.is_system());
    assert!(!Role::System.is_admin());
    assert!(!Role::System.is_support());
    // A stored user role never accepts the job-only system value.
    assert!(Role::parse("system").is_err());
}

#[test]
fn the_operation_class_names_are_the_contract_vocabulary() {
    assert_eq!(OperationClass::OwnerScoped.as_str(), "owner-scoped");
    assert_eq!(OperationClass::Collection.as_str(), "collection");
    assert_eq!(OperationClass::Administrative.as_str(), "administrative");
    assert_eq!(OperationClass::Confirmation.as_str(), "confirmation");
    assert_eq!(OperationClass::SupportMetadata.as_str(), "support-metadata");
    assert_eq!(OperationClass::Undeclared.as_str(), "undeclared");
    assert!(OperationClass::Administrative.is_administrative());
    assert!(!OperationClass::OwnerScoped.is_administrative());

    let operation = Operation::new("principal:read", OperationClass::OwnerScoped);
    assert_eq!(operation.name(), "principal:read");
    assert_eq!(operation.class(), OperationClass::OwnerScoped);
}

#[test]
fn a_row_ownership_carries_an_optional_owner_and_a_collection_carries_none() {
    let owner = UserId::generate();
    let row = ResourceOwnership::row("account", Some(owner));
    assert_eq!(row.scope(), ResourceScope::Resource);
    assert_eq!(row.resource_kind(), "account");
    assert_eq!(row.resource_owner_id(), Some(owner));

    let unowned = ResourceOwnership::row("account", None);
    assert_eq!(unowned.resource_owner_id(), None);

    let collection = ResourceOwnership::collection("transaction");
    assert_eq!(collection.scope(), ResourceScope::Collection);
    assert_eq!(collection.resource_owner_id(), None);
}

#[test]
fn a_user_identity_names_its_actor_and_a_system_identity_names_its_service_and_owner() {
    let user_id = UserId::generate();
    let user = CallerIdentity::user("subject-1", user_id, Role::Admin);
    assert_eq!(user.role(), Role::Admin);
    assert_eq!(user.user_id(), Some(user_id));
    assert_eq!(user.owner(), Some(user_id));
    assert_eq!(user.actor_label(), user_id.to_string());
    assert!(!user.is_system());

    // A job is bound to the one owner it iterates for (BR2.8).
    let iterated = UserId::generate();
    let system = CallerIdentity::system("reporting-worker", iterated);
    assert_eq!(system.role(), Role::System);
    assert_eq!(system.user_id(), None);
    assert_eq!(system.owner(), Some(iterated));
    assert_eq!(system.actor_label(), "reporting-worker");
    assert!(system.is_system());
}

#[test]
fn an_allow_is_200_and_may_carry_the_owner_filter() {
    let owner = UserId::generate();
    let decision = Decision::allow(Some(owner));
    assert!(decision.allowed());
    assert_eq!(decision.reason(), DenialReason::Allowed);
    assert_eq!(decision.reason().as_str(), "allowed");
    assert_eq!(decision.http_status(), 200);
    assert_eq!(decision.filter_owner_id(), Some(owner));

    assert_eq!(Decision::allow(None).filter_owner_id(), None);
}

#[test]
fn role_status_and_declared_denials_are_403_and_not_owner_is_404() {
    for reason in [
        DenialReason::NoRole,
        DenialReason::StatusDenied,
        DenialReason::NotDeclared,
    ] {
        let decision = Decision::deny(reason);
        assert!(!decision.allowed());
        assert_eq!(decision.http_status(), 403, "{}", reason.as_str());
        assert_eq!(decision.filter_owner_id(), None);
    }
    assert_eq!(DenialReason::NoRole.as_str(), "no_role");
    assert_eq!(DenialReason::StatusDenied.as_str(), "status_denied");
    assert_eq!(DenialReason::NotDeclared.as_str(), "not_declared");

    // A refusal never discloses whether the row exists (BR2.6).
    let not_owner = Decision::deny(DenialReason::NotOwner);
    assert_eq!(not_owner.http_status(), 404);
    assert_eq!(not_owner.reason().as_str(), "not_owner");
}

#[test]
fn audit_unavailable_is_the_only_503_a_decision_can_carry() {
    assert_eq!(DenialReason::AuditUnavailable.as_str(), "audit_unavailable");
    let decision = Decision::audit_unavailable();
    assert!(!decision.allowed());
    assert_eq!(decision.reason(), DenialReason::AuditUnavailable);
    assert_eq!(decision.http_status(), 503);

    // No other reason maps to 503; a status-source failure is an error, not a
    // decision.
    for reason in [
        DenialReason::Allowed,
        DenialReason::NoRole,
        DenialReason::StatusDenied,
        DenialReason::NotOwner,
        DenialReason::NotDeclared,
    ] {
        assert_ne!(reason.http_status(), 503, "{}", reason.as_str());
    }
}
