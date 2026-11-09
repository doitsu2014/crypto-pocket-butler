//! Application use-case tests against port fakes — no database, no network.
//!
//! Covers BR1.1–BR1.2 dedupe, BR2.1–BR2.3 preferences and suppression, BR3.1
//! mandatory email, BR4.1–BR4.3 delivery, BR5.1 content, NFR1.5 audit retry,
//! NFR13.1 erasure, and the twelve-month retention sweep.

mod fakes;

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::{DateTime, Duration, TimeZone, Utc};
use fakes::{
    FakeAlerts, FakeAttempts, FakeAudit, FakeContacts, FakeControl, FakeEmail, FakePreferences,
    FakeStore, MutableClock, RecordingPause, SequentialIds,
};
use notification_application::ports::{Clock, ExternalError, RepositoryError};
use notification_application::{
    DeliveryError, DeliveryService, EraseService, FeedError, FeedService, PreferenceChange,
    PreferenceError, PreferenceService, RetentionService, SubmitAlert, SubmitCommand, SubmitError,
    SubmitStatus, Trail,
};
use notification_domain::audit::{
    PREFERENCE_REFUSED, PRIVACY_ANONYMISED, SECURITY_ALERT_ABANDONED, SECURITY_ALERT_DELIVERED,
};
use notification_domain::{
    Alert, AlertId, AlertStatus, AttemptStatus, Channel, DeliveryAttempt, DeliveryAttemptId,
    ErrorCategory, NotificationPreference, NotificationType, PreferenceId, UserId, MAX_FEED_LIMIT,
};

const USER: &str = "11111111-1111-4111-8111-111111111111";
const OTHER: &str = "22222222-2222-4222-8222-222222222222";
const ADDRESS: &str = "user@example.com";

fn at(day: u32, hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, day, hour, 0, 0).unwrap()
}

fn user() -> UserId {
    UserId::parse(USER).unwrap()
}

struct Harness {
    store: Arc<FakeStore>,
    control: Arc<FakeControl>,
    contacts: Arc<FakeContacts>,
    email: Arc<FakeEmail>,
    audit: Arc<FakeAudit>,
    clock: Arc<MutableClock>,
    pause: Arc<RecordingPause>,
    submit: SubmitAlert,
    preference_service: PreferenceService,
    feed: FeedService,
    delivery: DeliveryService,
    retention: RetentionService,
    erase: EraseService,
    trail: Arc<Trail>,
}

fn harness() -> Harness {
    harness_with(Ok(Some(ADDRESS.to_string())), Ok(()))
}

fn harness_with(
    contact: Result<Option<String>, ExternalError>,
    provider: Result<(), ExternalError>,
) -> Harness {
    let store = Arc::new(FakeStore::default());
    let alerts = Arc::new(FakeAlerts::new(store.clone()));
    let attempts = Arc::new(FakeAttempts::new(store.clone()));
    let preferences = Arc::new(FakePreferences::new(store.clone()));
    let control = Arc::new(FakeControl::default());
    let contacts = Arc::new(FakeContacts::always(contact));
    let email = Arc::new(FakeEmail::always(provider));
    let audit = Arc::new(FakeAudit::default());
    let clock = Arc::new(MutableClock::at(at(8, 9)));
    let ids = Arc::new(SequentialIds::default());
    let pause = Arc::new(RecordingPause::default());
    let trail = Arc::new(Trail::new(audit.clone(), clock.clone(), pause.clone()));

    let submit = SubmitAlert::new(alerts.clone(), control.clone(), ids.clone(), clock.clone());
    let preference_service = PreferenceService::new(
        preferences.clone(),
        ids.clone(),
        clock.clone(),
        trail.clone(),
    );
    let feed = FeedService::new(alerts.clone());
    let delivery = DeliveryService::new(
        alerts.clone(),
        preferences.clone(),
        attempts.clone(),
        control.clone(),
        contacts.clone(),
        email.clone(),
        trail.clone(),
        clock.clone(),
        ids.clone(),
    );
    let retention = RetentionService::new(
        alerts.clone(),
        attempts.clone(),
        clock.clone(),
        pause.clone(),
    );
    let erase = EraseService::new(alerts.clone(), ids.clone(), trail.clone());

    Harness {
        store,
        control,
        contacts,
        email,
        audit,
        clock,
        pause,
        submit,
        preference_service,
        feed,
        delivery,
        retention,
        erase,
        trail,
    }
}

