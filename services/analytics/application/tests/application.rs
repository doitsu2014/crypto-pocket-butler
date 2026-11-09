//! Application use-case tests against port fakes — no database, no network.

mod fakes;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use analytics_application::aggregation::AggregateService;
use analytics_application::erase::{EraseError, EraseService};
use analytics_application::flags::{FlagCommand, FlagError, FlagPatch, FlagService};
use analytics_application::hashing::{
    flag_bucket, hmac_sha256_hex, pseudonymous_actor_id, sha256_hex,
};
use analytics_application::ingest::{IngestCommand, IngestError, IngestEvent};
use analytics_application::ports::{Clock, ExternalError, SaltProvider, SaltVersion};
use analytics_application::retention::RetentionService;
use analytics_application::trail::Trail;
use analytics_domain::audit::{FLAG_CREATED, INGEST_REFUSED};
use analytics_domain::{FlagStatus, Metric};
use chrono::{DateTime, Duration, TimeZone, Utc};

use fakes::{
    salt, stored_event, DenseEvents, FakeAggregates, FakeAudit, FakeConsentChecker, FakeConsents,
    FakeEvents, FakeFlags, FakeSalts, MutableClock, NoPause,
};

const SUBJECT: &str = "11111111-1111-4111-8111-111111111111";

fn at(day: u32, hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, day, hour, 0, 0).unwrap()
}

struct Harness {
    events: Arc<FakeEvents>,
    consents: Arc<FakeConsents>,
    checker: Arc<FakeConsentChecker>,
    audit: Arc<FakeAudit>,
    flags_repo: Arc<FakeFlags>,
    clock: Arc<MutableClock>,
    ingest: IngestEvent,
    flags: FlagService,
    aggregates: AggregateService,
    retention: RetentionService,
    erase: EraseService,
}

fn harness() -> Harness {
    harness_with(Ok(true), Ok(salt("v1")), vec![salt("v1"), salt("v2")])
}

fn harness_with(
    consent: Result<bool, ExternalError>,
    current: Result<SaltVersion, ExternalError>,
    retained: Vec<SaltVersion>,
) -> Harness {
    let events = Arc::new(FakeEvents::default());
    let consents = Arc::new(FakeConsents::default());
    let checker = Arc::new(FakeConsentChecker::new(consent));
    let salts = Arc::new(match current {
        Ok(value) => FakeSalts::working(value, retained),
        Err(error) => FakeSalts::failing_current(error),
    });
    let audit = Arc::new(FakeAudit::default());
    let flags_repo = Arc::new(FakeFlags::default());
    let aggregates_repo = Arc::new(FakeAggregates::default());
    let clock = Arc::new(MutableClock::at(at(6, 10)));
    let trail = Arc::new(Trail::new(audit.clone(), clock.clone()));

    let ingest = IngestEvent::new(
        events.clone(),
        checker.clone(),
        salts.clone(),
        trail.clone(),
        clock.clone(),
    );
    let flags = FlagService::new(flags_repo.clone(), clock.clone(), trail.clone());
    let aggregates = AggregateService::new(events.clone(), aggregates_repo.clone(), clock.clone());
    let retention = RetentionService::new(
        events.clone(),
        consents.clone(),
        clock.clone(),
        Arc::new(NoPause),
    );
    let erase = EraseService::new(events.clone(), consents.clone(), salts.clone(), trail);

    Harness {
        events,
        consents,
        checker,
        audit,
        flags_repo,
        clock,
        ingest,
        flags,
        aggregates,
        retention,
        erase,
    }
}

fn command(name: &str) -> IngestCommand {
    IngestCommand {
        event_name: name.to_string(),
        occurred_at: at(6, 9),
        properties: BTreeMap::new(),
        subject: SUBJECT.to_string(),
        correlation_id: Some("corr-1".to_string()),
    }
}

