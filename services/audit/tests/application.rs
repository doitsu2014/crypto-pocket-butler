//! Application use-case tests against port fakes — no database, no network.

mod common;

use std::collections::BTreeMap;

use audit_application::append::{AppendCommand, AppendError, Emitter};
use audit_application::export::ExportError;
use audit_application::ports::{ExportRepository, Principal, RepositoryError, Role};
use audit_application::privacy::{PrivacyError, RequestCommand};
use audit_application::query::{QueryCommand, QueryError};
use audit_application::retention::ReportCommand;
use audit_application::support::{CheckDecision, StartSessionCommand, SupportError};
use audit_domain::{
    AuditEvent, AuditEventId, AuditExport, AuditExportId, EventLabel, ExportFilter, ExportState,
    Outcome, PartitionSummary, PrivacyDeletionState, Sensitivity, SupportScope, SupportSession,
    SupportSessionId, SupportSessionState, UserId,
};
use chrono::{Duration, TimeZone, Utc};
use common::*;

fn instant(hour: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 6, hour, 0, 0).unwrap()
}

fn admin() -> Principal {
    Principal {
        subject: ADMIN_SUBJECT.to_string(),
        role: Role::Admin,
    }
}

fn support_principal() -> Principal {
    Principal {
        subject: SUPPORT_SUBJECT.to_string(),
        role: Role::Support,
    }
}

fn user_principal() -> Principal {
    Principal {
        subject: USER_SUBJECT.to_string(),
        role: Role::User,
    }
}

fn append_command(key: &str) -> AppendCommand {
    AppendCommand {
        actor_id: Some(USER_SUBJECT.to_string()),
        actor_kind: "user".to_string(),
        action: "asset.contract.added".to_string(),
        target: "asset-usdc".to_string(),
        occurred_at: Some(instant(11)),
        context: BTreeMap::new(),
        outcome: "success".to_string(),
        sensitivity: "routine".to_string(),
        event_class: None,
        legal_basis: None,
        deletion_request_id: None,
        idempotency_key: Some(key.to_string()),
    }
}

fn emitter(service: &str) -> Emitter {
    Emitter {
        service: service.to_string(),
    }
}

#[tokio::test]
async fn appends_a_valid_event_and_returns_its_id() {
    let harness = harness();
    let result = harness
        .state
        .append
        .execute(&emitter("identity"), append_command("key-1"))
        .await
        .expect("a valid event is accepted");
    assert!(result.created);
    assert_eq!(harness.events.stored().len(), 1);
    assert_eq!(harness.events.stored()[0].id(), result.id);
}

#[tokio::test]
async fn a_retried_append_returns_the_existing_id() {
    let harness = harness();
    let first = harness
        .state
        .append
        .execute(&emitter("identity"), append_command("key-1"))
        .await
        .unwrap();
    let second = harness
        .state
        .append
        .execute(&emitter("identity"), append_command("key-1"))
        .await
        .unwrap();
    assert!(!second.created);
    assert_eq!(second.id, first.id);
    assert_eq!(harness.events.stored().len(), 1);
}

#[tokio::test]
async fn a_reused_key_with_a_different_payload_is_a_conflict() {
    let harness = harness();
    harness
        .state
        .append
        .execute(&emitter("identity"), append_command("key-1"))
        .await
        .unwrap();
    let mut changed = append_command("key-1");
    changed.target = "asset-dai".to_string();
    let error = harness
        .state
        .append
        .execute(&emitter("identity"), changed)
        .await
        .unwrap_err();
    assert_eq!(error, AppendError::PayloadMismatch);
    assert_eq!(harness.events.stored().len(), 1);
}

#[tokio::test]
async fn rejects_a_secret_shaped_target_naming_the_field_only() {
    let harness = harness();
    let address = "0x1111111111111111111111111111111111111111";
    let mut command = append_command("key-1");
    command.target = address.to_string();
    let error = harness
        .state
        .append
        .execute(&emitter("identity"), command)
        .await
        .unwrap_err();
    match error {
        AppendError::Rejected(errors) => {
            assert_eq!(errors[0].field, "target");
            assert!(!errors[0].reason.contains(address));
        }
        other => panic!("expected a rejection, got {other:?}"),
    }
    assert!(harness.events.stored().is_empty());
}

