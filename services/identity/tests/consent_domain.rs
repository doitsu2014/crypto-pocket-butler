//! U4 domain tests: the consent kind and action closed sets, the newest-event
//! state and its `missing` computation, and the preference closed sets and
//! effective resolution (BR4.1–BR4.11).

use chrono::{DateTime, TimeZone as _, Utc};
use identity_domain::consent::{
    parse_iana_timezone, BaseCurrencyCode, ConsentAction, ConsentEvent, ConsentKind, ConsentPolicy,
    ConsentState, ConsentStateView, EffectivePreferences, PreferenceSource, PreferenceUpdate,
    ProviderProfile, StoredPreferences,
};
use identity_domain::ids::{ConsentRecordId, UserId};
use identity_domain::preferences::{BaseCurrency, Locale, TimeZone};

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 10, hour, 0, 0).unwrap()
}

fn event(user_id: UserId, seq: i64, kind: ConsentKind, action: ConsentAction) -> ConsentEvent {
    ConsentEvent::from_stored(
        ConsentRecordId::generate(),
        seq,
        user_id,
        kind,
        action,
        at(9),
        "2026-11-01.1",
    )
}

#[test]
fn consent_kinds_parse_the_five_values_and_refuse_an_unknown_one() {
    assert_eq!(
        ConsentKind::ALL.map(ConsentKind::as_str),
        ["terms", "risk", "age", "analytics", "marketing"]
    );
    for kind in ConsentKind::ALL {
        assert_eq!(ConsentKind::parse(kind.as_str()), Ok(kind));
    }
    assert!(ConsentKind::parse("disclosures").is_err());
    assert!(ConsentKind::parse("").is_err());
    assert_eq!(
        ConsentKind::REQUIRED.map(ConsentKind::as_str),
        ["terms", "risk", "age"],
        "analytics and marketing are never required"
    );
    assert!(ConsentKind::Terms.is_required());
    assert!(!ConsentKind::Analytics.is_required());
}

#[test]
fn consent_actions_states_and_policy_versions_name_their_stored_values() {
    assert_eq!(ConsentAction::parse("granted"), Ok(ConsentAction::Granted));
    assert_eq!(
        ConsentAction::parse("withdrawn"),
        Ok(ConsentAction::Withdrawn)
    );
    assert!(ConsentAction::parse("maybe").is_err());

    assert_eq!(ConsentState::None.as_str(), "none");
    assert_eq!(ConsentState::Granted.as_str(), "granted");
    assert_eq!(ConsentState::Withdrawn.as_str(), "withdrawn");

    assert_eq!(
        ConsentPolicy::parse("  2026-11-01.1 ").unwrap().as_str(),
        "2026-11-01.1"
    );
    let blank = ConsentPolicy::parse("   ");
    assert!(blank.is_err(), "a blank policy version is refused");
    let long = format!("v{}", "1".repeat(120));
    assert!(
        ConsentPolicy::parse(&long).is_err(),
        "a policy version over 120 characters is refused"
    );
}

#[test]
fn missing_lists_every_required_kind_that_is_not_granted_and_never_analytics() {
    let user = UserId::generate();
    let view = ConsentStateView::from_newest(
        user,
        [
            event(user, 1, ConsentKind::Terms, ConsentAction::Granted),
            event(user, 2, ConsentKind::Risk, ConsentAction::Withdrawn),
            event(user, 3, ConsentKind::Analytics, ConsentAction::Withdrawn),
        ],
    );

    assert_eq!(view.state(ConsentKind::Terms), ConsentState::Granted);
    assert_eq!(view.state(ConsentKind::Risk), ConsentState::Withdrawn);
    assert_eq!(view.state(ConsentKind::Age), ConsentState::None);
    assert_eq!(view.state(ConsentKind::Marketing), ConsentState::None);
    assert_eq!(view.missing(), vec![ConsentKind::Risk, ConsentKind::Age]);
    assert!(view.has_missing());
}

#[test]
fn missing_is_empty_when_the_three_required_kinds_are_granted() {
    let user = UserId::generate();
    let view = ConsentStateView::from_newest(
        user,
        [
            event(user, 1, ConsentKind::Terms, ConsentAction::Granted),
            event(user, 2, ConsentKind::Risk, ConsentAction::Granted),
            event(user, 3, ConsentKind::Age, ConsentAction::Granted),
        ],
    );

    assert!(view.missing().is_empty());
    assert!(!view.has_missing());
    assert_eq!(
        view.state(ConsentKind::Analytics),
        ConsentState::None,
        "an optional kind is absent, not missing"
    );
}

#[test]
fn a_regranted_kind_is_not_missing_and_a_withdrawn_one_is() {
    let user = UserId::generate();
    let view = ConsentStateView::from_newest(
        user,
        [
            event(user, 1, ConsentKind::Terms, ConsentAction::Granted),
            event(user, 2, ConsentKind::Terms, ConsentAction::Withdrawn),
            event(user, 3, ConsentKind::Terms, ConsentAction::Granted),
            event(user, 4, ConsentKind::Age, ConsentAction::Granted),
            event(user, 5, ConsentKind::Risk, ConsentAction::Granted),
            event(user, 6, ConsentKind::Risk, ConsentAction::Withdrawn),
        ],
    );

    assert_eq!(view.state(ConsentKind::Terms), ConsentState::Granted);
    assert_eq!(view.state(ConsentKind::Risk), ConsentState::Withdrawn);
    assert_eq!(view.missing(), vec![ConsentKind::Risk]);
}

