//! Domain tests: account lifecycle, venues and labels, credentials, wallet
//! addresses, sync leases, decimal holdings, CSV imports, secret detection,
//! audit records, and the service-token allow-list.
//!
//! Every test is deterministic: time is supplied, never read.

use account_domain::account::{normalise_label, Account, AccountKind, AccountStatus, Venue};
use account_domain::audit::{AuditOutcome, AuditRecord};
use account_domain::credential::{Credential, CredentialStatus};
use account_domain::holding::{parse_quantity, HoldingSnapshot};
use account_domain::ids::{
    AccountId, CredentialId, HoldingSnapshotId, ImportBatchId, ImportRowErrorId, SyncRunId, UserId,
    WalletConnectionId,
};
use account_domain::import::{
    dedupe_key_for, parse_csv_line, parse_executed_at, ImportBatch, ImportBatchStatus,
    ImportRowError, ImportSource, ImportSummary, ParsedTransaction, TransactionKind,
};
use account_domain::secrets::{ensure_not_secret, looks_like_secret};
use account_domain::service::{service_allowed, Operation};
use account_domain::sync::{
    ErrorCategory, SyncRun, SyncRunStatus, LEASE_SECONDS, MAX_ERROR_MESSAGE_CHARS,
};
use account_domain::wallet::{validate_address, WalletConnection};
use account_domain::{DomainError, MAX_LABEL_CHARS};
use chrono::{Duration, TimeZone, Utc};
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::str::FromStr;

fn at(hour: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, hour, 0, 0).unwrap()
}

fn user_id() -> UserId {
    UserId::generate()
}

fn account(kind: AccountKind, venue: &str) -> Account {
    Account::open(AccountId::generate(), user_id(), kind, venue, "Main", at(9))
        .expect("a valid account")
}

// ---------------------------------------------------------------------------
// Account kinds, statuses, and venues
// ---------------------------------------------------------------------------

#[test]
fn account_kinds_round_trip_through_their_labels() {
    for kind in [
        AccountKind::Exchange,
        AccountKind::Wallet,
        AccountKind::WalletConnect,
        AccountKind::Csv,
    ] {
        assert_eq!(AccountKind::parse(kind.as_str()).unwrap(), kind);
    }
}

#[test]
fn an_unknown_account_kind_is_refused() {
    assert!(matches!(
        AccountKind::parse("broker"),
        Err(DomainError::Invalid { field: "kind", .. })
    ));
}

#[test]
fn account_statuses_round_trip_through_their_labels() {
    for status in [
        AccountStatus::Active,
        AccountStatus::Syncing,
        AccountStatus::Error,
        AccountStatus::Disconnected,
    ] {
        assert_eq!(AccountStatus::parse(status.as_str()).unwrap(), status);
    }
}

#[test]
fn an_unknown_account_status_is_refused() {
    assert!(matches!(
        AccountStatus::parse("paused"),
        Err(DomainError::Invalid {
            field: "status",
            ..
        })
    ));
}

#[test]
fn exchange_venues_are_binance_and_okx_only() {
    assert_eq!(
        Venue::parse_for(AccountKind::Exchange, "Binance")
            .unwrap()
            .as_str(),
        "binance"
    );
    assert_eq!(
        Venue::parse_for(AccountKind::Exchange, "okx")
            .unwrap()
            .as_str(),
        "okx"
    );
    assert!(matches!(
        Venue::parse_for(AccountKind::Exchange, "kraken"),
        Err(DomainError::Invalid { field: "venue", .. })
    ));
}

#[test]
fn wallet_venues_are_evm_chain_ids_or_solana() {
    assert_eq!(
        Venue::parse_for(AccountKind::Wallet, "EVM:137")
            .unwrap()
            .as_str(),
        "evm:137"
    );
    assert_eq!(
        Venue::parse_for(AccountKind::Wallet, "solana")
            .unwrap()
            .as_str(),
        "solana"
    );
    assert!(matches!(
        Venue::parse_for(AccountKind::Wallet, "evm:"),
        Err(DomainError::Invalid { .. })
    ));
    assert!(matches!(
        Venue::parse_for(AccountKind::Wallet, "bitcoin"),
        Err(DomainError::Invalid { .. })
    ));
}

