//! Worker-layer tests: the supervisor runs both workers, a dead worker's lease
//! is re-claimed, and one worker's failure does not stop the other.

mod common;

use chrono::Duration;
use common::*;
use identity_application::ports::{DeletionRepository, ExportRepository};
use identity_domain::deletion::DeletionStatus;
use identity_domain::export::{Contributor, ExportStatus};

async fn provision(harness: &Harness) -> identity_domain::User {
    let view = harness
        .principal
        .resolve(claims(), "corr-provision")
        .await
        .expect("a confirmed user is provisioned");
    harness.users.get(view.user_id()).expect("the user exists")
}

#[tokio::test]
async fn the_supervisor_runs_the_export_then_the_deletion_and_reports_both_counts() {
    let harness = harness();
    let user = provision(&harness).await;
    let user_id = user.id();
    let caller = caller(user, claims());

    let export = harness.export_service.request(&caller).await.unwrap();
    let deletion = harness
        .account_lifecycle
        .request_deletion(&caller)
        .await
        .unwrap();

    let report = harness
        .worker
        .run_once("worker-1", fixed_now() + Duration::days(31))
        .await;
    assert_eq!(report.exports_processed, 1);
    assert_eq!(report.deletions_processed, 1);

    let stored_export = harness
        .exports
        .find_by_id(export.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored_export.status(), ExportStatus::Ready);
    let stored_deletion = harness
        .deletions
        .find_by_id(deletion.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored_deletion.status(), DeletionStatus::Completed);
    assert_eq!(harness.store.objects.lock().unwrap().len(), 1);
    assert_eq!(harness.auth_admin.deleted.lock().unwrap().len(), 1);
    assert_eq!(
        harness.erasure.erased.lock().unwrap().as_slice(),
        [user_id],
        "the erasure delete is routed through the identity_retention port"
    );
}

#[tokio::test]
async fn a_dead_workers_lease_is_reclaimed_by_the_next_pass() {
    let harness = harness();
    let user = provision(&harness).await;
    let caller = caller(user, claims());
    let export = harness.export_service.request(&caller).await.unwrap();

    let claimed = harness
        .exports
        .claim_next("dead-worker", fixed_now(), Duration::seconds(60))
        .await
        .unwrap()
        .expect("the export is claimed");
    assert_eq!(claimed.lease_owner(), Some("dead-worker"));
    assert_eq!(claimed.status(), ExportStatus::Building);

    let processed = harness
        .export_worker
        .run_once("worker-2", fixed_now() + Duration::seconds(61))
        .await
        .unwrap();
    assert_eq!(processed, 1);

    let stored = harness
        .exports
        .find_by_id(export.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.status(), ExportStatus::Ready);
    assert_eq!(stored.progress(), 100);
}

#[tokio::test]
async fn one_workers_failure_does_not_stop_the_other() {
    let harness = harness();
    let user = provision(&harness).await;
    let caller = caller(user, claims());

    let export = harness.export_service.request(&caller).await.unwrap();
    let deletion = harness
        .account_lifecycle
        .request_deletion(&caller)
        .await
        .unwrap();
    *harness.contributors.fail_anonymise.lock().unwrap() = Some(Contributor::Account);

    let report = harness
        .worker
        .run_once("worker-1", fixed_now() + Duration::days(31))
        .await;
    assert_eq!(report.exports_processed, 1);
    assert_eq!(report.deletions_processed, 1);

    let stored_export = harness
        .exports
        .find_by_id(export.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored_export.status(), ExportStatus::Ready);
    let stored_deletion = harness
        .deletions
        .find_by_id(deletion.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored_deletion.status(), DeletionStatus::Failed);
    assert!(
        harness.erasure.erased.lock().unwrap().is_empty(),
        "a failed contributor step never reaches the erasure delete"
    );
}