fn command(kind: &str, key: &str, payload: &[(&str, &str)]) -> SubmitCommand {
    SubmitCommand {
        user_id: USER.to_string(),
        kind: kind.to_string(),
        idempotency_key: key.to_string(),
        payload: payload
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect(),
        correlation_id: None,
        channel: None,
    }
}

async fn submit_drift(harness: &Harness, key: &str) -> AlertId {
    harness
        .submit
        .execute(command("drift", key, &[("assetId", "bitcoin")]))
        .await
        .expect("the alert is accepted")
        .alert_id
}

async fn submit_brief(harness: &Harness, key: &str) -> AlertId {
    harness
        .submit
        .execute(command("brief", key, &[("briefId", &uuid_like())]))
        .await
        .expect("the alert is accepted")
        .alert_id
}

fn uuid_like() -> String {
    "33333333-3333-4333-8333-333333333333".to_string()
}

fn stored_alert(harness: &Harness, id: AlertId) -> Alert {
    harness
        .store
        .alerts
        .lock()
        .unwrap()
        .iter()
        .find(|alert| alert.id() == id)
        .cloned()
        .expect("the alert is stored")
}

fn store_preference(harness: &Harness, kind: NotificationType, channel: Channel, enabled: bool) {
    harness
        .store
        .preferences
        .lock()
        .unwrap()
        .push(NotificationPreference::stored(
            PreferenceId::generate(),
            user(),
            kind,
            channel,
            enabled,
            at(8, 9),
        ));
}

async fn seed_attempt(
    harness: &Harness,
    alert_id: AlertId,
    channel: Channel,
    number: u8,
    status: AttemptStatus,
) {
    seed_attempt_at(harness, alert_id, channel, number, status, at(8, 9)).await;
}

async fn seed_attempt_at(
    harness: &Harness,
    alert_id: AlertId,
    channel: Channel,
    number: u8,
    status: AttemptStatus,
    attempted_at: DateTime<Utc>,
) {
    let attempt = match status {
        AttemptStatus::Failed => DeliveryAttempt::failed(
            DeliveryAttemptId::generate(),
            alert_id,
            user(),
            channel,
            number,
            Some(ErrorCategory::Network),
            attempted_at,
        ),
        AttemptStatus::Sent => DeliveryAttempt::sent(
            DeliveryAttemptId::generate(),
            alert_id,
            user(),
            channel,
            number,
            attempted_at,
        ),
        AttemptStatus::Abandoned => DeliveryAttempt::abandoned(
            DeliveryAttemptId::generate(),
            alert_id,
            user(),
            channel,
            number,
            ErrorCategory::Network,
            attempted_at,
        ),
    }
    .expect("the seeded attempt is valid");
    harness.store.attempts.lock().unwrap().push(attempt);
}

fn attempts_for(harness: &Harness, alert_id: AlertId) -> Vec<DeliveryAttempt> {
    harness
        .store
        .attempts
        .lock()
        .unwrap()
        .iter()
        .filter(|attempt| attempt.alert_id() == alert_id)
        .cloned()
        .collect()
}

// ---------------------------------------------------------------------------
// WF1 submit and dedupe (BR1.1)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn submit_queues_a_drift_alert() {
    let harness = harness();
    let result = harness
        .submit
        .execute(command("drift", "drift:1", &[("assetId", "bitcoin")]))
        .await
        .unwrap();
    assert_eq!(result.status, SubmitStatus::Queued);
    let alert = stored_alert(&harness, result.alert_id);
    assert_eq!(alert.status(), AlertStatus::Queued);
    assert_eq!(alert.next_attempt_at(), Some(at(8, 9)));
    assert!(!harness.control.wake_requested_now());
}

#[tokio::test]
async fn submit_dedupes_a_repeated_key() {
    let harness = harness();
    let first = harness
        .submit
        .execute(command("drift", "drift:1", &[]))
        .await
        .unwrap();
    let second = harness
        .submit
        .execute(command("drift", "drift:1", &[]))
        .await
        .unwrap();
    assert_eq!(second.status, SubmitStatus::Deduplicated);
    assert_eq!(second.alert_id, first.alert_id);
    assert_eq!(harness.store.alerts.lock().unwrap().len(), 1);
    assert!(harness.email.calls().is_empty());
}