#[tokio::test]
async fn rejects_a_contradicting_event_class() {
    let harness = harness();
    let mut command = append_command("key-1");
    command.sensitivity = "security".to_string();
    command.event_class = Some("domain".to_string());
    let error = harness
        .state
        .append
        .execute(&emitter("identity"), command)
        .await
        .unwrap_err();
    assert!(matches!(error, AppendError::Rejected(_)));
}

fn approver_principal() -> Principal {
    Principal {
        subject: SECOND_APPROVER.to_string(),
        role: Role::Support,
    }
}

#[tokio::test]
async fn an_emitter_cannot_append_a_system_event() {
    let harness = harness();
    let mut command = append_command("key-system");
    command.actor_id = None;
    command.actor_kind = "system".to_string();
    command.action = "system.retention.run".to_string();
    let error = harness
        .state
        .append
        .execute(&emitter("identity"), command)
        .await
        .unwrap_err();
    match error {
        AppendError::Rejected(errors) => assert_eq!(errors[0].field, "actorKind"),
        other => panic!("expected a rejection, got {other:?}"),
    }
    assert!(
        harness.events.stored().is_empty(),
        "system attribution is reserved for Audit's own write path"
    );
}

#[tokio::test]
async fn rejects_a_system_event_outside_the_allow_list() {
    let harness = harness();
    let mut command = append_command("key-1");
    command.actor_id = None;
    command.actor_kind = "system".to_string();
    command.action = "asset.created".to_string();
    let error = harness
        .state
        .append
        .execute(&emitter("identity"), command)
        .await
        .unwrap_err();
    assert!(matches!(error, AppendError::Rejected(_)));
}

fn query(limit: Option<u32>) -> QueryCommand {
    QueryCommand {
        action_prefix: None,
        target: None,
        sensitivity: None,
        occurred_from: None,
        occurred_to: None,
        recorded_from: Some(instant(0)),
        recorded_to: Some(instant(23)),
        cursor: None,
        limit,
    }
}

#[tokio::test]
async fn a_non_admin_query_is_forbidden_and_audited() {
    let harness = harness();
    let error = harness
        .state
        .query
        .execute(&user_principal(), query(None))
        .await
        .unwrap_err();
    assert_eq!(error, QueryError::Forbidden);
    let events = harness.events.stored();
    assert!(events
        .iter()
        .any(|event| event.action() == "audit.query.denied" && event.outcome() == Outcome::Denied));
}

#[tokio::test]
async fn a_query_without_the_recorded_range_is_refused() {
    let harness = harness();
    let mut command = query(None);
    command.recorded_from = None;
    let error = harness
        .state
        .query
        .execute(&admin(), command)
        .await
        .unwrap_err();
    match error {
        QueryError::Rejected(errors) => assert_eq!(errors[0].field, "recordedFrom"),
        other => panic!("expected a rejection, got {other:?}"),
    }
}

#[tokio::test]
async fn a_query_pages_newest_first_with_a_cursor() {
    let harness = harness();
    for hour in [9, 10, 11] {
        let mut command = append_command(&format!("key-{hour}"));
        command.occurred_at = Some(instant(hour));
        harness
            .state
            .append
            .execute(&emitter("identity"), command)
            .await
            .unwrap();
    }
    let first = harness
        .state
        .query
        .execute(&admin(), query(Some(2)))
        .await
        .unwrap();
    assert_eq!(first.events.len(), 2);
    assert_eq!(first.events[0].occurred_at(), instant(11));
    assert!(first.next_cursor.is_some());

    let mut second_command = query(Some(2));
    second_command.cursor = first.next_cursor;
    let second = harness
        .state
        .query
        .execute(&admin(), second_command)
        .await
        .unwrap();
    assert_eq!(second.events.len(), 1);
    assert_eq!(second.events[0].occurred_at(), instant(9));
    assert!(second.next_cursor.is_none());
}

#[tokio::test]
async fn a_cursor_keeps_sub_millisecond_precision() {
    let harness = harness();
    let base = instant(11);
    for (index, offset) in [0_i64, 1].into_iter().enumerate() {
        let mut command = append_command(&format!("nanos-{index}"));
        command.occurred_at = Some(base + Duration::nanoseconds(offset));
        harness
            .state
            .append
            .execute(&emitter("identity"), command)
            .await
            .unwrap();
    }

    let first = harness
        .state
        .query
        .execute(&admin(), query(Some(1)))
        .await
        .unwrap();
    assert_eq!(first.events.len(), 1);
    let mut next = query(Some(1));
    next.cursor = first.next_cursor;
    let second = harness.state.query.execute(&admin(), next).await.unwrap();
    assert_eq!(
        second.events.len(),
        1,
        "a second row inside the same millisecond must not be skipped"
    );
    assert_eq!(second.events[0].occurred_at(), base);
}

