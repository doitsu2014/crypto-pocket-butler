//! Domain rules from `functional-design/rules.md` and `functional-design/entities.md`.
//!
//! BR1.1 dedupe key shape, BR2.1–BR2.3 preferences, defaults, and suppression,
//! BR3.1 mandatory email, BR4.1 retry bounds, BR4.2 the address error category,
//! BR5.1 the content guard, and the twelve-month retention cutoff.

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, TimeZone, Utc};
use notification_domain::{
    alert_status, cutoff_twelve_months_before, default_enabled, effective_enabled, is_mandatory,
    is_sensitive, next_attempt_at, resolve_all, sanitise_payload, summarise, validate_change,
    Alert, AlertId, AlertStatus, AttemptStatus, Channel, ChannelOutcome, DeliveryAttempt,
    DeliveryAttemptId, DomainError, ErrorCategory, NotificationPreference, NotificationType,
    PreferenceId, UserId, MAX_ATTEMPTS, MAX_IDEMPOTENCY_KEY_LEN, MAX_PAYLOAD_VALUE_LEN,
};

const USER: &str = "11111111-1111-4111-8111-111111111111";
const USER_UUID: &str = USER;

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 8, hour, 0, 0).unwrap()
}

fn user() -> UserId {
    UserId::parse(USER).unwrap()
}

fn payload(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect()
}

// ---------------------------------------------------------------------------
// Notification types
// ---------------------------------------------------------------------------

#[test]
fn notification_types_round_trip_through_their_labels() {
    for kind in NotificationType::ALL {
        assert_eq!(NotificationType::parse(kind.as_str()), Ok(kind));
    }
}

#[test]
fn an_unknown_type_label_is_refused() {
    let error = NotificationType::parse("marketing").unwrap_err();
    assert_eq!(error.field(), "type");
}

#[test]
fn only_security_and_revoked_keys_force_an_email() {
    assert!(NotificationType::Security.is_mandatory_email());
    assert!(NotificationType::KeyRevoked.is_mandatory_email());
    assert!(!NotificationType::Drift.is_mandatory_email());
    assert!(!NotificationType::SyncFailed.is_mandatory_email());
    assert!(!NotificationType::Brief.is_mandatory_email());
}

#[test]
fn each_type_has_its_payload_allow_list() {
    assert_eq!(
        NotificationType::Drift.allowed_payload_fields(),
        &["portfolioId", "assetId", "driftPoints"]
    );
    assert_eq!(
        NotificationType::SyncFailed.allowed_payload_fields(),
        &["accountId", "reason"]
    );
    assert_eq!(
        NotificationType::KeyRevoked.allowed_payload_fields(),
        &["event", "deviceLabel"]
    );
    assert_eq!(
        NotificationType::Security.allowed_payload_fields(),
        &["event", "deviceLabel"]
    );
    assert_eq!(
        NotificationType::Brief.allowed_payload_fields(),
        &["briefId", "period", "asOf", "dataStatus", "hasUnpriced"]
    );
}

// ---------------------------------------------------------------------------
// BR5.1 content guard
// ---------------------------------------------------------------------------

#[test]
fn sanitise_keeps_allow_listed_identifiers() {
    // A UUID identifier is exactly what a drift alert must carry; it is not a
    // credential and must survive the guard (BR5.1).
    let sanitised = sanitise_payload(
        NotificationType::Drift,
        payload(&[
            ("portfolioId", "22222222-2222-4222-8222-222222222222"),
            ("assetId", "bitcoin"),
            ("driftPoints", "7.5"),
        ]),
    );
    assert_eq!(sanitised.fields.len(), 3);
    assert!(sanitised.dropped.is_empty());
    assert_eq!(
        sanitised.fields.get("assetId").map(String::as_str),
        Some("bitcoin")
    );
}

#[test]
fn sanitise_drops_an_email_and_records_the_field() {
    let sanitised = sanitise_payload(
        NotificationType::Security,
        payload(&[("event", "alice@example.com"), ("deviceLabel", "Chrome")]),
    );
    assert_eq!(sanitised.dropped, vec!["event".to_string()]);
    assert_eq!(
        sanitised.fields.get("deviceLabel").map(String::as_str),
        Some("Chrome")
    );
}

#[test]
fn sanitise_drops_a_full_evm_address() {
    let value = format!("0x{}", "a".repeat(40));
    let sanitised = sanitise_payload(
        NotificationType::SyncFailed,
        payload(&[
            ("accountId", "33333333-3333-4333-8333-333333333333"),
            ("reason", &value),
        ]),
    );
    assert_eq!(sanitised.dropped, vec!["reason".to_string()]);
    assert!(!sanitised.fields.contains_key("reason"));
}