#[tokio::test]
async fn submit_a_mandatory_email_requests_a_wake() {
    let harness = harness();
    harness
        .submit
        .execute(command(
            "security",
            "security:1",
            &[("event", "new device")],
        ))
        .await
        .unwrap();
    assert!(harness.control.wake_requested_now());
}

#[tokio::test]
async fn submit_does_not_wake_for_a_non_mandatory_alert() {
    let harness = harness();
    harness
        .submit
        .execute(command("sync_failed", "sync:1", &[]))
        .await
        .unwrap();
    assert!(!harness.control.wake_requested_now());
}

#[tokio::test]
async fn submit_sanitises_a_credential_before_storing() {
    let harness = harness();
    let result = harness
        .submit
        .execute(command(
            "security",
            "security:1",
            &[("event", "alice@example.com"), ("deviceLabel", "Chrome")],
        ))
        .await
        .unwrap();
    let alert = stored_alert(&harness, result.alert_id);
    assert_eq!(alert.payload().get("event").map(String::as_str), None);
    assert_eq!(alert.sanitised_fields(), &["event".to_string()]);
}

#[tokio::test]
async fn submit_rejects_a_blank_idempotency_key() {
    let harness = harness();
    let error = harness
        .submit
        .execute(command("drift", "   ", &[]))
        .await
        .unwrap_err();
    match error {
        SubmitError::Rejected(errors) => assert_eq!(errors[0].field, "idempotencyKey"),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn submit_rejects_an_unknown_type() {
    let harness = harness();
    let error = harness
        .submit
        .execute(command("marketing", "key", &[]))
        .await
        .unwrap_err();
    match error {
        SubmitError::Rejected(errors) => assert_eq!(errors[0].field, "type"),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn submit_rejects_a_non_uuid_user() {
    let harness = harness();
    let error = harness
        .submit
        .execute(SubmitCommand {
            user_id: "nobody".to_string(),
            kind: "drift".to_string(),
            idempotency_key: "key".to_string(),
            payload: BTreeMap::new(),
            correlation_id: None,
            channel: None,
        })
        .await
        .unwrap_err();
    match error {
        SubmitError::Rejected(errors) => assert_eq!(errors[0].field, "user"),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn submit_surfaces_a_storage_failure() {
    let harness = harness();
    harness.store.fail_next();
    let error = harness
        .submit
        .execute(command("drift", "drift:1", &[]))
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        SubmitError::Repository(RepositoryError::Unavailable(_))
    ));
}

// ---------------------------------------------------------------------------
// WF3 preferences (BR2.1–BR2.3, BR3.1)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn preferences_view_shows_defaults_for_every_pair() {
    let harness = harness();
    let view = harness.preference_service.view(USER).await.unwrap();
    assert_eq!(view.len(), NotificationType::ALL.len() * Channel::ALL.len());
    for preference in &view {
        assert!(preference.enabled || preference.channel == Channel::Email);
    }
}

#[tokio::test]
async fn preferences_view_marks_mandatory_emails() {
    let harness = harness();
    let view = harness.preference_service.view(USER).await.unwrap();
    let mandatory: Vec<_> = view.iter().filter(|entry| entry.mandatory).collect();
    assert_eq!(mandatory.len(), 2);
    assert!(mandatory
        .iter()
        .all(|entry| entry.channel == Channel::Email && entry.kind.is_mandatory_email()));
}

#[tokio::test]
async fn saving_a_change_applies_to_the_next_event() {
    let harness = harness();
    harness
        .preference_service
        .save(
            USER,
            vec![PreferenceChange {
                kind: "drift".to_string(),
                channel: "email".to_string(),
                enabled: true,
            }],
            None,
        )
        .await
        .unwrap();
    let id = submit_drift(&harness, "drift:1").await;
    harness.delivery.run().await.unwrap();
    assert_eq!(harness.email.calls().len(), 1);
    assert_eq!(stored_alert(&harness, id).status(), AlertStatus::Delivered);
}

#[tokio::test]
async fn saving_a_mandatory_email_off_is_refused_and_audited() {
    let harness = harness();
    let error = harness
        .preference_service
        .save(
            USER,
            vec![PreferenceChange {
                kind: "security".to_string(),
                channel: "email".to_string(),
                enabled: false,
            }],
            Some("corr-1"),
        )
        .await
        .unwrap_err();
    match error {
        PreferenceError::Rejected(errors) => assert_eq!(errors[0].field, "security.email"),
        other => panic!("unexpected error: {other:?}"),
    }
    let records = harness.audit.records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].action, PREFERENCE_REFUSED);
    assert_eq!(records[0].outcome, "denied");
    assert_eq!(records[0].target, "security");
}

#[tokio::test]
async fn saving_another_mandatory_email_off_is_refused() {
    let harness = harness();
    let error = harness
        .preference_service
        .save(
            USER,
            vec![PreferenceChange {
                kind: "key_revoked".to_string(),
                channel: "email".to_string(),
                enabled: false,
            }],
            None,
        )
        .await
        .unwrap_err();
    match error {
        PreferenceError::Rejected(errors) => assert_eq!(errors[0].field, "key_revoked.email"),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn saving_an_in_app_change_is_stored() {
    let harness = harness();
    let saved = harness
        .preference_service
        .save(
            USER,
            vec![PreferenceChange {
                kind: "drift".to_string(),
                channel: "in_app".to_string(),
                enabled: false,
            }],
            None,
        )
        .await
        .unwrap();
    assert_eq!(saved.preferences.len(), NotificationType::ALL.len() * 2);
    assert_eq!(harness.store.preferences.lock().unwrap().len(), 1);
    let drift = saved
        .preferences
        .iter()
        .find(|entry| entry.kind == NotificationType::Drift && entry.channel == Channel::InApp)
        .unwrap();
    assert!(!drift.enabled);
}

#[tokio::test]
async fn preferences_reject_a_non_uuid_user() {
    let harness = harness();
    let error = harness.preference_service.view("nobody").await.unwrap_err();
    match error {
        PreferenceError::Rejected(errors) => assert_eq!(errors[0].field, "user"),
        other => panic!("unexpected error: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// WF3 the in-app feed (C1)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn feed_returns_newest_first() {
    let harness = harness();
    submit_drift(&harness, "drift:1").await;
    harness.clock.advance(Duration::minutes(1));
    submit_drift(&harness, "drift:2").await;
    let page = harness.feed.list(USER, None, None).await.unwrap();
    assert_eq!(page.alerts.len(), 2);
    assert_eq!(page.alerts[0].idempotency_key(), "drift:2");
    assert!(page.next_cursor.is_none());
}

#[tokio::test]
async fn feed_cursor_returns_the_next_page() {
    let harness = harness();
    for index in 0..3 {
        harness.clock.advance(Duration::minutes(1));
        submit_drift(&harness, &format!("drift:{index}")).await;
    }
    let first = harness.feed.list(USER, None, Some(2)).await.unwrap();
    assert_eq!(first.alerts.len(), 2);
    let cursor = first.next_cursor.clone().expect("a full page has a cursor");
    let second = harness
        .feed
        .list(USER, Some(&cursor), Some(2))
        .await
        .unwrap();
    assert_eq!(second.alerts.len(), 1);
    assert!(second.next_cursor.is_none());
    assert_ne!(first.alerts[0].id(), second.alerts[0].id());
}

#[tokio::test]
async fn feed_page_reports_no_cursor_when_exhausted() {
    let harness = harness();
    submit_drift(&harness, "drift:1").await;
    let page = harness.feed.list(USER, None, Some(10)).await.unwrap();
    assert_eq!(page.alerts.len(), 1);
    assert!(page.next_cursor.is_none());
}

#[tokio::test]
async fn feed_clamps_an_oversized_limit() {
    let harness = harness();
    submit_drift(&harness, "drift:1").await;
    let page = harness.feed.list(USER, None, Some(10_000)).await.unwrap();
    assert_eq!(page.limit, MAX_FEED_LIMIT);
}

#[tokio::test]
async fn feed_rejects_a_malformed_cursor() {
    let harness = harness();
    let error = harness
        .feed
        .list(USER, Some("not-a-cursor"), None)
        .await
        .unwrap_err();
    match error {
        FeedError::Rejected(errors) => assert_eq!(errors[0].field, "cursor"),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn feed_is_empty_for_a_new_user() {
    let harness = harness();
    let page = harness.feed.list(OTHER, None, None).await.unwrap();
    assert!(page.alerts.is_empty());
    assert!(page.next_cursor.is_none());
}

#[tokio::test]
async fn feed_hides_a_type_whose_in_app_switch_is_off() {
    let harness = harness();
    store_preference(&harness, NotificationType::Drift, Channel::InApp, false);
    submit_drift(&harness, "drift:1").await;
    let page = harness.feed.list(USER, None, None).await.unwrap();
    assert!(page.alerts.is_empty());
}

#[tokio::test]
async fn feed_still_shows_other_types_when_one_is_off() {
    let harness = harness();
    store_preference(&harness, NotificationType::Drift, Channel::InApp, false);
    submit_brief(&harness, "brief:1").await;
    let page = harness.feed.list(USER, None, None).await.unwrap();
    assert_eq!(page.alerts.len(), 1);
    assert_eq!(page.alerts[0].kind(), NotificationType::Brief);
}

// ---------------------------------------------------------------------------
// WF4 delivery (BR2.1, BR3.1, BR4.1–BR4.3, BR5.1)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_drift_alert_with_defaults_delivers_in_app_only() {
    let harness = harness();
    let id = submit_drift(&harness, "drift:1").await;
    let report = harness.delivery.run().await.unwrap();
    assert_eq!(report.delivered, 1);
    assert_eq!(stored_alert(&harness, id).status(), AlertStatus::Delivered);
    let attempts = attempts_for(&harness, id);
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].channel(), Channel::InApp);
    assert_eq!(attempts[0].status(), AttemptStatus::Sent);
    assert!(harness.email.calls().is_empty());
    assert_eq!(
        harness
            .contacts
            .calls
            .load(std::sync::atomic::Ordering::Relaxed),
        0
    );
}

#[tokio::test]
async fn a_brief_with_email_on_sends_to_the_verified_address() {
    let harness = harness();
    let id = submit_brief(&harness, "brief:1").await;
    harness.delivery.run().await.unwrap();
    assert_eq!(stored_alert(&harness, id).status(), AlertStatus::Delivered);
    let calls = harness.email.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].to, ADDRESS);
    assert_eq!(attempts_for(&harness, id).len(), 2);
}

#[tokio::test]
async fn a_drift_alert_with_email_on_sends() {
    let harness = harness();
    store_preference(&harness, NotificationType::Drift, Channel::Email, true);
    let id = submit_drift(&harness, "drift:1").await;
    harness.delivery.run().await.unwrap();
    assert_eq!(stored_alert(&harness, id).status(), AlertStatus::Delivered);
    assert_eq!(harness.email.calls().len(), 1);
}

#[tokio::test]
async fn a_missing_verified_address_is_a_permanent_address_error() {
    let harness = harness_with(Ok(None), Ok(()));
    let id = submit_drift(&harness, "drift:1").await;
    store_preference(&harness, NotificationType::Drift, Channel::Email, true);
    harness.delivery.run().await.unwrap();
    assert_eq!(
        stored_alert(&harness, id).status(),
        AlertStatus::PartiallyDelivered
    );
    let email_attempts: Vec<_> = attempts_for(&harness, id)
        .into_iter()
        .filter(|attempt| attempt.channel() == Channel::Email)
        .collect();
    assert_eq!(email_attempts.len(), 1);
    assert_eq!(email_attempts[0].status(), AttemptStatus::Abandoned);
    assert_eq!(
        email_attempts[0].error_category(),
        Some(ErrorCategory::Address)
    );
    assert!(harness.email.calls().is_empty());
}

#[tokio::test]
async fn a_transient_contact_failure_retries_after_one_minute() {
    let harness = harness_with(
        Err(ExternalError::Unavailable("C3 timed out".to_string())),
        Ok(()),
    );
    store_preference(&harness, NotificationType::Drift, Channel::Email, true);
    let id = submit_drift(&harness, "drift:1").await;
    harness.delivery.run().await.unwrap();
    let alert = stored_alert(&harness, id);
    assert_eq!(alert.status(), AlertStatus::Queued);
    assert_eq!(
        alert.next_attempt_at(),
        Some(at(8, 9) + Duration::minutes(1))
    );
    let email_attempts: Vec<_> = attempts_for(&harness, id)
        .into_iter()
        .filter(|attempt| attempt.channel() == Channel::Email)
        .collect();
    assert_eq!(email_attempts[0].status(), AttemptStatus::Failed);
    assert_eq!(
        email_attempts[0].error_category(),
        Some(ErrorCategory::Network)
    );
}

#[tokio::test]
async fn a_second_transient_failure_waits_five_minutes() {
    let harness = harness_with(
        Err(ExternalError::Unavailable("C3 timed out".to_string())),
        Ok(()),
    );
    store_preference(&harness, NotificationType::Drift, Channel::Email, true);
    let id = submit_drift(&harness, "drift:1").await;
    seed_attempt(&harness, id, Channel::Email, 1, AttemptStatus::Failed).await;
    harness.delivery.run().await.unwrap();
    let alert = stored_alert(&harness, id);
    assert_eq!(
        alert.next_attempt_at(),
        Some(at(8, 9) + Duration::minutes(5))
    );
}

#[tokio::test]
async fn a_third_failure_abandons_the_email() {
    let harness = harness_with(
        Err(ExternalError::Unavailable("C3 timed out".to_string())),
        Ok(()),
    );
    store_preference(&harness, NotificationType::Drift, Channel::Email, true);
    let id = submit_drift(&harness, "drift:1").await;
    seed_attempt(&harness, id, Channel::Email, 1, AttemptStatus::Failed).await;
    seed_attempt(&harness, id, Channel::Email, 2, AttemptStatus::Failed).await;
    harness.delivery.run().await.unwrap();
    assert_eq!(
        stored_alert(&harness, id).status(),
        AlertStatus::PartiallyDelivered
    );
    let third: Vec<_> = attempts_for(&harness, id)
        .into_iter()
        .filter(|attempt| attempt.channel() == Channel::Email)
        .collect();
    assert_eq!(third.len(), 3);
    let abandoned = third.iter().find(|attempt| attempt.attempt() == 3).unwrap();
    assert_eq!(abandoned.status(), AttemptStatus::Abandoned);
}

#[tokio::test]
async fn a_mandatory_email_is_sent_even_when_the_preference_is_off() {
    let harness = harness();
    store_preference(&harness, NotificationType::Security, Channel::Email, false);
    let security = harness
        .submit
        .execute(command(
            "security",
            "security:1",
            &[("event", "new device")],
        ))
        .await
        .unwrap()
        .alert_id;
    harness.delivery.run().await.unwrap();
    assert_eq!(
        stored_alert(&harness, security).status(),
        AlertStatus::Delivered
    );
    assert_eq!(harness.email.calls().len(), 1);
}

#[tokio::test]
async fn a_mandatory_email_with_no_address_is_abandoned_and_audited() {
    let harness = harness_with(Ok(None), Ok(()));
    store_preference(&harness, NotificationType::Security, Channel::Email, false);
    let id = harness
        .submit
        .execute(command(
            "security",
            "security:1",
            &[("event", "new device")],
        ))
        .await
        .unwrap()
        .alert_id;
    harness.delivery.run().await.unwrap();
    assert_eq!(
        stored_alert(&harness, id).status(),
        AlertStatus::PartiallyDelivered
    );
    let records = harness.audit.records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].action, SECURITY_ALERT_ABANDONED);
    assert_eq!(records[0].outcome, "failed");
}