fn flag_command(key: &str, rollout: u16) -> FlagCommand {
    FlagCommand {
        key: key.to_string(),
        owner: "product".to_string(),
        expires_at: at(30, 0),
        rollout_percentage: rollout,
        status: Some("active".to_string()),
    }
}

#[tokio::test]
async fn an_essential_event_is_accepted_without_a_consent_check() {
    let harness = harness();
    let result = harness
        .ingest
        .execute(command("error_shown"))
        .await
        .unwrap();
    assert_eq!(*harness.checker.calls.lock().unwrap(), 0);
    assert_eq!(harness.events.stored.lock().unwrap().len(), 1);
    assert!(harness.events.consents.lock().unwrap().is_empty());
    assert!(result
        .user_ref
        .chars()
        .all(|character| character.is_ascii_hexdigit()));
}

#[tokio::test]
async fn an_analytics_event_is_accepted_with_consent_and_hashed() {
    let harness = harness();
    let mut request = command("valuation_read");
    request
        .properties
        .insert("freshness".to_string(), "live".to_string());
    let result = harness.ingest.execute(request).await.unwrap();
    assert_eq!(*harness.checker.calls.lock().unwrap(), 1);
    assert_eq!(harness.events.stored.lock().unwrap().len(), 1);
    assert_eq!(harness.events.consents.lock().unwrap().len(), 1);
    assert_eq!(result.user_ref.len(), 64);
    assert_ne!(result.user_ref, SUBJECT);
}

#[tokio::test]
async fn a_withdrawn_consent_is_refused_and_stores_no_event() {
    let harness = harness_with(Ok(false), Ok(salt("v1")), vec![salt("v1")]);
    let error = harness
        .ingest
        .execute(command("valuation_read"))
        .await
        .unwrap_err();
    assert_eq!(error, IngestError::ConsentNotGranted);
    assert!(harness.events.stored.lock().unwrap().is_empty());
    // BR1.1: a refusal stores nothing, not even a negative consent check.
    assert!(harness.consents.stored.lock().unwrap().is_empty());
    assert!(harness.events.consents.lock().unwrap().is_empty());
    let records = harness.audit.records.lock().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].action, INGEST_REFUSED);
    let expected_actor = pseudonymous_actor_id(&hmac_sha256_hex(&salt("v1").salt, SUBJECT));
    assert_eq!(
        records[0].actor_id.as_deref(),
        Some(expected_actor.as_str())
    );
}

#[tokio::test]
async fn an_unavailable_consent_service_fails_closed() {
    let harness = harness_with(
        Err(ExternalError::Unavailable("timeout".to_string())),
        Ok(salt("v1")),
        vec![salt("v1")],
    );
    let error = harness
        .ingest
        .execute(command("valuation_read"))
        .await
        .unwrap_err();
    assert!(matches!(error, IngestError::ConsentUnavailable(_)));
    assert!(harness.events.stored.lock().unwrap().is_empty());
    assert_eq!(harness.audit.records.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn a_missing_salt_fails_closed() {
    let harness = harness_with(
        Ok(true),
        Err(ExternalError::Unavailable("vault down".to_string())),
        Vec::new(),
    );
    let error = harness
        .ingest
        .execute(command("valuation_read"))
        .await
        .unwrap_err();
    assert!(matches!(error, IngestError::SaltUnavailable(_)));
    assert!(harness.events.stored.lock().unwrap().is_empty());
    assert_eq!(*harness.checker.calls.lock().unwrap(), 0);
    let records = harness.audit.records.lock().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].action, INGEST_REFUSED);
    assert_eq!(
        records[0].context.get("ruleId"),
        Some(&"salt-unavailable".to_string())
    );
}

#[tokio::test]
async fn an_unknown_event_is_rejected() {
    let harness = harness();
    let error = harness
        .ingest
        .execute(command("not_in_catalogue"))
        .await
        .unwrap_err();
    match error {
        IngestError::Rejected(errors) => assert_eq!(errors[0].field, "eventName"),
        other => panic!("expected a rejection, got {other:?}"),
    }
    assert!(harness.events.stored.lock().unwrap().is_empty());
    let records = harness.audit.records.lock().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].context.get("ruleId"),
        Some(&"event-rejected".to_string())
    );
}

