//! Domain tests: the User lifecycle, consent history, preferences, support
//! grants, device sessions, export and deletion state machines, the jurisdiction
//! gate, and the export secret scan.
//!
//! Every test is deterministic: time is supplied, never read.

use chrono::{Duration, TimeZone, Utc};
use identity_domain::claims::{AssuranceLevel, NextStep, Role, VerifiedClaims};
use identity_domain::consent::{ConsentPurpose, ConsentRecord, ConsentSource, ConsentView};
use identity_domain::deletion::{
    DeletionRequest, DeletionRequestId, DeletionStatus, RetainedCategory,
};
use identity_domain::error::DomainError;
use identity_domain::export::{
    pending_contributions, ContributionState, Contributor, ExportRequest, ExportRequestId,
    ExportStatus,
};
use identity_domain::ids::UserId;
use identity_domain::jurisdiction::{
    decide, evaluate, CountryCode, GateOutcome, JurisdictionListEntry, JurisdictionListVersion,
    JurisdictionRule, JurisdictionStatus, RestrictionSource, SignupDeclaration,
};
use identity_domain::preferences::{BaseCurrency, Locale, Preferences, TimeZone as IanaTimeZone};
use identity_domain::secrets::find_secret_field;
use identity_domain::session::DeviceSession;
use identity_domain::support::{SupportAccessGrant, SupportGrantId, SupportGrantStatus};
use identity_domain::user::{User, UserStatus};

fn at(hour: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, hour, 0, 0).unwrap()
}

fn claims() -> VerifiedClaims {
    VerifiedClaims {
        subject: "11111111-1111-4111-8111-111111111111".to_string(),
        role: Role::User,
        email_confirmed: true,
        email: Some("user@example.com".to_string()),
        display_name: Some("Test User".to_string()),
        assurance: AssuranceLevel::Aal1,
        has_exchange_credentials: false,
        session_id: Some("session-1".to_string()),
        user_agent: Some("Firefox".to_string()),
        location: Some("Hanoi, VN".to_string()),
        country_code: Some("VN".to_string()),
        client_ip: Some("192.0.2.10".to_string()),
    }
}

fn provisioned() -> User {
    User::provision(UserId::generate(), &claims(), at(9))
}

fn stored_with_status(status: UserStatus) -> User {
    User::from_stored(
        UserId::generate(),
        Role::User,
        status,
        None,
        false,
        None,
        false,
        false,
        Locale::En,
        BaseCurrency::parse("USD").unwrap(),
        IanaTimeZone::parse("UTC").unwrap(),
        at(9),
        at(9),
        None,
    )
}

#[test]
fn provision_creates_an_active_user_when_the_provider_confirms_the_address() {
    let user = provisioned();
    assert_eq!(user.status(), UserStatus::Active);
    assert_eq!(user.role(), Role::User);
    assert_eq!(user.display_name(), Some("Test User"));
    assert_eq!(user.locale(), Locale::En);
    assert_eq!(user.base_currency(), BaseCurrency::parse("USD").unwrap());
    assert_eq!(user.timezone(), IanaTimeZone::parse("UTC").unwrap());
    assert_eq!(user.created_at(), at(9));
    assert_eq!(user.anonymised_at(), None);
}

#[test]
fn user_registry_status_set_is_closed_and_names_the_four_public_states() {
    for (raw, expected) in [
        ("pending", UserStatus::Pending),
        ("active", UserStatus::Active),
        ("disabled", UserStatus::Disabled),
        ("pending_deletion", UserStatus::PendingDeletion),
        ("deleted", UserStatus::Deleted),
    ] {
        assert_eq!(UserStatus::parse(raw).unwrap(), expected);
        assert_eq!(expected.as_str(), raw);
    }
    assert!(UserStatus::parse("archived").is_err());
    assert!(UserStatus::parse("ACTIVE").is_err());
}

#[test]
fn user_registry_disabled_or_pending_deletion_is_denied() {
    for status in [UserStatus::Disabled, UserStatus::PendingDeletion] {
        let mut user = stored_with_status(status);
        assert!(matches!(
            user.apply_claims(&claims(), at(10)),
            Err(DomainError::Denied { .. })
        ));
    }
}

