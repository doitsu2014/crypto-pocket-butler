//! Support-grant domain rules: the request shape, the approval window, the
//! read-time clock decisions and the caller binding (BR3.1–BR3.8).
//!
//! Every case supplies its own instant; nothing reads the wall clock.

use chrono::{Duration, TimeZone, Utc};
use identity_domain::claims::Role;
use identity_domain::error::DomainError;
use identity_domain::support::{
    SupportAccessGrant, SupportGrantId, SupportGrantStatus, DEFAULT_GRANT_MINUTES, MAX_REASON_LEN,
};
use identity_domain::UserId;

fn at(hour: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, hour, 0, 0).unwrap()
}

fn requested() -> (SupportAccessGrant, UserId) {
    let agent = UserId::generate();
    let grant = SupportAccessGrant::request(
        SupportGrantId::generate(),
        UserId::generate(),
        agent,
        "  investigate a reported sync failure  ",
        at(9),
    )
    .expect("a well-formed request");
    (grant, agent)
}

fn approved() -> SupportAccessGrant {
    let (mut grant, _) = requested();
    grant
        .approve(
            UserId::generate(),
            Role::Admin,
            at(9) + Duration::minutes(30),
            Duration::minutes(DEFAULT_GRANT_MINUTES),
        )
        .expect("a distinct admin approves inside the hour");
    grant
}

#[test]
fn a_request_trims_the_reason_and_refuses_a_blank_or_oversized_one() {
    let (grant, _) = requested();
    assert_eq!(grant.reason(), "investigate a reported sync failure");

    let blank = SupportAccessGrant::request(
        SupportGrantId::generate(),
        UserId::generate(),
        UserId::generate(),
        "   ",
        at(9),
    );
    assert!(matches!(
        blank,
        Err(DomainError::Required { field: "reason" })
    ));

    let long = "x".repeat(MAX_REASON_LEN + 1);
    let oversized = SupportAccessGrant::request(
        SupportGrantId::generate(),
        UserId::generate(),
        UserId::generate(),
        &long,
        at(9),
    );
    assert!(matches!(
        oversized,
        Err(DomainError::TooLarge {
            field: "reason",
            ..
        })
    ));
}

#[test]
fn a_request_cannot_name_the_requester_as_its_target() {
    let agent = UserId::generate();
    let refused = SupportAccessGrant::request(
        SupportGrantId::generate(),
        agent,
        agent,
        "read my own record",
        at(9),
    );
    assert!(matches!(
        refused,
        Err(DomainError::Invalid {
            field: "userId",
            ..
        })
    ));
}

#[test]
fn an_approval_window_outside_one_to_sixty_minutes_is_refused() {
    let (mut zero, _) = requested();
    assert!(matches!(
        zero.approve(
            UserId::generate(),
            Role::Admin,
            at(9) + Duration::minutes(30),
            Duration::minutes(0)
        ),
        Err(DomainError::Invalid {
            field: "expiresAt",
            ..
        })
    ));

    let (mut too_long, _) = requested();
    assert!(matches!(
        too_long.approve(
            UserId::generate(),
            Role::Admin,
            at(9) + Duration::minutes(30),
            Duration::minutes(DEFAULT_GRANT_MINUTES + 1)
        ),
        Err(DomainError::Invalid {
            field: "expiresAt",
            ..
        })
    ));
    assert_eq!(
        too_long.status(),
        SupportGrantStatus::PendingApproval,
        "a refused approval changes nothing"
    );
}

#[test]
fn an_approval_after_the_request_window_is_refused() {
    let (mut grant, _) = requested();
    assert!(
        !grant.approval_window_closed(at(9) + Duration::minutes(59)),
        "the request is still approvable inside the hour"
    );
    assert!(grant.approval_window_closed(at(9) + Duration::minutes(60)));
    assert!(matches!(
        grant.approve(
            UserId::generate(),
            Role::Admin,
            at(9) + Duration::minutes(60),
            Duration::minutes(DEFAULT_GRANT_MINUTES)
        ),
        Err(DomainError::Conflict { .. })
    ));
}

#[test]
fn a_pending_request_older_than_sixty_minutes_expires_without_an_approver() {
    let (mut grant, _) = requested();
    assert_eq!(
        grant.effective_status(at(9) + Duration::minutes(59)),
        SupportGrantStatus::PendingApproval
    );
    assert_eq!(
        grant.effective_status(at(9) + Duration::minutes(60)),
        SupportGrantStatus::Expired
    );
    assert!(grant.expire_if_due(at(9) + Duration::minutes(60)));
    assert_eq!(grant.status(), SupportGrantStatus::Expired);
    assert_eq!(grant.approved_by(), None);
    assert_eq!(grant.approved_at(), None);
    assert_eq!(grant.expires_at(), None);
    assert!(
        !grant.expire_if_due(at(9) + Duration::hours(2)),
        "the observation is persisted once"
    );
}

#[test]
fn an_active_grant_reads_as_expired_at_its_expiry_and_the_observation_persists() {
    let mut grant = approved();
    let expiry = at(9) + Duration::minutes(90);
    assert_eq!(grant.expires_at(), Some(expiry));
    assert_eq!(
        grant.effective_status(expiry - Duration::seconds(1)),
        SupportGrantStatus::Active
    );
    assert_eq!(grant.effective_status(expiry), SupportGrantStatus::Expired);
    assert!(grant.expire_if_due(expiry));
    assert_eq!(grant.status(), SupportGrantStatus::Expired);
    assert_eq!(
        grant.effective_status(at(10)),
        SupportGrantStatus::Expired,
        "a terminal state is never revived by an earlier clock"
    );
}

#[test]
fn only_the_requesting_agent_holds_the_grant() {
    let (grant, agent) = requested();
    assert!(grant.is_held_by(agent));
    assert!(!grant.is_held_by(UserId::generate()));
}

#[test]
fn an_approval_sets_the_window_from_the_approval_instant() {
    let grant = approved();
    let approved_at = at(9) + Duration::minutes(30);
    assert_eq!(grant.status(), SupportGrantStatus::Active);
    assert_eq!(
        grant.expires_at(),
        Some(approved_at + Duration::minutes(DEFAULT_GRANT_MINUTES))
    );
    assert!(grant.is_valid_at(approved_at + Duration::minutes(59), grant.user_id()));
    assert!(!grant.is_valid_at(
        approved_at + Duration::minutes(DEFAULT_GRANT_MINUTES),
        grant.user_id()
    ));
    assert!(
        !grant.is_valid_at(approved_at, UserId::generate()),
        "a grant never authorises another record"
    );
}

#[test]
fn a_revoked_grant_is_terminal_and_keeps_the_revocation_instant() {
    let mut grant = approved();
    grant.revoke(at(10) + Duration::minutes(5)).unwrap();
    assert_eq!(grant.status(), SupportGrantStatus::Revoked);
    assert_eq!(grant.revoked_at(), Some(at(10) + Duration::minutes(5)));
    assert!(!grant.is_valid_at(at(10) + Duration::minutes(6), grant.user_id()));
    assert!(matches!(
        grant.revoke(at(10) + Duration::minutes(7)),
        Err(DomainError::InvalidTransition { .. })
    ));
}