#[tokio::test]
async fn a_pii_property_is_dropped_but_the_event_is_stored() {
    let harness = harness();
    let mut request = command("error_shown");
    request
        .properties
        .insert("route".to_string(), "person@example.com".to_string());
    harness.ingest.execute(request).await.unwrap();
    let stored = harness.events.stored.lock().unwrap();
    assert_eq!(stored.len(), 1);
    assert!(stored[0].properties().is_empty());
}

#[tokio::test]
async fn a_pii_event_name_is_rejected_without_echo() {
    let harness = harness();
    let email = "person@example.com";
    let error = harness.ingest.execute(command(email)).await.unwrap_err();
    match error {
        IngestError::Rejected(errors) => {
            assert_eq!(errors[0].field, "eventName");
            assert!(!errors[0].reason.contains(email));
        }
        other => panic!("expected a rejection, got {other:?}"),
    }
    let records = harness.audit.records.lock().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].action, INGEST_REFUSED);
}

#[tokio::test]
async fn creating_a_flag_audits_the_change() {
    let harness = harness();
    let flag = harness
        .flags
        .create(flag_command("new-dashboard", 25), SUBJECT)
        .await
        .unwrap();
    assert_eq!(flag.key(), "new-dashboard");
    let records = harness.audit.records.lock().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].action, FLAG_CREATED);
    assert_eq!(records[0].target, "new-dashboard");
}

#[tokio::test]
async fn a_flag_without_an_owner_is_rejected() {
    let harness = harness();
    let mut request = flag_command("new-dashboard", 25);
    request.owner = "  ".to_string();
    let error = harness.flags.create(request, SUBJECT).await.unwrap_err();
    match error {
        FlagError::Rejected(errors) => assert_eq!(errors[0].field, "owner"),
        other => panic!("expected a rejection, got {other:?}"),
    }
}

#[tokio::test]
async fn a_flag_with_a_past_expiry_is_rejected() {
    let harness = harness();
    let mut request = flag_command("new-dashboard", 25);
    request.expires_at = at(1, 0);
    let error = harness.flags.create(request, SUBJECT).await.unwrap_err();
    match error {
        FlagError::Rejected(errors) => assert_eq!(errors[0].field, "expiresAt"),
        other => panic!("expected a rejection, got {other:?}"),
    }
}

#[tokio::test]
async fn a_duplicate_flag_key_is_rejected() {
    let harness = harness();
    harness
        .flags
        .create(flag_command("new-dashboard", 25), SUBJECT)
        .await
        .unwrap();
    let error = harness
        .flags
        .create(flag_command("new-dashboard", 50), SUBJECT)
        .await
        .unwrap_err();
    match error {
        FlagError::Rejected(errors) => assert_eq!(errors[0].field, "key"),
        other => panic!("expected a rejection, got {other:?}"),
    }
}

#[tokio::test]
async fn updating_a_flag_changes_the_rollout() {
    let harness = harness();
    harness
        .flags
        .create(flag_command("new-dashboard", 25), SUBJECT)
        .await
        .unwrap();
    let updated = harness
        .flags
        .update(
            "new-dashboard",
            FlagPatch {
                rollout_percentage: Some(75),
                ..FlagPatch::default()
            },
            SUBJECT,
        )
        .await
        .unwrap();
    assert_eq!(updated.rollout_percentage(), 75);
}

#[tokio::test]
async fn updating_a_missing_flag_is_not_found() {
    let harness = harness();
    let error = harness
        .flags
        .update("missing", FlagPatch::default(), SUBJECT)
        .await
        .unwrap_err();
    assert_eq!(error, FlagError::NotFound);
}