#[test]
fn user_registry_provisioning_status_follows_the_provider_email_fact() {
    let verified = provisioned();
    assert_eq!(verified.status(), UserStatus::Active);

    let mut unconfirmed = claims();
    unconfirmed.email_confirmed = false;
    let pending = User::provision(UserId::generate(), &unconfirmed, at(9));
    assert_eq!(pending.status(), UserStatus::Pending);
}

#[test]
fn user_registry_profile_override_survives_a_claim_refresh() {
    let mut user = provisioned();
    user.update_profile(
        Some("Dana".to_string()),
        Some("dana@example.com".to_string()),
        at(10),
    )
    .unwrap();
    assert_eq!(user.display_name(), Some("Dana"));
    assert_eq!(user.email(), Some("dana@example.com"));
    assert!(
        !user.email_verified(),
        "an overridden address is not provider-verified"
    );
    assert!(user.display_name_overridden());
    assert!(user.email_overridden());

    let mut refreshed = claims();
    refreshed.display_name = Some("Provider Name".to_string());
    refreshed.email = Some("provider@example.com".to_string());
    user.apply_claims(&refreshed, at(11)).unwrap();
    assert_eq!(user.display_name(), Some("Dana"));
    assert_eq!(user.email(), Some("dana@example.com"));
}

#[test]
fn user_registry_invalid_profile_value_is_refused_and_the_previous_value_kept() {
    let mut user = provisioned();
    let error = user
        .update_profile(None, Some("not-an-email".to_string()), at(10))
        .expect_err("an address that is not RFC 5322 shaped is refused");
    assert_eq!(error.field(), "email");
    assert_eq!(user.email(), Some("user@example.com"));
    assert!(!user.email_overridden());

    let error = user
        .update_profile(Some("x".repeat(201)), None, at(10))
        .expect_err("a display name over 200 characters is refused");
    assert_eq!(error.field(), "displayName");
    assert_eq!(user.display_name(), Some("Test User"));
}

#[test]
fn provision_records_the_verified_address_and_anonymise_clears_it() {
    let mut user = provisioned();
    assert_eq!(user.email(), Some("user@example.com"));
    assert!(user.email_verified());
    assert_eq!(user.verified_email(), Some("user@example.com"));

    user.anonymise(at(10));
    assert_eq!(user.email(), None);
    assert!(!user.email_verified());
    assert_eq!(user.verified_email(), None);
}

#[test]
fn an_unconfirmed_email_is_never_a_verified_contact() {
    let mut unconfirmed = claims();
    unconfirmed.email_confirmed = false;
    let user = User::provision(UserId::generate(), &unconfirmed, at(9));
    assert_eq!(user.email(), Some("user@example.com"));
    assert!(!user.email_verified());
    assert_eq!(user.verified_email(), None);
}

#[test]
fn a_refresh_learns_the_address_from_the_token() {
    let mut user = provisioned();
    let mut claims = claims();
    claims.email = Some("new@example.com".to_string());
    user.apply_claims(&claims, at(10)).unwrap();
    assert_eq!(user.email(), Some("new@example.com"));
    assert_eq!(user.verified_email(), Some("new@example.com"));
}

#[test]
fn a_confirmed_user_without_exchange_credentials_becomes_active() {
    let mut user = provisioned();
    let next = user.apply_claims(&claims(), at(10)).unwrap();
    assert_eq!(next, NextStep::None);
    assert_eq!(user.status(), UserStatus::Active);
}

#[test]
fn an_unconfirmed_user_stays_pending_and_is_told_to_verify() {
    let mut unconfirmed = claims();
    unconfirmed.email_confirmed = false;
    let mut user = User::provision(UserId::generate(), &unconfirmed, at(9));
    assert_eq!(user.status(), UserStatus::Pending);

    let next = user.apply_claims(&unconfirmed, at(10)).unwrap();
    assert_eq!(next, NextStep::VerifyEmail);
    assert_eq!(user.status(), UserStatus::Pending);
}

