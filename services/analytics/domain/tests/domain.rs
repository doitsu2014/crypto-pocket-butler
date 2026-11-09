//! Domain tests: catalogue and PII rules, flags, aggregates, retention cutoffs.

use std::collections::BTreeMap;

use analytics_domain::aggregate::MetricAggregate;
use analytics_domain::aggregation::{compute_metrics, AggregateInputs, EventFact};
use analytics_domain::audit::RefusalKind;
use analytics_domain::consent::ConsentPurpose;
use analytics_domain::event::{sanitize_event, EventClass};
use analytics_domain::flag::{FeatureFlag, FlagStatus, FlagUpdate, NewFeatureFlag};
use analytics_domain::pii::{detect_pii, PiiKind};
use analytics_domain::{
    cutoff_consent_checks_before, cutoff_six_months_before, ConsentCheck, ConsentCheckId,
    EventDefinition, FeatureFlagId, Metric, MetricAggregateId, MetricCandidate, Period, CATALOGUE,
    SUPPRESSION_MIN_USERS,
};
use chrono::{DateTime, Duration, TimeZone, Utc};
use rust_decimal::Decimal;

fn at(day: u32, hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, day, hour, 0, 0).unwrap()
}

fn props(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect()
}

fn fact(name: &str, when: DateTime<Utc>, user: &str) -> EventFact {
    EventFact {
        event_name: name.to_string(),
        occurred_at: when,
        user_ref: Some(user.to_string()),
        properties: BTreeMap::new(),
    }
}

fn compute(events: &[EventFact], day: DateTime<Utc>) -> Vec<MetricCandidate> {
    compute_metrics(
        AggregateInputs {
            daily: events,
            cohort_d7: &[],
            cohort_d30: &[],
        },
        day,
    )
}

fn flag(now: DateTime<Utc>) -> FeatureFlag {
    FeatureFlag::create(
        FeatureFlagId::generate(),
        NewFeatureFlag {
            key: "new-dashboard".to_string(),
            owner: "product".to_string(),
            expires_at: now + Duration::days(30),
            rollout_percentage: 25,
            status: Some("active".to_string()),
        },
        now,
    )
    .expect("a valid flag is created")
}

#[test]
fn the_catalogue_has_the_nine_documented_events() {
    assert_eq!(CATALOGUE.len(), 9);
    let names: Vec<&str> = CATALOGUE.iter().map(|entry| entry.name).collect();
    for expected in [
        "signup_started",
        "onboarding_step",
        "onboarding_completed",
        "valuation_read",
        "account_connected",
        "drift_viewed",
        "upgrade_started",
        "support_opened",
        "error_shown",
    ] {
        assert!(names.contains(&expected), "catalogue is missing {expected}");
    }
}

#[test]
fn only_error_shown_is_essential() {
    let essential: Vec<&str> = CATALOGUE
        .iter()
        .filter(|entry| entry.class == EventClass::Essential)
        .map(|entry| entry.name)
        .collect();
    assert_eq!(essential, vec!["error_shown"]);
}

#[test]
fn sanitize_keeps_only_allow_listed_properties() {
    let sanitized = sanitize_event(
        "valuation_read",
        at(6, 10),
        props(&[
            ("freshness", "live"),
            ("freeText", "hello"),
            ("asset_count_bucket", "3"),
        ]),
    )
    .unwrap();
    assert_eq!(sanitized.properties().len(), 2);
    assert_eq!(sanitized.properties().get("freshness").unwrap(), "live");
    assert!(sanitized
        .dropped_properties()
        .contains(&"freeText".to_string()));
}

#[test]
fn sanitize_refuses_an_unknown_event_name() {
    let error = sanitize_event("not_in_catalogue", at(6, 10), BTreeMap::new()).unwrap_err();
    assert_eq!(error.field(), "eventName");
    assert_eq!(
        error.to_string(),
        "eventName is not in the closed event catalogue"
    );
}

