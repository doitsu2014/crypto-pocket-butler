//! Application use-case tests against port fakes — no database, no network.

mod common;

use std::sync::Arc;

use chrono::Duration;
use common::*;
use identity_application::common::AppError;
use identity_application::jurisdiction::{
    CheckJurisdiction, PublishEntry, PublishList, PublishRequest,
};
use identity_application::ports::{
    Clock, DeletionRepository, DeviceSessionRepository, ExportRepository, ExternalError,
    RepositoryError, SupportGrantRepository,
};
use identity_domain::claims::{AssuranceLevel, NextStep, Role};
use identity_domain::consent::{ConsentPurpose, ConsentSource};
use identity_domain::deletion::{
    DeletionRequest, DeletionRequestId, DeletionStatus, RetainedCategory,
};
use identity_domain::export::{Contributor, ExportStatus};
use identity_domain::jurisdiction::JurisdictionStatus;
use identity_domain::preferences::{BaseCurrency, Locale, Preferences, TimeZone};
use identity_domain::user::UserStatus;
use identity_domain::AuditOutcome;
use identity_domain::CountryCode;

fn preferences() -> Preferences {
    Preferences {
        locale: Locale::Vi,
        base_currency: BaseCurrency::parse("EUR").unwrap(),
        timezone: TimeZone::parse("Asia/Ho_Chi_Minh").unwrap(),
    }
}

/// A stored user in an explicit lifecycle state.
fn user_with_status(status: UserStatus) -> identity_domain::User {
    identity_domain::User::from_stored(
        identity_domain::UserId::generate(),
        identity_domain::claims::Role::User,
        status,
        Some("Test User".to_string()),
        false,
        Some("user@example.com".to_string()),
        false,
        true,
        Locale::En,
        BaseCurrency::parse("USD").unwrap(),
        TimeZone::parse("UTC").unwrap(),
        fixed_now(),
        fixed_now(),
        None,
    )
}

async fn provision(harness: &Harness) -> identity_domain::User {
    let view = harness
        .principal
        .resolve(claims(), "corr-provision")
        .await
        .expect("a confirmed user is provisioned");
    harness.users.get(view.user_id()).expect("the user exists")
}

#[tokio::test]
async fn c3_contact_returns_the_verified_address() {
    let harness = harness();
    let view = harness.principal.resolve(claims(), "corr-1").await.unwrap();
    let contact = harness.principal.contact(view.user_id()).await.unwrap();
    assert!(contact.verified);
    assert_eq!(contact.email.as_deref(), Some("user@example.com"));
}

#[tokio::test]
async fn an_unconfirmed_contact_answers_not_verified() {
    let harness = harness();
    let mut unconfirmed = claims();
    unconfirmed.email_confirmed = false;
    let view = harness
        .principal
        .resolve(unconfirmed, "corr-1")
        .await
        .unwrap();
    let contact = harness.principal.contact(view.user_id()).await.unwrap();
    assert!(!contact.verified);
    assert!(contact.email.is_none());
}

#[tokio::test]
async fn a_deleted_users_contact_is_not_found() {
    let harness = harness();
    let mut user = identity_domain::User::provision(
        identity_domain::UserId::generate(),
        &claims(),
        fixed_now(),
    );
    user.anonymise(fixed_now());
    harness.users.seed(user.clone(), USER_SUBJECT);
    let error = harness
        .principal
        .contact(user.id())
        .await
        .expect_err("a deleted user has no contact");
    assert!(matches!(error, AppError::NotFound("user")));
}

#[tokio::test]
async fn jit_provisioning_creates_a_user_a_key_and_an_audit_event() {
    let harness = harness();
    let mut unconfirmed = claims();
    unconfirmed.email_confirmed = false;

    let view = harness
        .principal
        .resolve(unconfirmed, "corr-1")
        .await
        .expect("the first authenticated request provisions a user");

    assert_eq!(view.status(), UserStatus::Pending);
    assert_eq!(view.next_step(), NextStep::VerifyEmail);
    assert_eq!(harness.users.len(), 1);
    assert!(
        harness.keys.key_for(view.user_id()).is_some(),
        "the first request creates the shredding-key reference"
    );
    assert!(harness
        .audit
        .actions()
        .iter()
        .any(|action| action == "identity.user.provisioned"));
}

#[tokio::test]
async fn a_second_resolution_refreshes_the_role_and_keeps_the_stored_preferences() {
    let harness = harness();
    let view = harness.principal.resolve(claims(), "corr-1").await.unwrap();
    let user = harness.users.get(view.user_id()).unwrap();
    let caller = caller(user, claims());
    harness
        .preferences
        .update(&caller, view.user_id(), preferences())
        .await
        .expect("the owner changes their preferences");

    let admin = identity_domain::claims::VerifiedClaims {
        role: identity_domain::claims::Role::Admin,
        ..claims()
    };
    let view = harness.principal.resolve(admin, "corr-2").await.unwrap();
    let stored = harness.users.get(view.user_id()).unwrap();
    assert_eq!(view.role(), identity_domain::claims::Role::Admin);
    assert_eq!(stored.role(), identity_domain::claims::Role::Admin);
    assert_eq!(stored.locale(), Locale::Vi);
    assert_eq!(stored.base_currency(), BaseCurrency::parse("EUR").unwrap());
}

#[tokio::test]
async fn a_deleted_user_is_refused_and_the_denial_is_audited() {
    let harness = harness();
    let mut user = identity_domain::User::provision(
        identity_domain::UserId::generate(),
        &claims(),
        fixed_now(),
    );
    user.anonymise(fixed_now());
    harness.users.seed(user, USER_SUBJECT);

    let error = harness
        .principal
        .resolve(claims(), "corr-1")
        .await
        .expect_err("a deleted user cannot resolve a principal");
    assert!(matches!(error, AppError::Forbidden(_)));
    // U5: an erased subject is audited under its own action, distinct from a
    // merely disabled account (BR5.8).
    assert!(harness
        .audit
        .actions()
        .iter()
        .any(|action| action == "identity.principal.erased"));
}

#[tokio::test]
async fn a_pending_user_is_told_to_verify_or_enrol_mfa_and_a_verified_user_becomes_active() {
    let harness = harness();
    let mut unconfirmed = claims();
    unconfirmed.email_confirmed = false;
    let pending = harness
        .principal
        .resolve(unconfirmed, "corr-1")
        .await
        .unwrap();
    assert_eq!(pending.status(), UserStatus::Pending);
    assert_eq!(pending.next_step(), NextStep::VerifyEmail);

    let mut credential_holder = claims();
    credential_holder.has_exchange_credentials = true;
    credential_holder.assurance = AssuranceLevel::Aal1;
    let gated = harness
        .principal
        .resolve(credential_holder, "corr-2")
        .await
        .unwrap();
    assert_eq!(gated.status(), UserStatus::Pending);
    assert_eq!(gated.next_step(), NextStep::EnrolMfa);

    let mut verified = claims();
    verified.has_exchange_credentials = true;
    verified.assurance = AssuranceLevel::Aal2;
    let active = harness.principal.resolve(verified, "corr-3").await.unwrap();
    assert_eq!(active.status(), UserStatus::Active);
    assert_eq!(active.next_step(), NextStep::None);
}

