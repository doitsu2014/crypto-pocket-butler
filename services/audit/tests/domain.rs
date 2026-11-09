//! Domain unit tests: the append-only event, the secret detector, support-session
//! state, retention windows, exports, and privacy classification.

use std::collections::BTreeMap;

use audit_domain::{
    classify, cutoff_six_calendar_months_before, detect_secret, partition_action, ActorKind,
    AuditEvent, AuditEventId, AuditExport, AuditExportId, Context, DeletionRequestId, DomainError,
    EventClass, EventLabel, ExportFilter, ExportOutcome, ExportState, NewAuditEvent, Outcome,
    PartitionAction, PartitionSummary, PrivacyDeletion, PrivacyDeletionId, RetentionOutcome,
    RetentionOwner, RetentionRun, RetentionRunId, SecretKind, Sensitivity, SupportDenial,
    SupportScope, SupportSession, SupportSessionId, SupportSessionOutcome, SupportSessionRequest,
    SupportSessionState, UserId,
};
use chrono::{TimeZone, Utc};

fn instant(year: i32, month: u32, day: u32, hour: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, day, hour, 0, 0).unwrap()
}

fn user(raw: &str) -> UserId {
    UserId::parse(raw).unwrap()
}

fn base_event() -> NewAuditEvent {
    NewAuditEvent {
        actor_id: Some(user("11111111-1111-4111-8111-111111111111")),
        actor_kind: ActorKind::User,
        action: "asset.contract.added".to_string(),
        target: "asset-usdc".to_string(),
        occurred_at: Some(instant(2026, 11, 6, 11)),
        context: Context::empty(),
        outcome: Outcome::Success,
        sensitivity: Sensitivity::Routine,
        event_class: None,
        idempotency_key: Some("retry-key-1".to_string()),
        legal_basis: None,
        deletion_request_id: None,
    }
}

fn record(new: NewAuditEvent) -> AuditEvent {
    AuditEvent::record(new, AuditEventId::generate(), instant(2026, 11, 6, 12)).unwrap()
}

#[test]
fn accepts_a_complete_event_and_derives_its_class() {
    let routine = record(base_event());
    assert_eq!(routine.event_class(), EventClass::Domain);
    assert_eq!(routine.recorded_at(), instant(2026, 11, 6, 12));

    let mut security = base_event();
    security.sensitivity = Sensitivity::Security;
    assert_eq!(record(security).event_class(), EventClass::Security);
}

#[test]
fn an_absent_occurred_at_is_recorded_as_the_receipt_time() {
    let mut new = base_event();
    new.occurred_at = None;
    assert_eq!(record(new).occurred_at(), instant(2026, 11, 6, 12));
}

#[test]
fn rejects_a_missing_target_with_its_field_path() {
    let mut new = base_event();
    new.target = "   ".to_string();
    let error =
        AuditEvent::record(new, AuditEventId::generate(), instant(2026, 11, 6, 12)).unwrap_err();
    assert_eq!(error, DomainError::Required { field: "target" });
    assert_eq!(error.field(), "target");
}

#[test]
fn rejects_a_secret_shaped_target_without_echoing_it() {
    let address = "0x1111111111111111111111111111111111111111";
    let mut new = base_event();
    new.target = address.to_string();
    let error =
        AuditEvent::record(new, AuditEventId::generate(), instant(2026, 11, 6, 12)).unwrap_err();
    assert_eq!(error, DomainError::SecretShaped { field: "target" });
    assert!(
        !error.to_string().contains(address),
        "the refusal must never repeat the value"
    );
}

#[test]
fn rejects_a_system_event_outside_the_allow_list() {
    let mut new = base_event();
    new.actor_id = None;
    new.actor_kind = ActorKind::System;
    new.action = "asset.created".to_string();
    let error =
        AuditEvent::record(new, AuditEventId::generate(), instant(2026, 11, 6, 12)).unwrap_err();
    assert_eq!(
        error,
        DomainError::Contradiction {
            field: "action",
            reason: "a system-attributed event must use an allow-listed action".to_string(),
        }
    );
}