#[tokio::test]
async fn deleting_a_flag_removes_it() {
    let harness = harness();
    harness
        .flags
        .create(flag_command("new-dashboard", 25), SUBJECT)
        .await
        .unwrap();
    harness
        .flags
        .delete("new-dashboard", SUBJECT)
        .await
        .unwrap();
    assert!(harness.flags.list(false).await.unwrap().is_empty());
}

#[tokio::test]
async fn deleting_a_missing_flag_is_not_found() {
    let harness = harness();
    let error = harness.flags.delete("missing", SUBJECT).await.unwrap_err();
    assert_eq!(error, FlagError::NotFound);
}

#[tokio::test]
async fn a_missing_flag_evaluates_off() {
    let harness = harness();
    let evaluation = harness.flags.evaluate("missing", SUBJECT).await.unwrap();
    assert!(!evaluation.enabled);
}

#[tokio::test]
async fn a_disabled_flag_evaluates_off() {
    let harness = harness();
    let mut request = flag_command("new-dashboard", 100);
    request.status = Some("disabled".to_string());
    harness.flags.create(request, SUBJECT).await.unwrap();
    assert!(
        !harness
            .flags
            .evaluate("new-dashboard", SUBJECT)
            .await
            .unwrap()
            .enabled
    );
}

#[tokio::test]
async fn rollout_is_deterministic_and_bucket_bounded() {
    let harness = harness();
    let bucket = flag_bucket(SUBJECT);
    assert!(bucket < 100);

    harness
        .flags
        .create(flag_command("always", 100), SUBJECT)
        .await
        .unwrap();
    assert!(
        harness
            .flags
            .evaluate("always", SUBJECT)
            .await
            .unwrap()
            .enabled
    );

    harness
        .flags
        .create(flag_command("never", 0), SUBJECT)
        .await
        .unwrap();
    assert!(
        !harness
            .flags
            .evaluate("never", SUBJECT)
            .await
            .unwrap()
            .enabled
    );

    let first = harness.flags.evaluate("always", SUBJECT).await.unwrap();
    let second = harness.flags.evaluate("always", SUBJECT).await.unwrap();
    assert_eq!(first, second);
}

#[tokio::test]
async fn an_expired_flag_evaluates_off() {
    let harness = harness();
    let mut request = flag_command("new-dashboard", 100);
    request.expires_at = at(7, 0);
    harness.flags.create(request, SUBJECT).await.unwrap();
    harness.clock.advance(Duration::days(2));
    assert!(
        !harness
            .flags
            .evaluate("new-dashboard", SUBJECT)
            .await
            .unwrap()
            .enabled
    );
}

#[tokio::test]
async fn listing_expired_flags_filters_the_result() {
    let harness = harness();
    let mut expired = flag_command("expired-flag", 10);
    expired.expires_at = at(7, 0);
    harness.flags.create(expired, SUBJECT).await.unwrap();
    harness
        .flags
        .create(flag_command("live-flag", 10), SUBJECT)
        .await
        .unwrap();
    harness.clock.advance(Duration::days(2));
    let flags = harness.flags.list(true).await.unwrap();
    assert_eq!(flags.len(), 1);
    assert_eq!(flags[0].key(), "expired-flag");
    assert_eq!(
        flags[0].effective_status(harness.clock.now()),
        FlagStatus::Expired
    );
}

#[tokio::test]
async fn a_flag_read_is_cached_for_thirty_seconds() {
    let harness = harness();
    harness
        .flags
        .create(flag_command("new-dashboard", 100), SUBJECT)
        .await
        .unwrap();
    assert!(
        harness
            .flags
            .evaluate("new-dashboard", SUBJECT)
            .await
            .unwrap()
            .enabled
    );

    harness
        .flags_repo
        .replace_status("new-dashboard", FlagStatus::Disabled);
    assert!(
        harness
            .flags
            .evaluate("new-dashboard", SUBJECT)
            .await
            .unwrap()
            .enabled
    );

    harness.clock.advance(Duration::seconds(31));
    assert!(
        !harness
            .flags
            .evaluate("new-dashboard", SUBJECT)
            .await
            .unwrap()
            .enabled
    );
}