#[tokio::test]
async fn a_delivered_mandatory_email_is_audited() {
    let harness = harness();
    harness
        .submit
        .execute(command(
            "key_revoked",
            "revoked:1",
            &[("event", "key revoked")],
        ))
        .await
        .unwrap();
    harness.delivery.run().await.unwrap();
    let records = harness.audit.records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].action, SECURITY_ALERT_DELIVERED);
    assert_eq!(records[0].outcome, "success");
}

#[tokio::test]
async fn a_mandatory_security_email_is_delivered_through_the_c3_contact() {
    // End to end across the ports: submit queues the mandatory email, the run
    // reads the verified address from the fake C3, the provider accepts it, and
    // the delivery is audited with the channel and the sanitised field names.
    let harness = harness();
    let id = harness
        .submit
        .execute(command(
            "security",
            "security:c3",
            &[("event", "alice@example.com"), ("deviceLabel", "Chrome")],
        ))
        .await
        .unwrap()
        .alert_id;
    harness.delivery.run().await.unwrap();

    assert_eq!(
        harness
            .contacts
            .calls
            .load(std::sync::atomic::Ordering::Relaxed),
        1,
        "the run asks C3 for the verified address exactly once"
    );
    let calls = harness.email.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].to, ADDRESS);
    assert_eq!(stored_alert(&harness, id).status(), AlertStatus::Delivered);
    let records = harness.audit.records();
    let delivered = records
        .iter()
        .find(|record| record.action == SECURITY_ALERT_DELIVERED)
        .expect("the mandatory delivery is audited");
    assert_eq!(
        delivered.context.get("channel").map(String::as_str),
        Some("email")
    );
    assert_eq!(
        delivered.context.get("sanitisedFields").map(String::as_str),
        Some("event"),
        "NFR9.5: the audit event names the removed field, never its value"
    );
}