#[test]
fn an_exchange_credential_holder_below_aal2_must_enrol_mfa() {
    let mut credential_holder = claims();
    credential_holder.has_exchange_credentials = true;
    credential_holder.assurance = AssuranceLevel::Aal1;
    let mut user = User::provision(UserId::generate(), &credential_holder, at(9));
    assert_eq!(user.status(), UserStatus::Pending);

    let next = user.apply_claims(&credential_holder, at(10)).unwrap();
    assert_eq!(next, NextStep::EnrolMfa);
    assert_eq!(user.status(), UserStatus::Pending);
}

#[test]
fn an_exchange_credential_holder_at_aal2_becomes_active() {
    let mut user = provisioned();
    let mut credential_holder = claims();
    credential_holder.has_exchange_credentials = true;
    credential_holder.assurance = AssuranceLevel::Aal2;
    let next = user.apply_claims(&credential_holder, at(10)).unwrap();
    assert_eq!(next, NextStep::None);
    assert_eq!(user.status(), UserStatus::Active);
}

#[test]
fn an_unknown_or_missing_role_is_a_plain_user() {
    assert_eq!(Role::from_claim(None), Role::User);
    assert_eq!(Role::from_claim(Some("owner")), Role::User);
    assert_eq!(Role::from_claim(Some("admin")), Role::Admin);
    assert_eq!(Role::from_claim(Some("support")), Role::Support);
}

#[test]
fn a_deleted_user_cannot_resolve_a_principal() {
    let mut user = provisioned();
    user.anonymise(at(11));
    assert_eq!(user.status(), UserStatus::Deleted);
    assert!(matches!(
        user.apply_claims(&claims(), at(12)),
        Err(DomainError::Denied { .. })
    ));
}

#[test]
fn preferences_accept_valid_values_and_reject_a_bad_currency() {
    assert!(BaseCurrency::parse("USD").is_ok());
    assert!(BaseCurrency::parse("EUR").is_ok());
    assert!(BaseCurrency::parse("usd").is_err());
    assert!(BaseCurrency::parse("US").is_err());
    assert!(BaseCurrency::parse("US1").is_err());
    assert!(Locale::parse("vi").is_ok());
    assert!(Locale::parse("fr").is_err());
}

#[test]
fn preferences_reject_a_timezone_that_is_not_an_iana_name() {
    assert!(IanaTimeZone::parse("UTC").is_ok());
    assert!(IanaTimeZone::parse("Europe/Berlin").is_ok());
    assert!(IanaTimeZone::parse("America/Argentina/Buenos_Aires").is_ok());
    assert!(IanaTimeZone::parse("Berlin").is_err());
    assert!(IanaTimeZone::parse("Europe/").is_err());
    assert!(IanaTimeZone::parse("/Berlin").is_err());
}

#[test]
fn a_preference_change_is_applied_and_the_previous_value_survives_a_refusal() {
    let mut user = provisioned();
    let preferences = Preferences {
        locale: Locale::Vi,
        base_currency: BaseCurrency::parse("EUR").unwrap(),
        timezone: IanaTimeZone::parse("Asia/Ho_Chi_Minh").unwrap(),
    };
    user.update_preferences(preferences.clone(), at(10));
    assert_eq!(user.locale(), Locale::Vi);
    assert_eq!(user.base_currency(), BaseCurrency::parse("EUR").unwrap());
    assert_eq!(user.timezone(), preferences.timezone);
}

#[test]
fn consent_is_deny_by_default_and_uses_the_newest_decision() {
    let user_id = UserId::generate();
    let older = ConsentRecord::record(
        identity_domain::ids::ConsentRecordId::generate(),
        user_id,
        ConsentPurpose::Analytics,
        true,
        ConsentSource::Onboarding,
        at(9),
    );
    let newer = ConsentRecord::record(
        identity_domain::ids::ConsentRecordId::generate(),
        user_id,
        ConsentPurpose::Analytics,
        false,
        ConsentSource::Settings,
        at(10),
    );
    let view = ConsentView::from_history(user_id, &[older, newer]);
    assert!(!view.analytics());
    assert!(!view.marketing());
}

#[test]
fn a_support_grant_needs_a_non_blank_reason() {
    let agent = UserId::generate();
    assert!(SupportAccessGrant::request(
        SupportGrantId::generate(),
        UserId::generate(),
        agent,
        "   ",
        at(9),
    )
    .is_err());
}

