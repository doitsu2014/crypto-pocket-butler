//! Domain rules of the account lifecycle: the status machine (BR5.1), the
//! 30-day midnight-UTC recovery window (BR5.3/BR5.4), the transition record
//! (BR5.9), and the erasure retry bound (BR5.5/BR5.6).

mod common;

use chrono::{Duration, TimeZone, Utc};
use identity_domain::deletion::{DeletionRecord, DeletionRequest, DeletionStatus};
use identity_domain::{
    backoff_minutes, is_permitted, optional_reason, recoverable_until, require_edge,
    validate_reason, DeletionRequestId, DomainError, StatusTransition, StatusTransitionId, User,
    UserId, UserStatus, MAX_ERASURE_ATTEMPTS, MAX_REASON_LEN, RECOVERY_WINDOW_DAYS,
};

const STATUSES: [UserStatus; 5] = [
    UserStatus::Pending,
    UserStatus::Active,
    UserStatus::Disabled,
    UserStatus::PendingDeletion,
    UserStatus::Deleted,
];

fn user_at(status: UserStatus, now: chrono::DateTime<Utc>) -> User {
    let mut user = common::active_user();
    if status != UserStatus::Active {
        // `from_stored` is the only cross-status constructor; reuse the stored
        // shape rather than adding a test-only setter.
        user = User::from_stored(
            user.id(),
            user.role(),
            status,
            user.display_name().map(ToString::to_string),
            user.display_name_overridden(),
            user.email().map(ToString::to_string),
            user.email_overridden(),
            user.email_verified(),
            user.locale(),
            user.base_currency(),
            user.timezone(),
            user.created_at(),
            now,
            user.anonymised_at(),
        );
    }
    user
}

#[test]
fn only_the_six_permitted_edges_are_accepted() {
    let permitted = [
        (UserStatus::Pending, UserStatus::Active),
        (UserStatus::Active, UserStatus::Disabled),
        (UserStatus::Active, UserStatus::PendingDeletion),
        (UserStatus::Disabled, UserStatus::Active),
        (UserStatus::PendingDeletion, UserStatus::Active),
        (UserStatus::PendingDeletion, UserStatus::Deleted),
    ];

    for from in STATUSES {
        for to in STATUSES {
            let expected = permitted.contains(&(from, to));
            assert_eq!(
                is_permitted(from, to),
                expected,
                "{} -> {}",
                from.as_str(),
                to.as_str()
            );
        }
    }

    assert!(require_edge(UserStatus::Deleted, UserStatus::Active).is_err());
    assert!(require_edge(UserStatus::Active, UserStatus::Deleted).is_err());
    assert!(require_edge(UserStatus::Disabled, UserStatus::PendingDeletion).is_err());
}

#[test]
fn a_user_moves_only_along_a_permitted_edge_and_records_the_change() {
    let now = Utc.with_ymd_and_hms(2026, 11, 11, 9, 0, 0).unwrap();
    let mut user = user_at(UserStatus::Active, now);

    user.transition(UserStatus::Disabled, now).unwrap();
    assert_eq!(user.status(), UserStatus::Disabled);
    assert_eq!(user.updated_at(), now);

    let refused = user.transition(UserStatus::PendingDeletion, now);
    assert!(matches!(
        refused,
        Err(DomainError::InvalidTransition { .. })
    ));
    assert_eq!(
        user.status(),
        UserStatus::Disabled,
        "a refused edge changes nothing"
    );
}

#[test]
fn the_recovery_window_closes_at_midnight_utc_on_the_thirtieth_calendar_day() {
    let requested = Utc.with_ymd_and_hms(2026, 1, 1, 14, 37, 12).unwrap();
    let deadline = recoverable_until(requested);
    assert_eq!(
        deadline,
        Utc.with_ymd_and_hms(2026, 1, 31, 0, 0, 0).unwrap()
    );
    assert_eq!(
        deadline - requested,
        Duration::days(30) - Duration::hours(14) - Duration::minutes(37) - Duration::seconds(12)
    );
    assert_eq!(RECOVERY_WINDOW_DAYS, 30);

    let request =
        DeletionRequest::request(DeletionRequestId::generate(), UserId::generate(), requested);
    assert_eq!(request.recoverable_until(), deadline);
    assert!(request.is_open());
    assert_eq!(request.attempt_count(), 0);
    assert_eq!(request.next_attempt_at(), requested);
}

#[test]
fn an_admin_reason_is_trimmed_bounded_and_optional_only_where_the_rule_allows() {
    assert_eq!(
        validate_reason("  account abuse  ").unwrap(),
        "account abuse"
    );
    assert!(matches!(
        validate_reason("   "),
        Err(DomainError::Required { field: "reason" })
    ));
    let long = "x".repeat(MAX_REASON_LEN + 1);
    assert!(matches!(
        validate_reason(&long),
        Err(DomainError::TooLarge {
            field: "reason",
            ..
        })
    ));
    assert_eq!(optional_reason(None).unwrap(), None);
    assert_eq!(optional_reason(Some("")).unwrap(), None);
    assert_eq!(
        optional_reason(Some(" restore ")).unwrap().as_deref(),
        Some("restore")
    );
}