#[test]
fn sanitize_refuses_a_pii_shaped_event_name_without_echoing_it() {
    let email = "person@example.com";
    let error = sanitize_event(email, at(6, 10), BTreeMap::new()).unwrap_err();
    assert_eq!(error.field(), "eventName");
    assert!(!error.to_string().contains(email));
}

#[test]
fn sanitize_refuses_a_blank_event_name() {
    let error = sanitize_event("   ", at(6, 10), BTreeMap::new()).unwrap_err();
    assert_eq!(error.field(), "eventName");
}

#[test]
fn sanitize_drops_an_email_property() {
    let sanitized = sanitize_event(
        "support_opened",
        at(6, 10),
        props(&[("entry", "person@example.com")]),
    )
    .unwrap();
    assert!(sanitized.properties().is_empty());
    assert_eq!(sanitized.dropped_properties(), &["entry".to_string()]);
}

#[test]
fn sanitize_drops_an_evm_address_property() {
    let sanitized = sanitize_event(
        "account_connected",
        at(6, 10),
        props(&[("kind", "0x1111111111111111111111111111111111111111")]),
    )
    .unwrap();
    assert!(sanitized.properties().is_empty());
}

#[test]
fn sanitize_drops_a_base58_key_property() {
    let key = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";
    let sanitized =
        sanitize_event("account_connected", at(6, 10), props(&[("kind", key)])).unwrap();
    assert!(sanitized.properties().is_empty());
}

#[test]
fn sanitize_drops_ipv4_and_ipv6_properties() {
    for value in ["192.168.1.10", "2001:db8::1"] {
        let sanitized =
            sanitize_event("error_shown", at(6, 10), props(&[("route", value)])).unwrap();
        assert!(sanitized.properties().is_empty(), "{value} must be dropped");
    }
}

#[test]
fn sanitize_drops_a_secret_shaped_property() {
    for value in [
        "sk-live-1234567890abcdef",
        "Bearer abcdef1234567890",
        "abcdef1234567890abcdef",
    ] {
        let sanitized =
            sanitize_event("error_shown", at(6, 10), props(&[("code", value)])).unwrap();
        assert!(sanitized.properties().is_empty(), "{value} must be dropped");
    }
}

#[test]
fn sanitize_keeps_ordinary_property_values() {
    let sanitized = sanitize_event(
        "onboarding_completed",
        at(6, 10),
        props(&[("path", "csv"), ("duration_bucket", "1-5m")]),
    )
    .unwrap();
    assert_eq!(sanitized.properties().get("path").unwrap(), "csv");
    assert_eq!(
        sanitized.properties().get("duration_bucket").unwrap(),
        "1-5m"
    );
}

#[test]
fn the_pii_detector_classifies_each_shape() {
    assert_eq!(detect_pii("a@b.co"), Some(PiiKind::Email));
    assert_eq!(
        detect_pii("0x1111111111111111111111111111111111111111"),
        Some(PiiKind::EvmAddress)
    );
    assert_eq!(detect_pii("192.168.0.1"), Some(PiiKind::IpLiteral));
    assert_eq!(detect_pii("2001:db8::1"), Some(PiiKind::IpLiteral));
    assert_eq!(
        detect_pii("sk-live-1234567890abcdef"),
        Some(PiiKind::SecretShaped)
    );
    assert_eq!(detect_pii("csv"), None);
}

#[test]
fn an_event_class_is_parsed_and_printed() {
    assert_eq!(
        EventClass::parse("essential").unwrap(),
        EventClass::Essential
    );
    assert_eq!(EventClass::Analytics.as_str(), "analytics");
    assert!(EventClass::parse("other").is_err());
}

#[test]
fn a_consent_check_records_its_outcome() {
    let check = ConsentCheck::record(
        ConsentCheckId::generate(),
        ConsentPurpose::Analytics,
        true,
        at(6, 10),
        Some("hash".to_string()),
    );
    assert!(check.granted());
    assert_eq!(check.purpose(), ConsentPurpose::Analytics);
    assert_eq!(check.user_ref(), Some("hash"));
    assert_eq!(
        ConsentPurpose::parse("essential").unwrap(),
        ConsentPurpose::Essential
    );
    assert!(ConsentPurpose::parse("marketing").is_err());
}