#[test]
fn a_support_grant_cannot_be_self_approved_or_approved_by_a_non_admin() {
    let agent = UserId::generate();
    let mut grant = SupportAccessGrant::request(
        SupportGrantId::generate(),
        UserId::generate(),
        agent,
        "investigate a reported sync failure",
        at(9),
    )
    .unwrap();
    assert!(matches!(
        grant.approve(agent, Role::Support, at(10), Duration::minutes(60)),
        Err(DomainError::Denied { .. })
    ));
    let other_admin = UserId::generate();
    assert!(matches!(
        grant.approve(other_admin, Role::Support, at(10), Duration::minutes(60)),
        Err(DomainError::Denied { .. })
    ));
    grant
        .approve(
            other_admin,
            Role::Admin,
            at(9) + Duration::minutes(30),
            Duration::minutes(60),
        )
        .unwrap();
    assert_eq!(grant.status(), SupportGrantStatus::Active);
    assert_eq!(grant.expires_at(), Some(at(9) + Duration::minutes(90)));
    assert_eq!(
        grant.effective_status(at(9) + Duration::minutes(91)),
        SupportGrantStatus::Expired
    );
}

#[test]
fn an_active_grant_can_be_revoked_but_a_revoked_one_is_terminal() {
    let mut grant = SupportAccessGrant::request(
        SupportGrantId::generate(),
        UserId::generate(),
        UserId::generate(),
        "investigate a reported sync failure",
        at(9),
    )
    .unwrap();
    grant
        .approve(
            UserId::generate(),
            Role::Admin,
            at(9) + Duration::minutes(30),
            Duration::minutes(60),
        )
        .unwrap();
    grant.revoke(at(11)).unwrap();
    assert_eq!(grant.status(), SupportGrantStatus::Revoked);
    assert!(grant.revoke(at(12)).is_err());
}

#[test]
fn export_progress_rises_monotonically_and_ready_needs_every_contributor() {
    let mut export = ExportRequest::request(
        ExportRequestId::generate(),
        UserId::generate(),
        at(9),
        at(9) + Duration::minutes(15),
    );
    export
        .claim("worker-1", at(9), Duration::seconds(60))
        .unwrap();
    assert_eq!(export.status(), ExportStatus::Building);
    export.record_success(Contributor::Account, at(9));
    assert_eq!(export.progress(), 33);
    export.record_success(Contributor::Account, at(9));
    assert_eq!(
        export.progress(),
        33,
        "a repeated answer never lowers progress"
    );
    assert!(export
        .mark_ready("export.json", at(9) + Duration::hours(24), at(9))
        .is_err());
    export.record_success(Contributor::Ledger, at(9));
    export.record_success(Contributor::Portfolio, at(9));
    assert_eq!(export.progress(), 100);
    export
        .mark_ready("export.json", at(9) + Duration::hours(24), at(9))
        .unwrap();
    assert_eq!(export.status(), ExportStatus::Ready);
    assert_eq!(export.file_reference(), Some("export.json"));
}

#[test]
fn a_failed_contribution_fails_the_export_and_never_marks_ready() {
    let mut export = ExportRequest::request(
        ExportRequestId::generate(),
        UserId::generate(),
        at(9),
        at(9) + Duration::minutes(15),
    );
    export
        .claim("worker-1", at(9), Duration::seconds(60))
        .unwrap();
    export.record_success(Contributor::Account, at(9));
    export.record_failure(Contributor::Ledger, "ledger timed out", at(9));
    assert_eq!(export.status(), ExportStatus::Failed);
    assert_eq!(export.failure_reason(), Some("ledger timed out"));
    assert!(export
        .mark_ready("export.json", at(9) + Duration::hours(24), at(9))
        .is_err());
}