#[tokio::test]
async fn aggregation_suppresses_small_cohorts_and_stores_large_ones() {
    let harness = harness();
    for user in 0..12 {
        let event = stored_event(
            "valuation_read",
            at(6, 10),
            &format!("user-{user}"),
            BTreeMap::new(),
        );
        harness.events.stored.lock().unwrap().push(event);
    }
    let report = harness.aggregates.run(at(6, 0)).await.unwrap();
    assert_eq!(report.scanned.daily, 12);
    assert!(report
        .stored
        .iter()
        .any(|aggregate| aggregate.metric() == Metric::Engagement));
    assert!(report
        .suppressed
        .iter()
        .any(|candidate| candidate.metric == Metric::Activation));
}

#[tokio::test]
async fn aggregation_stores_activation_for_a_large_signup_cohort() {
    let harness = harness();
    for user in 0..12 {
        let id = format!("user-{user}");
        harness.events.stored.lock().unwrap().push(stored_event(
            "signup_started",
            at(6, 1),
            &id,
            BTreeMap::new(),
        ));
        harness.events.stored.lock().unwrap().push(stored_event(
            "onboarding_completed",
            at(6, 2),
            &id,
            BTreeMap::new(),
        ));
    }
    let report = harness.aggregates.run(at(6, 0)).await.unwrap();
    let activation = report
        .stored
        .iter()
        .find(|aggregate| aggregate.metric() == Metric::Activation)
        .expect("activation is not suppressed for twelve users");
    assert_eq!(activation.value(), rust_decimal::Decimal::ONE);
    assert_eq!(report.scanned.daily, 24);
}

#[tokio::test]
async fn aggregation_ignores_essential_events() {
    let harness = harness();
    for user in 0..12 {
        harness.events.stored.lock().unwrap().push(stored_event(
            "error_shown",
            at(6, 10),
            &format!("user-{user}"),
            BTreeMap::new(),
        ));
    }
    let report = harness.aggregates.run(at(6, 0)).await.unwrap();
    assert!(report.stored.is_empty());
    assert_eq!(report.scanned.daily, 12);
}

#[tokio::test]
async fn aggregation_reads_delegate_to_the_store() {
    let harness = harness();
    for user in 0..12 {
        harness.events.stored.lock().unwrap().push(stored_event(
            "valuation_read",
            at(6, 10),
            &format!("user-{user}"),
            BTreeMap::new(),
        ));
    }
    let report = harness.aggregates.run(at(6, 0)).await.unwrap();
    assert_eq!(report.scanned.daily, 12);
    let listed = harness
        .aggregates
        .list(Some(Metric::Engagement), None, None, None, 10)
        .await
        .unwrap();
    assert_eq!(listed.len(), 1);
}