#[test]
fn sanitise_drops_a_base58_key() {
    let value = "5HueCGU8rMjxEXxiPuD5BDku4MkFqeZyd4dZ1jvhTVqvbTLvyTJ";
    let sanitised = sanitise_payload(NotificationType::SyncFailed, payload(&[("reason", value)]));
    assert_eq!(sanitised.dropped, vec!["reason".to_string()]);
}

#[test]
fn sanitise_drops_a_bearer_token() {
    let sanitised = sanitise_payload(
        NotificationType::Security,
        payload(&[("deviceLabel", "Bearer abc123def456")]),
    );
    assert_eq!(sanitised.dropped, vec!["deviceLabel".to_string()]);
}

#[test]
fn sanitise_drops_a_prefixed_api_key() {
    let sanitised = sanitise_payload(
        NotificationType::KeyRevoked,
        payload(&[("event", "sk-live-51H8xQ2example")]),
    );
    assert_eq!(sanitised.dropped, vec!["event".to_string()]);
}

#[test]
fn sanitise_drops_ip_literals() {
    let sanitised = sanitise_payload(
        NotificationType::Security,
        payload(&[("deviceLabel", "203.0.113.7"), ("event", "2001:db8::1")]),
    );
    assert_eq!(
        sanitised.dropped,
        vec!["deviceLabel".to_string(), "event".to_string()]
    );
}

#[test]
fn sanitise_drops_fields_outside_the_allow_list() {
    let sanitised = sanitise_payload(
        NotificationType::Brief,
        payload(&[
            ("briefId", "44444444-4444-4444-8444-444444444444"),
            ("email", "x@y.z"),
        ]),
    );
    assert_eq!(sanitised.fields.len(), 1);
    assert!(sanitised.fields.contains_key("briefId"));
    // An unknown key is not stored but is not a credential drop either.
    assert!(sanitised.dropped.is_empty());
}

#[test]
fn sanitise_drops_an_over_long_value() {
    let long = "a".repeat(MAX_PAYLOAD_VALUE_LEN + 1);
    let sanitised = sanitise_payload(NotificationType::SyncFailed, payload(&[("reason", &long)]));
    assert!(sanitised.fields.is_empty());
}

#[test]
fn is_sensitive_leaves_plain_labels_alone() {
    assert!(!is_sensitive("New device signed in"));
    assert!(!is_sensitive("Chrome on Windows"));
    assert!(!is_sensitive("7.5"));
    assert!(!is_sensitive(USER_UUID));
}

// ---------------------------------------------------------------------------
// BR1.1 the stored alert
// ---------------------------------------------------------------------------

fn queued_alert(kind: NotificationType, key: &str) -> Alert {
    let sanitised = sanitise_payload(kind, payload(&[]));
    Alert::queue(AlertId::generate(), user(), kind, key, sanitised, at(9))
        .expect("a well-formed alert is accepted")
}

#[test]
fn alert_requires_a_non_blank_idempotency_key() {
    let sanitised = sanitise_payload(NotificationType::Drift, payload(&[]));
    let error = Alert::queue(
        AlertId::generate(),
        user(),
        NotificationType::Drift,
        "   ",
        sanitised,
        at(9),
    )
    .unwrap_err();
    assert_eq!(error.field(), "idempotencyKey");
}

#[test]
fn alert_refuses_an_over_long_idempotency_key() {
    let key = "k".repeat(MAX_IDEMPOTENCY_KEY_LEN + 1);
    let sanitised = sanitise_payload(NotificationType::Drift, payload(&[]));
    let error = Alert::queue(
        AlertId::generate(),
        user(),
        NotificationType::Drift,
        &key,
        sanitised,
        at(9),
    )
    .unwrap_err();
    assert_eq!(error.field(), "idempotencyKey");
}

#[test]
fn alert_refuses_a_non_uuid_user() {
    let error = UserId::parse("not-a-uuid").unwrap_err();
    assert!(matches!(error, DomainError::InvalidId { .. }));

    let sanitised = sanitise_payload(NotificationType::Drift, payload(&[]));
    let error = Alert::queue(
        AlertId::generate(),
        UserId::new(uuid::Uuid::nil()),
        NotificationType::Drift,
        "key",
        sanitised,
        at(9),
    )
    .unwrap_err();
    // The nil UUID is a valid UUID but an impossible user reference.
    assert_eq!(error.field(), "userId");
}