fn export_filter() -> ExportFilter {
    ExportFilter::new(Vec::new(), None, None, instant(0), instant(23), None, None).unwrap()
}

#[tokio::test]
async fn a_non_admin_export_is_forbidden() {
    let harness = harness();
    let error = harness
        .state
        .export
        .start(&support_principal(), export_filter())
        .await
        .unwrap_err();
    assert_eq!(error, ExportError::Forbidden);
    assert!(harness
        .events
        .stored()
        .iter()
        .any(|event| event.action() == "audit.export.denied"));
}

#[tokio::test]
async fn records_the_request_before_it_counts() {
    let harness = harness();
    harness
        .events
        .fail_next(RepositoryError::Unavailable("count failed".to_string()));
    let error = harness
        .state
        .export
        .start(&admin(), export_filter())
        .await
        .unwrap_err();
    assert!(matches!(error, ExportError::Repository(_)));
    assert_eq!(
        harness.exports.stored()[0].state(),
        ExportState::Requested,
        "the request is recorded before the rows are counted"
    );
}

#[tokio::test]
async fn an_export_over_the_cap_is_refused_and_recorded_as_failed() {
    let harness = harness();
    for _ in 0..=audit_domain::MAX_EXPORT_ROWS {
        let event = AuditEvent::record(
            audit_domain::NewAuditEvent {
                actor_id: Some(UserId::parse(USER_SUBJECT).unwrap()),
                actor_kind: audit_domain::ActorKind::User,
                action: "asset.contract.added".to_string(),
                target: "asset-usdc".to_string(),
                occurred_at: Some(instant(11)),
                context: audit_domain::Context::empty(),
                outcome: Outcome::Success,
                sensitivity: Sensitivity::Routine,
                event_class: None,
                idempotency_key: None,
                legal_basis: None,
                deletion_request_id: None,
            },
            AuditEventId::generate(),
            instant(12),
        )
        .unwrap();
        harness.events.seed(event);
    }

    let error = harness
        .state
        .export
        .start(&admin(), export_filter())
        .await
        .unwrap_err();
    assert_eq!(
        error,
        ExportError::TooLarge {
            cap: audit_application::export::EXPORT_ROW_CAP,
        }
    );
    assert_eq!(harness.exports.stored()[0].state(), ExportState::Failed);
}

#[tokio::test]
async fn a_streamed_export_records_success_with_its_row_count_and_digest() {
    let harness = harness();
    for hour in [9, 10, 11] {
        let mut command = append_command(&format!("export-{hour}"));
        command.occurred_at = Some(instant(hour));
        harness
            .state
            .append
            .execute(&emitter("identity"), command)
            .await
            .unwrap();
    }
    let mut session = harness
        .state
        .export
        .start(&admin(), export_filter())
        .await
        .unwrap();
    let mut rows = 0;
    while let Some(batch) = session.next_batch().await.unwrap() {
        rows += batch.len();
    }
    assert_eq!(rows, 3);
    let digest = "b".repeat(64);
    let export = session.finish(&digest).await.unwrap();
    assert_eq!(export.row_count(), 3);
    assert_eq!(export.state(), ExportState::Success);
    assert_eq!(
        harness.exports.stored()[0].content_sha256(),
        Some(digest.as_str())
    );
    assert!(harness
        .events
        .stored()
        .iter()
        .any(|event| event.action() == "audit.export.completed"));
}

#[tokio::test]
async fn a_stream_error_marks_the_export_failed() {
    let harness = harness();
    let mut session = harness
        .state
        .export
        .start(&admin(), export_filter())
        .await
        .unwrap();
    harness
        .events
        .fail_next(RepositoryError::Unavailable("cursor lost".to_string()));
    assert!(session.next_batch().await.is_err());
    session.fail("cursor lost").await.unwrap();
    assert_eq!(harness.exports.stored()[0].state(), ExportState::Failed);
}