#[tokio::test]
async fn aggregation_keeps_the_closed_day_complete_above_the_old_window_cap() {
    let day = at(6, 0);
    let mut properties = BTreeMap::new();
    properties.insert("freshness".to_string(), "live".to_string());
    let closed: Vec<_> = (0..12)
        .map(|index| {
            stored_event(
                "valuation_read",
                day + Duration::hours(2),
                &format!("closed-{index}"),
                properties.clone(),
            )
        })
        .collect();
    let events = Arc::new(DenseEvents {
        day_start: day,
        closed,
        prior_count: 205_000,
        calls: Mutex::new(Vec::new()),
    });
    let service = AggregateService::new(
        events.clone(),
        Arc::new(FakeAggregates::default()),
        Arc::new(MutableClock::at(at(6, 10))),
    );

    let report = service.run(day).await.unwrap();

    // The closed day is complete even though the preceding week holds far more
    // events than the old single 200 000-event window could carry.
    assert_eq!(report.scanned.daily, 205_012);
    assert_eq!(report.scanned.cohort_d7, 0);
    assert_eq!(report.scanned.cohort_d30, 0);
    let quality = report
        .stored
        .iter()
        .find(|aggregate| aggregate.metric() == Metric::Quality)
        .expect("the closed day's quality figure survives");
    assert_eq!(quality.value(), rust_decimal::Decimal::ONE);

    let calls = events.calls.lock().unwrap();
    assert_eq!(calls.len(), 3);
    assert_eq!(
        calls[0],
        (
            day - Duration::days(7),
            day + Duration::days(2),
            analytics_domain::MAX_DAILY_WINDOW_EVENTS + 1,
        )
    );
    assert_eq!(
        calls[1],
        (
            day - Duration::days(7),
            day - Duration::days(6),
            analytics_domain::MAX_COHORT_WINDOW_EVENTS + 1,
        )
    );
    assert_eq!(
        calls[2],
        (
            day - Duration::days(30),
            day - Duration::days(29),
            analytics_domain::MAX_COHORT_WINDOW_EVENTS + 1,
        )
    );
}

#[tokio::test]
async fn retention_deletes_old_events_and_consent_checks() {
    let harness = harness();
    harness.events.stored.lock().unwrap().push(stored_event(
        "valuation_read",
        at(6, 10) - Duration::days(200),
        "old-user",
        BTreeMap::new(),
    ));
    harness.events.stored.lock().unwrap().push(stored_event(
        "valuation_read",
        at(6, 10) - Duration::days(10),
        "new-user",
        BTreeMap::new(),
    ));
    let old_check = analytics_domain::ConsentCheck::record(
        analytics_domain::ConsentCheckId::generate(),
        analytics_domain::ConsentPurpose::Analytics,
        true,
        at(6, 10) - Duration::days(100),
        Some("old-user".to_string()),
    );
    harness.consents.stored.lock().unwrap().push(old_check);

    let report = harness.retention.run().await.unwrap();
    assert_eq!(report.events_deleted, 1);
    assert_eq!(report.consent_checks_deleted, 1);
    assert_eq!(harness.events.stored.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn retention_runs_in_bounded_batches() {
    let harness = harness();
    for index in 0..5 {
        harness.events.stored.lock().unwrap().push(stored_event(
            "valuation_read",
            at(6, 10) - Duration::days(200 + index),
            &format!("user-{index}"),
            BTreeMap::new(),
        ));
    }
    let retention = RetentionService::new(
        harness.events.clone(),
        harness.consents.clone(),
        harness.clock.clone(),
        Arc::new(NoPause),
    )
    .with_bounds(2, 10);
    let report = retention.run().await.unwrap();
    assert_eq!(report.events_deleted, 5);
    assert_eq!(*harness.events.delete_calls.lock().unwrap(), 3);
}

#[tokio::test]
async fn erasure_hashes_under_every_retained_version() {
    let harness = harness();
    let user = "22222222-2222-4222-8222-222222222222";
    let v1 = hmac_sha256_hex(&salt("v1").salt, user);
    let v2 = hmac_sha256_hex(&salt("v2").salt, user);
    harness.events.stored.lock().unwrap().push(stored_event(
        "valuation_read",
        at(6, 10),
        &v1,
        BTreeMap::new(),
    ));
    harness.events.stored.lock().unwrap().push(stored_event(
        "valuation_read",
        at(6, 10),
        &v2,
        BTreeMap::new(),
    ));
    harness.events.stored.lock().unwrap().push(stored_event(
        "valuation_read",
        at(6, 10),
        "someone-else",
        BTreeMap::new(),
    ));

    let deleted = harness.erase.erase(user, SUBJECT).await.unwrap();
    assert_eq!(deleted, 2);
    assert_eq!(harness.events.stored.lock().unwrap().len(), 1);
    let records = harness.audit.records.lock().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].actor_kind, "user");
    let expected_actor = pseudonymous_actor_id(&v1);
    assert_eq!(
        records[0].actor_id.as_deref(),
        Some(expected_actor.as_str())
    );
    assert_ne!(records[0].actor_id.as_deref(), Some(user));
    assert_eq!(
        records[0].context.get("sourceService"),
        Some(&SUBJECT.to_string())
    );
    assert_eq!(records[0].context.get("ruleId"), Some(&"BR6.1".to_string()));
}