#[test]
fn an_export_expires_at_its_assembly_deadline_and_after_its_download_window() {
    let mut export = ExportRequest::request(
        ExportRequestId::generate(),
        UserId::generate(),
        at(9),
        at(9) + Duration::minutes(15),
    );
    assert!(!export.expire_if_due(at(9) + Duration::minutes(14)));
    assert!(export.expire_if_due(at(9) + Duration::minutes(15)));
    assert_eq!(export.status(), ExportStatus::Expired);

    let mut ready = ExportRequest::request(
        ExportRequestId::generate(),
        UserId::generate(),
        at(9),
        at(9) + Duration::minutes(15),
    );
    ready
        .claim("worker-1", at(9), Duration::seconds(60))
        .unwrap();
    for contributor in Contributor::ALL {
        ready.record_success(contributor, at(9));
    }
    ready
        .mark_ready("export.json", at(9) + Duration::hours(24), at(9))
        .unwrap();
    assert!(!ready.expire_if_due(at(9) + Duration::hours(23)));
    assert!(ready.expire_if_due(at(9) + Duration::hours(24)));
    assert_eq!(ready.status(), ExportStatus::Expired);
}

#[test]
fn a_worker_claims_only_a_free_or_expired_lease() {
    let mut export = ExportRequest::request(
        ExportRequestId::generate(),
        UserId::generate(),
        at(9),
        at(9) + Duration::minutes(15),
    );
    export
        .claim("worker-1", at(9), Duration::seconds(60))
        .unwrap();
    assert!(export
        .claim(
            "worker-2",
            at(9) + Duration::seconds(30),
            Duration::seconds(60)
        )
        .is_err());
    export
        .claim(
            "worker-2",
            at(9) + Duration::seconds(61),
            Duration::seconds(60),
        )
        .unwrap();
    assert_eq!(export.lease_owner(), Some("worker-2"));
}

#[test]
fn deletion_requires_every_step_and_destroys_the_key_last() {
    let mut deletion =
        DeletionRequest::request(DeletionRequestId::generate(), UserId::generate(), at(9));
    deletion
        .claim("worker-1", at(9), Duration::seconds(60))
        .unwrap();
    assert_eq!(deletion.status(), DeletionStatus::InProgress);
    assert!(deletion.record_key_destroyed(at(9)).is_err());
    for contributor in Contributor::ALL {
        deletion.record_anonymised(contributor);
    }
    deletion.record_auth_deleted();
    deletion.record_identity_anonymised(at(9));
    deletion.record_key_destroyed(at(9)).unwrap();
    deletion
        .complete(
            vec![
                RetainedCategory::new("financial_records", "two-year legal retention")
                    .expect("a labelled retention category"),
            ],
            at(9),
        )
        .unwrap();
    assert_eq!(deletion.status(), DeletionStatus::Completed);
    assert_eq!(deletion.retained_categories().len(), 1);
}

#[test]
fn a_failed_deletion_is_retryable_and_resumes_without_repeating_destruction() {
    let mut deletion =
        DeletionRequest::request(DeletionRequestId::generate(), UserId::generate(), at(9));
    deletion
        .claim("worker-1", at(9), Duration::seconds(60))
        .unwrap();
    deletion.record_anonymised(Contributor::Account);
    deletion.fail("ledger timed out", at(10)).unwrap();
    assert_eq!(deletion.status(), DeletionStatus::Failed);
    deletion
        .claim("worker-2", at(11), Duration::seconds(60))
        .unwrap();
    assert_eq!(deletion.status(), DeletionStatus::InProgress);
    assert!(deletion.progress().is_anonymised(Contributor::Account));
    assert!(!deletion.progress().is_anonymised(Contributor::Ledger));
}

#[test]
fn the_jurisdiction_gate_blocks_unknown_underage_and_unacknowledged_signups() {
    let rule = JurisdictionRule {
        code: CountryCode::parse("VN").unwrap(),
        allowed: true,
        minimum_age: 18,
        disclosures_required: true,
        updated_at: at(9),
    };
    assert!(matches!(
        evaluate(None, 30, true),
        GateOutcome::Denied { .. }
    ));
    assert!(matches!(
        evaluate(Some(&rule), 17, true),
        GateOutcome::Denied { .. }
    ));
    assert_eq!(
        evaluate(Some(&rule), 18, false),
        GateOutcome::DisclosuresRequired
    );
    assert_eq!(evaluate(Some(&rule), 18, true), GateOutcome::Allowed);
}

