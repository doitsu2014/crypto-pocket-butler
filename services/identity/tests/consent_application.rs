//! U4 application tests: the consent-state query, grant, withdrawal (including
//! the outbox failure path), history, and the preference read/write use cases
//! over port fakes.

mod common;

use std::sync::Arc;

use common::fakes::{
    FakeConsentRepository, FakeObservationSink, FakePreferencesRepository, FakeUserLookup,
    MutableClock,
};
use common::{active_user, caller, claims, fixed_now};
use identity_application::common::{AppError, Caller};
use identity_application::consent::ports::{
    ConsentRepository as ConsentEventRepository, HistoryCursor, PreferenceWriteOutcome,
    StateQueryOutcome, WithdrawalRollbackCause,
};
use identity_application::consent::{
    ConsentGrant, ConsentHistory, ConsentStateQuery, ConsentWithdraw, WithdrawResult,
};
use identity_application::ports::RepositoryError;
use identity_application::preferences::read::ReadUserPreferences;
use identity_application::preferences::write::WriteUserPreferences;
use identity_domain::consent::{
    ConsentAction, ConsentEvent, ConsentEventDraft, ConsentKind, ConsentPolicy, ConsentState,
};
use identity_domain::ids::{ConsentRecordId, UserId};
use identity_domain::preferences::{BaseCurrency, Locale, TimeZone};
use identity_domain::{PreferenceUpdate, ProviderProfile};

struct ConsentHarness {
    repository: Arc<FakeConsentRepository>,
    observation: Arc<FakeObservationSink>,
    state: ConsentStateQuery,
    grant: ConsentGrant,
    withdraw: ConsentWithdraw,
    history: ConsentHistory,
    caller: Caller,
    user_id: UserId,
}

fn consent_harness() -> ConsentHarness {
    let user = active_user();
    let user_id = user.id();
    let repository = Arc::new(FakeConsentRepository::default());
    let observation = Arc::new(FakeObservationSink::default());
    ConsentHarness {
        state: ConsentStateQuery::new(repository.clone(), observation.clone()),
        grant: ConsentGrant::new(repository.clone(), observation.clone()),
        withdraw: ConsentWithdraw::new(repository.clone(), observation.clone()),
        history: ConsentHistory::new(repository.clone()),
        repository,
        observation,
        caller: caller(user, claims()),
        user_id,
    }
}

fn grant_event(user_id: UserId, kind: ConsentKind, version: &str) -> ConsentEvent {
    ConsentEvent::from_stored(
        ConsentRecordId::generate(),
        1,
        user_id,
        kind,
        ConsentAction::Granted,
        fixed_now(),
        version,
    )
}

fn policy(version: &str) -> ConsentPolicy {
    ConsentPolicy::parse(version).expect("the fixture policy version is valid")
}

struct PreferencesHarness {
    repository: Arc<FakePreferencesRepository>,
    users: Arc<FakeUserLookup>,
    observation: Arc<FakeObservationSink>,
    read: ReadUserPreferences,
    write: WriteUserPreferences,
    caller: Caller,
    user_id: UserId,
}

fn preferences_harness() -> PreferencesHarness {
    let user = active_user();
    let user_id = user.id();
    let repository = Arc::new(FakePreferencesRepository::default());
    let users = Arc::new(FakeUserLookup::default());
    let observation = Arc::new(FakeObservationSink::default());
    users.seed(
        user_id,
        ProviderProfile::new(
            Locale::En,
            BaseCurrency::parse("USD").unwrap(),
            TimeZone::parse("UTC").unwrap(),
        ),
    );
    let clock = Arc::new(MutableClock::at(fixed_now()));
    PreferencesHarness {
        read: ReadUserPreferences::new(repository.clone(), users.clone()),
        write: WriteUserPreferences::new(
            repository.clone(),
            users.clone(),
            clock,
            observation.clone(),
        ),
        repository,
        users,
        observation,
        caller: caller(user, claims()),
        user_id,
    }
}