#[test]
fn preference_closed_sets_refuse_values_outside_them() {
    assert_eq!(
        BaseCurrencyCode::ALL.map(BaseCurrencyCode::as_str),
        ["USD", "EUR", "GBP", "VND"]
    );
    for code in BaseCurrencyCode::ALL {
        assert_eq!(BaseCurrencyCode::parse(code.as_str()), Ok(code));
    }
    assert!(BaseCurrencyCode::parse("CAD").is_err(), "outside the set");
    assert!(BaseCurrencyCode::parse("usd").is_err(), "lower case");
    assert!(BaseCurrencyCode::parse("US").is_err(), "too short");

    assert!(parse_iana_timezone("Europe/London").is_ok());
    assert_eq!(parse_iana_timezone("UTC").unwrap().as_str(), "UTC");
    assert!(parse_iana_timezone("Not/AZone").is_err());
    assert!(parse_iana_timezone("").is_err());

    assert_eq!(Locale::parse("vi").unwrap(), Locale::Vi);
    assert!(Locale::parse("fr").is_err());
}

#[test]
fn effective_preferences_prefer_the_stored_choice_and_mark_its_source() {
    let user = UserId::generate();
    let stored = StoredPreferences {
        locale: Some(Locale::Vi),
        base_currency: Some(BaseCurrencyCode::Eur),
        timezone: Some(parse_iana_timezone("Europe/London").unwrap()),
        notification_email: Some(true),
        notification_push: None,
    };
    let provider = ProviderProfile::new(
        Locale::En,
        BaseCurrency::parse("USD").unwrap(),
        TimeZone::parse("UTC").unwrap(),
    );

    let effective = EffectivePreferences::resolve(user, Some(&stored), &provider);

    assert_eq!(effective.locale().value(), &Locale::Vi);
    assert_eq!(effective.locale().source(), PreferenceSource::User);
    assert_eq!(effective.base_currency().value(), &BaseCurrencyCode::Eur);
    assert_eq!(effective.base_currency().source(), PreferenceSource::User);
    assert_eq!(effective.timezone().value().as_str(), "Europe/London");
    assert_eq!(effective.timezone().source(), PreferenceSource::User);
    assert_eq!(effective.notification_email().value(), &true);
    assert_eq!(
        effective.notification_email().source(),
        PreferenceSource::User
    );
    assert_eq!(
        effective.notification_push().value(),
        &false,
        "an unset channel is not enabled"
    );
    assert_eq!(
        effective.notification_push().source(),
        PreferenceSource::Default
    );
}

#[test]
fn effective_preferences_fall_back_to_the_provider_then_the_default() {
    let user = UserId::generate();
    let provider = ProviderProfile::new(
        Locale::Vi,
        BaseCurrency::parse("GBP").unwrap(),
        TimeZone::parse("Asia/Ho_Chi_Minh").unwrap(),
    );

    let effective = EffectivePreferences::resolve(user, None, &provider);

    assert_eq!(effective.locale().value(), &Locale::Vi);
    assert_eq!(effective.locale().source(), PreferenceSource::Default);
    assert_eq!(effective.base_currency().value(), &BaseCurrencyCode::Gbp);
    assert_eq!(
        effective.base_currency().source(),
        PreferenceSource::Default,
        "a provider-derived value is not the user's choice"
    );
    assert_eq!(effective.timezone().value().as_str(), "Asia/Ho_Chi_Minh");
    assert_eq!(effective.timezone().source(), PreferenceSource::Default);

    let no_provider_value = ProviderProfile::new(
        Locale::En,
        BaseCurrency::parse("USD").unwrap(),
        TimeZone::parse("UTC").unwrap(),
    );
    let defaults =
        EffectivePreferences::resolve(user, Some(&StoredPreferences::empty()), &no_provider_value);
    assert_eq!(defaults.base_currency().value(), &BaseCurrencyCode::Usd);
}

#[test]
fn effective_preferences_ignore_a_provider_value_outside_the_closed_set() {
    let user = UserId::generate();
    // u1 stores any three-letter ISO code and an IANA-lexical name; U4's read
    // must not echo a value the preference set cannot hold.
    let provider = ProviderProfile::new(
        Locale::En,
        BaseCurrency::parse("CAD").unwrap(),
        TimeZone::parse("Mars/Olympus").unwrap(),
    );

    let effective = EffectivePreferences::resolve(user, None, &provider);

    assert_eq!(effective.base_currency().value(), &BaseCurrencyCode::Usd);
    assert_eq!(effective.timezone().value().as_str(), "UTC");
}

#[test]
fn a_partial_update_supplies_only_its_fields_and_can_be_empty() {
    let update = PreferenceUpdate {
        locale: Some(Locale::Vi),
        base_currency: None,
        timezone: None,
        notification_email: None,
        notification_push: None,
    };
    assert!(!update.is_empty());
    assert!(PreferenceUpdate::default().is_empty());
}
