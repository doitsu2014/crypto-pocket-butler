//! Application use-case tests against port fakes — no database, no network.

mod common;

use chrono::Duration;
use common::*;
use identity_application::common::AppError;
use identity_application::ports::{
    DeletionRepository, DeviceSessionRepository, ExportRepository, SupportGrantRepository,
};
use identity_domain::claims::{AssuranceLevel, NextStep};
use identity_domain::consent::{ConsentPurpose, ConsentSource};
use identity_domain::deletion::{DeletionStatus, RetainedCategory};
use identity_domain::export::{Contributor, ExportStatus};
use identity_domain::preferences::{BaseCurrency, Locale, Preferences, TimeZone};
use identity_domain::user::UserStatus;
use identity_domain::CountryCode;

fn preferences() -> Preferences {
    Preferences {
        locale: Locale::Vi,
        base_currency: BaseCurrency::parse("EUR").unwrap(),
        timezone: TimeZone::parse("Asia/Ho_Chi_Minh").unwrap(),
    }
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
    assert!(harness
        .audit
        .actions()
        .iter()
        .any(|action| action == "identity.principal.denied"));
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
        .check(grant.id(), target.id(), "corr-check")
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
        .check(grant.id(), target.id(), "corr-check")
        .await
        .expect("an active grant serves the read");
    assert!(harness
        .audit
        .actions()
        .iter()
        .any(|action| action == "identity.support_read.served"));

    harness.audit.fail_next();
    let error = harness
        .support
        .check(grant.id(), target.id(), "corr-check-2")
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
        .deletion_service
        .request(&caller)
        .await
        .expect("a deletion is requested");
    let repeated = harness
        .deletion_service
        .request(&caller)
        .await
        .expect("a repeated confirmation reuses the request");
    assert_eq!(repeated.id(), deletion.id());
    assert_eq!(
        *harness.wake.calls.lock().unwrap(),
        1,
        "reusing the active request does not wake the worker again"
    );

    harness
        .deletion_worker
        .run_once("worker-1", fixed_now())
        .await
        .expect("the worker completes the deletion");

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
    assert!(harness.consent.records.lock().unwrap().is_empty());

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

    let deletion = harness.deletion_service.request(&caller).await.unwrap();
    harness
        .deletion_worker
        .run_once("worker-1", fixed_now())
        .await
        .unwrap();
    let failed = harness
        .deletions
        .find_by_id(deletion.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(failed.status(), DeletionStatus::Failed);
    assert!(harness.auth_admin.deleted.lock().unwrap().is_empty());

    *harness.contributors.fail_anonymise.lock().unwrap() = None;
    harness
        .deletion_worker
        .run_once("worker-2", fixed_now() + Duration::seconds(1))
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