#[tokio::test]
async fn state_query_answers_every_kind_and_reports_missing_once() {
    let harness = consent_harness();
    harness.repository.seed(grant_event(
        harness.user_id,
        ConsentKind::Terms,
        "2026-11-01.1",
    ));
    harness.repository.seed(ConsentEvent::from_stored(
        ConsentRecordId::generate(),
        2,
        harness.user_id,
        ConsentKind::Risk,
        ConsentAction::Withdrawn,
        fixed_now(),
        "2026-11-01.1",
    ));

    let view = harness.state.state(harness.user_id).await.unwrap();

    assert_eq!(view.state(ConsentKind::Terms), ConsentState::Granted);
    assert_eq!(view.state(ConsentKind::Risk), ConsentState::Withdrawn);
    assert_eq!(view.state(ConsentKind::Age), ConsentState::None);
    assert_eq!(view.missing(), vec![ConsentKind::Risk, ConsentKind::Age]);
    assert_eq!(
        harness.observation.state_queries.lock().unwrap().as_slice(),
        [(StateQueryOutcome::Answered, true)],
        "one query counter, never one per kind"
    );
}

#[tokio::test]
async fn state_query_fails_closed_when_the_store_is_unreadable() {
    let harness = consent_harness();
    harness.repository.fail_next(RepositoryError::Unavailable(
        "connection refused".to_string(),
    ));

    let error = harness.state.state(harness.user_id).await.unwrap_err();

    assert!(matches!(error, AppError::Unavailable(_)));
    assert_eq!(
        harness.observation.state_queries.lock().unwrap().as_slice(),
        [(StateQueryOutcome::Error, false)],
        "a failed read is never an empty, permissive answer"
    );
}

#[tokio::test]
async fn grant_records_a_granted_event_and_reports_its_outcome() {
    let harness = consent_harness();

    let event = harness
        .grant
        .grant(&harness.caller, ConsentKind::Terms, policy("2026-11-01.1"))
        .await
        .unwrap();

    assert_eq!(event.kind(), ConsentKind::Terms);
    assert_eq!(event.action(), ConsentAction::Granted);
    assert_eq!(harness.repository.events().len(), 1);
    assert_eq!(
        harness.observation.events.lock().unwrap().as_slice(),
        [(ConsentKind::Terms, ConsentAction::Granted)]
    );
}

#[tokio::test]
async fn grant_surfaces_an_insert_failure_and_never_claims_success() {
    let harness = consent_harness();
    harness
        .repository
        .fail_next(RepositoryError::Unavailable("write failed".to_string()));

    let error = harness
        .grant
        .grant(&harness.caller, ConsentKind::Risk, policy("2026-11-01.1"))
        .await
        .unwrap_err();

    assert!(matches!(error, AppError::Unavailable(_)));
    assert!(harness.repository.events().is_empty());
    assert!(harness.observation.events.lock().unwrap().is_empty());
}

#[tokio::test]
async fn withdraw_records_the_inherited_policy_version_and_enqueues_the_audit_intent() {
    let harness = consent_harness();
    harness.repository.seed(grant_event(
        harness.user_id,
        ConsentKind::Terms,
        "2026-11-01.7",
    ));

    let result = harness
        .withdraw
        .withdraw(&harness.caller, ConsentKind::Terms)
        .await
        .unwrap();

    let WithdrawResult::Withdrawn(event) = result else {
        panic!("a withdrawal of a granted kind writes an event");
    };
    assert_eq!(event.action(), ConsentAction::Withdrawn);
    assert_eq!(
        event.policy_version(),
        "2026-11-01.7",
        "the withdrawal carries the grant's version, never a client-supplied one"
    );
    let outbox = harness.repository.outbox();
    assert_eq!(outbox.len(), 1);
    assert_eq!(outbox[0].action, "identity.consent.withdrawn");
    assert_eq!(outbox[0].target, "consent:terms");
    assert_eq!(outbox[0].context["policyVersion"], "2026-11-01.7");
    assert_eq!(harness.repository.commits(), 1);
}