#[tokio::test]
async fn stale_requested_exports_are_recovered_on_start() {
    let harness = harness();
    let stale = AuditExport::requested(
        AuditExportId::generate(),
        UserId::parse(ADMIN_SUBJECT).unwrap(),
        export_filter(),
        fixed_now() - Duration::minutes(30),
    );
    harness
        .exports
        .insert(&stale)
        .await
        .expect("seed a stale request");

    let recovered = harness.state.export.recover_stale().await.unwrap();
    assert_eq!(recovered, 1);
    assert_eq!(harness.exports.stored()[0].state(), ExportState::Failed);
    assert_eq!(
        harness.exports.stored()[0].failure_reason(),
        Some(audit_domain::export::REPLICA_LOST_REASON),
        "an operator can tell a crashed replica from an ordinary failure"
    );
}

fn session_command(expires_at: chrono::DateTime<Utc>, reason: &str) -> StartSessionCommand {
    StartSessionCommand {
        support_agent_id: Some(SUPPORT_SUBJECT.to_string()),
        subject_user_id: SUBJECT_USER.to_string(),
        reason: reason.to_string(),
        second_approver_id: SECOND_APPROVER.to_string(),
        scope: "account_metadata".to_string(),
        expires_at,
    }
}

#[tokio::test]
async fn records_a_requested_session_and_waits_for_the_second_approver() {
    let harness = harness();
    let session = harness
        .state
        .support
        .request(
            &support_principal(),
            session_command(instant(18), "customer reported a billing error"),
        )
        .await
        .unwrap();
    assert_eq!(session.stored_state(), SupportSessionState::Requested);
    assert_eq!(session.approved_at(), None);
    let events = harness.events.stored();
    assert!(events
        .iter()
        .any(|event| event.action() == "support.session.requested"));
    assert!(
        !events
            .iter()
            .any(|event| event.action() == "support.session.started"),
        "nothing is active until the named approver acts"
    );
}

#[tokio::test]
async fn the_named_second_approver_activates_the_session_and_both_actors_are_audited() {
    let harness = harness();
    let requested = harness
        .state
        .support
        .request(
            &support_principal(),
            session_command(instant(18), "customer reported a billing error"),
        )
        .await
        .unwrap();
    let active = harness
        .state
        .support
        .approve(&approver_principal(), requested.id())
        .await
        .unwrap();
    assert_eq!(active.stored_state(), SupportSessionState::Active);
    assert_eq!(active.approved_at(), Some(fixed_now()));
    let events = harness.events.stored();
    let agent = UserId::parse(SUPPORT_SUBJECT).unwrap();
    let approver = UserId::parse(SECOND_APPROVER).unwrap();
    assert!(events.iter().any(
        |event| event.action() == "support.session.started" && event.actor_id() == Some(agent)
    ));
    assert!(events
        .iter()
        .any(|event| event.action() == "support.session.approved"
            && event.actor_id() == Some(approver)));
}

#[tokio::test]
async fn the_requesting_agent_cannot_approve_their_own_session() {
    let harness = harness();
    let requested = harness
        .state
        .support
        .request(
            &support_principal(),
            session_command(instant(18), "customer reported a billing error"),
        )
        .await
        .unwrap();
    let error = harness
        .state
        .support
        .approve(&support_principal(), requested.id())
        .await
        .unwrap_err();
    assert_eq!(error, SupportError::Forbidden);
    assert_eq!(
        harness.support.sessions.lock().unwrap()[0].stored_state(),
        SupportSessionState::Requested
    );
}

#[tokio::test]
async fn a_short_reason_is_denied_and_stored() {
    let harness = harness();
    let error = harness
        .state
        .support
        .request(&support_principal(), session_command(instant(18), "look"))
        .await
        .unwrap_err();
    assert!(matches!(error, SupportError::Denied { .. }));
    assert_eq!(harness.support.sessions.lock().unwrap().len(), 1);
    assert_eq!(
        harness.support.sessions.lock().unwrap()[0].stored_state(),
        audit_domain::SupportSessionState::Denied
    );
    assert!(harness
        .events
        .stored()
        .iter()
        .any(|event| event.action() == "support.session.denied"));
}

#[tokio::test]
async fn a_non_support_caller_cannot_start_a_session() {
    let harness = harness();
    let error = harness
        .state
        .support
        .request(
            &user_principal(),
            session_command(instant(18), "customer reported a billing error"),
        )
        .await
        .unwrap_err();
    assert_eq!(error, SupportError::Forbidden);
}