#[test]
fn csv_venues_carry_a_label() {
    assert_eq!(
        Venue::parse_for(AccountKind::Csv, "csv:binance-2024")
            .unwrap()
            .as_str(),
        "csv:binance-2024"
    );
    assert!(matches!(
        Venue::parse_for(AccountKind::Csv, "csv:"),
        Err(DomainError::Invalid { .. })
    ));
}

#[test]
fn a_blank_venue_is_required() {
    assert!(matches!(
        Venue::parse_for(AccountKind::Exchange, "   "),
        Err(DomainError::Required { field: "venue" })
    ));
}

#[test]
fn the_venue_exposes_its_exchange_or_chain() {
    let exchange = Venue::parse_for(AccountKind::Exchange, "binance").unwrap();
    assert_eq!(exchange.exchange(), Some("binance"));
    assert_eq!(exchange.chain(), None);

    let chain = Venue::parse_for(AccountKind::Wallet, "evm:1").unwrap();
    assert_eq!(chain.exchange(), None);
    assert_eq!(chain.chain(), Some("evm:1"));
}

// ---------------------------------------------------------------------------
// Labels and account lifecycle
// ---------------------------------------------------------------------------

#[test]
fn labels_are_trimmed_and_bounded() {
    assert_eq!(account_domain::validate_label("  Main  ").unwrap(), "Main");
    assert!(matches!(
        account_domain::validate_label("   "),
        Err(DomainError::Required { field: "label" })
    ));
    let too_long = "x".repeat(MAX_LABEL_CHARS + 1);
    assert!(matches!(
        account_domain::validate_label(&too_long),
        Err(DomainError::TooLarge { field: "label", .. })
    ));
}

#[test]
fn label_normalisation_folds_case_for_uniqueness() {
    assert_eq!(normalise_label("  Main Binance "), "main binance");
    assert_eq!(normalise_label("MAIN"), normalise_label("main"));
}

#[test]
fn an_open_account_starts_active_with_no_sync_time() {
    let account = account(AccountKind::Exchange, "binance");
    assert_eq!(account.status(), AccountStatus::Active);
    assert_eq!(account.last_synced_at(), None);
    assert_eq!(account.label(), "Main");
    assert_eq!(account.kind(), AccountKind::Exchange);
    assert!(account.is_syncable());
    assert!(!account.is_csv());
}

#[test]
fn a_csv_account_is_never_syncable() {
    let account = account(AccountKind::Csv, "csv:summary");
    assert!(account.is_csv());
    assert!(!account.is_syncable());
}

#[test]
fn a_sync_moves_the_account_to_syncing_and_back_to_active() {
    let mut account = account(AccountKind::Exchange, "binance");
    account.begin_sync(at(10)).unwrap();
    assert_eq!(account.status(), AccountStatus::Syncing);
    account.finish_sync_success(at(11)).unwrap();
    assert_eq!(account.status(), AccountStatus::Active);
    assert_eq!(account.last_synced_at(), Some(at(11)));
}

#[test]
fn a_failed_sync_moves_the_account_to_error_and_keeps_the_last_time() {
    let mut account = account(AccountKind::Exchange, "binance");
    account.begin_sync(at(10)).unwrap();
    account.finish_sync_success(at(11)).unwrap();
    account.begin_sync(at(12)).unwrap();
    account.finish_sync_failure(at(13)).unwrap();
    assert_eq!(account.status(), AccountStatus::Error);
    assert_eq!(account.last_synced_at(), Some(at(11)));
}

#[test]
fn success_and_failure_require_the_syncing_state() {
    let mut account = account(AccountKind::Exchange, "binance");
    assert!(matches!(
        account.finish_sync_success(at(10)),
        Err(DomainError::InvalidTransition { .. })
    ));
    assert!(matches!(
        account.finish_sync_failure(at(10)),
        Err(DomainError::InvalidTransition { .. })
    ));
}

#[test]
fn a_disconnected_account_refuses_a_sync_until_it_reactivates() {
    let mut account = account(AccountKind::Exchange, "binance");
    account.disconnect(at(10));
    assert_eq!(account.status(), AccountStatus::Disconnected);
    assert!(!account.is_syncable());
    assert!(matches!(
        account.begin_sync(at(11)),
        Err(DomainError::Denied { .. })
    ));
    account.reactivate(at(12)).unwrap();
    assert_eq!(account.status(), AccountStatus::Active);
    assert!(account.is_syncable());
}