#[tokio::test]
async fn an_email_off_brief_stays_in_app() {
    let harness = harness();
    store_preference(&harness, NotificationType::Brief, Channel::Email, false);
    let id = submit_brief(&harness, "brief:1").await;
    harness.delivery.run().await.unwrap();
    assert_eq!(stored_alert(&harness, id).status(), AlertStatus::Delivered);
    assert!(harness.email.calls().is_empty());
    let attempts = attempts_for(&harness, id);
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].channel(), Channel::InApp);
}

#[tokio::test]
async fn a_both_channels_off_alert_is_delivered_without_attempts() {
    let harness = harness();
    store_preference(&harness, NotificationType::Drift, Channel::InApp, false);
    store_preference(&harness, NotificationType::Drift, Channel::Email, false);
    let id = submit_drift(&harness, "drift:1").await;
    harness.delivery.run().await.unwrap();
    assert_eq!(stored_alert(&harness, id).status(), AlertStatus::Delivered);
    assert!(attempts_for(&harness, id).is_empty());
}

#[tokio::test]
async fn a_provider_refusal_is_abandoned_with_a_provider_error() {
    let harness = harness_with(
        Ok(Some(ADDRESS.to_string())),
        Err(ExternalError::Refused(
            "provider refused the message".to_string(),
        )),
    );
    store_preference(&harness, NotificationType::Drift, Channel::Email, true);
    let id = submit_drift(&harness, "drift:1").await;
    harness.delivery.run().await.unwrap();
    let email_attempts: Vec<_> = attempts_for(&harness, id)
        .into_iter()
        .filter(|attempt| attempt.channel() == Channel::Email)
        .collect();
    assert_eq!(email_attempts[0].status(), AttemptStatus::Abandoned);
    assert_eq!(
        email_attempts[0].error_category(),
        Some(ErrorCategory::Provider)
    );
}