#[test]
fn a_signup_declaration_rejects_an_impossible_age_and_a_bad_country_code() {
    assert!(SignupDeclaration::parse("VN", 30, true).is_ok());
    assert!(SignupDeclaration::parse("VN", 0, true).is_err());
    assert!(SignupDeclaration::parse("VN", 131, true).is_err());
    assert!(SignupDeclaration::parse("vn", 30, true).is_err());
    assert!(SignupDeclaration::parse("VNM", 30, true).is_err());
}

#[test]
fn a_device_session_tracks_activity_and_revocation() {
    let mut session = DeviceSession::new(
        identity_domain::ids::DeviceSessionId::generate(),
        UserId::generate(),
        "auth-session-1",
        Some("Firefox".to_string()),
        Some("Hanoi, VN".to_string()),
        at(9),
    );
    session.touch(Some("Firefox 130".to_string()), None, at(10));
    assert_eq!(session.last_activity_at(), at(10));
    assert_eq!(session.location(), None);
    session.revoke(at(11));
    assert!(session.is_revoked());
}

#[test]
fn the_export_secret_scan_finds_credential_shaped_fields_and_ignores_clean_data() {
    assert_eq!(
        find_secret_field(r#"{"accounts":[{"apiKey":"fake-value"}]}"#).as_deref(),
        Some("apiKey")
    );
    assert_eq!(find_secret_field(r#"{"ledger":{"transactions":[]}}"#), None);
    assert_eq!(
        find_secret_field(r#"{"note":"a token of appreciation"}"#),
        None,
        "the scan looks at field names, not prose values"
    );
}

#[test]
fn pending_contributions_start_every_contributor_pending() {
    let contributions = pending_contributions();
    assert_eq!(contributions.len(), 3);
    assert!(contributions
        .values()
        .all(|state| *state == ContributionState::Pending));
}

// ---------------------------------------------------------------------------
// U6 — the runtime jurisdiction gate (BR1.1–BR1.4, BR1.10)
// ---------------------------------------------------------------------------

fn entry(code: &str, status: JurisdictionStatus) -> JurisdictionListEntry {
    JurisdictionListEntry {
        code: CountryCode::parse(code).expect("a fixture country code is valid"),
        status,
        source: "compliance".to_string(),
        updated_at: at(9),
    }
}

fn list_fixture() -> JurisdictionListVersion {
    JurisdictionListVersion::new(
        "2026-11-10.1",
        at(9),
        vec![
            entry("VN", JurisdictionStatus::Allowed),
            entry("US", JurisdictionStatus::Allowed),
            entry("KP", JurisdictionStatus::Restricted),
        ],
    )
    .expect("a complete list is valid")
}

#[test]
fn jurisdiction_country_code_accepts_two_upper_case_letters_and_refuses_other_forms() {
    assert_eq!(CountryCode::parse("VN").unwrap().as_str(), "VN");
    assert_eq!(
        CountryCode::parse(" VN ").unwrap().as_str(),
        "VN",
        "surrounding whitespace is trimmed"
    );

    assert!(CountryCode::parse("vn").is_err(), "lowercase is refused");
    assert!(
        CountryCode::parse("VNM").is_err(),
        "three letters is refused"
    );
    assert!(CountryCode::parse("V").is_err(), "one letter is refused");
    assert!(CountryCode::parse("V1").is_err(), "a digit is refused");
    assert!(CountryCode::parse("ÅÅ").is_err(), "non-ASCII is refused");
}

#[test]
fn jurisdiction_status_parses_only_the_two_listed_values() {
    assert_eq!(
        JurisdictionStatus::parse("allowed").unwrap(),
        JurisdictionStatus::Allowed
    );
    assert_eq!(
        JurisdictionStatus::parse(" restricted ").unwrap(),
        JurisdictionStatus::Restricted
    );
    assert_eq!(JurisdictionStatus::Allowed.as_str(), "allowed");
    assert_eq!(JurisdictionStatus::Restricted.as_str(), "restricted");
    assert!(JurisdictionStatus::parse("ALLOWED").is_err());
    assert!(JurisdictionStatus::parse("unknown").is_err());
}

#[test]
fn jurisdiction_decision_allows_only_when_both_signals_are_allowed() {
    let decision = decide(
        JurisdictionStatus::Allowed,
        JurisdictionStatus::Allowed,
        "2026-11-10.1",
    );

    assert!(decision.is_allowed());
    assert_eq!(decision.restricted_by(), None);
    assert_eq!(decision.list_version(), "2026-11-10.1");
}

#[test]
fn jurisdiction_decision_names_a_restricted_self_declared_country() {
    let decision = decide(
        JurisdictionStatus::Restricted,
        JurisdictionStatus::Allowed,
        "2026-11-10.1",
    );

    assert!(!decision.is_allowed());
    assert_eq!(
        decision.restricted_by(),
        Some(RestrictionSource::SelfDeclared)
    );
    assert_eq!(RestrictionSource::SelfDeclared.as_str(), "self_declared");
}

#[test]
fn jurisdiction_decision_names_a_restricted_ip_country_and_both_when_both_restrict() {
    let ip_only = decide(
        JurisdictionStatus::Allowed,
        JurisdictionStatus::Restricted,
        "2026-11-10.1",
    );
    assert!(!ip_only.is_allowed());
    assert_eq!(ip_only.restricted_by(), Some(RestrictionSource::Ip));
    assert_eq!(RestrictionSource::Ip.as_str(), "ip");

    let both = decide(
        JurisdictionStatus::Restricted,
        JurisdictionStatus::Restricted,
        "2026-11-10.1",
    );
    assert_eq!(both.restricted_by(), Some(RestrictionSource::Both));
    assert_eq!(RestrictionSource::Both.as_str(), "both");
}

#[test]
fn jurisdiction_list_treats_an_absent_country_as_restricted() {
    let list = list_fixture();

    assert_eq!(
        list.status_for(&CountryCode::parse("VN").unwrap()),
        JurisdictionStatus::Allowed
    );
    assert_eq!(
        list.status_for(&CountryCode::parse("US").unwrap()),
        JurisdictionStatus::Allowed
    );
    assert_eq!(
        list.status_for(&CountryCode::parse("KP").unwrap()),
        JurisdictionStatus::Restricted
    );
    assert_eq!(
        list.status_for(&CountryCode::parse("ZZ").unwrap()),
        JurisdictionStatus::Restricted,
        "a country the list does not name is refused, never allowed"
    );
}

#[test]
fn jurisdiction_list_computes_a_stable_checksum_over_its_ordered_entries() {
    let ordered = list_fixture();
    let reordered = JurisdictionListVersion::new(
        "2026-11-10.1",
        at(9),
        vec![
            entry("KP", JurisdictionStatus::Restricted),
            entry("VN", JurisdictionStatus::Allowed),
            entry("US", JurisdictionStatus::Allowed),
        ],
    )
    .unwrap();

    assert_eq!(ordered.entry_count(), 3);
    assert_eq!(ordered.version(), "2026-11-10.1");
    assert_eq!(
        ordered.checksum().len(),
        64,
        "the checksum is a SHA-256 hex digest"
    );
    assert_eq!(
        ordered.checksum(),
        reordered.checksum(),
        "the digest covers the entries in code order, not insertion order"
    );
}

#[test]
fn jurisdiction_list_refuses_a_stored_checksum_that_does_not_match_its_entries() {
    let entries = vec![entry("VN", JurisdictionStatus::Allowed)];

    let mismatch = JurisdictionListVersion::from_stored(
        "2026-11-10.1",
        at(9),
        &"0".repeat(64),
        entries.clone(),
    );
    assert!(mismatch.is_err(), "a mismatched checksum is never served");

    assert!(
        JurisdictionListVersion::new("2026-11-10.1", at(9), Vec::new()).is_err(),
        "an empty list is refused"
    );
    assert!(
        JurisdictionListVersion::new(
            "2026-11-10.1",
            at(9),
            vec![
                entry("VN", JurisdictionStatus::Allowed),
                entry("VN", JurisdictionStatus::Restricted),
            ],
        )
        .is_err(),
        "a duplicate country code is refused"
    );
}