#[tokio::test]
async fn withdrawing_an_already_withdrawn_kind_is_a_no_op() {
    let harness = consent_harness();
    harness.repository.seed(ConsentEvent::from_stored(
        ConsentRecordId::generate(),
        1,
        harness.user_id,
        ConsentKind::Risk,
        ConsentAction::Withdrawn,
        fixed_now(),
        "2026-11-01.1",
    ));

    let result = harness
        .withdraw
        .withdraw(&harness.caller, ConsentKind::Risk)
        .await
        .unwrap();

    assert_eq!(result, WithdrawResult::NoOp(ConsentState::Withdrawn));
    assert_eq!(harness.repository.events().len(), 1, "no duplicate event");
    assert!(harness.repository.outbox().is_empty());
    assert_eq!(harness.repository.rollbacks(), 1);
    assert_eq!(harness.repository.commits(), 0);
    assert!(
        harness.observation.rollbacks.lock().unwrap().is_empty(),
        "a 200 no-op is not a withdrawal failure"
    );
}

#[tokio::test]
async fn withdrawing_a_never_granted_kind_is_a_conflict() {
    let harness = consent_harness();

    let error = harness
        .withdraw
        .withdraw(&harness.caller, ConsentKind::Age)
        .await
        .unwrap_err();

    assert!(matches!(error, AppError::Conflict(_)));
    assert!(harness.repository.events().is_empty());
    assert!(harness.repository.outbox().is_empty());
    assert!(
        harness.observation.rollbacks.lock().unwrap().is_empty(),
        "a 409 never-granted kind is not a withdrawal failure"
    );
}

#[tokio::test]
async fn a_failed_outbox_insert_rolls_the_withdrawal_back() {
    let harness = consent_harness();
    harness.repository.seed(grant_event(
        harness.user_id,
        ConsentKind::Terms,
        "2026-11-01.1",
    ));
    harness
        .repository
        .fail_outbox_next(RepositoryError::Unavailable(
            "outbox write failed".to_string(),
        ));

    let error = harness
        .withdraw
        .withdraw(&harness.caller, ConsentKind::Terms)
        .await
        .unwrap_err();

    assert!(matches!(error, AppError::Unavailable(_)));
    assert_eq!(
        harness.repository.events().len(),
        1,
        "the withdrawal event was not committed"
    );
    assert_eq!(harness.repository.commits(), 0);
    assert_eq!(
        harness.observation.rollbacks.lock().unwrap().as_slice(),
        [WithdrawalRollbackCause::OutboxFailed]
    );
}

#[tokio::test]
async fn withdrawing_analytics_never_blocks_a_connection() {
    let harness = consent_harness();
    for (seq, kind) in [ConsentKind::Terms, ConsentKind::Risk, ConsentKind::Age]
        .into_iter()
        .enumerate()
    {
        harness.repository.seed(ConsentEvent::from_stored(
            ConsentRecordId::generate(),
            seq as i64 + 1,
            harness.user_id,
            kind,
            ConsentAction::Granted,
            fixed_now(),
            "2026-11-01.1",
        ));
    }
    harness.repository.seed(ConsentEvent::from_stored(
        ConsentRecordId::generate(),
        10,
        harness.user_id,
        ConsentKind::Analytics,
        ConsentAction::Granted,
        fixed_now(),
        "2026-11-01.1",
    ));

    harness
        .withdraw
        .withdraw(&harness.caller, ConsentKind::Analytics)
        .await
        .unwrap();
    let view = harness.state.state(harness.user_id).await.unwrap();

    assert!(view.missing().is_empty(), "analytics never gates a source");
    assert_eq!(view.state(ConsentKind::Analytics), ConsentState::Withdrawn);
}