#[tokio::test]
async fn a_busy_lease_refuses_a_second_run() {
    let harness = harness();
    let other = uuid::Uuid::new_v4();
    harness.control.hold_lease(other, at(8, 10));
    let error = harness.delivery.run().await.unwrap_err();
    assert!(matches!(error, DeliveryError::Busy));
}

#[tokio::test]
async fn a_run_releases_the_lease() {
    let harness = harness();
    submit_drift(&harness, "drift:1").await;
    harness.delivery.run().await.unwrap();
    assert!(!harness.control.leased());
}

#[tokio::test]
async fn an_email_over_the_run_cap_is_deferred() {
    let harness = harness();
    let first = submit_brief(&harness, "brief:1").await;
    harness.clock.advance(Duration::minutes(1));
    let second = submit_brief(&harness, "brief:2").await;
    let delivery = harness.delivery.with_limits(200, 1, 4);
    let report = delivery.run().await.unwrap();
    assert_eq!(report.deferred, 1);
    assert_eq!(harness.email.calls().len(), 1);
    let statuses = [first, second]
        .iter()
        .map(|id| stored_alert(&harness, *id).status())
        .collect::<Vec<_>>();
    assert!(statuses.contains(&AlertStatus::Queued));
    assert!(statuses.contains(&AlertStatus::Delivered));
}