async fn started_session(harness: &Harness) -> SupportSession {
    let requested = harness
        .state
        .support
        .request(
            &support_principal(),
            session_command(instant(18), "customer reported a billing error"),
        )
        .await
        .unwrap();
    harness
        .state
        .support
        .approve(&approver_principal(), requested.id())
        .await
        .unwrap()
}

#[tokio::test]
async fn check_allows_an_active_session_and_records_the_read() {
    let harness = harness();
    let session = started_session(&harness).await;
    let before = harness.events.stored().len();
    let decision = harness
        .state
        .support
        .check(
            session.id(),
            audit_domain::SupportScope::AccountMetadata,
            "gateway",
        )
        .await;
    assert_eq!(decision, CheckDecision::Allow);
    let events = harness.events.stored();
    assert_eq!(events.len(), before + 1);
    assert_eq!(
        events.last().unwrap().action(),
        "support.account_metadata.read"
    );
}

#[tokio::test]
async fn check_denies_after_expiry() {
    let harness = harness();
    let session = started_session(&harness).await;
    harness.mutable_clock.set(instant(19));
    let decision = harness
        .state
        .support
        .check(
            session.id(),
            audit_domain::SupportScope::AccountMetadata,
            "gateway",
        )
        .await;
    assert_eq!(
        decision,
        CheckDecision::Deny {
            reason: "session-inactive"
        }
    );
}

#[tokio::test]
async fn check_denies_out_of_scope() {
    let harness = harness();
    let session = started_session(&harness).await;
    let decision = harness
        .state
        .support
        .check(session.id(), audit_domain::SupportScope::Tickets, "gateway")
        .await;
    assert_eq!(
        decision,
        CheckDecision::Deny {
            reason: "out-of-scope"
        }
    );
}

#[tokio::test]
async fn an_unknown_session_check_is_denied_and_the_denial_is_recorded() {
    let harness = harness();
    let id = SupportSessionId::generate();
    let decision = harness
        .state
        .support
        .check(id, SupportScope::AccountMetadata, "gateway")
        .await;
    assert!(matches!(decision, CheckDecision::Deny { .. }));
    assert!(
        harness.events.stored().iter().any(|event| {
            event.action() == "system.support.check_denied" && event.outcome() == Outcome::Denied
        }),
        "a denial that cannot be attributed to an agent is still written"
    );
}

#[tokio::test]
async fn check_denies_when_the_read_cannot_be_recorded() {
    let harness = harness();
    let session = started_session(&harness).await;
    harness
        .events
        .fail_next_append(RepositoryError::Unavailable("trail down".to_string()));
    let decision = harness
        .state
        .support
        .check(
            session.id(),
            audit_domain::SupportScope::AccountMetadata,
            "gateway",
        )
        .await;
    assert_eq!(
        decision,
        CheckDecision::Deny {
            reason: "not-recorded"
        }
    );
}

#[tokio::test]
async fn an_admin_revokes_an_active_session_and_a_user_cannot() {
    let harness = harness();
    let session = started_session(&harness).await;
    assert_eq!(
        harness
            .state
            .support
            .revoke(&user_principal(), session.id())
            .await
            .unwrap_err(),
        SupportError::Forbidden
    );
    let revoked = harness
        .state
        .support
        .revoke(&admin(), session.id())
        .await
        .unwrap();
    assert_eq!(
        revoked.stored_state(),
        audit_domain::SupportSessionState::Revoked
    );
}

fn partition(
    name: &str,
    from: chrono::DateTime<Utc>,
    to: chrono::DateTime<Utc>,
    held: bool,
    rows: i64,
) -> PartitionSummary {
    PartitionSummary {
        name: name.to_string(),
        from,
        to,
        holds_legal_hold: held,
        row_count: rows,
    }
}

fn retention_partitions() -> Vec<PartitionSummary> {
    vec![
        partition(
            "audit_events_202601",
            instant(0) - Duration::days(300),
            instant(0) - Duration::days(270),
            false,
            10,
        ),
        partition(
            "audit_events_202605",
            instant(0) - Duration::days(300),
            instant(0) + Duration::days(20),
            false,
            7,
        ),
        partition(
            "audit_events_202611",
            instant(0) - Duration::days(1),
            instant(0) + Duration::days(29),
            false,
            5,
        ),
    ]
}