#[test]
fn accepts_an_allow_listed_system_event() {
    let mut new = base_event();
    new.actor_id = None;
    new.actor_kind = ActorKind::System;
    new.action = "system.smoke.check".to_string();
    assert_eq!(record(new).actor_kind(), ActorKind::System);
}

#[test]
fn rejects_a_system_event_carrying_an_actor_id() {
    let mut new = base_event();
    new.actor_kind = ActorKind::System;
    new.action = "system.smoke.check".to_string();
    let error =
        AuditEvent::record(new, AuditEventId::generate(), instant(2026, 11, 6, 12)).unwrap_err();
    assert_eq!(
        error,
        DomainError::Contradiction {
            field: "actorId",
            reason: "a system event carries no actorId".to_string(),
        }
    );
}

#[test]
fn rejects_a_downgraded_event_class() {
    let mut new = base_event();
    new.sensitivity = Sensitivity::Security;
    new.event_class = Some(EventClass::Domain);
    let error =
        AuditEvent::record(new, AuditEventId::generate(), instant(2026, 11, 6, 12)).unwrap_err();
    assert_eq!(error.field(), "eventClass");
}

#[test]
fn rejects_an_unknown_context_key_and_too_many_keys() {
    let mut unknown = BTreeMap::new();
    unknown.insert("payload".to_string(), "a whole record".to_string());
    let error = Context::parse(unknown).unwrap_err();
    assert!(matches!(error, DomainError::ContextKeyNotAllowed { .. }));

    let mut many = BTreeMap::new();
    for key in [
        "correlationId",
        "requestId",
        "sourceService",
        "ruleId",
        "sessionId",
        "runId",
    ] {
        many.insert(key.to_string(), "value".to_string());
    }
    assert_eq!(
        Context::parse(many).unwrap_err(),
        DomainError::TooLarge {
            field: "context",
            limit: "5 keys",
        }
    );
}

#[test]
fn rejects_an_oversized_context() {
    let mut map = BTreeMap::new();
    map.insert("correlationId".to_string(), "x".repeat(2_100));
    assert!(matches!(
        Context::parse(map).unwrap_err(),
        DomainError::TooLarge {
            field: "context",
            ..
        }
    ));
}

#[test]
fn detects_address_token_key_seed_and_connection_shapes() {
    assert_eq!(
        detect_secret("0x1111111111111111111111111111111111111111"),
        Some(SecretKind::EvmAddress)
    );
    assert_eq!(
        detect_secret("eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiJ1c2VyLTEifQ.c2lnbmF0dXJl"),
        Some(SecretKind::Jwt)
    );
    assert_eq!(
        detect_secret("cpb_live_abcdefghijklmnop"),
        Some(SecretKind::ApiKey)
    );
    assert_eq!(
        detect_secret(
            "abandon ability able about above absent absorb abstract absurd abuse access accident"
        ),
        Some(SecretKind::SeedPhrase)
    );
    assert_eq!(
        detect_secret("postgres://user:secretpassword@db.internal/audit"),
        Some(SecretKind::ConnectionString)
    );
}

#[test]
fn the_seed_detector_uses_the_full_bip39_word_list() {
    assert_eq!(
        audit_domain::seed_word_count(),
        2048,
        "the detector must carry the whole official list, not a slice"
    );
    assert_eq!(
        detect_secret("zoo zone zero youth young you yellow year wrong wrist write wreck"),
        Some(SecretKind::SeedPhrase),
        "words outside an alphabetical slice must still be recognised"
    );
}

#[test]
fn does_not_flag_plain_identifiers_or_actions() {
    assert_eq!(detect_secret("asset-usdc"), None);
    assert_eq!(detect_secret("asset.contract.added"), None);
    assert_eq!(detect_secret("11111111-1111-4111-8111-111111111111"), None);
    assert_eq!(detect_secret("GET /v1/events"), None);
}