#[tokio::test]
async fn a_run_reports_attempts_and_outcomes() {
    let harness = harness();
    let id = submit_brief(&harness, "brief:1").await;
    let report = harness.delivery.run().await.unwrap();
    assert_eq!(report.processed, 1);
    assert_eq!(report.attempts_sent, 2);
    assert_eq!(report.alerts.len(), 1);
    assert_eq!(report.alerts[0].alert_id, id);
    assert_eq!(report.alerts[0].kind, "brief");
    assert_eq!(report.alerts[0].status, "delivered");
    assert_eq!(report.attempts.len(), 2);
}

#[tokio::test]
async fn poll_runs_when_an_alert_is_due() {
    let harness = harness();
    submit_drift(&harness, "drift:1").await;
    let report = harness.delivery.poll().await.unwrap();
    assert!(report.is_some());
    assert_eq!(report.unwrap().delivered, 1);
}

#[tokio::test]
async fn poll_runs_when_the_wake_flag_is_set() {
    let harness = harness();
    use notification_application::ports::DeliveryControl;
    harness.control.request_wake(at(8, 9)).await.unwrap();
    let report = harness.delivery.poll().await.unwrap();
    assert!(report.is_some());
    assert!(!harness.control.wake_requested_now());
}

#[tokio::test]
async fn poll_does_nothing_when_the_queue_is_empty() {
    let harness = harness();
    assert!(harness.delivery.poll().await.unwrap().is_none());
}