#[tokio::test]
async fn a_sweep_drops_an_old_partition_and_batch_deletes_a_straddling_one() {
    let harness = harness();
    *harness.retention.partitions.lock().unwrap() = retention_partitions();
    *harness.retention.delete_batches.lock().unwrap() = vec![3, 0];

    let run = harness.state.retention.sweep().await.unwrap();
    assert_eq!(run.deleted_count(), 13);
    assert_eq!(run.state(), audit_domain::RetentionState::Success);
    assert_eq!(harness.retention.dropped.lock().unwrap().len(), 1);
    assert!(harness
        .events
        .stored()
        .iter()
        .any(|event| event.action() == "system.retention.run"));
}

#[tokio::test]
async fn a_sweep_sweeps_expired_idempotency_rows() {
    let harness = harness();
    *harness.retention.expired_idempotency.lock().unwrap() = 3;
    let run = harness.state.retention.sweep().await.unwrap();
    assert_eq!(run.state(), audit_domain::RetentionState::Success);
    assert_eq!(
        *harness.retention.expired_idempotency.lock().unwrap(),
        0,
        "the sweep consumed the expired rows"
    );
}

#[tokio::test]
async fn a_sweep_keeps_an_in_window_partition() {
    let harness = harness();
    *harness.retention.partitions.lock().unwrap() = vec![partition(
        "audit_events_202611",
        instant(0) - Duration::days(1),
        instant(0) + Duration::days(29),
        false,
        5,
    )];
    let run = harness.state.retention.sweep().await.unwrap();
    assert_eq!(run.deleted_count(), 0);
    assert!(harness.retention.dropped.lock().unwrap().is_empty());
}

#[tokio::test]
async fn a_sweep_that_cannot_open_a_run_is_audited() {
    let harness = harness();
    *harness.retention.fail_insert.lock().unwrap() =
        Some(RepositoryError::Unavailable("no insert grant".to_string()));
    let error = harness.state.retention.sweep().await.unwrap_err();
    assert!(matches!(
        error,
        audit_application::retention::RetentionError::CouldNotStart(_)
    ));
    assert!(harness.events.stored().iter().any(
        |event| event.action() == "system.retention.run" && event.outcome() == Outcome::Failed
    ));
}

#[tokio::test]
async fn a_sweep_failure_records_the_run_as_failed() {
    let harness = harness();
    *harness.retention.fail_partitions.lock().unwrap() = Some(RepositoryError::Unavailable(
        "catalog unavailable".to_string(),
    ));
    let error = harness.state.retention.sweep().await.unwrap_err();
    assert!(matches!(
        error,
        audit_application::retention::RetentionError::RunFailed(_)
    ));
    let runs = harness.retention.stored();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].state(), audit_domain::RetentionState::Failed);
    assert!(runs[0].failure_reason().is_some());
}

fn report_command() -> ReportCommand {
    ReportCommand {
        category: "reported".to_string(),
        owner: "ledger".to_string(),
        cutoff_at: instant(0) - Duration::days(730),
        started_at: instant(7),
        finished_at: instant(8),
        deleted_count: 120,
        privileged_role: "ledger_retention".to_string(),
        outcome: "success".to_string(),
        failure_reason: None,
    }
}

#[tokio::test]
async fn report_records_a_reported_run() {
    let harness = harness();
    let run = harness
        .state
        .retention
        .report("ledger", report_command())
        .await
        .unwrap();
    assert_eq!(run.owner(), audit_domain::RetentionOwner::Ledger);
    assert_eq!(run.deleted_count(), 120);
    assert!(harness
        .events
        .stored()
        .iter()
        .any(|event| event.action() == "system.retention.reported"));
}

#[tokio::test]
async fn a_report_without_a_role_is_refused_and_audited() {
    let harness = harness();
    let mut command = report_command();
    command.privileged_role = "  ".to_string();
    let error = harness
        .state
        .retention
        .report("ledger", command)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        audit_application::retention::RetentionError::Rejected(_)
    ));
    assert!(harness
        .events
        .stored()
        .iter()
        .any(|event| event.action() == "system.retention.reported"
            && event.outcome() == Outcome::Denied));
}

#[tokio::test]
async fn a_service_may_not_report_another_services_run() {
    let harness = harness();
    let error = harness
        .state
        .retention
        .report("gateway", report_command())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        audit_application::retention::RetentionError::Forbidden(_)
    ));
}