#[test]
fn a_queued_alert_is_due_immediately() {
    let alert = queued_alert(NotificationType::Drift, "drift:1");
    assert_eq!(alert.status(), AlertStatus::Queued);
    assert_eq!(alert.next_attempt_at(), Some(at(9)));
    assert_eq!(alert.created_at(), at(9));
}

#[test]
fn alert_refuses_an_unknown_stored_status() {
    let error = Alert::from_stored(
        AlertId::generate(),
        user(),
        NotificationType::Drift,
        "drift:1",
        BTreeMap::new(),
        "exploded",
        Vec::new(),
        at(9),
        Some(at(9)),
    )
    .unwrap_err();
    assert_eq!(error.field(), "status");
}

// ---------------------------------------------------------------------------
// BR2.1–BR2.3 preferences
// ---------------------------------------------------------------------------

#[test]
fn in_app_defaults_on_and_email_defaults_off_except_for_brief() {
    for kind in NotificationType::ALL {
        assert!(default_enabled(kind, Channel::InApp));
        assert_eq!(
            default_enabled(kind, Channel::Email),
            kind == NotificationType::Brief
        );
    }
}

#[test]
fn mandatory_email_cannot_be_disabled_even_when_stored_off() {
    let stored = NotificationPreference::stored(
        PreferenceId::generate(),
        user(),
        NotificationType::Security,
        Channel::Email,
        false,
        at(9),
    );
    assert!(is_mandatory(NotificationType::Security, Channel::Email));
    assert!(effective_enabled(
        NotificationType::Security,
        Channel::Email,
        Some(stored.enabled())
    ));
}

#[test]
fn stored_disabled_email_is_honoured_for_drift() {
    assert!(!effective_enabled(
        NotificationType::Drift,
        Channel::Email,
        Some(false)
    ));
    assert!(effective_enabled(
        NotificationType::Drift,
        Channel::Email,
        Some(true)
    ));
}

#[test]
fn resolved_view_covers_every_type_and_channel() {
    let rows = vec![NotificationPreference::stored(
        PreferenceId::generate(),
        user(),
        NotificationType::Drift,
        Channel::Email,
        true,
        at(9),
    )];
    let resolved = resolve_all(user(), &rows);
    assert_eq!(
        resolved.len(),
        NotificationType::ALL.len() * Channel::ALL.len()
    );
    let drift_email = resolved
        .iter()
        .find(|view| view.kind == NotificationType::Drift && view.channel == Channel::Email)
        .unwrap();
    assert!(drift_email.enabled);
    let sync_in_app = resolved
        .iter()
        .find(|view| view.kind == NotificationType::SyncFailed && view.channel == Channel::InApp)
        .unwrap();
    assert!(sync_in_app.enabled);
    let brief_email = resolved
        .iter()
        .find(|view| view.kind == NotificationType::Brief && view.channel == Channel::Email)
        .unwrap();
    assert!(brief_email.enabled);
    assert_eq!(resolved.iter().filter(|view| view.mandatory).count(), 2);
}

#[test]
fn validate_change_refuses_security_email_off() {
    let error = validate_change(NotificationType::Security, Channel::Email, false).unwrap_err();
    assert_eq!(error.field(), "security.email");
    assert!(validate_change(NotificationType::Security, Channel::Email, true).is_ok());
}

#[test]
fn validate_change_refuses_key_revoked_email_off() {
    let error = validate_change(NotificationType::KeyRevoked, Channel::Email, false).unwrap_err();
    assert_eq!(error.field(), "key_revoked.email");
}

#[test]
fn validate_change_allows_security_in_app_off() {
    assert!(validate_change(NotificationType::Security, Channel::InApp, false).is_ok());
    assert!(validate_change(NotificationType::Drift, Channel::Email, false).is_ok());
}

// ---------------------------------------------------------------------------
// BR4.1 retry bounds
// ---------------------------------------------------------------------------

#[test]
fn attempts_retry_once_then_five_minutes_then_stop() {
    let now = at(9);
    assert_eq!(MAX_ATTEMPTS, 3);
    assert_eq!(next_attempt_at(1, now), Some(now + Duration::minutes(1)));
    assert_eq!(next_attempt_at(2, now), Some(now + Duration::minutes(5)));
    assert_eq!(next_attempt_at(3, now), None);
}

#[test]
fn attempt_refuses_a_fourth_try() {
    let error = DeliveryAttempt::sent(
        DeliveryAttemptId::generate(),
        AlertId::generate(),
        user(),
        Channel::Email,
        4,
        at(9),
    )
    .unwrap_err();
    assert_eq!(error.field(), "attempt");
}

