//! Application rules of the account lifecycle, exercised against port fakes:
//! admin disable/restore (BR5.2), the deletion window (BR5.3/BR5.4), the expiry
//! job (BR5.5/BR5.6), and the tombstone denial (BR5.8).

mod common;

use chrono::Duration;
use common::{admin_claims, caller, claims, fixed_now, harness, Harness};
use identity_application::common::AppError;
use identity_application::ports::{DeletionRepository, UserRepository};
use identity_domain::{Contributor, User, UserStatus, MAX_ERASURE_ATTEMPTS};

async fn provision(harness: &Harness) -> User {
    let view = harness
        .principal
        .resolve(claims(), "corr-provision")
        .await
        .expect("a confirmed user is provisioned");
    harness.users.get(view.user_id()).expect("the user exists")
}

async fn admin_caller(harness: &Harness) -> identity_application::common::Caller {
    let view = harness
        .principal
        .resolve(admin_claims(), "corr-admin")
        .await
        .expect("an admin is provisioned");
    let user = harness.users.get(view.user_id()).expect("the admin exists");
    caller(user, admin_claims())
}

#[tokio::test]
async fn an_admin_disables_and_restores_with_a_recorded_reason() {
    let harness = harness();
    let user = provision(&harness).await;
    let admin = admin_caller(&harness).await;

    let disabled = harness
        .account_lifecycle
        .disable(&admin, user.id(), "account abuse")
        .await
        .expect("an admin disables the account");
    assert_eq!(disabled.status(), UserStatus::Disabled);
    assert_eq!(
        harness.users.get(user.id()).unwrap().status(),
        UserStatus::Disabled
    );
    let stored = harness.transitions.stored();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].from_status(), UserStatus::Active);
    assert_eq!(stored[0].to_status(), UserStatus::Disabled);
    assert_eq!(stored[0].actor(), admin.user_id().to_string());
    assert_eq!(stored[0].reason(), Some("account abuse"));

    let restored = harness
        .account_lifecycle
        .restore(&admin, user.id(), "appeal upheld")
        .await
        .expect("an admin restores the account");
    assert_eq!(restored.status(), UserStatus::Active);
    assert_eq!(
        harness
            .account_observation
            .transitions
            .lock()
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn a_non_admin_cannot_disable_or_restore_and_a_blank_reason_is_refused() {
    let harness = harness();
    let user = provision(&harness).await;
    let owner = caller(user.clone(), claims());

    let refused = harness
        .account_lifecycle
        .disable(&owner, user.id(), "abuse")
        .await
        .expect_err("a non-admin is refused");
    assert!(matches!(refused, AppError::Forbidden(_)));

    let admin = admin_caller(&harness).await;
    let blank = harness
        .account_lifecycle
        .disable(&admin, user.id(), "   ")
        .await
        .expect_err("a blank reason is refused");
    assert!(matches!(blank, AppError::Validation(_)));
    let too_long = harness
        .account_lifecycle
        .disable(&admin, user.id(), &"x".repeat(501))
        .await
        .expect_err("an over-long reason is refused");
    assert!(matches!(too_long, AppError::Validation(_)));

    assert!(
        harness.transitions.stored().is_empty(),
        "a refused request writes nothing"
    );
    assert_eq!(
        harness.users.get(user.id()).unwrap().status(),
        UserStatus::Active
    );
}

#[tokio::test]
async fn an_illegal_edge_is_refused_with_nothing_written() {
    let harness = harness();
    let user = provision(&harness).await;
    let admin = admin_caller(&harness).await;

    harness
        .account_lifecycle
        .disable(&admin, user.id(), "first")
        .await
        .expect("the first disable is legal");

    let again = harness
        .account_lifecycle
        .disable(&admin, user.id(), "second")
        .await
        .expect_err("disabled -> disabled is not an edge");
    assert!(matches!(again, AppError::Conflict(_)));
    assert_eq!(
        harness.transitions.stored().len(),
        1,
        "the refused edge wrote no transition"
    );
}

#[tokio::test]
async fn requesting_deletion_opens_the_thirty_day_window() {
    let harness = harness();
    let user = provision(&harness).await;
    let owner = caller(user.clone(), claims());

    let request = harness
        .account_lifecycle
        .request_deletion(&owner)
        .await
        .expect("the owner opens a request");
    assert!(request.is_open());
    assert_eq!(
        request.recoverable_until(),
        identity_domain::recoverable_until(fixed_now())
    );
    assert_eq!(
        harness.users.get(user.id()).unwrap().status(),
        UserStatus::PendingDeletion
    );
    assert_eq!(
        harness
            .account_observation
            .deletion_events
            .lock()
            .unwrap()
            .as_slice(),
        ["requested"]
    );

    let second = harness
        .account_lifecycle
        .request_deletion(&owner)
        .await
        .expect_err("a second open request is refused");
    assert!(matches!(second, AppError::Conflict(_)));
}

#[tokio::test]
async fn restoring_inside_the_window_returns_to_active_and_after_it_is_gone() {
    let harness = harness();
    let user = provision(&harness).await;
    let owner = caller(user.clone(), claims());
    let request = harness
        .account_lifecycle
        .request_deletion(&owner)
        .await
        .unwrap();

    harness.clock.set(fixed_now() + Duration::days(1));
    let restored = harness
        .account_lifecycle
        .cancel_deletion(&owner)
        .await
        .expect("the owner restores inside the window");
    assert_eq!(
        restored.cancelled_at(),
        Some(fixed_now() + Duration::days(1))
    );
    assert!(!restored.is_open());
    assert_eq!(
        harness.users.get(user.id()).unwrap().status(),
        UserStatus::Active
    );

    // A second restore has no open request to cancel.
    let again = harness.account_lifecycle.cancel_deletion(&owner).await;
    assert!(matches!(again, Err(AppError::Conflict(_))));

    // A fresh request, restored after the deadline, is `410 Gone`.
    harness.clock.set(fixed_now() + Duration::days(32));
    let second = harness
        .account_lifecycle
        .request_deletion(&owner)
        .await
        .expect("a closed request no longer blocks a new one");
    assert_ne!(second.id(), request.id());
    harness.clock.set(fixed_now() + Duration::days(63));
    let late = harness.account_lifecycle.cancel_deletion(&owner).await;
    assert!(matches!(late, Err(AppError::Gone(_))));
}

#[tokio::test]
async fn the_expiry_job_erases_a_due_account_and_its_tombstone_denies_the_token() {
    let harness = harness();
    let user = provision(&harness).await;
    let owner = caller(user.clone(), claims());
    harness
        .account_lifecycle
        .request_deletion(&owner)
        .await
        .unwrap();

    // Before the deadline nothing is claimed.
    let early = harness
        .deletion_worker
        .run_once("worker-1", fixed_now())
        .await
        .unwrap();
    assert_eq!(early, 0, "an open window is never erased early");

    let after = fixed_now() + Duration::days(31);
    harness.clock.set(after);
    let processed = harness
        .deletion_worker
        .run_once("worker-1", after)
        .await
        .unwrap();
    assert_eq!(processed, 1);
    harness
        .deletion_worker
        .run_once("worker-2", after + Duration::seconds(1))
        .await
        .expect("a second run is a no-op success");

    let erased = harness.users.get(user.id()).unwrap();
    assert_eq!(erased.status(), UserStatus::Deleted);
    assert!(erased.anonymised_at().is_some());
    assert!(erased.email().is_none());
    let stored = harness
        .deletions
        .find_active_for_user(user.id())
        .await
        .unwrap();
    assert!(stored.is_none(), "the erased request is closed");

    // The tombstone denies the still-valid token instead of provisioning a new
    // account for it.
    let denied = harness
        .principal
        .resolve(claims(), "corr-after-erasure")
        .await
        .expect_err("an erased subject is denied");
    assert!(matches!(denied, AppError::Forbidden(_)));
    assert_eq!(harness.users.len(), 1, "no new account was provisioned");
    assert!(harness
        .audit
        .actions()
        .iter()
        .any(|action| action == "identity.principal.erased"));
}

#[tokio::test]
async fn a_restore_racing_the_sweep_cancels_the_request_instead_of_erasing() {
    let harness = harness();
    let user = provision(&harness).await;
    let owner = caller(user.clone(), claims());
    let request = harness
        .account_lifecycle
        .request_deletion(&owner)
        .await
        .unwrap();

    // The owner restored through another path: the stored status moved back.
    harness.clock.set(fixed_now() + Duration::days(2));
    let mut restored = harness.users.get(user.id()).unwrap();
    restored
        .transition(UserStatus::Active, fixed_now() + Duration::days(2))
        .unwrap();
    harness.users.update(&restored).await.unwrap();

    let after = fixed_now() + Duration::days(31);
    let processed = harness
        .deletion_worker
        .run_once("worker-1", after)
        .await
        .unwrap();
    assert_eq!(processed, 1, "the sweep processed and skipped the request");
    let stored = harness
        .deletions
        .find_by_id(request.id())
        .await
        .unwrap()
        .unwrap();
    assert!(stored.cancelled_at().is_some());
    assert!(stored.erased_at().is_none());
    assert_eq!(
        harness.users.get(user.id()).unwrap().status(),
        UserStatus::Active
    );
    assert!(harness.auth_admin.deleted.lock().unwrap().is_empty());
}

#[tokio::test]
async fn a_failing_step_backs_off_and_the_fifth_attempt_parks_the_request() {
    let harness = harness();
    let user = provision(&harness).await;
    let owner = caller(user.clone(), claims());
    let request = harness
        .account_lifecycle
        .request_deletion(&owner)
        .await
        .unwrap();
    *harness.contributors.fail_anonymise.lock().unwrap() = Some(Contributor::Ledger);

    let mut at = fixed_now() + Duration::days(31);
    for (index, attempt) in (1u32..=5).enumerate() {
        harness
            .deletion_worker
            .run_once(&format!("worker-{index}"), at)
            .await
            .unwrap();
        let stored = harness
            .deletions
            .find_by_id(request.id())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.attempt_count(), attempt);
        at = stored.next_attempt_at();
    }

    let parked = harness
        .deletions
        .find_by_id(request.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(parked.attempt_count(), MAX_ERASURE_ATTEMPTS);
    assert!(
        !parked.is_open(),
        "a parked request no longer blocks a new one"
    );
    assert_eq!(
        harness
            .account_observation
            .erasure_failures
            .lock()
            .unwrap()
            .len(),
        5
    );
    // The transition to `deleted` committed before the failing step (W4 step 2),
    // so the account is already erased locally; the parked `failed` request is
    // where an operator resumes the remaining downstream erase.
    assert_eq!(
        harness.users.get(user.id()).unwrap().status(),
        UserStatus::Deleted
    );
    assert!(parked.erased_at().is_none(), "the cascade did not complete");
}