fn deletion_command() -> RequestCommand {
    RequestCommand {
        deletion_request_id: DELETION_REQUEST.to_string(),
        subject_user_id: SUBJECT_USER.to_string(),
    }
}

#[tokio::test]
async fn request_records_and_audits_a_deletion() {
    let harness = harness();
    let deletion = harness
        .state
        .privacy
        .request("identity", deletion_command())
        .await
        .unwrap();
    assert_eq!(deletion.state(), PrivacyDeletionState::Requested);
    assert!(harness
        .events
        .stored()
        .iter()
        .any(|event| event.action() == "system.privacy.deletion"));
}

#[tokio::test]
async fn a_repeated_request_returns_the_recorded_row() {
    let harness = harness();
    let first = harness
        .state
        .privacy
        .request("identity", deletion_command())
        .await
        .unwrap();
    let second = harness
        .state
        .privacy
        .request("identity", deletion_command())
        .await
        .unwrap();
    assert_eq!(first.id(), second.id());
    assert_eq!(harness.privacy.stored().len(), 1);
}

#[tokio::test]
async fn execute_deletes_unlabelled_records_and_keeps_a_legal_basis() {
    let harness = harness();
    seed_labels(&harness, UserId::parse(SUBJECT_USER).unwrap());
    harness
        .state
        .privacy
        .request("identity", deletion_command())
        .await
        .unwrap();
    let done = harness.state.privacy.execute_pending(10).await.unwrap();
    assert_eq!(done.len(), 1);
    assert_eq!(done[0].state(), PrivacyDeletionState::Completed);
    assert_eq!(done[0].deleted_count(), 1);
    assert_eq!(done[0].kept_with_legal_basis(), 1);
}

fn seed_labels(harness: &Harness, subject: UserId) {
    let labels = vec![
        EventLabel {
            id: AuditEventId::generate(),
            legal_basis: None,
        },
        EventLabel {
            id: AuditEventId::generate(),
            legal_basis: Some("legal-obligation".to_string()),
        },
    ];
    harness
        .privacy
        .labels
        .lock()
        .unwrap()
        .insert(subject.to_string(), labels);
}

#[tokio::test]
async fn a_count_difference_is_recorded_on_the_row_instead_of_failing_after_the_delete() {
    let harness = harness();
    seed_labels(&harness, UserId::parse(SUBJECT_USER).unwrap());
    *harness.privacy.delete_count.lock().unwrap() = Some(0);
    harness
        .state
        .privacy
        .request("identity", deletion_command())
        .await
        .unwrap();
    let done = harness.state.privacy.execute_pending(10).await.unwrap();
    assert_eq!(done.len(), 1);
    assert_eq!(done[0].state(), PrivacyDeletionState::Completed);
    assert_eq!(
        done[0].deleted_count(),
        0,
        "the row records what was actually deleted"
    );
    assert_eq!(done[0].kept_with_legal_basis(), 1);
}

#[tokio::test]
async fn a_non_identity_service_cannot_request_a_deletion() {
    let harness = harness();
    let error = harness
        .state
        .privacy
        .request("ledger", deletion_command())
        .await
        .unwrap_err();
    assert!(matches!(error, PrivacyError::Forbidden(_)));
    assert!(
        harness.privacy.stored().is_empty(),
        "no deletion is recorded for a caller that may not start one"
    );
    assert!(harness.events.stored().iter().any(|event| {
        event.action() == "system.privacy.deletion" && event.outcome() == Outcome::Denied
    }));
}

#[tokio::test]
async fn one_failed_deletion_does_not_stop_the_rest() {
    let harness = harness();
    harness
        .state
        .privacy
        .request("identity", deletion_command())
        .await
        .unwrap();
    harness
        .state
        .privacy
        .request(
            "identity",
            RequestCommand {
                deletion_request_id: "77777777-7777-4777-8777-777777777777".to_string(),
                subject_user_id: USER_SUBJECT.to_string(),
            },
        )
        .await
        .unwrap();
    *harness.privacy.fail_labels.lock().unwrap() =
        Some(RepositoryError::Unavailable("scan failed".to_string()));

    let done = harness.state.privacy.execute_pending(10).await.unwrap();
    assert_eq!(
        done.len(),
        1,
        "the second request is executed after the first fails"
    );
}