const AGENT: &str = "22222222-2222-4222-8222-222222222222";
const APPROVER: &str = "44444444-4444-4444-8444-444444444444";

fn request(expires_at: chrono::DateTime<Utc>, reason: &str) -> SupportSessionRequest {
    SupportSessionRequest::new(
        user(AGENT),
        user("33333333-3333-4333-8333-333333333333"),
        reason,
        user(APPROVER),
        SupportScope::AccountMetadata,
        instant(2026, 11, 6, 12),
        expires_at,
    )
    .unwrap()
}

fn active_session() -> SupportSession {
    SupportSession::request(
        request(
            instant(2026, 11, 6, 18),
            "customer reported a billing error",
        ),
        SupportSessionId::generate(),
    )
    .approve(user(APPROVER), instant(2026, 11, 6, 12))
    .unwrap()
}

#[test]
fn a_requested_session_becomes_active_only_when_the_named_approver_approves() {
    let requested = SupportSession::request(
        request(
            instant(2026, 11, 6, 18),
            "customer reported a billing error",
        ),
        SupportSessionId::generate(),
    );
    assert_eq!(requested.stored_state(), SupportSessionState::Requested);
    assert_eq!(
        requested.outcome(),
        None,
        "a requested session has no terminal outcome"
    );
    assert!(!requested.is_active_at(instant(2026, 11, 6, 12)));

    let error = requested
        .clone()
        .approve(user(AGENT), instant(2026, 11, 6, 12))
        .unwrap_err();
    assert_eq!(
        error,
        DomainError::Contradiction {
            field: "secondApproverId",
            reason: "only the named second approver may approve the session".to_string(),
        },
        "the requesting agent cannot approve their own session"
    );

    let active = requested
        .approve(user(APPROVER), instant(2026, 11, 6, 12))
        .unwrap();
    assert_eq!(active.stored_state(), SupportSessionState::Active);
    assert_eq!(active.outcome(), None, "approval is not a terminal outcome");
    assert_eq!(active.approved_at(), Some(instant(2026, 11, 6, 12)));
    assert!(active.is_active_at(instant(2026, 11, 6, 17)));
}

#[test]
fn denies_a_short_reason_and_stores_the_refusal() {
    let request = request(instant(2026, 11, 6, 18), "look");
    assert_eq!(request.check().unwrap_err(), SupportDenial::ReasonTooShort);
    let session = SupportSession::deny(request, SupportSessionId::generate());
    assert_eq!(session.stored_state(), SupportSessionState::Denied);
    assert_eq!(session.outcome(), Some(SupportSessionOutcome::Denied));
}

#[test]
fn denies_the_agents_own_approval() {
    let agent = user("22222222-2222-4222-8222-222222222222");
    let request = SupportSessionRequest::new(
        agent,
        user("33333333-3333-4333-8333-333333333333"),
        "customer reported a billing error",
        agent,
        SupportScope::Tickets,
        instant(2026, 11, 6, 12),
        instant(2026, 11, 6, 13),
    )
    .unwrap();
    let error = request.check().unwrap_err();
    assert_eq!(error, SupportDenial::ApproverIsAgent);
    assert_eq!(error.field(), "secondApproverId");
}

#[test]
fn allows_exactly_eight_hours_and_denies_one_second_more() {
    assert_eq!(
        request(
            instant(2026, 11, 6, 20),
            "customer reported a billing error"
        )
        .check(),
        Ok(())
    );
    let error = request(
        instant(2026, 11, 6, 20) + chrono::Duration::seconds(1),
        "customer reported a billing error",
    )
    .check()
    .unwrap_err();
    assert_eq!(error, SupportDenial::ExpiryBeyondWindow);
}

#[test]
fn expires_one_second_after_the_window_but_not_at_it() {
    let session = active_session();
    assert!(session.is_active_at(instant(2026, 11, 6, 18)));
    assert!(!session.is_active_at(instant(2026, 11, 6, 18) + chrono::Duration::seconds(1)));
    assert_eq!(
        session.state_at(instant(2026, 11, 6, 18) + chrono::Duration::seconds(1)),
        SupportSessionState::Expired
    );
}

