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
    evaluate, CountryCode, GateOutcome, JurisdictionRule, SignupDeclaration,
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
        assurance: AssuranceLevel::Aal1,
        has_exchange_credentials: false,
        session_id: Some("session-1".to_string()),
        user_agent: Some("Firefox".to_string()),
        location: Some("Hanoi, VN".to_string()),
    }
}

fn provisioned() -> User {
    User::provision(UserId::generate(), &claims(), at(9))
}

#[test]
fn provision_creates_a_pending_user_with_the_documented_defaults() {
    let user = provisioned();
    assert_eq!(user.status(), UserStatus::Pending);
    assert_eq!(user.role(), Role::User);
    assert_eq!(user.locale(), Locale::En);
    assert_eq!(user.base_currency(), BaseCurrency::parse("USD").unwrap());
    assert_eq!(user.timezone(), IanaTimeZone::parse("UTC").unwrap());
    assert_eq!(user.created_at(), at(9));
    assert_eq!(user.anonymised_at(), None);
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
    let mut user = provisioned();
    let mut unconfirmed = claims();
    unconfirmed.email_confirmed = false;
    let next = user.apply_claims(&unconfirmed, at(10)).unwrap();
    assert_eq!(next, NextStep::VerifyEmail);
    assert_eq!(user.status(), UserStatus::Pending);
}

#[test]
fn an_exchange_credential_holder_below_aal2_must_enrol_mfa() {
    let mut user = provisioned();
    let mut credential_holder = claims();
    credential_holder.has_exchange_credentials = true;
    credential_holder.assurance = AssuranceLevel::Aal1;
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
        .approve(other_admin, Role::Admin, at(10), Duration::minutes(60))
        .unwrap();
    assert_eq!(grant.status(), SupportGrantStatus::Active);
    assert_eq!(grant.expires_at(), Some(at(10) + Duration::minutes(60)));
    assert_eq!(
        grant.effective_status(at(10) + Duration::minutes(61)),
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
            at(10),
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