#[test]
fn a_flag_is_created_with_governance_metadata() {
    let created = flag(at(6, 10));
    assert_eq!(created.key(), "new-dashboard");
    assert_eq!(created.owner(), "product");
    assert_eq!(created.rollout_percentage(), 25);
    assert_eq!(created.status(), FlagStatus::Active);
}

#[test]
fn a_flag_requires_an_owner() {
    let error = FeatureFlag::create(
        FeatureFlagId::generate(),
        NewFeatureFlag {
            key: "k".to_string(),
            owner: "   ".to_string(),
            expires_at: at(7, 10),
            rollout_percentage: 10,
            status: None,
        },
        at(6, 10),
    )
    .unwrap_err();
    assert_eq!(error.field(), "owner");
}

#[test]
fn a_flag_expiry_must_be_in_the_future() {
    let error = FeatureFlag::create(
        FeatureFlagId::generate(),
        NewFeatureFlag {
            key: "k".to_string(),
            owner: "product".to_string(),
            expires_at: at(5, 10),
            rollout_percentage: 10,
            status: None,
        },
        at(6, 10),
    )
    .unwrap_err();
    assert_eq!(error.field(), "expiresAt");
}

#[test]
fn a_rollout_above_one_hundred_is_refused() {
    let error = FeatureFlag::create(
        FeatureFlagId::generate(),
        NewFeatureFlag {
            key: "k".to_string(),
            owner: "product".to_string(),
            expires_at: at(7, 10),
            rollout_percentage: 101,
            status: None,
        },
        at(6, 10),
    )
    .unwrap_err();
    assert_eq!(error.field(), "rolloutPercentage");
}

#[test]
fn an_expired_status_cannot_be_written() {
    let error = FeatureFlag::create(
        FeatureFlagId::generate(),
        NewFeatureFlag {
            key: "k".to_string(),
            owner: "product".to_string(),
            expires_at: at(7, 10),
            rollout_percentage: 10,
            status: Some("expired".to_string()),
        },
        at(6, 10),
    )
    .unwrap_err();
    assert_eq!(error.field(), "status");
}

#[test]
fn a_flag_key_may_not_carry_spaces() {
    let error = FeatureFlag::create(
        FeatureFlagId::generate(),
        NewFeatureFlag {
            key: "my flag".to_string(),
            owner: "product".to_string(),
            expires_at: at(7, 10),
            rollout_percentage: 10,
            status: None,
        },
        at(6, 10),
    )
    .unwrap_err();
    assert_eq!(error.field(), "key");
}

#[test]
fn an_expired_flag_is_off_and_derived() {
    let created = flag(at(6, 10));
    assert_eq!(
        created.effective_status(created.expires_at() + Duration::seconds(1)),
        FlagStatus::Expired
    );
    assert!(!created.enabled_for_bucket(1, created.expires_at() + Duration::seconds(1)));
}

#[test]
fn a_bucket_below_the_rollout_is_enabled() {
    let created = flag(at(6, 10));
    assert!(created.enabled_for_bucket(24, at(6, 11)));
    assert!(!created.enabled_for_bucket(25, at(6, 11)));
}

#[test]
fn a_disabled_flag_is_off_for_every_bucket() {
    let mut created = flag(at(6, 10));
    created
        .apply_update(
            FlagUpdate {
                status: Some("disabled".to_string()),
                ..FlagUpdate::default()
            },
            at(6, 11),
        )
        .unwrap();
    assert!(!created.enabled_for_bucket(0, at(6, 12)));
}

#[test]
fn an_update_rejects_a_past_expiry() {
    let mut created = flag(at(6, 10));
    let error = created
        .apply_update(
            FlagUpdate {
                expires_at: Some(at(5, 10)),
                ..FlagUpdate::default()
            },
            at(6, 11),
        )
        .unwrap_err();
    assert_eq!(error.field(), "expiresAt");
}