#[tokio::test]
async fn erasure_deletes_the_users_consent_check_rows_too() {
    let harness = harness();
    let user = "22222222-2222-4222-8222-222222222222";
    let v1 = hmac_sha256_hex(&salt("v1").salt, user);
    harness
        .consents
        .stored
        .lock()
        .unwrap()
        .push(analytics_domain::ConsentCheck::record(
            analytics_domain::ConsentCheckId::generate(),
            analytics_domain::ConsentPurpose::Analytics,
            true,
            at(6, 10),
            Some(v1),
        ));
    harness
        .consents
        .stored
        .lock()
        .unwrap()
        .push(analytics_domain::ConsentCheck::record(
            analytics_domain::ConsentCheckId::generate(),
            analytics_domain::ConsentPurpose::Analytics,
            true,
            at(6, 10),
            Some("someone-else".to_string()),
        ));

    let deleted = harness.erase.erase(user, SUBJECT).await.unwrap();
    assert_eq!(deleted, 0);
    assert_eq!(harness.consents.stored.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn erasure_rejects_a_malformed_identifier() {
    let harness = harness();
    let error = harness
        .erase
        .erase("not-a-uuid", SUBJECT)
        .await
        .unwrap_err();
    match error {
        EraseError::Rejected(errors) => assert_eq!(errors[0].field, "user"),
        other => panic!("expected a rejection, got {other:?}"),
    }
}

#[tokio::test]
async fn erasure_fails_when_the_salt_is_unavailable() {
    let salts = Arc::new(FakeSalts::failing_retained(ExternalError::Unavailable(
        "vault down".to_string(),
    )));
    let audit = Arc::new(FakeAudit::default());
    let clock = Arc::new(MutableClock::at(at(6, 10)));
    let trail = Arc::new(Trail::new(audit, clock));
    let erase = EraseService::new(
        Arc::new(FakeEvents::default()),
        Arc::new(FakeConsents::default()),
        salts,
        trail,
    );
    let error = erase
        .erase("22222222-2222-4222-8222-222222222222", SUBJECT)
        .await
        .unwrap_err();
    assert!(matches!(error, EraseError::SaltUnavailable(_)));
}

#[tokio::test]
async fn erasure_is_idempotent_when_nothing_matches() {
    let harness = harness();
    let deleted = harness
        .erase
        .erase("22222222-2222-4222-8222-222222222222", SUBJECT)
        .await
        .unwrap();
    assert_eq!(deleted, 0);
}

#[tokio::test]
async fn hmac_sha256_matches_the_rfc_4231_vector() {
    let digest = hmac_sha256_hex(b"Jefe", "what do ya want for nothing?");
    assert_eq!(
        digest,
        "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
    );
}

#[tokio::test]
async fn the_flag_bucket_is_stable_and_in_range() {
    let bucket = flag_bucket(SUBJECT);
    assert_eq!(bucket, flag_bucket(SUBJECT));
    assert!(bucket < 100);
}

#[test]
fn sha256_hex_is_a_lowercase_digest() {
    let digest = sha256_hex("abc");
    assert_eq!(digest.len(), 64);
    assert_eq!(
        digest,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn the_harness_salt_provider_is_usable() {
    let salts = FakeSalts::working(salt("v1"), vec![salt("v1")]);
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let current = runtime.block_on(salts.current()).unwrap();
    assert_eq!(current.salt.len(), 32);
}