#[test]
fn attempt_status_labels_round_trip() {
    for status in [
        AttemptStatus::Sent,
        AttemptStatus::Failed,
        AttemptStatus::Abandoned,
    ] {
        assert_eq!(AttemptStatus::parse(status.as_str()), Ok(status));
    }
    for category in [
        ErrorCategory::Provider,
        ErrorCategory::Network,
        ErrorCategory::Address,
        ErrorCategory::Content,
    ] {
        assert_eq!(ErrorCategory::parse(category.as_str()), Ok(category));
    }
}

#[test]
fn a_failed_attempt_must_carry_an_error_category() {
    let error = DeliveryAttempt::failed(
        DeliveryAttemptId::generate(),
        AlertId::generate(),
        user(),
        Channel::Email,
        1,
        None,
        at(9),
    )
    .unwrap_err();
    assert_eq!(error.field(), "errorCategory");

    assert!(DeliveryAttempt::sent(
        DeliveryAttemptId::generate(),
        AlertId::generate(),
        user(),
        Channel::InApp,
        1,
        at(9),
    )
    .is_ok());
}

// ---------------------------------------------------------------------------
// Alert status derivation
// ---------------------------------------------------------------------------

fn outcome(attempts: u8, sent: bool, abandoned: bool) -> ChannelOutcome {
    ChannelOutcome {
        attempts,
        sent,
        abandoned,
    }
}

#[test]
fn all_sent_channels_make_the_alert_delivered() {
    let outcomes = BTreeMap::from([
        (Channel::InApp, outcome(1, true, false)),
        (Channel::Email, outcome(1, true, false)),
    ]);
    assert_eq!(
        alert_status(&[Channel::InApp, Channel::Email], &outcomes),
        AlertStatus::Delivered
    );
}

#[test]
fn one_abandoned_channel_makes_the_alert_partially_delivered() {
    let outcomes = BTreeMap::from([
        (Channel::InApp, outcome(1, true, false)),
        (Channel::Email, outcome(3, false, true)),
    ]);
    assert_eq!(
        alert_status(&[Channel::InApp, Channel::Email], &outcomes),
        AlertStatus::PartiallyDelivered
    );
}

#[test]
fn every_abandoned_channel_makes_the_alert_failed() {
    let outcomes = BTreeMap::from([(Channel::Email, outcome(3, false, true))]);
    assert_eq!(
        alert_status(&[Channel::Email], &outcomes),
        AlertStatus::Failed
    );
}

#[test]
fn a_pending_channel_keeps_the_alert_queued() {
    let outcomes = BTreeMap::from([
        (Channel::InApp, outcome(1, true, false)),
        (Channel::Email, outcome(1, false, false)),
    ]);
    assert_eq!(
        alert_status(&[Channel::InApp, Channel::Email], &outcomes),
        AlertStatus::Queued
    );
}

#[test]
fn no_enabled_channel_is_delivered_without_attempts() {
    assert_eq!(alert_status(&[], &BTreeMap::new()), AlertStatus::Delivered);
}

#[test]
fn summarise_counts_attempts_by_channel() {
    let alert = AlertId::generate();
    let attempts = vec![
        DeliveryAttempt::failed(
            DeliveryAttemptId::generate(),
            alert,
            user(),
            Channel::Email,
            1,
            Some(ErrorCategory::Network),
            at(9),
        )
        .unwrap(),
        DeliveryAttempt::failed(
            DeliveryAttemptId::generate(),
            alert,
            user(),
            Channel::Email,
            2,
            Some(ErrorCategory::Network),
            at(10),
        )
        .unwrap(),
        DeliveryAttempt::sent(
            DeliveryAttemptId::generate(),
            alert,
            user(),
            Channel::InApp,
            1,
            at(9),
        )
        .unwrap(),
    ];
    let summary = summarise(&attempts);
    assert_eq!(
        summary.get(&Channel::Email).map(|entry| entry.attempts),
        Some(2)
    );
    assert!(summary.get(&Channel::InApp).is_some_and(|entry| entry.sent));
}

// ---------------------------------------------------------------------------
// Retention
// ---------------------------------------------------------------------------

#[test]
fn retention_cutoff_is_twelve_months() {
    let now = Utc.with_ymd_and_hms(2027, 11, 8, 0, 0, 0).unwrap();
    let cutoff = cutoff_twelve_months_before(now);
    assert_eq!(cutoff, Utc.with_ymd_and_hms(2026, 11, 8, 0, 0, 0).unwrap());
}