#[test]
fn only_a_disconnected_account_can_reactivate() {
    let mut account = account(AccountKind::Exchange, "binance");
    assert!(matches!(
        account.reactivate(at(10)),
        Err(DomainError::InvalidTransition { .. })
    ));
}

#[test]
fn relabelling_replaces_the_display_label() {
    let mut account = account(AccountKind::Exchange, "binance");
    account.relabel("  Savings  ", at(10)).unwrap();
    assert_eq!(account.label(), "Savings");
    assert_eq!(account.updated_at(), at(10));
}

#[test]
fn anonymising_removes_the_personal_label_and_stamps_the_time() {
    let mut account = account(AccountKind::Exchange, "binance");
    account.anonymise(at(10));
    assert_eq!(account.anonymised_at(), Some(at(10)));
    assert!(account.label().starts_with("anonymised-"));
}

// ---------------------------------------------------------------------------
// Credentials
// ---------------------------------------------------------------------------

fn credential() -> Credential {
    Credential::pending(
        CredentialId::generate(),
        AccountId::generate(),
        "vault://secret/1",
        at(9),
    )
    .unwrap()
}

#[test]
fn a_new_credential_is_pending_and_not_usable() {
    let credential = credential();
    assert_eq!(credential.verification_status(), CredentialStatus::Pending);
    assert!(!credential.read_only());
    assert!(!credential.is_usable());
}

#[test]
fn verification_marks_the_credential_read_only_and_usable() {
    let mut credential = credential();
    credential.verify(at(10));
    assert_eq!(credential.verification_status(), CredentialStatus::Verified);
    assert!(credential.read_only());
    assert!(credential.is_usable());
    assert_eq!(credential.permissions_verified_at(), Some(at(10)));
}

#[test]
fn a_rejected_credential_is_never_usable_again() {
    let mut credential = credential();
    credential.verify(at(10));
    credential.reject(at(11));
    assert_eq!(credential.verification_status(), CredentialStatus::Rejected);
    assert!(!credential.read_only());
    assert!(!credential.is_usable());
}

#[test]
fn a_transient_failure_marks_the_credential_stale_but_usable() {
    let mut credential = credential();
    credential.verify(at(10));
    credential.mark_stale(at(11));
    assert_eq!(credential.verification_status(), CredentialStatus::Stale);
    assert!(credential.is_usable());
}

#[test]
fn a_replacement_reference_swaps_in_only_after_verification() {
    let mut credential = credential();
    credential
        .replace_reference("vault://secret/2", at(12))
        .unwrap();
    assert_eq!(credential.vault_secret_ref(), "vault://secret/2");
    assert_eq!(credential.verification_status(), CredentialStatus::Verified);
    assert!(matches!(
        credential.replace_reference("   ", at(13)),
        Err(DomainError::Required {
            field: "vaultSecretRef"
        })
    ));
}

#[test]
fn a_credential_requires_a_vault_reference() {
    assert!(matches!(
        Credential::pending(CredentialId::generate(), AccountId::generate(), " ", at(9)),
        Err(DomainError::Required {
            field: "vaultSecretRef"
        })
    ));
}

// ---------------------------------------------------------------------------
// Wallet addresses
// ---------------------------------------------------------------------------

#[test]
fn evm_and_solana_addresses_are_accepted() {
    assert!(validate_address("evm:1", "0x52908400098527886E0F7030069857D2E4169EE7").is_ok());
    assert!(validate_address("solana", "So11111111111111111111111111111111111111112").is_ok());
}