#[test]
fn a_transition_refuses_a_blank_actor_or_an_illegal_edge() {
    let now = Utc.with_ymd_and_hms(2026, 11, 11, 9, 0, 0).unwrap();
    let user_id = UserId::generate();

    let blank_actor = StatusTransition::new(
        StatusTransitionId::generate(),
        user_id,
        UserStatus::Active,
        UserStatus::Disabled,
        "  ",
        Some("abuse"),
        now,
    );
    assert!(matches!(blank_actor, Err(DomainError::Required { .. })));

    let illegal = StatusTransition::new(
        StatusTransitionId::generate(),
        user_id,
        UserStatus::Deleted,
        UserStatus::Active,
        "admin-1",
        Some("restore"),
        now,
    );
    assert!(matches!(
        illegal,
        Err(DomainError::InvalidTransition { .. })
    ));

    let transition = StatusTransition::new(
        StatusTransitionId::generate(),
        user_id,
        UserStatus::Active,
        UserStatus::PendingDeletion,
        "user",
        None,
        now,
    )
    .unwrap();
    assert_eq!(transition.from_status(), UserStatus::Active);
    assert_eq!(transition.to_status(), UserStatus::PendingDeletion);
    assert_eq!(transition.actor(), "user");
    assert_eq!(transition.reason(), None);
    assert_eq!(transition.occurred_at(), now);
}

#[test]
fn the_erasure_backoff_doubles_from_one_minute_and_is_capped_at_an_hour() {
    assert_eq!(backoff_minutes(1), 1);
    assert_eq!(backoff_minutes(2), 2);
    assert_eq!(backoff_minutes(3), 4);
    assert_eq!(backoff_minutes(4), 8);
    assert_eq!(backoff_minutes(5), 16);
    assert_eq!(backoff_minutes(6), 32);
    assert_eq!(backoff_minutes(7), 60, "capped at one hour");
    assert_eq!(backoff_minutes(99), 60);
    assert_eq!(MAX_ERASURE_ATTEMPTS, 5);
}

#[test]
fn a_failed_attempt_backs_off_and_the_fifth_parks_the_request() {
    let now = Utc.with_ymd_and_hms(2026, 11, 11, 9, 0, 0).unwrap();
    let mut request =
        DeletionRequest::request(DeletionRequestId::generate(), UserId::generate(), now);

    let first = request.record_attempt_failure(now).unwrap();
    assert_eq!(first, 1);
    assert_eq!(request.next_attempt_at(), now + Duration::minutes(1));

    for _ in 0..(MAX_ERASURE_ATTEMPTS - 1) {
        request.record_attempt_failure(now).unwrap();
    }
    assert_eq!(request.attempt_count(), MAX_ERASURE_ATTEMPTS);
    assert!(
        !request.is_open(),
        "a parked request no longer blocks a new one"
    );
    assert_eq!(request.status(), DeletionStatus::Failed);
    assert!(request.cancelled_at().is_some());

    let parked = request.record_attempt_failure(now);
    assert!(
        parked.is_err(),
        "a parked request is not retried further by the counter"
    );
}

#[test]
fn cancelling_an_open_request_keeps_it_as_evidence_and_erasure_is_idempotent() {
    let now = Utc.with_ymd_and_hms(2026, 11, 11, 9, 0, 0).unwrap();
    let id = DeletionRequestId::generate();
    let user_id = UserId::generate();
    let mut request = DeletionRequest::request(id, user_id, now);

    request.cancel(now + Duration::days(3)).unwrap();
    assert!(!request.is_open());
    assert_eq!(request.cancelled_at(), Some(now + Duration::days(3)));
    assert_eq!(
        request.status(),
        DeletionStatus::Requested,
        "cancellation is recorded as its own fact; the row is evidence"
    );
    assert!(
        request.cancel(now + Duration::days(4)).is_err(),
        "a cancelled request cannot be cancelled twice"
    );
}

#[test]
fn a_record_round_trips_the_window_and_the_outcome() {
    let now = Utc.with_ymd_and_hms(2026, 11, 11, 9, 0, 0).unwrap();
    let record = DeletionRecord {
        id: DeletionRequestId::generate(),
        user_id: UserId::generate(),
        status: DeletionStatus::Completed,
        progress: identity_domain::DeletionProgress::default(),
        requested_at: now,
        retained_categories: Vec::new(),
        completed_at: Some(now),
        failure_reason: None,
        lease_owner: None,
        lease_until: None,
        recoverable_until: recoverable_until(now),
        cancelled_at: None,
        erased_at: Some(now),
        attempt_count: 0,
        next_attempt_at: now,
    };
    let request = DeletionRequest::from_stored(record);
    assert_eq!(request.erased_at(), Some(now));
    assert_eq!(request.recoverable_until(), recoverable_until(now));
    assert!(!request.is_open());
}