#[tokio::test]
async fn consent_is_deny_by_default_and_reads_the_newest_decision() {
    let harness = harness();
    let user = provision(&harness).await;
    let caller = caller(user, claims());

    let view = harness
        .consent_service
        .get(&caller, caller.user.id())
        .await
        .unwrap();
    assert!(!view.analytics());
    assert!(!view.marketing());

    harness
        .consent_service
        .append(
            &caller,
            ConsentPurpose::Analytics,
            true,
            ConsentSource::Onboarding,
        )
        .await
        .unwrap();
    harness
        .consent_service
        .append(
            &caller,
            ConsentPurpose::Analytics,
            false,
            ConsentSource::Settings,
        )
        .await
        .unwrap();

    let view = harness
        .consent_service
        .get(&caller, caller.user.id())
        .await
        .unwrap();
    assert!(!view.analytics(), "the newest decision wins");
    assert!(!view.marketing());
    assert_eq!(harness.consent.records.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn a_non_owner_cannot_read_or_change_another_users_records() {
    let harness = harness();
    let mut other = identity_domain::User::provision(
        identity_domain::UserId::generate(),
        &claims(),
        fixed_now(),
    );
    other.apply_claims(&claims(), fixed_now()).unwrap();
    harness.users.seed(other.clone(), OTHER_SUBJECT);
    let intruder = caller(other, other_claims());
    let victim = identity_domain::User::provision(
        identity_domain::UserId::generate(),
        &claims(),
        fixed_now(),
    );
    harness.users.seed(victim.clone(), USER_SUBJECT);

    assert!(matches!(
        harness
            .consent_service
            .get(&intruder, victim.id())
            .await
            .expect_err("consent is owner-only"),
        AppError::Forbidden(_)
    ));
    assert!(matches!(
        harness
            .preferences
            .update(&intruder, victim.id(), preferences())
            .await
            .expect_err("preferences are owner-only"),
        AppError::Forbidden(_)
    ));
}

#[tokio::test]
async fn a_preference_change_is_applied_and_persisted() {
    let harness = harness();
    let user = provision(&harness).await;
    let caller = caller(user, claims());
    let updated = harness
        .preferences
        .update(&caller, caller.user.id(), preferences())
        .await
        .unwrap();
    assert_eq!(updated.locale(), Locale::Vi);
    assert_eq!(updated.base_currency(), BaseCurrency::parse("EUR").unwrap());
    let stored = harness.users.get(updated.id()).unwrap();
    assert_eq!(stored.timezone(), preferences().timezone);
    assert!(harness
        .audit
        .actions()
        .iter()
        .any(|action| action == "identity.preferences.changed"));
}

#[tokio::test]
async fn the_session_list_is_owner_only_and_revocation_calls_supabase_auth() {
    let harness = harness();
    let user = provision(&harness).await;
    let caller = caller(user, claims());

    let listed = harness
        .sessions_service
        .list(&caller, caller.user.id())
        .await
        .unwrap();
    assert_eq!(listed.len(), 1, "principal resolution records the session");

    let error = harness
        .sessions_service
        .revoke(&caller, caller.user.id(), listed[0].id(), false)
        .await
        .expect_err("revoking the current session needs confirmation");
    assert!(matches!(error, AppError::Conflict(_)));
    assert!(harness.auth_admin.revoked.lock().unwrap().is_empty());

    harness
        .sessions_service
        .revoke(&caller, caller.user.id(), listed[0].id(), true)
        .await
        .expect("a confirmed current-session revoke");
    assert_eq!(
        harness.auth_admin.revoked.lock().unwrap().as_slice(),
        &[(USER_SUBJECT.to_string(), "auth-session-1".to_string())]
    );
    let stored = harness
        .sessions
        .find(listed[0].id())
        .await
        .unwrap()
        .unwrap();
    assert!(stored.is_revoked());
    assert!(harness
        .audit
        .actions()
        .iter()
        .any(|action| action == "identity.session.revoked"));
}

#[tokio::test]
async fn a_supabase_auth_failure_refuses_the_revocation_and_keeps_the_session() {
    let harness = harness();
    let user = provision(&harness).await;
    let caller = caller(user, claims());
    let listed = harness
        .sessions_service
        .list(&caller, caller.user.id())
        .await
        .unwrap();
    *harness.auth_admin.failure.lock().unwrap() = Some(
        identity_application::ports::ExternalError::Unavailable("auth down".to_string()),
    );

    let error = harness
        .sessions_service
        .revoke(&caller, caller.user.id(), listed[0].id(), true)
        .await
        .expect_err("an unavailable Auth admin refuses the revoke");
    assert!(matches!(error, AppError::Unavailable(_)));
    let stored = harness
        .sessions
        .find(listed[0].id())
        .await
        .unwrap()
        .unwrap();
    assert!(!stored.is_revoked());
}

#[tokio::test]
async fn a_support_grant_needs_the_support_role_and_a_distinct_admin_approval() {
    let harness = harness();
    let target = provision(&harness).await;
    let support_view = harness
        .principal
        .resolve(support_claims(), "corr-support")
        .await
        .unwrap();
    let support_user = harness.users.get(support_view.user_id()).unwrap();
    let support = caller(support_user, support_claims());

    let error = harness
        .support
        .request(
            &caller(target.clone(), claims()),
            target.id(),
            "investigate sync",
        )
        .await
        .expect_err("a regular user cannot request support access");
    assert!(matches!(error, AppError::Forbidden(_)));

    let grant = harness
        .support
        .request(&support, target.id(), "investigate a reported sync failure")
        .await
        .expect("a support agent requests a grant");
    assert_eq!(
        grant.status(),
        identity_domain::support::SupportGrantStatus::PendingApproval
    );

    let admin_view = harness
        .principal
        .resolve(admin_claims(), "corr-admin")
        .await
        .unwrap();
    let admin_user = harness.users.get(admin_view.user_id()).unwrap();
    let admin = caller(admin_user, admin_claims());

    let approved = harness.support.approve(&admin, grant.id()).await.unwrap();
    assert_eq!(
        approved.status(),
        identity_domain::support::SupportGrantStatus::Active
    );
    assert_eq!(approved.approved_by(), Some(admin.user.id()));
    assert_eq!(
        approved.expires_at(),
        Some(fixed_now() + Duration::minutes(60))
    );

    let revoked = harness.support.revoke(&admin, grant.id()).await.unwrap();
    assert_eq!(
        revoked.status(),
        identity_domain::support::SupportGrantStatus::Revoked
    );
}

#[tokio::test]
async fn a_non_admin_cannot_approve_or_revoke_and_an_expired_grant_is_refused() {
    let harness = harness();
    let target = provision(&harness).await;
    let support_view = harness
        .principal
        .resolve(support_claims(), "corr-support")
        .await
        .unwrap();
    let support_user = harness.users.get(support_view.user_id()).unwrap();
    let support = caller(support_user, support_claims());
    let grant = harness
        .support
        .request(&support, target.id(), "investigate a reported sync failure")
        .await
        .unwrap();

    assert!(matches!(
        harness
            .support
            .approve(&support, grant.id())
            .await
            .expect_err("a support agent cannot approve"),
        AppError::Forbidden(_)
    ));

    let admin_view = harness
        .principal
        .resolve(admin_claims(), "corr-admin")
        .await
        .unwrap();
    let admin_user = harness.users.get(admin_view.user_id()).unwrap();
    let admin = caller(admin_user, admin_claims());
    harness.support.approve(&admin, grant.id()).await.unwrap();

    harness.clock.set(fixed_now() + Duration::minutes(61));
    let error = harness
        .support
        .metadata(&support, grant.id())
        .await
        .expect_err("an expired grant serves nothing");
    assert!(matches!(error, AppError::Forbidden(_)));
    let stored = harness
        .grants
        .find_by_id(grant.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        stored.status(),
        identity_domain::support::SupportGrantStatus::Expired
    );
}

#[tokio::test]
async fn a_support_read_is_audited_before_the_answer_and_an_audit_failure_refuses_it() {
    let harness = harness();
    let target = provision(&harness).await;
    let support_view = harness
        .principal
        .resolve(support_claims(), "corr-support")
        .await
        .unwrap();
    let support_user = harness.users.get(support_view.user_id()).unwrap();
    let support = caller(support_user, support_claims());
    let grant = harness
        .support
        .request(&support, target.id(), "investigate a reported sync failure")
        .await
        .unwrap();
    let admin_view = harness
        .principal
        .resolve(admin_claims(), "corr-admin")
        .await
        .unwrap();
    let admin_user = harness.users.get(admin_view.user_id()).unwrap();
    let admin = caller(admin_user, admin_claims());
    harness.support.approve(&admin, grant.id()).await.unwrap();

    harness
        .support
        .metadata(&support, grant.id())
        .await
        .expect("an active grant serves the read");
    assert_eq!(
        harness.support_observation.served.lock().unwrap().len(),
        1,
        "the served event is written before the answer"
    );

    *harness.support_observation.failure.lock().unwrap() = Some(
        identity_application::ports::RepositoryError::Unavailable("audit down".to_string()),
    );
    let error = harness
        .support
        .metadata(&support, grant.id())
        .await
        .expect_err("an unrecorded read is refused");
    assert!(matches!(error, AppError::Unavailable(_)));
}

#[tokio::test]
async fn an_export_request_wakes_the_worker_and_the_worker_assembles_a_ready_archive() {
    let harness = harness();
    let user = provision(&harness).await;
    let caller = caller(user, claims());

    let export = harness
        .export_service
        .request(&caller)
        .await
        .expect("an export is requested");
    assert_eq!(export.status(), ExportStatus::Pending);
    assert_eq!(*harness.wake.calls.lock().unwrap(), 1);

    let processed = harness
        .export_worker
        .run_once("worker-1", fixed_now())
        .await
        .expect("the worker runs");
    assert_eq!(processed, 1);

    let stored = harness
        .exports
        .find_by_id(export.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.status(), ExportStatus::Ready);
    assert_eq!(stored.progress(), 100);
    assert!(stored.file_reference().is_some());
    assert_eq!(harness.store.objects.lock().unwrap().len(), 1);

    let status = harness
        .export_service
        .status(&caller, export.id())
        .await
        .expect("the owner reads the status");
    assert!(status.signed_download().is_some());
    assert!(harness
        .audit
        .actions()
        .iter()
        .any(|action| action == "identity.export.downloaded"));
}

#[tokio::test]
async fn a_failed_contribution_fails_the_export_and_never_stores_an_archive() {
    let harness = harness();
    let user = provision(&harness).await;
    let caller = caller(user, claims());
    *harness.contributors.fail_export.lock().unwrap() = Some(Contributor::Ledger);

    let export = harness.export_service.request(&caller).await.unwrap();
    harness
        .export_worker
        .run_once("worker-1", fixed_now())
        .await
        .unwrap();

    let stored = harness
        .exports
        .find_by_id(export.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.status(), ExportStatus::Failed);
    assert!(stored.failure_reason().is_some());
    assert!(harness.store.objects.lock().unwrap().is_empty());
    assert!(harness
        .audit
        .actions()
        .iter()
        .any(|action| action == "identity.export.failed"));
}

#[tokio::test]
async fn a_credential_shaped_contribution_is_refused_and_never_presented_as_ready() {
    let harness = harness();
    let user = provision(&harness).await;
    let caller = caller(user, claims());
    harness.contributors.with_export(
        Contributor::Account,
        serde_json::json!({ "accounts": [{ "apiKey": "fake-value" }] }),
    );

    let export = harness.export_service.request(&caller).await.unwrap();
    harness
        .export_worker
        .run_once("worker-1", fixed_now())
        .await
        .unwrap();

    let stored = harness
        .exports
        .find_by_id(export.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.status(), ExportStatus::Failed);
    assert!(stored
        .failure_reason()
        .unwrap_or_default()
        .contains("credential"));
    assert!(harness.store.objects.lock().unwrap().is_empty());
}

#[tokio::test]
async fn an_export_is_owner_only_and_its_archive_is_deleted_after_the_window() {
    let harness = harness();
    let user = provision(&harness).await;
    let owner = common::caller(user, claims());
    let export = harness.export_service.request(&owner).await.unwrap();
    harness
        .export_worker
        .run_once("worker-1", fixed_now())
        .await
        .unwrap();
    let key = harness
        .exports
        .find_by_id(export.id())
        .await
        .unwrap()
        .unwrap()
        .file_reference()
        .unwrap()
        .to_string();

    let mut outsider = identity_domain::User::provision(
        identity_domain::UserId::generate(),
        &claims(),
        fixed_now(),
    );
    outsider.apply_claims(&claims(), fixed_now()).unwrap();
    harness.users.seed(outsider.clone(), OTHER_SUBJECT);
    let other_caller = common::caller(outsider, other_claims());
    assert!(matches!(
        harness
            .export_service
            .status(&other_caller, export.id())
            .await
            .expect_err("only the owner may read an export"),
        AppError::Forbidden(_)
    ));

    harness.clock.set(fixed_now() + Duration::hours(25));
    harness
        .export_worker
        .run_once("worker-1", fixed_now() + Duration::hours(25))
        .await
        .unwrap();
    assert!(harness.store.deleted.lock().unwrap().contains(&key));
    let status = harness.export_service.status(&owner, export.id()).await;
    match status {
        Ok(view) => {
            assert_eq!(view.export().status(), ExportStatus::Expired);
            assert!(view.signed_download().is_none());
        }
        Err(error) => panic!("the owner reads the expired status: {error:?}"),
    }
}

#[tokio::test]
async fn a_deletion_request_reuses_the_active_request_and_the_worker_runs_every_step_in_order() {
    let harness = harness();
    let user = provision(&harness).await;
    let caller = caller(user.clone(), claims());
    harness
        .consent_service
        .append(
            &caller,
            ConsentPurpose::Analytics,
            true,
            ConsentSource::Onboarding,
        )
        .await
        .unwrap();
    harness.contributors.retained.lock().unwrap().insert(
        "ledger".to_string(),
        vec![RetainedCategory::new("financial_records", "two-year legal retention").unwrap()],
    );

    let deletion = harness
        .account_lifecycle
        .request_deletion(&caller)
        .await
        .expect("a deletion is requested");
    let repeated = harness.account_lifecycle.request_deletion(&caller).await;
    assert!(
        matches!(repeated, Err(AppError::Conflict(_))),
        "a second open request is refused with 409 (W2)"
    );
    assert_eq!(
        *harness.wake.calls.lock().unwrap(),
        1,
        "a refused second request does not wake the worker"
    );

    harness
        .deletion_worker
        .run_once("worker-1", fixed_now() + Duration::days(31))
        .await
        .expect("the worker completes the deletion after the window");

    assert_eq!(
        harness
            .contributors
            .anonymise_calls
            .lock()
            .unwrap()
            .as_slice(),
        &[
            Contributor::Account,
            Contributor::Ledger,
            Contributor::Portfolio
        ]
    );
    assert_eq!(
        harness.auth_admin.deleted.lock().unwrap().as_slice(),
        &[USER_SUBJECT.to_string()]
    );
    let key = harness
        .keys
        .key_for(user.id())
        .expect("the key reference exists");
    assert_eq!(key.status(), identity_domain::ShreddingKeyStatus::Destroyed);
    let stored = harness.users.get(user.id()).unwrap();
    assert_eq!(stored.status(), UserStatus::Deleted);
    assert_eq!(
        harness.erasure.erased.lock().unwrap().as_slice(),
        &[user.id()],
        "the consent erasure is routed through the identity_retention port"
    );

    let done = harness
        .deletions
        .find_by_id(deletion.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(done.status(), DeletionStatus::Completed);
    assert_eq!(done.retained_categories().len(), 1);
    assert!(harness
        .audit
        .actions()
        .iter()
        .any(|action| action == "identity.deletion.completed"));
}

#[tokio::test]
async fn a_failed_deletion_step_is_retryable_without_repeating_completed_destruction() {
    let harness = harness();
    let user = provision(&harness).await;
    let caller = caller(user, claims());
    *harness.contributors.fail_anonymise.lock().unwrap() = Some(Contributor::Ledger);

    let deletion = harness
        .account_lifecycle
        .request_deletion(&caller)
        .await
        .unwrap();
    harness
        .deletion_worker
        .run_once("worker-1", fixed_now() + Duration::days(31))
        .await
        .unwrap();
    let failed = harness
        .deletions
        .find_by_id(deletion.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(failed.status(), DeletionStatus::Failed);
    assert_eq!(failed.attempt_count(), 1);
    assert_eq!(
        harness.auth_admin.deleted.lock().unwrap().len(),
        1,
        "the Auth identity leaves first, so a later step's failure is retryable"
    );

    *harness.contributors.fail_anonymise.lock().unwrap() = None;
    harness
        .deletion_worker
        .run_once(
            "worker-2",
            fixed_now() + Duration::days(31) + Duration::seconds(61),
        )
        .await
        .unwrap();

    let calls = harness.contributors.anonymise_calls.lock().unwrap().clone();
    assert_eq!(
        calls.iter().filter(|c| **c == Contributor::Account).count(),
        1,
        "a completed anonymise is not repeated"
    );
    assert_eq!(
        calls.iter().filter(|c| **c == Contributor::Ledger).count(),
        2,
        "the failed step is retried"
    );
    assert_eq!(
        harness.auth_admin.deleted.lock().unwrap().len(),
        1,
        "the Auth delete is recorded in the progress and not repeated"
    );
    let completed = harness
        .deletions
        .find_by_id(deletion.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(completed.status(), DeletionStatus::Completed);
}

#[tokio::test]
async fn the_signup_gate_issues_a_token_for_an_allowed_adult_and_refuses_blocked_declarations() {
    let harness = harness();
    harness.rules.seed(identity_domain::JurisdictionRule {
        code: CountryCode::parse("VN").unwrap(),
        allowed: true,
        minimum_age: 18,
        disclosures_required: true,
        updated_at: fixed_now(),
    });

    let disclosures = harness
        .signup_gate
        .evaluate("VN", 21, false, "corr-1")
        .await
        .expect("an adult from a supported market reaches the disclosures");
    assert!(disclosures.disclosures_required());
    assert!(disclosures.gate_token().is_none());

    let allowed = harness
        .signup_gate
        .evaluate("VN", 21, true, "corr-2")
        .await
        .expect("an acknowledged declaration is allowed");
    assert!(!allowed.disclosures_required());
    assert!(allowed.gate_token().is_some());
    assert_eq!(
        harness.gate.issued.lock().unwrap().as_slice(),
        &["VN".to_string()]
    );

    assert!(matches!(
        harness
            .signup_gate
            .evaluate("US", 21, true, "corr-3")
            .await
            .expect_err("an unknown jurisdiction is blocked"),
        AppError::Forbidden(_)
    ));
    assert!(matches!(
        harness
            .signup_gate
            .evaluate("VN", 17, true, "corr-4")
            .await
            .expect_err("a below-minimum age is denied"),
        AppError::Forbidden(_)
    ));
}

// ---------------------------------------------------------------------------
// U6 — the runtime jurisdiction gate (BR1.1–BR1.9)
// ---------------------------------------------------------------------------

fn jurisdiction_entry(
    code: &str,
    status: JurisdictionStatus,
) -> identity_domain::JurisdictionListEntry {
    identity_domain::JurisdictionListEntry {
        code: CountryCode::parse(code).expect("a fixture country code is valid"),
        status,
        source: "compliance".to_string(),
        updated_at: fixed_now(),
    }
}

fn jurisdiction_list() -> identity_domain::JurisdictionListVersion {
    identity_domain::JurisdictionListVersion::new(
        "2026-11-10.1",
        fixed_now(),
        vec![
            jurisdiction_entry("VN", JurisdictionStatus::Allowed),
            jurisdiction_entry("US", JurisdictionStatus::Allowed),
            jurisdiction_entry("KP", JurisdictionStatus::Restricted),
        ],
    )
    .expect("a complete list is valid")
}

fn jurisdiction_service() -> (
    Arc<common::fakes::FakeListStore>,
    Arc<common::fakes::FakeGeoLocator>,
    Arc<common::fakes::FakeDecisionAudit>,
    CheckJurisdiction,
) {
    let lists = Arc::new(common::fakes::FakeListStore::default());
    lists.seed(jurisdiction_list());
    let geo = Arc::new(common::fakes::FakeGeoLocator::default());
    let audit = Arc::new(common::fakes::FakeDecisionAudit::default());
    let clock: Arc<dyn Clock> = Arc::new(common::fakes::MutableClock::at(fixed_now()));
    let check = CheckJurisdiction::new(lists.clone(), geo.clone(), audit.clone(), clock);
    (lists, geo, audit, check)
}

#[tokio::test]
async fn jurisdiction_a_self_declared_restricted_country_denies_and_is_audited() {
    let (_lists, geo, audit, check) = jurisdiction_service();
    geo.set("192.0.2.10", "VN");

    let outcome = check
        .check("KP", "192.0.2.10", "corr-1")
        .await
        .expect("a restricted declaration returns a decision, not an error");
    assert!(!outcome.decision().is_allowed());
    assert_eq!(
        outcome.decision().restricted_by(),
        Some(identity_domain::RestrictionSource::SelfDeclared)
    );
    assert_eq!(outcome.decision().list_version(), "2026-11-10.1");
    assert!(!outcome.evidence_id().is_empty());

    assert_eq!(audit.actions(), vec!["identity.jurisdiction.check"]);
    let record = &audit.records.lock().unwrap()[0].0;
    assert_eq!(record.target(), "KP");
    assert_eq!(
        record.context().get("listVersion").map(String::as_str),
        Some("2026-11-10.1")
    );
    assert!(
        !record.context().values().any(|value| value == "192.0.2.10"),
        "the raw IP is never written to evidence"
    );
}

#[tokio::test]
async fn jurisdiction_an_ip_restricted_country_denies_and_the_stricter_signal_wins() {
    let (_lists, geo, audit, check) = jurisdiction_service();
    geo.set("192.0.2.20", "KP");

    let ip_only = check
        .check("VN", "192.0.2.20", "corr-2")
        .await
        .expect("a restricted IP returns a decision");
    assert!(!ip_only.decision().is_allowed());
    assert_eq!(
        ip_only.decision().restricted_by(),
        Some(identity_domain::RestrictionSource::Ip)
    );

    let both = check
        .check("KP", "192.0.2.20", "corr-3")
        .await
        .expect("both restricted signals return a decision");
    assert_eq!(
        both.decision().restricted_by(),
        Some(identity_domain::RestrictionSource::Both)
    );

    assert_eq!(audit.actions().len(), 2, "every decision is evidence");
}

#[tokio::test]
async fn jurisdiction_both_allowed_signals_allow_and_the_decision_cites_the_list_version() {
    let (_lists, geo, audit, check) = jurisdiction_service();
    geo.set("192.0.2.10", "US");

    let outcome = check
        .check("VN", "192.0.2.10", "corr-4")
        .await
        .expect("two allowed signals are allowed");

    assert!(outcome.decision().is_allowed());
    assert_eq!(outcome.decision().restricted_by(), None);
    assert_eq!(outcome.decision().list_version(), "2026-11-10.1");
    assert_eq!(audit.actions(), vec!["identity.jurisdiction.check"]);
}

#[tokio::test]
async fn jurisdiction_a_country_absent_from_the_list_fails_closed() {
    let (_lists, geo, _audit, check) = jurisdiction_service();
    geo.set("192.0.2.30", "ZZ");
    geo.set("192.0.2.10", "VN");

    let ip_absent = check
        .check("VN", "192.0.2.30", "corr-5")
        .await
        .expect("an unlisted IP country is still a decision");
    assert!(!ip_absent.decision().is_allowed());

    let declared_absent = check
        .check("ZZ", "192.0.2.10", "corr-6")
        .await
        .expect("an unlisted declaration is still a decision");
    assert!(!declared_absent.decision().is_allowed());
    assert_eq!(
        declared_absent.decision().restricted_by(),
        Some(identity_domain::RestrictionSource::SelfDeclared)
    );
}

#[tokio::test]
async fn jurisdiction_an_invalid_country_or_ip_is_refused_before_the_provider_is_called() {
    let (_lists, geo, audit, check) = jurisdiction_service();

    let bad_country = check
        .check("vn", "192.0.2.10", "corr-7")
        .await
        .expect_err("a lowercase country code is invalid");
    assert!(matches!(bad_country, AppError::Validation(_)));

    let bad_ip = check
        .check("VN", "not-an-address", "corr-8")
        .await
        .expect_err("a non-IP address is invalid");
    assert!(matches!(bad_ip, AppError::Validation(_)));

    assert_eq!(
        geo.call_count(),
        0,
        "invalid input never reaches the provider"
    );
    assert_eq!(
        audit.actions().len(),
        0,
        "an invalid request is not a decision"
    );
}

#[tokio::test]
async fn jurisdiction_a_provider_failure_fails_closed_and_opens_the_circuit() {
    let (_lists, geo, audit, check) = jurisdiction_service();

    // One attempt plus one retry both fail: the check refuses.
    geo.fail_next(ExternalError::Unavailable("provider timeout".to_string()));
    geo.fail_next(ExternalError::Unavailable("provider timeout".to_string()));
    let error = check
        .check("VN", "192.0.2.10", "corr-9")
        .await
        .expect_err("a provider failure never allows");
    assert!(matches!(error, AppError::Unavailable(_)));
    assert_eq!(
        audit.actions().len(),
        0,
        "no decision is recorded on failure"
    );

    // Five consecutive failed checks open the circuit.
    for _ in 0..5 {
        geo.fail_next(ExternalError::Unavailable("provider timeout".to_string()));
        geo.fail_next(ExternalError::Unavailable("provider timeout".to_string()));
    }
    for _ in 0..5 {
        let _ = check.check("VN", "192.0.2.10", "corr-10").await;
    }
    let calls_before = geo.call_count();
    let error = check
        .check("VN", "192.0.2.10", "corr-11")
        .await
        .expect_err("the open circuit fails fast");
    assert!(matches!(error, AppError::Unavailable(_)));
    assert_eq!(
        geo.call_count(),
        calls_before,
        "the open circuit does not call the provider"
    );
}

#[tokio::test]
async fn jurisdiction_an_audit_failure_refuses_the_decision_and_the_retry_is_idempotent() {
    let (_lists, geo, audit, check) = jurisdiction_service();
    geo.set("192.0.2.10", "US");

    // The first attempt fails, the retry succeeds: one decision, one event.
    audit.fail_next(RepositoryError::Unavailable("audit timed out".to_string()));
    let outcome = check
        .check("VN", "192.0.2.10", "corr-12")
        .await
        .expect("a retried audit write completes the decision");
    assert!(outcome.decision().is_allowed());
    assert_eq!(audit.actions(), vec!["identity.jurisdiction.check"]);

    let keys = audit.attempted_keys();
    assert_eq!(keys.len(), 2);
    assert_eq!(keys[0], outcome.evidence_id());
    assert_eq!(keys[0], keys[1], "a retry reuses the idempotency key");

    // Both attempts fail: the decision is refused.
    audit.fail_next(RepositoryError::Unavailable("audit timed out".to_string()));
    audit.fail_next(RepositoryError::Unavailable("audit timed out".to_string()));
    let error = check
        .check("VN", "192.0.2.10", "corr-13")
        .await
        .expect_err("no decision without evidence");
    assert!(matches!(error, AppError::Unavailable(_)));
}

#[tokio::test]
async fn jurisdiction_publishing_a_list_validates_entries_and_refuses_a_checksum_mismatch() {
    let lists = Arc::new(common::fakes::FakeListStore::default());
    let audit = Arc::new(common::fakes::FakeDecisionAudit::default());
    let clock: Arc<dyn Clock> = Arc::new(common::fakes::MutableClock::at(fixed_now()));
    let publish = PublishList::new(lists.clone(), audit.clone(), clock);

    let published = publish
        .publish(
            PublishRequest {
                version: "2026-11-10.2".to_string(),
                entries: vec![
                    PublishEntry {
                        country_code: "VN".to_string(),
                        status: "allowed".to_string(),
                        source: "compliance".to_string(),
                    },
                    PublishEntry {
                        country_code: "KP".to_string(),
                        status: "restricted".to_string(),
                        source: "compliance".to_string(),
                    },
                ],
                expected_checksum: None,
            },
            "corr-14",
        )
        .await
        .expect("a complete list publishes");

    assert_eq!(published.version(), "2026-11-10.2");
    assert_eq!(published.entry_count(), 2);
    assert_eq!(published.checksum().len(), 64);
    assert_eq!(lists.published_versions(), vec!["2026-11-10.2"]);
    assert_eq!(audit.actions(), vec!["identity.jurisdiction.publish"]);

    let mismatch = publish
        .publish(
            PublishRequest {
                version: "2026-11-10.3".to_string(),
                entries: vec![PublishEntry {
                    country_code: "VN".to_string(),
                    status: "allowed".to_string(),
                    source: "compliance".to_string(),
                }],
                expected_checksum: Some("0".repeat(64)),
            },
            "corr-15",
        )
        .await
        .expect_err("a mismatched checksum refuses the publish");
    assert!(matches!(mismatch, AppError::Validation(_)));

    let invalid = publish
        .publish(
            PublishRequest {
                version: "2026-11-10.4".to_string(),
                entries: vec![PublishEntry {
                    country_code: "vn".to_string(),
                    status: "allowed".to_string(),
                    source: "compliance".to_string(),
                }],
                expected_checksum: None,
            },
            "corr-16",
        )
        .await
        .expect_err("an invalid country code refuses the publish");
    assert!(matches!(invalid, AppError::Validation(_)));

    assert_eq!(
        lists.published_versions(),
        vec!["2026-11-10.2"],
        "a refused publish leaves the active version untouched"
    );
}

#[tokio::test]
async fn user_registry_gate_denial_refuses_provisioning_and_stores_nothing() {
    let harness = harness();
    harness
        .gate_clearance
        .deny_next("the declared country is restricted");

    let error = harness
        .principal
        .resolve(claims(), "corr-gate")
        .await
        .expect_err("a denied provider is never provisioned");
    match error {
        AppError::Validation(fields) => {
            assert_eq!(fields[0].field, "jurisdiction");
        }
        other => panic!("expected a validation refusal, got {other:?}"),
    }
    assert_eq!(harness.users.len(), 0, "nothing is stored");
    assert!(
        harness
            .audit
            .actions()
            .iter()
            .any(|action| { action == "identity.user.provision_denied" }),
        "the denial is audited"
    );
}

#[tokio::test]
async fn user_registry_gate_unavailable_is_a_retryable_refusal() {
    let harness = harness();
    harness
        .gate_clearance
        .fail_next("the geolocation provider timed out");

    let error = harness
        .principal
        .resolve(claims(), "corr-gate")
        .await
        .expect_err("an unavailable gate fails closed");
    assert!(matches!(error, AppError::Unavailable(_)), "{error:?}");
    assert_eq!(harness.users.len(), 0);
}

#[tokio::test]
async fn user_registry_missing_declaration_is_a_validation_refusal() {
    let harness = harness();
    let mut undeclared = claims();
    undeclared.country_code = None;
    undeclared.client_ip = None;

    let error = harness
        .principal
        .resolve(undeclared, "corr-gate")
        .await
        .expect_err("provisioning needs a declaration and the observed address");
    match error {
        AppError::Validation(fields) => assert_eq!(fields[0].field, "jurisdiction"),
        other => panic!("expected a validation refusal, got {other:?}"),
    }
    assert_eq!(harness.users.len(), 0);
    assert!(harness.gate_clearance.calls().is_empty());
}

#[tokio::test]
async fn user_registry_disabled_and_pending_deletion_are_denied_and_audited() {
    for status in [UserStatus::Disabled, UserStatus::PendingDeletion] {
        let harness = harness();
        harness.users.seed(user_with_status(status), USER_SUBJECT);

        let error = harness
            .principal
            .resolve(claims(), "corr-status")
            .await
            .expect_err("a denied status cannot resolve a principal");
        assert!(matches!(error, AppError::Forbidden(_)), "{error:?}");
        assert!(harness
            .audit
            .actions()
            .iter()
            .any(|action| action == "identity.principal.denied"));
    }
}

#[tokio::test]
async fn user_registry_a_hash_collision_is_a_validation_refusal_and_is_audited() {
    let harness = harness();
    harness
        .users
        .seed(user_with_status(UserStatus::Active), USER_SUBJECT);
    harness.users.collide_next();

    let error = harness
        .principal
        .resolve(claims(), "corr-collision")
        .await
        .expect_err("a collision fails closed");
    match error {
        AppError::Validation(fields) => assert_eq!(fields[0].field, "subject"),
        other => panic!("expected a validation refusal, got {other:?}"),
    }
    assert!(harness
        .audit
        .actions()
        .iter()
        .any(|action| action == "identity.principal.collision"));
}

#[tokio::test]
async fn user_registry_admin_lookup_is_admin_only_exact_and_audited() {
    let harness = harness();
    let target = user_with_status(UserStatus::Active);
    harness.users.seed(target.clone(), USER_SUBJECT);

    let admin = caller(target.clone(), admin_claims());
    let found = harness
        .user_registry
        .by_email(&admin, "user@example.com")
        .await
        .expect("an admin finds the exact address");
    assert_eq!(found.user_id(), target.id());
    assert_eq!(found.display_name(), Some("Test User"));

    let non_admin = caller(target.clone(), claims());
    let error = harness
        .user_registry
        .by_email(&non_admin, "user@example.com")
        .await
        .expect_err("a regular user may not search");
    assert!(matches!(error, AppError::Forbidden(_)), "{error:?}");

    assert!(matches!(
        harness
            .user_registry
            .by_email(&admin, "missing@example.com")
            .await
            .expect_err("an unknown address is not found"),
        AppError::NotFound("user")
    ));
    assert!(matches!(
        harness
            .user_registry
            .by_email(&admin, "not-an-address")
            .await
            .expect_err("a malformed address is refused"),
        AppError::Validation(_)
    ));
    let actions = harness.audit.actions();
    assert!(actions
        .iter()
        .any(|action| action == "identity.admin.user_lookup"));
}

#[tokio::test]
async fn user_registry_the_admin_lookup_returns_the_role_the_provider_holds() {
    let harness = harness();
    let target = user_with_status(UserStatus::Active);
    harness.users.seed(target.clone(), USER_SUBJECT);
    harness.role_reader.set(USER_SUBJECT, Role::Support);

    let admin = caller(target.clone(), admin_claims());
    let view = harness
        .user_registry
        .by_email_admin(&admin, "user@example.com")
        .await
        .expect("an admin finds the exact address");
    assert_eq!(view.user_id(), target.id());
    assert_eq!(
        view.role(),
        Role::Support,
        "the provider role is authoritative"
    );
    assert_eq!(view.display_name(), Some("Test User"));
    assert_eq!(view.email(), Some("user@example.com"));
    assert_eq!(view.status(), UserStatus::Active);

    let lookup_events: Vec<_> = harness
        .audit
        .records
        .lock()
        .unwrap()
        .iter()
        .filter(|(record, _)| record.action() == "identity.admin.user_lookup")
        .map(|(record, _)| record.clone())
        .collect();
    assert_eq!(
        lookup_events.len(),
        1,
        "a successful lookup records exactly one event, after the role read"
    );
    assert_eq!(lookup_events[0].outcome(), AuditOutcome::Success);
}

#[tokio::test]
async fn user_registry_a_failed_role_read_is_a_503_and_is_audited_as_failed() {
    let harness = harness();
    let target = user_with_status(UserStatus::Active);
    harness.users.seed(target.clone(), USER_SUBJECT);
    harness
        .role_reader
        .fail_next(ExternalError::Unavailable("provider timed out".to_string()));

    let admin = caller(target.clone(), admin_claims());
    let error = harness
        .user_registry
        .by_email_admin(&admin, "user@example.com")
        .await
        .expect_err("no record with a guessed role");
    assert!(matches!(error, AppError::Unavailable(_)), "{error:?}");

    let records = harness.audit.records.lock().unwrap().clone();
    assert_eq!(records.len(), 1, "only the failed event is written");
    let (record, _) = &records[0];
    assert_eq!(record.action(), "identity.admin.user_lookup");
    assert_eq!(
        record.outcome(),
        AuditOutcome::Failed,
        "the trail never shows a successful lookup that answered 503"
    );
    assert_eq!(
        record.context().get("reason").map(String::as_str),
        Some("provider-unavailable")
    );
}

#[tokio::test]
async fn user_registry_the_widened_admin_lookup_is_admin_only_exact_and_audited() {
    let harness = harness();
    let target = user_with_status(UserStatus::Active);
    harness.users.seed(target.clone(), USER_SUBJECT);
    harness.role_reader.set(USER_SUBJECT, Role::Admin);

    let non_admin = caller(target.clone(), support_claims());
    let error = harness
        .user_registry
        .by_email_admin(&non_admin, "user@example.com")
        .await
        .expect_err("a support operator may not search");
    assert!(matches!(error, AppError::Forbidden(_)), "{error:?}");

    let admin = caller(target.clone(), admin_claims());
    assert!(matches!(
        harness
            .user_registry
            .by_email_admin(&admin, "missing@example.com")
            .await
            .expect_err("an unknown address is not found"),
        AppError::NotFound("user")
    ));
    assert!(matches!(
        harness
            .user_registry
            .by_email_admin(&admin, "not-an-address")
            .await
            .expect_err("a malformed address is refused"),
        AppError::Validation(_)
    ));
    let actions = harness.audit.actions();
    assert!(actions
        .iter()
        .any(|action| action == "identity.admin.user_lookup"));
}

#[tokio::test]
async fn user_registry_the_owner_read_keeps_its_shape_and_does_not_read_the_provider() {
    let harness = harness();
    let target = user_with_status(UserStatus::Active);
    harness.users.seed(target.clone(), USER_SUBJECT);
    harness
        .role_reader
        .fail_next(ExternalError::Unavailable("provider down".to_string()));

    let admin = caller(target.clone(), admin_claims());
    // C3's shared record read is unchanged: it answers from the stored record
    // and never touches the provider, so a provider outage cannot break it.
    let view = harness
        .user_registry
        .by_email(&admin, "user@example.com")
        .await
        .expect("the owner read is provider-free");
    assert_eq!(view.user_id(), target.id());
}

#[tokio::test]
async fn user_registry_the_owner_or_an_admin_may_read_a_record() {
    let harness = harness();
    let target = user_with_status(UserStatus::Active);
    harness.users.seed(target.clone(), USER_SUBJECT);

    let owner = caller(target.clone(), claims());
    assert_eq!(
        harness
            .user_registry
            .by_id(&owner, target.id())
            .await
            .unwrap()
            .user_id(),
        target.id()
    );

    let other = caller(user_with_status(UserStatus::Active), other_claims());
    assert!(matches!(
        harness
            .user_registry
            .by_id(&other, target.id())
            .await
            .expect_err("another user may not read the record"),
        AppError::Forbidden(_)
    ));

    let admin = caller(target.clone(), admin_claims());
    assert_eq!(
        harness
            .user_registry
            .by_id(&admin, target.id())
            .await
            .unwrap()
            .user_id(),
        target.id()
    );
}

#[tokio::test]
async fn user_registry_a_role_refresh_does_not_emit_a_role_changed_event() {
    let harness = harness();
    harness
        .principal
        .resolve(claims(), "corr-role")
        .await
        .unwrap();
    let mut promoted = claims();
    promoted.role = identity_domain::claims::Role::Admin;
    harness
        .principal
        .resolve(promoted, "corr-role")
        .await
        .unwrap();

    assert!(
        !harness
            .audit
            .actions()
            .iter()
            .any(|action| action == "identity.role.changed"),
        "role changes belong to the authorisation unit, not the registry"
    );
}

// ── U7: the settings use cases (P1, W1–W4, D0) ─────────────────────────────

#[tokio::test]
async fn p1_the_profile_view_carries_both_names_flags_and_the_verified_email() {
    let harness = harness();
    let user = provision(&harness).await;
    let caller = caller(user.clone(), claims());

    let view = harness.profile.view(&caller);
    assert_eq!(view.user_id(), user.id());
    assert_eq!(view.display_name(), Some("Test User"));
    assert!(!view.display_name_overridden());
    assert_eq!(
        view.provider_display_name(),
        Some("Test User"),
        "the provider name travels in the claim, never in a second column"
    );
    assert_eq!(view.email(), Some("user@example.com"));
    assert!(view.email_verified());
    assert_eq!(view.status(), UserStatus::Active);
    assert!(
        !harness
            .audit
            .actions()
            .iter()
            .any(|action| action == "identity.user.profile.updated"),
        "a plain profile read writes no audit event"
    );
}

#[tokio::test]
async fn w1_a_display_name_save_trims_stores_the_override_and_audits_it() {
    let harness = harness();
    let user = provision(&harness).await;
    let caller = caller(user.clone(), claims());

    let view = harness
        .profile
        .update_display_name(&caller, "  Mai Anh  ")
        .await
        .expect("a valid name is stored");

    assert_eq!(view.user_id(), user.id());
    assert_eq!(view.display_name(), Some("Mai Anh"));
    assert!(view.display_name_overridden());
    assert_eq!(
        view.provider_display_name(),
        Some("Test User"),
        "the provider name is still offered after an override"
    );
    assert_eq!(view.email(), Some("user@example.com"));
    assert_eq!(view.status(), UserStatus::Active);

    let stored = harness.users.get(user.id()).unwrap();
    assert_eq!(stored.display_name(), Some("Mai Anh"));
    assert!(stored.display_name_overridden());

    let records = harness.audit.records.lock().unwrap();
    let events: Vec<_> = records
        .iter()
        .filter(|(record, _)| record.action() == "identity.user.profile.updated")
        .collect();
    assert_eq!(events.len(), 1, "one save writes exactly one audit event");
    let (record, correlation_id) = events[0];
    assert_eq!(correlation_id, "corr-test");
    assert_eq!(
        record.context().get("field").map(String::as_str),
        Some("displayName")
    );
    assert_eq!(
        record.context().get("overridden").map(String::as_str),
        Some("true")
    );
}

#[tokio::test]
async fn w1_an_empty_or_over_long_name_names_the_field_and_writes_nothing() {
    let harness = harness();
    let user = provision(&harness).await;
    let caller = caller(user.clone(), claims());

    for refused in ["   ".to_string(), "x".repeat(201)] {
        match harness.profile.update_display_name(&caller, &refused).await {
            Err(AppError::Validation(fields)) => {
                assert_eq!(fields.len(), 1, "one offending field is named");
                assert_eq!(fields[0].field, "displayName");
            }
            other => panic!("a refused name answers a field problem: {other:?}"),
        }
    }

    assert_eq!(
        harness.users.get(user.id()).unwrap().display_name(),
        Some("Test User"),
        "a refused save leaves the stored name alone"
    );
    assert!(
        !harness
            .audit
            .actions()
            .iter()
            .any(|action| action == "identity.user.profile.updated"),
        "a refused save writes no audit event"
    );
}

#[tokio::test]
async fn w2_adopting_the_provider_name_clears_the_override_and_audits_it() {
    let harness = harness();
    let mut user = provision(&harness).await;
    user.update_profile(Some("Mai Anh".to_string()), None, fixed_now())
        .unwrap();
    harness.users.seed(user.clone(), &claims().subject);
    let caller = caller(user.clone(), claims());

    let view = harness
        .profile
        .adopt_provider_name(&caller)
        .await
        .expect("the claim carries a provider name");

    assert_eq!(view.display_name(), Some("Test User"));
    assert!(!view.display_name_overridden());
    assert_eq!(view.provider_display_name(), Some("Test User"));
    assert_eq!(
        harness.users.get(user.id()).unwrap().display_name(),
        Some("Test User")
    );

    let records = harness.audit.records.lock().unwrap();
    let events: Vec<_> = records
        .iter()
        .filter(|(record, _)| record.action() == "identity.user.profile.updated")
        .collect();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].0.context().get("overridden").map(String::as_str),
        Some("false")
    );
    assert_eq!(
        events[0].0.context().get("field").map(String::as_str),
        Some("displayName")
    );
}

#[tokio::test]
async fn w2_without_a_provider_name_the_adoption_conflicts_and_changes_nothing() {
    let harness = harness();
    let mut user = provision(&harness).await;
    user.update_profile(Some("Mai Anh".to_string()), None, fixed_now())
        .unwrap();
    harness.users.seed(user.clone(), &claims().subject);
    let mut nameless = claims();
    nameless.display_name = None;
    let caller = caller(user.clone(), nameless);

    match harness.profile.adopt_provider_name(&caller).await {
        Err(AppError::Conflict(_)) => {}
        other => panic!("a missing provider name answers 409: {other:?}"),
    }

    let stored = harness.users.get(user.id()).unwrap();
    assert_eq!(stored.display_name(), Some("Mai Anh"));
    assert!(
        stored.display_name_overridden(),
        "the override is not cleared without a name to adopt"
    );
    assert!(!harness
        .audit
        .actions()
        .iter()
        .any(|action| action == "identity.user.profile.updated"));
}

#[tokio::test]
async fn d0_every_status_is_answered_without_a_denial_or_an_audit_event() {
    let harness = harness();
    for status in [
        UserStatus::Pending,
        UserStatus::Active,
        UserStatus::Disabled,
        UserStatus::PendingDeletion,
        UserStatus::Deleted,
    ] {
        let caller = caller(user_with_status(status), claims());
        assert_eq!(
            harness.profile.account_status(&caller),
            status,
            "D0 answers {} without the denial predicate",
            status.as_str()
        );
    }
    assert!(
        harness.audit.actions().is_empty(),
        "a status read is not an audited event"
    );
}

#[tokio::test]
async fn w3_the_open_request_is_read_without_an_audit_event_or_a_state_change() {
    let harness = harness();
    let user = provision(&harness).await;
    let caller = caller(user.clone(), claims());
    let request = DeletionRequest::request(DeletionRequestId::generate(), user.id(), fixed_now());
    harness.deletions.insert(&request).await.unwrap();

    let open = harness
        .account_lifecycle
        .open_deletion(&caller)
        .await
        .expect("the open request is read");

    assert_eq!(open.id(), request.id());
    assert_eq!(open.status(), DeletionStatus::Requested);
    assert_eq!(
        harness.users.get(user.id()).unwrap().status(),
        UserStatus::Active,
        "the read never opens the request or moves the account"
    );
    assert!(
        !harness
            .audit
            .actions()
            .iter()
            .any(|action| action.starts_with("identity.deletion")),
        "the read writes no audit event"
    );
}

#[tokio::test]
async fn w3_without_an_open_request_the_read_answers_not_found() {
    let harness = harness();
    let user = provision(&harness).await;
    let caller = caller(user, claims());

    match harness.account_lifecycle.open_deletion(&caller).await {
        Err(AppError::NotFound(entity)) => assert_eq!(entity, "deletion"),
        other => panic!("no open request is a not-found, never an error: {other:?}"),
    }
}

#[tokio::test]
async fn w4_the_cancel_delegates_and_restores_the_account() {
    let harness = harness();
    let user = provision(&harness).await;
    let caller = caller(user.clone(), claims());
    let request = harness
        .account_lifecycle
        .request_deletion(&caller)
        .await
        .expect("the owner opens a request");

    let cancelled = harness
        .account_lifecycle
        .cancel_deletion(&caller)
        .await
        .expect("the owner restores inside the window");

    assert_eq!(cancelled.id(), request.id());
    assert!(
        cancelled.cancelled_at().is_some(),
        "the request is cancelled, not erased"
    );
    assert_eq!(
        harness.users.get(user.id()).unwrap().status(),
        UserStatus::Active
    );
    assert!(harness
        .account_observation
        .deletion_events
        .lock()
        .unwrap()
        .iter()
        .any(|event| event == "cancelled"));
}

#[tokio::test]
async fn w4_a_cancel_with_no_open_request_answers_conflict() {
    let harness = harness();
    let user = provision(&harness).await;
    let caller = caller(user, claims());

    match harness.account_lifecycle.cancel_deletion(&caller).await {
        Err(AppError::Conflict(_)) => {}
        other => panic!("no open request answers 409: {other:?}"),
    }
}

#[tokio::test]
async fn w4_a_cancel_after_the_window_answers_gone() {
    let harness = harness();
    let user = provision(&harness).await;
    let caller = caller(user.clone(), claims());
    harness
        .account_lifecycle
        .request_deletion(&caller)
        .await
        .expect("the owner opens a request");

    harness.clock.set(fixed_now() + Duration::days(31));
    match harness.account_lifecycle.cancel_deletion(&caller).await {
        Err(AppError::Gone(reason)) => assert!(
            reason.contains("recovery window"),
            "the refusal names the closed window: {reason}"
        ),
        other => panic!("a closed window answers 410: {other:?}"),
    }
    assert_eq!(
        harness.users.get(user.id()).unwrap().status(),
        UserStatus::PendingDeletion,
        "a refused cancel leaves the account pending deletion"
    );
}