#[test]
fn a_metric_candidate_is_suppressed_below_ten_users() {
    let candidate = MetricCandidate {
        metric: Metric::Activation,
        period: Period::Daily,
        cohort: "all".to_string(),
        period_start: at(6, 0),
        value: Decimal::new(5, 1),
        distinct_users: SUPPRESSION_MIN_USERS - 1,
    };
    assert!(candidate.is_suppressed());
}

#[test]
fn an_aggregate_refuses_a_value_above_one() {
    let candidate = MetricCandidate {
        metric: Metric::Quality,
        period: Period::Daily,
        cohort: "all".to_string(),
        period_start: at(6, 0),
        value: Decimal::new(15, 1),
        distinct_users: 20,
    };
    assert!(
        MetricAggregate::from_candidate(MetricAggregateId::generate(), &candidate, at(6, 12))
            .is_err()
    );
}

#[test]
fn activation_counts_completion_within_twenty_four_hours() {
    let day = at(6, 0);
    let mut events = Vec::new();
    for user in 0..10 {
        let id = format!("user-{user}");
        events.push(fact("signup_started", day + Duration::hours(1), &id));
    }
    for user in 0..9 {
        let id = format!("user-{user}");
        events.push(fact("onboarding_completed", day + Duration::hours(3), &id));
    }
    // The tenth user completes after the 24-hour window.
    events.push(fact(
        "onboarding_completed",
        day + Duration::hours(30),
        "user-9",
    ));

    let metrics = compute(&events, day);
    let activation = metrics
        .iter()
        .find(|candidate| candidate.metric == Metric::Activation)
        .unwrap();
    assert_eq!(activation.value, Decimal::new(9, 1));
    assert_eq!(activation.distinct_users, 10);
}

#[test]
fn engagement_divides_daily_by_weekly_active() {
    let day = at(6, 0);
    let mut events = Vec::new();
    for user in 0..10 {
        events.push(fact(
            "valuation_read",
            day + Duration::hours(1),
            &format!("day-{user}"),
        ));
    }
    for user in 0..10 {
        events.push(fact(
            "valuation_read",
            day - Duration::days(2),
            &format!("week-{user}"),
        ));
    }
    let metrics = compute(&events, day);
    let engagement = metrics
        .iter()
        .find(|candidate| candidate.metric == Metric::Engagement)
        .unwrap();
    assert_eq!(engagement.value, Decimal::new(5, 1));
    assert_eq!(engagement.distinct_users, 20);
}

#[test]
fn conversion_divides_upgrades_by_active_users() {
    let day = at(6, 0);
    let mut events = Vec::new();
    for user in 0..10 {
        events.push(fact(
            "valuation_read",
            day + Duration::hours(1),
            &format!("u{user}"),
        ));
    }
    for user in 0..2 {
        events.push(fact(
            "upgrade_started",
            day + Duration::hours(2),
            &format!("u{user}"),
        ));
    }
    let metrics = compute(&events, day);
    let conversion = metrics
        .iter()
        .find(|candidate| candidate.metric == Metric::Conversion)
        .unwrap();
    assert_eq!(conversion.value, Decimal::new(2, 1));
}

#[test]
fn quality_divides_live_valuation_reads_by_all_reads() {
    let day = at(6, 0);
    let mut events = Vec::new();
    for (index, freshness) in ["live", "live", "live", "stale"].iter().enumerate() {
        let mut event = fact(
            "valuation_read",
            day + Duration::hours(1),
            &format!("u{index}"),
        );
        event
            .properties
            .insert("freshness".to_string(), (*freshness).to_string());
        events.push(event);
    }
    let metrics = compute(&events, day);
    let quality = metrics
        .iter()
        .find(|candidate| candidate.metric == Metric::Quality)
        .unwrap();
    assert_eq!(quality.value, Decimal::new(75, 2));
}