// ---------------------------------------------------------------------------
// Retention (NFR20.6)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn retention_deletes_rows_older_than_twelve_months() {
    let harness = harness();
    harness.clock.advance(Duration::days(-400));
    let old = harness.clock.now();
    let id = submit_drift(&harness, "drift:old").await;
    seed_attempt_at(&harness, id, Channel::InApp, 1, AttemptStatus::Sent, old).await;
    harness.clock.advance(Duration::days(400));
    let report = harness.retention.run().await.unwrap();
    assert_eq!(report.alerts_deleted, 1);
    assert_eq!(report.attempts_deleted, 1);
    assert!(harness.store.alerts.lock().unwrap().is_empty());
}

#[tokio::test]
async fn retention_keeps_a_recent_row() {
    let harness = harness();
    submit_drift(&harness, "drift:1").await;
    let report = harness.retention.run().await.unwrap();
    assert_eq!(report.alerts_deleted, 0);
    assert_eq!(report.attempts_deleted, 0);
    assert_eq!(harness.store.alerts.lock().unwrap().len(), 1);
}

// ---------------------------------------------------------------------------
// Erasure (NFR13.1)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn erasure_replaces_the_user_with_one_random_reference() {
    let harness = harness();
    let id = submit_drift(&harness, "drift:1").await;
    harness.delivery.run().await.unwrap();
    assert_eq!(attempts_for(&harness, id).len(), 1);
    let report = harness
        .erase
        .anonymise(USER, Some("corr-erase"))
        .await
        .unwrap();
    assert_eq!(report.alerts_anonymised, 1);
    assert_eq!(report.attempts_anonymised, 1);
    let alert = stored_alert(&harness, id);
    assert_ne!(alert.user_id().as_uuid().to_string(), USER);
    let replacement = alert.user_id();
    for attempt in attempts_for(&harness, id) {
        assert_eq!(attempt.user_id(), replacement);
    }
    assert!(!replacement.as_uuid().is_nil());
    let records = harness.audit.records();
    assert!(records
        .iter()
        .any(|record| record.action == PRIVACY_ANONYMISED));
}

#[tokio::test]
async fn erasure_of_an_unknown_user_changes_nothing() {
    let harness = harness();
    let report = harness.erase.anonymise(OTHER, None).await.unwrap();
    assert_eq!(report.alerts_anonymised, 0);
    assert_eq!(report.attempts_anonymised, 0);
    assert!(!report.audit_failed);
    assert!(harness.audit.records().is_empty());
}

#[tokio::test]
async fn erasure_rejects_a_non_uuid_user() {
    let harness = harness();
    let error = harness.erase.anonymise("nobody", None).await.unwrap_err();
    match error {
        notification_application::EraseError::Rejected(errors) => {
            assert_eq!(errors[0].field, "user");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// C10 retry (NFR1.5)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn audit_retries_three_times_then_counts_a_failure() {
    let harness = harness();
    harness.audit.fail_next(3);
    let written = harness
        .trail
        .security_alert_delivered(NotificationType::Security, AlertId::generate(), user(), &[])
        .await;
    assert!(!written);
    assert_eq!(harness.trail.failures(), 1);
    assert_eq!(
        harness.pause.waited(),
        vec![
            std::time::Duration::from_secs(1),
            std::time::Duration::from_secs(2),
            std::time::Duration::from_secs(5)
        ]
    );
    assert!(harness.audit.records().is_empty());
}

#[tokio::test]
async fn audit_succeeds_on_a_retry() {
    let harness = harness();
    harness.audit.fail_next(1);
    let written = harness
        .trail
        .security_alert_delivered(NotificationType::Security, AlertId::generate(), user(), &[])
        .await;
    assert!(written);
    assert_eq!(harness.trail.failures(), 0);
    assert_eq!(harness.audit.records().len(), 1);
}