#[test]
fn malformed_addresses_are_refused_without_echoing_them() {
    let error = validate_address("evm:1", "0x1234").unwrap_err();
    match error {
        DomainError::Invalid { field, reason } => {
            assert_eq!(field, "address");
            assert!(!reason.contains("1234"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
    assert!(validate_address("solana", "0OIl").is_err());
    assert!(validate_address("bitcoin", "1BoatSLRHtKNngkdXEeobR76b53LETtpyT").is_err());
}

#[test]
fn a_wallet_address_never_carries_a_session_reference() {
    let connection = WalletConnection::new_address(
        WalletConnectionId::generate(),
        AccountId::generate(),
        "evm:1",
        "0x52908400098527886E0F7030069857D2E4169EE7",
        at(9),
    )
    .unwrap();
    assert_eq!(connection.session_ref(), None);
    assert_eq!(connection.observed_chain(), "evm:1");
}

#[test]
fn a_walletconnect_connection_requires_an_opaque_session_reference() {
    let id = WalletConnectionId::generate();
    let account = AccountId::generate();
    let address = "So11111111111111111111111111111111111111112";
    let connection =
        WalletConnection::walletconnect(id, account, "solana", address, "wc-session-1", at(9))
            .unwrap();
    assert_eq!(connection.session_ref(), Some("wc-session-1"));

    assert!(matches!(
        WalletConnection::walletconnect(id, account, "solana", address, " ", at(9)),
        Err(DomainError::Required {
            field: "sessionRef"
        })
    ));
    let long = "x".repeat(201);
    assert!(matches!(
        WalletConnection::walletconnect(id, account, "solana", address, &long, at(9)),
        Err(DomainError::TooLarge {
            field: "sessionRef",
            ..
        })
    ));
}

// ---------------------------------------------------------------------------
// Sync runs and leases
// ---------------------------------------------------------------------------

#[test]
fn a_claimed_run_is_running_with_a_live_lease() {
    let run = SyncRun::claim(
        SyncRunId::generate(),
        AccountId::generate(),
        at(10),
        LEASE_SECONDS,
    );
    assert_eq!(run.status(), SyncRunStatus::Running);
    assert_eq!(run.lease_until(), at(10) + Duration::seconds(LEASE_SECONDS));
    assert!(run.is_live(at(10)));
    assert!(!run.is_live(at(10) + Duration::seconds(LEASE_SECONDS)));
}

#[test]
fn renewal_extends_a_live_lease_and_refuses_an_expired_one() {
    let mut run = SyncRun::claim(SyncRunId::generate(), AccountId::generate(), at(10), 60);
    assert!(run.renew(at(10) + Duration::seconds(20), 60));
    assert_eq!(run.lease_until(), at(10) + Duration::seconds(80));
    assert!(!run.renew(at(11), 60));
}

#[test]
fn an_expired_running_run_is_failed_by_the_sweep() {
    let mut run = SyncRun::claim(SyncRunId::generate(), AccountId::generate(), at(10), 60);
    assert!(run.expire_if_due(at(11)));
    assert_eq!(run.status(), SyncRunStatus::Failed);
    assert_eq!(run.error_category(), Some(ErrorCategory::Data));
    assert_eq!(run.error_message(), Some("lease expired"));
    assert!(!run.expire_if_due(at(12)));
}

#[test]
fn a_successful_run_records_its_finish_time() {
    let mut run = SyncRun::claim(SyncRunId::generate(), AccountId::generate(), at(10), 60);
    run.succeed(at(10) + Duration::minutes(1)).unwrap();
    assert_eq!(run.status(), SyncRunStatus::Succeeded);
    assert_eq!(run.finished_at(), Some(at(10) + Duration::minutes(1)));
}

#[test]
fn a_failed_run_stores_a_bounded_non_secret_reason() {
    let mut run = SyncRun::claim(SyncRunId::generate(), AccountId::generate(), at(10), 60);
    let long = "x".repeat(MAX_ERROR_MESSAGE_CHARS + 10);
    run.fail(at(11), ErrorCategory::Provider, &long).unwrap();
    assert_eq!(run.status(), SyncRunStatus::Failed);
    assert_eq!(run.error_category(), Some(ErrorCategory::Provider));
    assert_eq!(
        run.error_message().unwrap().chars().count(),
        MAX_ERROR_MESSAGE_CHARS
    );
}

#[test]
fn a_secret_shaped_failure_reason_is_not_stored() {
    let mut run = SyncRun::claim(SyncRunId::generate(), AccountId::generate(), at(10), 60);
    let secret = format!("0x{}", "a".repeat(64));
    run.fail(at(11), ErrorCategory::Auth, &secret).unwrap();
    assert_eq!(run.error_message(), Some("sync failed"));
}

#[test]
fn a_finished_run_cannot_finish_again() {
    let mut run = SyncRun::claim(SyncRunId::generate(), AccountId::generate(), at(10), 60);
    run.succeed(at(11)).unwrap();
    assert!(matches!(
        run.fail(at(12), ErrorCategory::Data, "again"),
        Err(DomainError::InvalidTransition { .. })
    ));
}

// ---------------------------------------------------------------------------
// Holdings
// ---------------------------------------------------------------------------

#[test]
fn decimal_quantities_are_parsed_exactly() {
    assert_eq!(
        parse_quantity(" 1234.5678 ").unwrap(),
        Decimal::from_str("1234.5678").unwrap()
    );
}

#[test]
fn blank_negative_scientific_and_over_precise_quantities_are_refused() {
    assert!(matches!(
        parse_quantity(" "),
        Err(DomainError::Required { field: "quantity" })
    ));
    assert!(matches!(
        parse_quantity("-1"),
        Err(DomainError::Invalid { .. })
    ));
    assert!(matches!(
        parse_quantity("1e5"),
        Err(DomainError::Invalid { .. })
    ));
    assert!(matches!(
        parse_quantity("0.0000000000000000001"),
        Err(DomainError::Invalid { .. })
    ));
}

#[test]
fn a_resolved_holding_carries_the_canonical_asset_id() {
    let snapshot = HoldingSnapshot::record(
        HoldingSnapshotId::generate(),
        SyncRunId::generate(),
        AccountId::generate(),
        "BTC",
        Some("asset-btc"),
        Decimal::from_str("1.5").unwrap(),
        at(10),
    )
    .unwrap();
    assert_eq!(snapshot.asset_id(), Some("asset-btc"));
    assert!(!snapshot.spam());
}

#[test]
fn an_unresolved_holding_is_labelled_spam_and_keeps_the_observed_token() {
    let snapshot = HoldingSnapshot::record(
        HoldingSnapshotId::generate(),
        SyncRunId::generate(),
        AccountId::generate(),
        "0xdeadbeef",
        None,
        Decimal::from_str("7").unwrap(),
        at(10),
    )
    .unwrap();
    assert_eq!(snapshot.asset_id(), None);
    assert!(snapshot.spam());
    assert_eq!(snapshot.observed_token(), "0xdeadbeef");
}

#[test]
fn an_empty_observed_token_is_refused() {
    assert!(matches!(
        HoldingSnapshot::record(
            HoldingSnapshotId::generate(),
            SyncRunId::generate(),
            AccountId::generate(),
            " ",
            Some("asset"),
            Decimal::from_str("1").unwrap(),
            at(10)
        ),
        Err(DomainError::Required {
            field: "observedToken"
        })
    ));
}

// ---------------------------------------------------------------------------
// CSV imports
// ---------------------------------------------------------------------------

#[test]
fn the_three_formats_are_detected_from_the_header() {
    assert_eq!(
        ImportSource::detect("UTC_Time,Account,Operation,Coin,Change,Remark").unwrap(),
        ImportSource::Binance
    );
    assert_eq!(
        ImportSource::detect("timestamp,type,asset,amount,price").unwrap(),
        ImportSource::Okx
    );
    assert_eq!(
        ImportSource::detect("external_id,kind,asset,quantity,price,executed_at").unwrap(),
        ImportSource::Generic
    );
}

#[test]
fn an_unknown_format_is_refused() {
    assert!(matches!(
        ImportSource::detect("date,amount,note"),
        Err(DomainError::Invalid { field: "file", .. })
    ));
}

#[test]
fn csv_lines_honour_quotes_and_doubled_quotes() {
    let fields = parse_csv_line("1,\"buy, spot\",\"he said \"\"hi\"\"\",BTC").unwrap();
    assert_eq!(fields, vec!["1", "buy, spot", "he said \"hi\"", "BTC"]);
    assert!(parse_csv_line("\"unterminated").is_err());
}

#[test]
fn transaction_kinds_accept_venue_aliases() {
    assert_eq!(
        TransactionKind::parse("Deposit").unwrap(),
        TransactionKind::TransferIn
    );
    assert_eq!(
        TransactionKind::parse("withdrawal").unwrap(),
        TransactionKind::TransferOut
    );
    assert_eq!(
        TransactionKind::parse("Commission").unwrap(),
        TransactionKind::Fee
    );
    assert!(TransactionKind::parse("stake").is_err());
}

#[test]
fn executed_times_accept_rfc3339_and_plain_forms() {
    assert_eq!(parse_executed_at("2026-11-07T10:00:00Z").unwrap(), at(10));
    assert_eq!(parse_executed_at("2026-11-07 10:00:00").unwrap(), at(10));
    assert!(parse_executed_at("yesterday").is_err());
}

#[test]
fn the_dedupe_key_uses_the_external_id_when_present() {
    let account = AccountId::generate();
    assert_eq!(
        dedupe_key_for(
            account,
            Some("trade-1"),
            TransactionKind::Buy,
            "asset-btc",
            Decimal::from_str("1").unwrap(),
            at(10)
        ),
        format!("{account}|ext|trade-1")
    );
}

#[test]
fn the_dedupe_key_falls_back_to_the_row_shape() {
    let account = AccountId::generate();
    let key = dedupe_key_for(
        account,
        None,
        TransactionKind::Buy,
        "asset-btc",
        Decimal::from_str("1.500").unwrap(),
        at(10),
    );
    assert_eq!(
        key,
        format!("{account}|row|buy|asset-btc|1.5|{}", at(10).to_rfc3339())
    );
}

#[test]
fn a_parsed_transaction_derives_the_same_key_as_the_helper() {
    let account = AccountId::generate();
    let parsed = ParsedTransaction {
        external_id: None,
        asset_id: "asset-eth".to_string(),
        kind: TransactionKind::Sell,
        quantity: Decimal::from_str("2").unwrap(),
        price: None,
        executed_at: at(10),
    };
    assert_eq!(
        parsed.dedupe_key(account),
        dedupe_key_for(
            account,
            None,
            TransactionKind::Sell,
            "asset-eth",
            Decimal::from_str("2").unwrap(),
            at(10)
        )
    );
}

#[test]
fn a_batch_starts_pending_and_records_reconciling_counts() {
    let mut batch = ImportBatch::new(
        ImportBatchId::generate(),
        AccountId::generate(),
        ImportSource::Generic,
        "trades.csv",
        at(9),
    )
    .unwrap();
    assert_eq!(batch.status(), ImportBatchStatus::Pending);
    batch.begin_processing();
    assert_eq!(batch.status(), ImportBatchStatus::Processing);
    batch.complete(8, 1, 1, at(10));
    assert_eq!(batch.status(), ImportBatchStatus::Complete);
    assert_eq!(batch.imported_count(), 8);
    assert_eq!(batch.skipped_count(), 1);
    assert_eq!(batch.failed_count(), 1);
    assert_eq!(batch.completed_at(), Some(at(10)));
}

#[test]
fn a_batch_requires_a_bounded_file_name() {
    assert!(matches!(
        ImportBatch::new(
            ImportBatchId::generate(),
            AccountId::generate(),
            ImportSource::Generic,
            " ",
            at(9)
        ),
        Err(DomainError::Required { field: "fileName" })
    ));
    let long = "x".repeat(256);
    assert!(matches!(
        ImportBatch::new(
            ImportBatchId::generate(),
            AccountId::generate(),
            ImportSource::Generic,
            &long,
            at(9)
        ),
        Err(DomainError::TooLarge {
            field: "fileName",
            ..
        })
    ));
}

#[test]
fn a_row_error_needs_a_line_number_and_reason() {
    let batch = ImportBatchId::generate();
    assert!(matches!(
        ImportRowError::record(ImportRowErrorId::generate(), batch, 0, "bad"),
        Err(DomainError::Invalid {
            field: "lineNumber",
            ..
        })
    ));
    assert!(matches!(
        ImportRowError::record(ImportRowErrorId::generate(), batch, 2, " "),
        Err(DomainError::Required { field: "reason" })
    ));
}

#[test]
fn the_summary_reconciles_imported_skipped_and_failed() {
    let summary = ImportSummary {
        imported: 8,
        skipped: 1,
        failed: 1,
        row_errors: Vec::new(),
    };
    assert!(summary.reconciles(10));
    assert!(!summary.reconciles(11));
}

// ---------------------------------------------------------------------------
// Secret detection
// ---------------------------------------------------------------------------

#[test]
fn a_hex_private_key_is_detected_with_or_without_the_prefix() {
    assert!(looks_like_secret(&format!("0x{}", "a".repeat(64))));
    assert!(looks_like_secret(&"f".repeat(64)));
}

#[test]
fn a_twelve_word_seed_phrase_is_detected() {
    assert!(looks_like_secret(
        "abandon ability able about above absent absorb abstract absurd abuse access accident"
    ));
    assert!(!looks_like_secret("Binance read-only key"));
}

#[test]
fn a_wif_private_key_is_detected() {
    let wif = format!("5{}", "a".repeat(50));
    assert!(looks_like_secret(&wif));
}

#[test]
fn a_secret_shaped_field_is_refused() {
    assert!(matches!(
        ensure_not_secret("apiKey", &"a".repeat(64)),
        Err(DomainError::SecretLike { field: "apiKey" })
    ));
    assert!(ensure_not_secret("label", "Main").is_ok());
}

// ---------------------------------------------------------------------------
// Audit and service identity
// ---------------------------------------------------------------------------

#[test]
fn an_audit_record_carries_the_required_fields() {
    let mut context = BTreeMap::new();
    context.insert("accountId".to_string(), "abc".to_string());
    let record = AuditRecord::new(
        Some(user_id()),
        "account.credential.added",
        "account-1",
        AuditOutcome::Success,
        at(10),
        context,
    )
    .unwrap();
    assert_eq!(record.action(), "account.credential.added");
    assert_eq!(record.target(), "account-1");
    assert_eq!(record.outcome(), AuditOutcome::Success);
    assert_eq!(record.context().get("accountId").unwrap(), "abc");
}

#[test]
fn an_audit_record_requires_an_action_and_target() {
    assert!(matches!(
        AuditRecord::new(
            None,
            " ",
            "target",
            AuditOutcome::Success,
            at(10),
            BTreeMap::new()
        ),
        Err(DomainError::Required { field: "action" })
    ));
    assert!(matches!(
        AuditRecord::new(
            None,
            "action",
            " ",
            AuditOutcome::Success,
            at(10),
            BTreeMap::new()
        ),
        Err(DomainError::Required { field: "target" })
    ));
}

#[test]
fn each_internal_service_is_limited_to_its_own_operations() {
    assert!(service_allowed("identity", Operation::PrivacyExport));
    assert!(service_allowed("identity", Operation::PrivacyAnonymise));
    assert!(!service_allowed("identity", Operation::HoldingsRead));
    assert!(service_allowed("portfolio", Operation::HoldingsRead));
    assert!(service_allowed("ledger", Operation::TransactionsRead));
    assert!(!service_allowed("ledger", Operation::PrivacyExport));
    assert!(service_allowed(
        "job-orchestration",
        Operation::RunScheduledSync
    ));
    assert!(!service_allowed("unknown", Operation::HoldingsRead));
}

#[test]
fn operation_labels_round_trip() {
    for operation in [
        Operation::PrivacyExport,
        Operation::PrivacyAnonymise,
        Operation::HoldingsRead,
        Operation::TransactionsRead,
        Operation::RunScheduledSync,
    ] {
        assert_eq!(Operation::parse(operation.as_str()).unwrap(), operation);
    }
    assert!(Operation::parse("delete_everything").is_err());
}

#[test]
fn ids_parse_and_reject_bad_text() {
    let id = AccountId::generate();
    assert_eq!(AccountId::parse(&id.to_string()).unwrap(), id);
    assert!(matches!(
        AccountId::parse("not-a-uuid"),
        Err(DomainError::InvalidId { entity: "account" })
    ));
    assert_eq!(
        SyncRunStatus::parse("succeeded").unwrap(),
        SyncRunStatus::Succeeded
    );
    assert_eq!(
        ErrorCategory::parse("rate_limit").unwrap(),
        ErrorCategory::RateLimit
    );
}