#[test]
fn retention_uses_the_day_seven_signup_cohort_and_reports_retention_d7() {
    let day = at(6, 0);
    let mut cohort = Vec::new();
    for user in 0..10 {
        cohort.push(fact(
            "signup_started",
            day - Duration::days(7) + Duration::hours(2),
            &format!("u{user}"),
        ));
    }
    let mut daily = Vec::new();
    for user in 0..4 {
        daily.push(fact(
            "valuation_read",
            day + Duration::hours(2),
            &format!("u{user}"),
        ));
    }
    let metrics = compute_metrics(
        AggregateInputs {
            daily: &daily,
            cohort_d7: &cohort,
            cohort_d30: &[],
        },
        day,
    );
    let retention = metrics
        .iter()
        .find(|candidate| candidate.metric == Metric::RetentionD7)
        .unwrap();
    assert_eq!(retention.value, Decimal::new(4, 1));
    assert_eq!(retention.period, Period::Weekly);
    assert_eq!(retention.cohort, "all");
    assert_eq!(Metric::RetentionD7.as_str(), "retention_d7");
}

#[test]
fn retention_uses_the_day_thirty_signup_cohort_and_reports_retention_d30() {
    let day = at(6, 0);
    let mut cohort = Vec::new();
    for user in 0..10 {
        cohort.push(fact(
            "signup_started",
            day - Duration::days(30) + Duration::hours(2),
            &format!("u{user}"),
        ));
    }
    let mut daily = Vec::new();
    for user in 0..7 {
        daily.push(fact(
            "valuation_read",
            day + Duration::hours(2),
            &format!("u{user}"),
        ));
    }
    let metrics = compute_metrics(
        AggregateInputs {
            daily: &daily,
            cohort_d7: &[],
            cohort_d30: &cohort,
        },
        day,
    );
    let retention = metrics
        .iter()
        .find(|candidate| candidate.metric == Metric::RetentionD30)
        .unwrap();
    assert_eq!(retention.value, Decimal::new(7, 1));
    assert_eq!(retention.cohort, "all");
    assert_eq!(Metric::RetentionD30.as_str(), "retention_d30");
}

#[test]
fn a_small_cohort_is_reported_suppressed() {
    let day = at(6, 0);
    let mut events = Vec::new();
    for user in 0..3 {
        events.push(fact(
            "signup_started",
            day + Duration::hours(1),
            &format!("u{user}"),
        ));
    }
    let metrics = compute(&events, day);
    let activation = metrics
        .iter()
        .find(|candidate| candidate.metric == Metric::Activation)
        .unwrap();
    assert!(activation.is_suppressed());
}

#[test]
fn the_retention_cutoffs_are_six_months_and_ninety_days() {
    let now = at(6, 12);
    assert_eq!(
        cutoff_six_months_before(now),
        Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap()
    );
    assert_eq!(cutoff_consent_checks_before(now), now - Duration::days(90));
}

#[test]
fn a_stored_aggregate_round_trips_its_labels() {
    let aggregate = MetricAggregate::from_stored(
        MetricAggregateId::generate(),
        "quality",
        "daily",
        at(6, 0),
        "all",
        Decimal::new(75, 2),
        at(6, 12),
    )
    .unwrap();
    assert_eq!(aggregate.metric(), Metric::Quality);
    assert_eq!(aggregate.period(), Period::Daily);
    assert_eq!(aggregate.cohort(), "all");
}

#[test]
fn refusal_kinds_have_stable_labels() {
    assert_eq!(
        RefusalKind::ConsentNotGranted.as_str(),
        "consent-not-granted"
    );
    assert_eq!(RefusalKind::SaltUnavailable.as_str(), "salt-unavailable");
}

#[test]
fn the_catalogue_definitions_are_well_formed() {
    for EventDefinition {
        name,
        allowed_properties,
        ..
    } in CATALOGUE
    {
        assert!(!name.is_empty());
        assert!(!allowed_properties.is_empty());
    }
}