#[test]
fn revokes_an_active_session_but_refuses_an_expired_one() {
    let active = active_session();
    let revoked = active.clone().revoke(instant(2026, 11, 6, 13)).unwrap();
    assert_eq!(revoked.stored_state(), SupportSessionState::Revoked);
    assert_eq!(revoked.ended_at(), Some(instant(2026, 11, 6, 13)));

    let error = active.revoke(instant(2026, 11, 6, 19)).unwrap_err();
    assert!(matches!(error, DomainError::InvalidTransition { .. }));
}

#[test]
fn cutoff_is_six_calendar_months_and_clamps_to_month_end() {
    assert_eq!(
        cutoff_six_calendar_months_before(instant(2026, 11, 6, 12)),
        instant(2026, 5, 6, 12)
    );
    assert_eq!(
        cutoff_six_calendar_months_before(instant(2026, 8, 31, 9)),
        instant(2026, 2, 28, 9)
    );
    assert_eq!(
        cutoff_six_calendar_months_before(instant(2024, 8, 31, 9)),
        instant(2024, 2, 29, 9),
        "a leap year clamps to the 29th"
    );
}

fn partition(
    from: chrono::DateTime<Utc>,
    to: chrono::DateTime<Utc>,
    held: bool,
) -> PartitionSummary {
    PartitionSummary {
        name: "audit_events_202601".to_string(),
        from,
        to,
        holds_legal_hold: held,
        row_count: 10,
    }
}

#[test]
fn drops_an_entirely_old_partition() {
    let cutoff = instant(2026, 5, 6, 12);
    assert_eq!(
        partition_action(
            &partition(instant(2026, 4, 1, 0), instant(2026, 5, 1, 0), false),
            cutoff
        ),
        PartitionAction::Drop
    );
}

#[test]
fn batch_deletes_a_straddling_partition_and_respects_a_legal_hold() {
    let cutoff = instant(2026, 5, 6, 12);
    assert_eq!(
        partition_action(
            &partition(instant(2026, 5, 1, 0), instant(2026, 6, 1, 0), false),
            cutoff
        ),
        PartitionAction::BatchDelete
    );
    assert_eq!(
        partition_action(
            &partition(instant(2026, 4, 1, 0), instant(2026, 5, 1, 0), true),
            cutoff
        ),
        PartitionAction::BatchDelete,
        "an old partition holding a legally held row is never dropped whole"
    );
    assert_eq!(
        partition_action(
            &partition(instant(2026, 6, 1, 0), instant(2026, 7, 1, 0), false),
            cutoff
        ),
        PartitionAction::Keep
    );
}

fn running_run() -> RetentionRun {
    RetentionRun::start(
        RetentionRunId::generate(),
        instant(2026, 5, 6, 0),
        instant(2026, 11, 6, 0),
        "audit_retention",
    )
    .unwrap()
}

#[test]
fn finishes_a_run_once_and_refuses_a_second_finish() {
    let finished = running_run()
        .finish_success(42, instant(2026, 11, 6, 1))
        .unwrap();
    assert_eq!(finished.state(), audit_domain::RetentionState::Success);
    assert_eq!(finished.deleted_count(), 42);
    assert!(finished
        .clone()
        .finish_success(1, instant(2026, 11, 6, 2))
        .is_err());
}

#[test]
fn a_failed_run_redacts_a_secret_shaped_reason() {
    let failed = running_run()
        .finish_failed(
            "connect postgres://user:secretpassword@db failed",
            instant(2026, 11, 6, 1),
        )
        .unwrap();
    let reason = failed.failure_reason().unwrap();
    assert!(!reason.contains("secretpassword"));
}