#[tokio::test]
async fn history_returns_newest_first_and_caps_the_page() {
    let harness = consent_harness();
    let draft =
        ConsentEventDraft::grant(harness.user_id, ConsentKind::Terms, policy("2026-11-01.1"));
    let repository = harness.repository.clone();
    repository.insert(&draft).await.unwrap();
    repository
        .insert(&ConsentEventDraft::grant(
            harness.user_id,
            ConsentKind::Risk,
            policy("2026-11-01.1"),
        ))
        .await
        .unwrap();
    repository
        .insert(&ConsentEventDraft::grant(
            harness.user_id,
            ConsentKind::Age,
            policy("2026-11-01.1"),
        ))
        .await
        .unwrap();

    let page = harness
        .history
        .history(&harness.caller, 2, None)
        .await
        .unwrap();
    assert_eq!(page.len(), 2);
    assert!(page[0].seq() > page[1].seq());

    let next = harness
        .history
        .history(
            &harness.caller,
            2,
            Some(HistoryCursor {
                occurred_at: page[1].occurred_at(),
                seq: page[1].seq(),
            }),
        )
        .await
        .unwrap();
    assert_eq!(next.len(), 1);
    assert!(
        next[0].seq() < page[1].seq(),
        "the page continues below the cursor"
    );

    let capped = harness
        .history
        .history(&harness.caller, 500, None)
        .await
        .unwrap();
    assert_eq!(capped.len(), 3, "the cap is applied, not exceeded");
}

#[tokio::test]
async fn preferences_read_before_the_first_write_returns_defaults_with_no_row() {
    let harness = preferences_harness();

    let effective = harness.read.read(&harness.caller).await.unwrap();

    assert_eq!(effective.locale().value(), &Locale::En);
    assert_eq!(
        effective.locale().source(),
        identity_domain::consent::PreferenceSource::Default
    );
    assert_eq!(
        effective.notification_email().value(),
        &false,
        "an unset channel is not enabled"
    );
    assert!(
        harness.repository.row(harness.user_id).is_none(),
        "a read never creates a row"
    );
}

#[tokio::test]
async fn preferences_write_changes_only_the_supplied_field() {
    let harness = preferences_harness();

    harness
        .write
        .write(
            &harness.caller,
            PreferenceUpdate {
                locale: Some(Locale::Vi),
                ..PreferenceUpdate::default()
            },
        )
        .await
        .unwrap();
    let effective = harness
        .write
        .write(
            &harness.caller,
            PreferenceUpdate {
                base_currency: Some(identity_domain::consent::BaseCurrencyCode::Eur),
                ..PreferenceUpdate::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(effective.locale().value(), &Locale::Vi);
    assert_eq!(
        effective.locale().source(),
        identity_domain::consent::PreferenceSource::User
    );
    assert_eq!(
        effective.base_currency().value(),
        &identity_domain::consent::BaseCurrencyCode::Eur
    );
    assert_eq!(
        effective.timezone().value().as_str(),
        "UTC",
        "an unsupplied field keeps its resolved default"
    );
    assert_eq!(
        harness
            .observation
            .preference_writes
            .lock()
            .unwrap()
            .as_slice(),
        [
            PreferenceWriteOutcome::Applied,
            PreferenceWriteOutcome::Applied
        ]
    );
}

#[tokio::test]
async fn preferences_read_marks_a_provider_value_as_default() {
    let harness = preferences_harness();
    harness.users.seed(
        harness.user_id,
        ProviderProfile::new(
            Locale::Vi,
            BaseCurrency::parse("GBP").unwrap(),
            TimeZone::parse("Asia/Ho_Chi_Minh").unwrap(),
        ),
    );

    let effective = harness.read.read(&harness.caller).await.unwrap();

    assert_eq!(effective.locale().value(), &Locale::Vi);
    assert_eq!(
        effective.locale().source(),
        identity_domain::consent::PreferenceSource::Default,
        "a provider-derived value is not the user's choice"
    );
    assert_eq!(
        effective.base_currency().value(),
        &identity_domain::consent::BaseCurrencyCode::Gbp
    );
}

#[tokio::test]
async fn preferences_write_with_no_fields_is_a_no_op() {
    let harness = preferences_harness();

    let effective = harness
        .write
        .write(&harness.caller, PreferenceUpdate::default())
        .await
        .unwrap();

    assert_eq!(effective.locale().value(), &Locale::En);
    assert!(
        harness.repository.row(harness.user_id).is_none(),
        "an empty write creates no row"
    );
    assert!(
        harness
            .observation
            .preference_writes
            .lock()
            .unwrap()
            .is_empty(),
        "an empty write is not a preference write"
    );
}