#[test]
fn a_reported_run_requires_a_role_and_cannot_claim_audit_owner() {
    let error = RetentionRun::reported(
        RetentionRunId::generate(),
        RetentionOwner::Ledger,
        instant(2026, 1, 1, 0),
        instant(2026, 2, 1, 0),
        instant(2026, 2, 1, 1),
        5,
        "  ",
        RetentionOutcome::Success,
        None,
    )
    .unwrap_err();
    assert_eq!(
        error,
        DomainError::Required {
            field: "privilegedRole"
        }
    );

    let error = RetentionRun::reported(
        RetentionRunId::generate(),
        RetentionOwner::Audit,
        instant(2026, 1, 1, 0),
        instant(2026, 2, 1, 0),
        instant(2026, 2, 1, 1),
        5,
        "audit_retention",
        RetentionOutcome::Success,
        None,
    )
    .unwrap_err();
    assert_eq!(error.field(), "owner");
}

fn filter() -> ExportFilter {
    ExportFilter::new(
        vec!["asset.".to_string()],
        None,
        None,
        instant(2026, 1, 1, 0),
        instant(2026, 11, 1, 0),
        None,
        None,
    )
    .unwrap()
}

#[test]
fn an_export_filter_requires_a_forward_recorded_range() {
    let error = ExportFilter::new(
        Vec::new(),
        None,
        None,
        instant(2026, 11, 1, 0),
        instant(2026, 1, 1, 0),
        None,
        None,
    )
    .unwrap_err();
    assert_eq!(error.field(), "recordedFrom");
}

#[test]
fn an_export_requires_a_hex_digest_and_is_immutable_afterwards() {
    let export = AuditExport::requested(
        AuditExportId::generate(),
        user("11111111-1111-4111-8111-111111111111"),
        filter(),
        instant(2026, 11, 6, 12),
    );
    assert!(export
        .clone()
        .succeed(1, "not-a-digest", instant(2026, 11, 6, 12))
        .is_err());

    let succeeded = export
        .clone()
        .succeed(3, &"a".repeat(64), instant(2026, 11, 6, 13))
        .unwrap();
    assert_eq!(succeeded.outcome(), Some(ExportOutcome::Success));
    assert!(succeeded
        .clone()
        .fail("too late", instant(2026, 11, 6, 14))
        .is_err());
}

#[test]
fn a_failed_export_records_a_redacted_reason() {
    let export = AuditExport::requested(
        AuditExportId::generate(),
        user("11111111-1111-4111-8111-111111111111"),
        filter(),
        instant(2026, 11, 6, 12),
    );
    let failed = export
        .fail(
            "postgres://user:secretpassword@db.internal/audit",
            instant(2026, 11, 6, 13),
        )
        .unwrap();
    assert_eq!(failed.state(), ExportState::Failed);
    let reason = failed.failure_reason().expect("a failure reason is stored");
    assert!(!reason.contains("secretpassword"));
}

#[test]
fn classifies_deleted_kept_and_unlabelled_kept_records() {
    let labels = vec![
        EventLabel {
            id: AuditEventId::generate(),
            legal_basis: None,
        },
        EventLabel {
            id: AuditEventId::generate(),
            legal_basis: Some("legitimate-interest".to_string()),
        },
        EventLabel {
            id: AuditEventId::generate(),
            legal_basis: Some(String::new()),
        },
    ];
    let report = classify(&labels);
    assert_eq!(report.deleted, 1);
    assert_eq!(report.kept_with_legal_basis, 2);
    assert_eq!(report.defects, 1);
}

#[test]
fn a_completed_privacy_deletion_cannot_be_completed_again() {
    let deletion = PrivacyDeletion::requested(
        PrivacyDeletionId::generate(),
        DeletionRequestId::parse("66666666-6666-4666-8666-666666666666").unwrap(),
        user("33333333-3333-4333-8333-333333333333"),
        instant(2026, 11, 6, 12),
    );
    let completed = deletion
        .clone()
        .complete(
            audit_domain::DeletionReport::default(),
            instant(2026, 11, 6, 13),
        )
        .unwrap();
    assert!(completed.fail("late", instant(2026, 11, 6, 14)).is_err());
}
