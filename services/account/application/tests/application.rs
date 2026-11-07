//! Application tests: account onboarding, credential verification and
//! replacement, wallet and WalletConnect onboarding, sync leases and snapshots,
//! CSV imports, C5 reads, and privacy export/anonymise — all against port fakes.

mod fakes;

use std::sync::Arc;

use account_application::accounts::{AccountService, ExchangeKey, ResyncOutcome};
use account_application::common::{
    AppError, AssuranceLevel, Caller, FieldError, ServiceCaller, VerifiedClaims,
};
use account_application::holdings::HoldingService;
use account_application::imports::ImportService;
use account_application::ports::{
    AccountRepository, Clock, CredentialRepository, ExternalError, ImportRepository,
    ObservedBalance, PrivacyRepository, SyncRunRepository, VerifiedPermissions,
    WalletConnectionRepository,
};
use account_application::privacy::PrivacyService;
use account_application::sync::SyncService;
use account_application::trail::Trail;
use account_application::verification::VerificationService;
use account_domain::{
    Account, AccountId, AccountKind, AccountStatus, Credential, CredentialId, CredentialStatus,
    DomainError, HoldingSnapshot, ImportSource, SyncRun, SyncRunId, TransactionKind, UserId,
    WalletConnection, WalletConnectionId, CSV_MAX_BYTES, LEASE_SECONDS,
};
use chrono::{DateTime, Duration, TimeZone, Utc};
use rust_decimal::Decimal;
use std::str::FromStr;

use fakes::{
    FakeAccounts, FakeAudit, FakeAuthAdmin, FakeChain, FakeCredentials, FakeExchange,
    FakeHoldingStore, FakeHoldings, FakeImports, FakeNotifications, FakePrivacy, FakeRegistry,
    FakeRuns, FakeVault, FakeWake, FakeWallets, MutableClock,
};

const USER_A: &str = "11111111-1111-4111-8111-111111111111";
const USER_B: &str = "22222222-2222-4222-8222-222222222222";

fn user_a() -> UserId {
    UserId::parse(USER_A).unwrap()
}

fn user_b() -> UserId {
    UserId::parse(USER_B).unwrap()
}

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, hour, 0, 0).unwrap()
}

fn caller() -> Caller {
    Caller {
        user_id: user_a(),
        assurance: AssuranceLevel::Aal2,
        correlation_id: "corr-test".to_string(),
    }
}

fn aal1_caller() -> Caller {
    Caller {
        assurance: AssuranceLevel::Aal1,
        ..caller()
    }
}

fn other_caller() -> Caller {
    Caller {
        user_id: user_b(),
        assurance: AssuranceLevel::Aal2,
        correlation_id: "corr-other".to_string(),
    }
}

fn service(name: &str) -> ServiceCaller {
    ServiceCaller::new(name, "corr-service")
}

fn key() -> ExchangeKey {
    ExchangeKey::new("api-key-placeholder", "api-secret-placeholder")
}

fn decimal(raw: &str) -> Decimal {
    Decimal::from_str(raw).unwrap()
}

struct Harness {
    accounts: Arc<FakeAccounts>,
    credentials: Arc<FakeCredentials>,
    wallets: Arc<FakeWallets>,
    runs: Arc<FakeRuns>,
    holding_store: Arc<FakeHoldingStore>,
    imports: Arc<FakeImports>,
    privacy: Arc<FakePrivacy>,
    exchange: Arc<FakeExchange>,
    chain: Arc<FakeChain>,
    registry: Arc<FakeRegistry>,
    vault: Arc<FakeVault>,
    auth_admin: Arc<FakeAuthAdmin>,
    audit: Arc<FakeAudit>,
    notifications: Arc<FakeNotifications>,
    wake: Arc<FakeWake>,
    clock: Arc<MutableClock>,
    account_service: Arc<AccountService>,
    sync: Arc<SyncService>,
    verification: Arc<VerificationService>,
    import_service: Arc<ImportService>,
    holding_service: Arc<HoldingService>,
    privacy_service: Arc<PrivacyService>,
}

fn harness() -> Harness {
    let accounts = Arc::new(FakeAccounts::default());
    let credentials = Arc::new(FakeCredentials::new(accounts.clone()));
    let wallets = Arc::new(FakeWallets::new(accounts.clone()));
    let holding_store = Arc::new(FakeHoldingStore::default());
    let runs = Arc::new(FakeRuns::new(holding_store.clone()));
    let holdings = Arc::new(FakeHoldings::new(holding_store.clone(), accounts.clone()));
    let imports = Arc::new(FakeImports::new(accounts.clone()));
    let privacy = Arc::new(FakePrivacy::default());
    let exchange = Arc::new(FakeExchange::default());
    let chain = Arc::new(FakeChain::default());
    let registry = Arc::new(FakeRegistry::default());
    let vault = Arc::new(FakeVault::default());
    let auth_admin = Arc::new(FakeAuthAdmin::default());
    let audit = Arc::new(FakeAudit::default());
    let notifications = Arc::new(FakeNotifications::default());
    let wake = Arc::new(FakeWake::default());
    let clock = Arc::new(MutableClock::at(at(10)));

    let trail = Arc::new(Trail::new(audit.clone(), clock.clone()));

    let account_service = Arc::new(AccountService::new(
        accounts.clone(),
        credentials.clone(),
        wallets.clone(),
        runs.clone(),
        exchange.clone(),
        registry.clone(),
        vault.clone(),
        auth_admin.clone(),
        wake.clone(),
        trail.clone(),
        clock.clone(),
    ));
    let sync = Arc::new(SyncService::new(
        accounts.clone(),
        credentials.clone(),
        wallets.clone(),
        runs.clone(),
        exchange.clone(),
        chain.clone(),
        registry.clone(),
        vault.clone(),
        notifications.clone(),
        trail.clone(),
        clock.clone(),
    ));
    let verification = Arc::new(VerificationService::new(
        credentials.clone(),
        accounts.clone(),
        exchange.clone(),
        vault.clone(),
        auth_admin.clone(),
        notifications.clone(),
        trail.clone(),
        clock.clone(),
    ));
    let import_service = Arc::new(ImportService::new(
        accounts.clone(),
        imports.clone(),
        trail,
        clock.clone(),
    ));
    let holding_service = Arc::new(HoldingService::new(
        accounts.clone(),
        holdings.clone(),
        imports.clone(),
    ));
    let privacy_service = Arc::new(PrivacyService::new(
        accounts.clone(),
        credentials.clone(),
        wallets.clone(),
        holdings.clone(),
        imports.clone(),
        privacy.clone(),
        vault.clone(),
        Arc::new(Trail::new(audit.clone(), clock.clone())),
        clock.clone(),
    ));

    Harness {
        accounts,
        credentials,
        wallets,
        runs,
        holding_store,
        imports,
        privacy,
        exchange,
        chain,
        registry,
        vault,
        auth_admin,
        audit,
        notifications,
        wake,
        clock,
        account_service,
        sync,
        verification,
        import_service,
        holding_service,
        privacy_service,
    }
}

fn seed_account(h: &Harness, kind: AccountKind, venue: &str, label: &str) -> Account {
    let account =
        Account::open(AccountId::generate(), user_a(), kind, venue, label, at(9)).unwrap();
    h.accounts.seed(account.clone());
    account
}

fn seed_credential(h: &Harness, account: &Account) -> Credential {
    let mut credential = Credential::pending(
        CredentialId::generate(),
        account.id(),
        "vault://account/seed",
        at(9),
    )
    .unwrap();
    credential.verify(at(9));
    h.credentials.seed(credential.clone());
    h.vault.seed(
        credential.vault_secret_ref(),
        r#"{"apiKey":"api-key-placeholder","apiSecret":"api-secret-placeholder"}"#,
    );
    credential
}

fn seed_connection(h: &Harness, account: &Account) -> WalletConnection {
    let address = if account.venue().as_str() == "solana" {
        "So11111111111111111111111111111111111111112"
    } else {
        "0x52908400098527886E0F7030069857D2E4169EE7"
    };
    let connection = WalletConnection::new_address(
        WalletConnectionId::generate(),
        account.id(),
        account.venue().as_str(),
        address,
        at(9),
    )
    .unwrap();
    h.wallets.seed(connection.clone());
    connection
}

fn seed_run(h: &Harness, account: &Account) -> SyncRun {
    let run = SyncRun::claim(
        SyncRunId::generate(),
        account.id(),
        h.clock.now(),
        LEASE_SECONDS,
    );
    h.runs.seed(run.clone());
    run
}

// ---------------------------------------------------------------------------
// Account onboarding (WF1–WF3)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn adding_an_exchange_account_verifies_before_creating_and_records_everything() {
    let h = harness();
    let account = h
        .account_service
        .add_exchange(&caller(), "binance", "Main Binance", key())
        .await
        .unwrap();

    assert_eq!(account.kind(), AccountKind::Exchange);
    assert_eq!(account.status(), AccountStatus::Active);
    assert_eq!(h.accounts.all().len(), 1, "one account row");
    let credentials = h.credentials.all();
    assert_eq!(credentials.len(), 1);
    assert_eq!(
        credentials[0].verification_status(),
        CredentialStatus::Verified
    );
    assert!(credentials[0].read_only());
    assert_eq!(
        h.vault
            .store_calls
            .load(std::sync::atomic::Ordering::Relaxed),
        1
    );
    assert_eq!(h.auth_admin.calls(), vec![(user_a(), true)]);
    assert_eq!(h.runs.all().len(), 1, "the first sync is queued");
    assert_eq!(h.wake.calls.load(std::sync::atomic::Ordering::Relaxed), 1);
    let actions = h.audit.actions();
    assert!(actions.contains(&"account.added".to_string()));
    assert!(actions.contains(&"account.credential.added".to_string()));
}

#[tokio::test]
async fn adding_an_exchange_account_below_aal2_is_refused_before_any_side_effect() {
    let h = harness();
    let error = h
        .account_service
        .add_exchange(&aal1_caller(), "binance", "Main", key())
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Forbidden(_)));
    assert_eq!(
        h.vault
            .store_calls
            .load(std::sync::atomic::Ordering::Relaxed),
        0
    );
    assert!(h.accounts.all().is_empty());
    assert!(h.credentials.all().is_empty());
}

#[tokio::test]
async fn secret_shaped_key_material_is_refused_before_storage() {
    let h = harness();
    let secret = "a".repeat(64);
    let error = h
        .account_service
        .add_exchange(
            &caller(),
            "binance",
            "Main",
            ExchangeKey::new(&secret, "api-secret-placeholder"),
        )
        .await
        .unwrap_err();
    match error {
        AppError::Validation(errors) => {
            assert_eq!(
                errors,
                vec![FieldError::new(
                    "apiKey",
                    "looks like key material and is never accepted"
                )]
            );
        }
        other => panic!("unexpected error: {other:?}"),
    }
    assert_eq!(
        h.vault
            .store_calls
            .load(std::sync::atomic::Ordering::Relaxed),
        0
    );
}

#[tokio::test]
async fn a_key_that_fails_verification_destroys_the_candidate_and_stores_nothing() {
    let h = harness();
    h.exchange.set_verification(Err(ExternalError::Refused(
        "permissions endpoint refused".to_string(),
    )));
    let error = h
        .account_service
        .add_exchange(&caller(), "binance", "Main", key())
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Unavailable(_)));
    assert_eq!(
        h.vault
            .store_calls
            .load(std::sync::atomic::Ordering::Relaxed),
        1
    );
    assert_eq!(h.vault.destroyed().len(), 1);
    assert!(h.accounts.all().is_empty());
    assert!(h.credentials.all().is_empty());
}

#[tokio::test]
async fn a_key_with_trading_permission_is_refused_and_destroyed() {
    let h = harness();
    h.exchange
        .set_verification(Ok(VerifiedPermissions { read_only: false }));
    let error = h
        .account_service
        .add_exchange(&caller(), "binance", "Main", key())
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Forbidden(_)));
    assert_eq!(h.vault.destroyed().len(), 1);
    assert!(h.accounts.all().is_empty());
}

#[tokio::test]
async fn a_duplicate_label_ignoring_case_is_refused() {
    let h = harness();
    seed_account(&h, AccountKind::Exchange, "binance", "Main");
    let error = h
        .account_service
        .add_exchange(&caller(), "binance", "MAIN", key())
        .await
        .unwrap_err();
    match error {
        AppError::Validation(errors) => assert_eq!(errors[0].field, "label"),
        other => panic!("unexpected error: {other:?}"),
    }
    assert_eq!(
        h.vault
            .store_calls
            .load(std::sync::atomic::Ordering::Relaxed),
        0
    );
}

#[tokio::test]
async fn an_exchange_venue_outside_binance_and_okx_is_refused() {
    let h = harness();
    let error = h
        .account_service
        .add_exchange(&caller(), "kraken", "Main", key())
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Validation(_)));
    assert_eq!(
        h.vault
            .store_calls
            .load(std::sync::atomic::Ordering::Relaxed),
        0
    );
}

#[tokio::test]
async fn the_credential_audit_event_carries_no_key_or_secret() {
    let h = harness();
    h.account_service
        .add_exchange(&caller(), "binance", "Main", key())
        .await
        .unwrap();
    let events = h.audit.events();
    let credential_event = events
        .iter()
        .find(|event| event.action() == "account.credential.added")
        .expect("the credential event is recorded");
    for key in credential_event.context().keys() {
        assert!(!key.to_ascii_lowercase().contains("key"));
        assert!(!key.to_ascii_lowercase().contains("secret"));
    }
    assert_eq!(
        credential_event
            .context()
            .get("accountId")
            .map(String::as_str),
        Some(credential_event.target())
    );
}

#[tokio::test]
async fn replacing_a_key_verifies_then_swaps_and_destroys_the_old_secret() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Exchange, "binance", "Main");
    let credential = seed_credential(&h, &account);
    let old_reference = credential.vault_secret_ref().to_string();

    let updated = h
        .account_service
        .replace_key(&caller(), account.id(), key())
        .await
        .unwrap();
    assert_eq!(updated.verification_status(), CredentialStatus::Verified);
    assert_ne!(updated.vault_secret_ref(), old_reference);
    assert_eq!(h.vault.destroyed(), vec![old_reference]);
    assert!(h
        .audit
        .actions()
        .contains(&"account.credential.replaced".to_string()));
}

#[tokio::test]
async fn a_failed_replacement_keeps_the_old_credential_usable() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Exchange, "binance", "Main");
    let credential = seed_credential(&h, &account);
    let old_reference = credential.vault_secret_ref().to_string();
    h.exchange.set_verification(Err(ExternalError::Refused(
        "withdrawal permission".to_string(),
    )));

    let error = h
        .account_service
        .replace_key(&caller(), account.id(), key())
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Unavailable(_)));
    let stored = h
        .credentials
        .find_for_account(account.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.vault_secret_ref(), old_reference);
    assert_eq!(stored.verification_status(), CredentialStatus::Verified);
    assert!(
        h.vault.secret(&old_reference).is_some(),
        "the old secret survives"
    );
    assert_eq!(
        h.vault.destroyed().len(),
        1,
        "only the candidate is destroyed"
    );
}

#[tokio::test]
async fn replacing_the_key_of_a_disconnected_account_reactivates_it() {
    let h = harness();
    let mut account = seed_account(&h, AccountKind::Exchange, "binance", "Main");
    account.disconnect(at(9));
    h.accounts.update(&account).await.unwrap();
    seed_credential(&h, &account);

    let updated = h
        .account_service
        .replace_key(&caller(), account.id(), key())
        .await
        .unwrap();
    assert_eq!(updated.verification_status(), CredentialStatus::Verified);
    let stored = h.accounts.find_by_id(account.id()).await.unwrap().unwrap();
    assert_eq!(stored.status(), AccountStatus::Active);
    assert_eq!(h.runs.all().len(), 1);
}

#[tokio::test]
async fn replacing_a_key_for_another_user_is_forbidden() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Exchange, "binance", "Main");
    seed_credential(&h, &account);
    let error = h
        .account_service
        .replace_key(&other_caller(), account.id(), key())
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Forbidden(_)));
}

#[tokio::test]
async fn adding_a_wallet_resolves_the_chain_and_stores_the_connection() {
    let h = harness();
    let account = h
        .account_service
        .add_wallet(
            &caller(),
            "evm:1",
            "0x52908400098527886E0F7030069857D2E4169EE7",
            "Main wallet",
        )
        .await
        .unwrap();
    assert_eq!(account.kind(), AccountKind::Wallet);
    let connection = h
        .wallets
        .find_for_account(account.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(connection.observed_chain(), "evm:1");
    assert_eq!(h.runs.all().len(), 1);
    assert_eq!(h.wake.calls.load(std::sync::atomic::Ordering::Relaxed), 1);
}

#[tokio::test]
async fn an_unknown_wallet_chain_is_refused_with_the_supported_list() {
    let h = harness();
    let error = h
        .account_service
        .add_wallet(
            &caller(),
            "evm:999",
            "0x52908400098527886E0F7030069857D2E4169EE7",
            "Main wallet",
        )
        .await
        .unwrap_err();
    match error {
        AppError::Validation(errors) => {
            assert_eq!(errors[0].field, "chain");
            assert!(errors[0].reason.contains("solana"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
    assert!(h.accounts.all().is_empty());
}

#[tokio::test]
async fn a_malformed_wallet_address_is_refused_and_nothing_is_stored() {
    let h = harness();
    let error = h
        .account_service
        .add_wallet(&caller(), "evm:1", "0x1234", "Main wallet")
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Validation(_)));
    assert!(h.accounts.all().is_empty());
    assert!(h.wallets.all().is_empty());
}

#[tokio::test]
async fn a_duplicate_wallet_label_is_refused() {
    let h = harness();
    seed_account(&h, AccountKind::Wallet, "evm:1", "Main wallet");
    let error = h
        .account_service
        .add_wallet(
            &caller(),
            "evm:1",
            "0x52908400098527886E0F7030069857D2E4169EE7",
            "MAIN WALLET",
        )
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Validation(_)));
}

#[tokio::test]
async fn a_walletconnect_connection_stores_an_opaque_reference_and_no_credential() {
    let h = harness();
    let account = h
        .account_service
        .add_walletconnect(
            &caller(),
            "solana",
            "So11111111111111111111111111111111111111112",
            "WC wallet",
            "wc-session-1",
        )
        .await
        .unwrap();
    assert_eq!(account.kind(), AccountKind::WalletConnect);
    let connection = h
        .wallets
        .find_for_account(account.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(connection.session_ref(), Some("wc-session-1"));
    assert!(h.credentials.all().is_empty());
}

#[tokio::test]
async fn a_walletconnect_connection_requires_a_session_reference() {
    let h = harness();
    let error = h
        .account_service
        .add_walletconnect(
            &caller(),
            "solana",
            "So11111111111111111111111111111111111111112",
            "WC wallet",
            " ",
        )
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Validation(_)));
    assert!(h.accounts.all().is_empty());
}

#[tokio::test]
async fn listing_accounts_requires_the_owner() {
    let h = harness();
    let error = h
        .account_service
        .list(&other_caller(), user_a())
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Forbidden(_)));
}

#[tokio::test]
async fn listing_accounts_returns_the_users_accounts() {
    let h = harness();
    let first = seed_account(&h, AccountKind::Exchange, "binance", "Main");
    let second = seed_account(&h, AccountKind::Wallet, "evm:1", "Wallet");
    let accounts = h.account_service.list(&caller(), user_a()).await.unwrap();
    let ids: Vec<AccountId> = accounts.iter().map(Account::id).collect();
    assert!(ids.contains(&first.id()));
    assert!(ids.contains(&second.id()));
}

#[tokio::test]
async fn a_csv_account_cannot_be_resynced() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Csv, "csv:summary", "History");
    let error = h.resync_error(&caller(), account.id()).await;
    assert!(matches!(error, AppError::Validation(_)));
}

#[tokio::test]
async fn resync_claims_a_run_and_wakes_the_worker() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Exchange, "binance", "Main");
    let outcome = h
        .account_service
        .resync(&caller(), account.id())
        .await
        .unwrap();
    match outcome {
        ResyncOutcome::Started(run) => {
            assert_eq!(run.status(), account_domain::SyncRunStatus::Running)
        }
        other => panic!("unexpected outcome: {other:?}"),
    }
    assert_eq!(h.runs.all().len(), 1);
    assert_eq!(h.wake.calls.load(std::sync::atomic::Ordering::Relaxed), 1);
    assert!(h
        .audit
        .actions()
        .contains(&"account.resync.requested".to_string()));
}

#[tokio::test]
async fn a_manual_resync_with_a_live_run_returns_the_active_run() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Exchange, "binance", "Main");
    let active = seed_run(&h, &account);
    let outcome = h
        .account_service
        .resync(&caller(), account.id())
        .await
        .unwrap();
    match outcome {
        ResyncOutcome::AlreadyRunning(run) => assert_eq!(run.id(), active.id()),
        other => panic!("unexpected outcome: {other:?}"),
    }
    assert_eq!(h.runs.all().len(), 1);
    assert_eq!(h.wake.calls.load(std::sync::atomic::Ordering::Relaxed), 0);
}

#[tokio::test]
async fn resync_for_another_user_is_forbidden() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Exchange, "binance", "Main");
    let error = h
        .account_service
        .resync(&other_caller(), account.id())
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Forbidden(_)));
}

impl Harness {
    async fn resync_error(&self, caller: &Caller, account_id: AccountId) -> AppError {
        self.account_service
            .resync(caller, account_id)
            .await
            .unwrap_err()
    }
}

// ---------------------------------------------------------------------------
// Sync (WF4)
// ---------------------------------------------------------------------------

fn seed_exchange_sync(
    h: &Harness,
    balances: Vec<ObservedBalance>,
) -> (Account, Credential, SyncRun) {
    let account = seed_account(h, AccountKind::Exchange, "binance", "Main");
    let credential = seed_credential(h, &account);
    h.exchange.set_balances(Ok(balances));
    let run = seed_run(h, &account);
    (account, credential, run)
}

#[tokio::test]
async fn a_successful_exchange_sync_writes_a_snapshot_and_activates_the_account() {
    let h = harness();
    let (account, _credential, run) = seed_exchange_sync(
        &h,
        vec![ObservedBalance {
            observed_token: "BTC".to_string(),
            quantity: decimal("1.5"),
        }],
    );
    h.registry.set_resolution("BTC", Some("asset-btc"));

    let outcome = h.sync.process(run.id(), "corr").await.unwrap();
    assert!(matches!(
        outcome,
        account_application::sync::RunOutcome::Succeeded(_)
    ));

    let snapshots = h.runs.snapshots();
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].asset_id(), Some("asset-btc"));
    assert_eq!(snapshots[0].quantity(), decimal("1.5"));
    assert!(!snapshots[0].spam());

    let stored = h.accounts.find_by_id(account.id()).await.unwrap().unwrap();
    assert_eq!(stored.status(), AccountStatus::Active);
    assert_eq!(stored.last_synced_at(), Some(at(10)));
    let stored_run = h.runs.find_by_id(run.id()).await.unwrap().unwrap();
    assert_eq!(
        stored_run.status(),
        account_domain::SyncRunStatus::Succeeded
    );
    assert!(h
        .audit
        .actions()
        .contains(&"account.sync.succeeded".to_string()));
}

#[tokio::test]
async fn an_unresolved_token_is_labelled_spam_and_kept() {
    let h = harness();
    let (_account, _credential, run) = seed_exchange_sync(
        &h,
        vec![ObservedBalance {
            observed_token: "DOGE".to_string(),
            quantity: decimal("10"),
        }],
    );
    h.sync.process(run.id(), "corr").await.unwrap();
    let snapshots = h.runs.snapshots();
    assert_eq!(snapshots.len(), 1);
    assert!(snapshots[0].spam());
    assert_eq!(snapshots[0].asset_id(), None);
    assert_eq!(snapshots[0].observed_token(), "DOGE");
}

#[tokio::test]
async fn a_wallet_sync_uses_the_chain_provider() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Wallet, "solana", "Wallet");
    seed_connection(&h, &account);
    h.chain.set_balances(Ok(vec![ObservedBalance {
        observed_token: "SOL".to_string(),
        quantity: decimal("2"),
    }]));
    h.registry.set_resolution("SOL", Some("asset-sol"));
    let run = seed_run(&h, &account);

    h.sync.process(run.id(), "corr").await.unwrap();
    assert_eq!(h.chain.calls.load(std::sync::atomic::Ordering::Relaxed), 1);
    assert_eq!(h.runs.snapshots().len(), 1);
}

#[tokio::test]
async fn a_csv_account_is_not_synced() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Csv, "csv:summary", "History");
    let run = seed_run(&h, &account);
    let outcome = h.sync.process(run.id(), "corr").await.unwrap();
    assert!(matches!(
        outcome,
        account_application::sync::RunOutcome::Failed(_)
    ));
    assert!(h.runs.snapshots().is_empty());
    assert_eq!(
        h.exchange
            .balance_calls
            .load(std::sync::atomic::Ordering::Relaxed),
        0
    );
    assert_eq!(h.chain.calls.load(std::sync::atomic::Ordering::Relaxed), 0);
}

#[tokio::test]
async fn a_provider_failure_keeps_previous_holdings_and_notifies() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Exchange, "binance", "Main");
    let credential = seed_credential(&h, &account);
    let previous = HoldingSnapshot::record(
        account_domain::HoldingSnapshotId::generate(),
        SyncRunId::generate(),
        account.id(),
        "BTC",
        Some("asset-btc"),
        decimal("1"),
        at(9),
    )
    .unwrap();
    h.holding_store.snapshots.lock().unwrap().push(previous);
    h.exchange.set_balances(Err(ExternalError::Unavailable(
        "the venue is unreachable".to_string(),
    )));
    let run = seed_run(&h, &account);

    let outcome = h.sync.process(run.id(), "corr").await.unwrap();
    assert!(matches!(
        outcome,
        account_application::sync::RunOutcome::Failed(_)
    ));
    assert_eq!(h.runs.snapshots().len(), 1, "the previous snapshot is kept");
    let stored = h.accounts.find_by_id(account.id()).await.unwrap().unwrap();
    assert_eq!(stored.status(), AccountStatus::Error);
    assert_eq!(h.notifications.sent().len(), 1);
    assert_eq!(h.notifications.sent()[0].1, "sync_failed");
    assert!(h
        .audit
        .actions()
        .contains(&"account.sync.failed".to_string()));
    let stored_run = h.runs.find_by_id(run.id()).await.unwrap().unwrap();
    assert_eq!(stored_run.status(), account_domain::SyncRunStatus::Failed);
    assert_eq!(
        stored_run.error_category(),
        Some(account_domain::ErrorCategory::Network)
    );
    let _ = credential;
}

#[tokio::test]
async fn a_missing_usable_credential_fails_the_run_as_auth() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Exchange, "binance", "Main");
    let run = seed_run(&h, &account);
    h.sync.process(run.id(), "corr").await.unwrap();
    let stored_run = h.runs.find_by_id(run.id()).await.unwrap().unwrap();
    assert_eq!(
        stored_run.error_category(),
        Some(account_domain::ErrorCategory::Auth)
    );
}

#[tokio::test]
async fn a_stale_credential_remains_usable_for_a_sync() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Exchange, "binance", "Main");
    let mut credential = seed_credential(&h, &account);
    credential.mark_stale(at(9));
    h.credentials.update(&credential).await.unwrap();
    h.exchange.set_balances(Ok(Vec::new()));
    let run = seed_run(&h, &account);
    let outcome = h.sync.process(run.id(), "corr").await.unwrap();
    assert!(matches!(
        outcome,
        account_application::sync::RunOutcome::Succeeded(_)
    ));
}

#[tokio::test]
async fn an_expired_lease_skips_the_run_without_a_snapshot() {
    let h = harness();
    let (_account, _credential, run) = seed_exchange_sync(
        &h,
        vec![ObservedBalance {
            observed_token: "BTC".to_string(),
            quantity: decimal("1"),
        }],
    );
    h.clock.advance(Duration::seconds(LEASE_SECONDS + 1));
    let outcome = h.sync.process(run.id(), "corr").await.unwrap();
    assert!(matches!(
        outcome,
        account_application::sync::RunOutcome::Skipped(_)
    ));
    assert!(h.runs.snapshots().is_empty());
}

#[tokio::test]
async fn renewing_extends_a_live_run_and_refuses_an_expired_one() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Exchange, "binance", "Main");
    let run = seed_run(&h, &account);
    h.sync.renew(run.id()).await.unwrap();
    let stored = h.runs.find_by_id(run.id()).await.unwrap().unwrap();
    assert_eq!(
        stored.lease_until(),
        at(10) + Duration::seconds(LEASE_SECONDS)
    );
    h.clock.advance(Duration::seconds(LEASE_SECONDS * 2));
    let error = h.sync.renew(run.id()).await.unwrap_err();
    assert!(matches!(error, AppError::Conflict(_)));
}

#[tokio::test]
async fn the_sweep_marks_an_expired_run_failed() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Exchange, "binance", "Main");
    let run = seed_run(&h, &account);
    h.clock.advance(Duration::seconds(LEASE_SECONDS + 1));
    let swept = h.sync.sweep().await.unwrap();
    assert_eq!(swept, 1);
    let stored = h.runs.find_by_id(run.id()).await.unwrap().unwrap();
    assert_eq!(stored.status(), account_domain::SyncRunStatus::Failed);
}

#[tokio::test]
async fn processing_pending_runs_reports_the_counts() {
    let h = harness();
    let (_account, _credential, _run) = seed_exchange_sync(&h, Vec::new());
    let report = h.sync.process_pending("corr").await.unwrap();
    assert_eq!(report.processed, 1);
    assert_eq!(report.failed, 0);
    assert_eq!(report.skipped, 0);
}

#[tokio::test]
async fn processing_pending_with_no_runs_reports_zero() {
    let h = harness();
    let report = h.sync.process_pending("corr").await.unwrap();
    assert_eq!(report.processed + report.skipped + report.failed, 0);
}

#[tokio::test]
async fn a_registry_outage_fails_the_run_rather_than_fabricating_holdings() {
    let h = harness();
    let (_account, _credential, run) = seed_exchange_sync(
        &h,
        vec![ObservedBalance {
            observed_token: "BTC".to_string(),
            quantity: decimal("1"),
        }],
    );
    h.registry.set_fail(true);
    h.sync.process(run.id(), "corr").await.unwrap();
    assert!(h.runs.snapshots().is_empty());
    let stored = h.runs.find_by_id(run.id()).await.unwrap().unwrap();
    assert_eq!(stored.status(), account_domain::SyncRunStatus::Failed);
    assert_eq!(
        stored.error_category(),
        Some(account_domain::ErrorCategory::Network)
    );
}

#[tokio::test]
async fn a_vault_reveal_failure_fails_the_run_as_auth() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Exchange, "binance", "Main");
    let credential = Credential::pending(
        CredentialId::generate(),
        account.id(),
        "vault://account/missing",
        at(9),
    )
    .unwrap();
    let mut credential = credential;
    credential.verify(at(9));
    h.credentials.seed(credential);
    let run = seed_run(&h, &account);
    h.sync.process(run.id(), "corr").await.unwrap();
    let stored = h.runs.find_by_id(run.id()).await.unwrap().unwrap();
    assert_eq!(
        stored.error_category(),
        Some(account_domain::ErrorCategory::Auth)
    );
}

#[tokio::test]
async fn an_unknown_run_is_not_found() {
    let h = harness();
    let error = h
        .sync
        .process(SyncRunId::generate(), "corr")
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::NotFound(_)));
}

#[tokio::test]
async fn a_wallet_without_a_connection_fails_the_run() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Wallet, "evm:1", "Wallet");
    let run = seed_run(&h, &account);
    h.sync.process(run.id(), "corr").await.unwrap();
    let stored = h.runs.find_by_id(run.id()).await.unwrap().unwrap();
    assert_eq!(
        stored.error_category(),
        Some(account_domain::ErrorCategory::Data)
    );
}

// ---------------------------------------------------------------------------
// Periodic verification (WF7)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn reverification_keeps_a_read_only_credential_verified() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Exchange, "binance", "Main");
    let credential = seed_credential(&h, &account);
    let report = h.verification.reverify_all("corr").await.unwrap();
    assert_eq!(report.verified, 1);
    assert_eq!(report.rejected, 0);
    let stored = h
        .credentials
        .find_by_id(credential.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.verification_status(), CredentialStatus::Verified);
    assert!(h.vault.destroyed().is_empty());
    assert!(h.notifications.sent().is_empty());
}

#[tokio::test]
async fn a_key_that_gained_trading_permission_is_revoked() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Exchange, "binance", "Main");
    let credential = seed_credential(&h, &account);
    h.exchange
        .set_verification(Ok(VerifiedPermissions { read_only: false }));

    let report = h.verification.reverify_all("corr").await.unwrap();
    assert_eq!(report.rejected, 1);
    let stored = h
        .credentials
        .find_by_id(credential.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.verification_status(), CredentialStatus::Rejected);
    assert_eq!(
        h.vault.destroyed(),
        vec![credential.vault_secret_ref().to_string()]
    );
    let stored_account = h.accounts.find_by_id(account.id()).await.unwrap().unwrap();
    assert_eq!(stored_account.status(), AccountStatus::Disconnected);
    assert_eq!(h.auth_admin.calls(), vec![(user_a(), false)]);
    assert_eq!(h.notifications.sent()[0].1, "key_revoked");
    let actions = h.audit.actions();
    assert!(actions.contains(&"account.credential.rejected".to_string()));
    assert!(actions.contains(&"account.disconnected".to_string()));
}

#[tokio::test]
async fn a_transient_verification_failure_marks_the_credential_stale() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Exchange, "binance", "Main");
    let credential = seed_credential(&h, &account);
    h.exchange
        .set_verification(Err(ExternalError::Unavailable("venue timeout".to_string())));
    let report = h.verification.reverify_all("corr").await.unwrap();
    assert_eq!(report.stale, 1);
    let stored = h
        .credentials
        .find_by_id(credential.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.verification_status(), CredentialStatus::Stale);
    assert!(h.vault.destroyed().is_empty());
    let stored_account = h.accounts.find_by_id(account.id()).await.unwrap().unwrap();
    assert_eq!(stored_account.status(), AccountStatus::Active);
}

#[tokio::test]
async fn a_refused_verification_revokes_the_credential() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Exchange, "binance", "Main");
    let credential = seed_credential(&h, &account);
    h.exchange.set_verification(Err(ExternalError::Refused(
        "the key is malformed".to_string(),
    )));
    let report = h.verification.reverify_all("corr").await.unwrap();
    assert_eq!(report.rejected, 1);
    let stored = h
        .credentials
        .find_by_id(credential.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.verification_status(), CredentialStatus::Rejected);
    assert_eq!(h.vault.destroyed().len(), 1);
}

#[tokio::test]
async fn reverification_reports_each_verified_credential() {
    let h = harness();
    for index in 0..2 {
        let account = seed_account(
            &h,
            AccountKind::Exchange,
            "binance",
            &format!("Main {index}"),
        );
        seed_credential(&h, &account);
    }
    let report = h.verification.reverify_all("corr").await.unwrap();
    assert_eq!(report.verified, 2);
}

// ---------------------------------------------------------------------------
// CSV imports (WF5)
// ---------------------------------------------------------------------------

const GENERIC_CSV: &str = include_str!("fixtures/csv/generic.csv");

fn csv_account(h: &Harness) -> Account {
    seed_account(h, AccountKind::Csv, "csv:summary", "History")
}

#[tokio::test]
async fn uploading_a_csv_stages_the_bytes_and_detects_the_format() {
    let h = harness();
    let account = csv_account(&h);
    let batch = h
        .import_service
        .upload(
            &caller(),
            account.id(),
            "trades.csv",
            GENERIC_CSV.as_bytes(),
        )
        .await
        .unwrap();
    assert_eq!(batch.source(), ImportSource::Generic);
    assert_eq!(batch.status(), account_domain::ImportBatchStatus::Pending);
    assert!(h.imports.has_staging(batch.id()));
}

#[tokio::test]
async fn an_unknown_csv_format_is_refused_before_staging() {
    let h = harness();
    let account = csv_account(&h);
    let error = h
        .import_service
        .upload(&caller(), account.id(), "trades.csv", b"date,amount,note\n")
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Validation(_)));
    assert!(h.imports.batches().is_empty());
}

#[tokio::test]
async fn a_csv_over_ten_mebibytes_is_refused() {
    let h = harness();
    let account = csv_account(&h);
    let oversized = vec![b'a'; CSV_MAX_BYTES + 1];
    let error = h
        .import_service
        .upload(&caller(), account.id(), "huge.csv", &oversized)
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Validation(_)));
    assert!(h.imports.batches().is_empty());
}

#[tokio::test]
async fn a_non_csv_account_cannot_take_an_import() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Exchange, "binance", "Main");
    let error = h
        .import_service
        .upload(
            &caller(),
            account.id(),
            "trades.csv",
            GENERIC_CSV.as_bytes(),
        )
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Validation(_)));
}

#[tokio::test]
async fn importing_for_another_user_is_forbidden() {
    let h = harness();
    let account = csv_account(&h);
    let error = h
        .import_service
        .upload(
            &other_caller(),
            account.id(),
            "trades.csv",
            GENERIC_CSV.as_bytes(),
        )
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Forbidden(_)));
}

#[tokio::test]
async fn processing_a_generic_csv_imports_rows_and_completes_the_batch() {
    let h = harness();
    let account = csv_account(&h);
    let batch = h
        .import_service
        .upload(
            &caller(),
            account.id(),
            "trades.csv",
            GENERIC_CSV.as_bytes(),
        )
        .await
        .unwrap();
    let summary = h.import_service.process(batch.id(), "corr").await.unwrap();
    assert_eq!(summary.imported, 2);
    assert_eq!(summary.skipped, 0);
    assert_eq!(summary.failed, 0);
    assert!(summary.reconciles(2));
    assert_eq!(h.imports.transactions().len(), 2);
    assert!(
        !h.imports.has_staging(batch.id()),
        "the staging row is deleted"
    );
    let stored = h.imports.find_batch(batch.id()).await.unwrap().unwrap();
    assert_eq!(stored.status(), account_domain::ImportBatchStatus::Complete);
    assert!(h
        .audit
        .actions()
        .contains(&"account.import.completed".to_string()));
}

#[tokio::test]
async fn a_duplicate_row_is_skipped_and_counted() {
    let h = harness();
    let account = csv_account(&h);
    let first = h
        .import_service
        .upload(
            &caller(),
            account.id(),
            "trades.csv",
            GENERIC_CSV.as_bytes(),
        )
        .await
        .unwrap();
    h.import_service.process(first.id(), "corr").await.unwrap();
    let second = h
        .import_service
        .upload(
            &caller(),
            account.id(),
            "trades.csv",
            GENERIC_CSV.as_bytes(),
        )
        .await
        .unwrap();
    let summary = h.import_service.process(second.id(), "corr").await.unwrap();
    assert_eq!(summary.imported, 0);
    assert_eq!(summary.skipped, 2);
    assert_eq!(h.imports.transactions().len(), 2);
}

#[tokio::test]
async fn a_bad_row_is_reported_with_its_line_number_and_later_rows_import() {
    let h = harness();
    let account = csv_account(&h);
    let csv = include_str!("fixtures/csv/malformed.csv");
    let batch = h
        .import_service
        .upload(&caller(), account.id(), "trades.csv", csv.as_bytes())
        .await
        .unwrap();
    let summary = h.import_service.process(batch.id(), "corr").await.unwrap();
    assert_eq!(summary.imported, 1);
    assert_eq!(summary.failed, 1);
    assert_eq!(summary.row_errors.len(), 1);
    assert_eq!(summary.row_errors[0].line_number(), 2);
    assert!(summary.reconciles(2));
    assert_eq!(h.imports.transactions().len(), 1);
}

#[tokio::test]
async fn a_binance_export_is_parsed_into_transactions() {
    let h = harness();
    let account = csv_account(&h);
    let csv = include_str!("fixtures/csv/binance.csv");
    let batch = h
        .import_service
        .upload(&caller(), account.id(), "binance.csv", csv.as_bytes())
        .await
        .unwrap();
    let summary = h.import_service.process(batch.id(), "corr").await.unwrap();
    assert_eq!(summary.imported, 2);
    let transactions = h.imports.transactions();
    assert_eq!(transactions[0].kind(), TransactionKind::Buy);
    assert_eq!(transactions[0].asset_id(), "BTC");
    assert_eq!(transactions[1].kind(), TransactionKind::Sell);
    assert_eq!(transactions[1].quantity(), decimal("2"));
}

#[tokio::test]
async fn an_okx_export_is_parsed_into_transactions() {
    let h = harness();
    let account = csv_account(&h);
    let csv = include_str!("fixtures/csv/okx.csv");
    let batch = h
        .import_service
        .upload(&caller(), account.id(), "okx.csv", csv.as_bytes())
        .await
        .unwrap();
    let summary = h.import_service.process(batch.id(), "corr").await.unwrap();
    assert_eq!(summary.imported, 2);
    assert_eq!(
        h.imports.transactions()[0].kind(),
        TransactionKind::TransferIn
    );
    assert_eq!(h.imports.transactions()[1].kind(), TransactionKind::Fee);
}

#[tokio::test]
async fn the_import_summary_reconciles_a_mixed_file() {
    let h = harness();
    let account = csv_account(&h);
    let csv = include_str!("fixtures/csv/duplicate.csv");
    let batch = h
        .import_service
        .upload(&caller(), account.id(), "mixed.csv", csv.as_bytes())
        .await
        .unwrap();
    let summary = h.import_service.process(batch.id(), "corr").await.unwrap();
    assert_eq!(summary.imported, 1);
    assert_eq!(summary.skipped, 1);
    assert_eq!(summary.failed, 1);
    assert!(summary.reconciles(3));
}

#[tokio::test]
async fn process_pending_picks_the_pending_batch() {
    let h = harness();
    let account = csv_account(&h);
    h.import_service
        .upload(
            &caller(),
            account.id(),
            "trades.csv",
            GENERIC_CSV.as_bytes(),
        )
        .await
        .unwrap();
    let summary = h
        .import_service
        .process_pending("corr")
        .await
        .unwrap()
        .expect("one pending batch");
    assert_eq!(summary.imported, 2);
    assert!(h.imports.batches()[0].completed_at().is_some());
}

#[tokio::test]
async fn a_batch_can_be_read_back_by_its_owner() {
    let h = harness();
    let account = csv_account(&h);
    let batch = h
        .import_service
        .upload(
            &caller(),
            account.id(),
            "trades.csv",
            GENERIC_CSV.as_bytes(),
        )
        .await
        .unwrap();
    let found = h.import_service.find(&caller(), batch.id()).await.unwrap();
    assert_eq!(found.id(), batch.id());
    let error = h
        .import_service
        .find(&other_caller(), batch.id())
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Forbidden(_)));
}

// ---------------------------------------------------------------------------
// C5 holdings and transactions
// ---------------------------------------------------------------------------

#[tokio::test]
async fn holdings_include_the_last_sync_time_and_the_spam_label() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Exchange, "binance", "Main");
    let mut synced = account.clone();
    synced.begin_sync(at(9)).unwrap();
    synced.finish_sync_success(at(10)).unwrap();
    h.accounts.update(&synced).await.unwrap();
    let snapshot = HoldingSnapshot::record(
        account_domain::HoldingSnapshotId::generate(),
        SyncRunId::generate(),
        account.id(),
        "DOGE",
        None,
        decimal("10"),
        at(10),
    )
    .unwrap();
    h.holding_store.snapshots.lock().unwrap().push(snapshot);

    let accounts = h
        .holding_service
        .for_user(&service("portfolio"), user_a())
        .await
        .unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].last_synced_at, Some(at(10)));
    assert!(accounts[0].holdings[0].spam());
}

#[tokio::test]
async fn holdings_read_is_refused_for_a_service_without_the_operation() {
    let h = harness();
    let error = h
        .holding_service
        .for_user(&service("identity"), user_a())
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Forbidden(_)));
}

#[tokio::test]
async fn transactions_return_imported_rows_since_a_time() {
    let h = harness();
    let account = csv_account(&h);
    let batch = h
        .import_service
        .upload(
            &caller(),
            account.id(),
            "trades.csv",
            GENERIC_CSV.as_bytes(),
        )
        .await
        .unwrap();
    h.import_service.process(batch.id(), "corr").await.unwrap();
    let early = Utc.with_ymd_and_hms(2026, 11, 1, 0, 0, 0).unwrap();
    let transactions = h
        .holding_service
        .transactions(&service("ledger"), account.id(), Some(early))
        .await
        .unwrap();
    assert_eq!(transactions.len(), 2);
    let none = h
        .holding_service
        .transactions(&service("ledger"), account.id(), Some(at(23)))
        .await
        .unwrap();
    assert!(none.is_empty());
}

#[tokio::test]
async fn transactions_read_is_refused_for_a_service_without_the_operation() {
    let h = harness();
    let error = h
        .holding_service
        .transactions(&service("identity"), AccountId::generate(), None)
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Forbidden(_)));
}

// ---------------------------------------------------------------------------
// Privacy export and anonymise (WF8)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_export_returns_accounts_wallets_holdings_and_transactions() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Wallet, "evm:1", "Wallet");
    seed_connection(&h, &account);
    let snapshot = HoldingSnapshot::record(
        account_domain::HoldingSnapshotId::generate(),
        SyncRunId::generate(),
        account.id(),
        "ETH",
        Some("asset-eth"),
        decimal("1"),
        at(10),
    )
    .unwrap();
    h.holding_store.snapshots.lock().unwrap().push(snapshot);

    let export = h
        .privacy_service
        .export(&service("identity"), user_a())
        .await
        .unwrap();
    assert_eq!(export.user_id, user_a());
    assert_eq!(export.accounts.len(), 1);
    assert_eq!(export.wallets.len(), 1);
    assert_eq!(export.holdings.len(), 1);
}

#[tokio::test]
async fn the_export_is_refused_for_a_service_without_the_operation() {
    let h = harness();
    let error = h
        .privacy_service
        .export(&service("portfolio"), user_a())
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Forbidden(_)));
}

#[tokio::test]
async fn anonymising_removes_labels_and_addresses_and_destroys_secrets() {
    let h = harness();
    let account = seed_account(&h, AccountKind::Exchange, "binance", "Main");
    let credential = seed_credential(&h, &account);
    let connection_account = seed_account(&h, AccountKind::Wallet, "evm:1", "Wallet");
    seed_connection(&h, &connection_account);

    let result = h
        .privacy_service
        .anonymise(&service("identity"), user_a(), "idem-1")
        .await
        .unwrap();
    assert!(!result.already_applied);
    assert!(result.retained_categories.contains(&"holdings".to_string()));
    let stored = h.accounts.find_by_id(account.id()).await.unwrap().unwrap();
    assert!(stored.label().starts_with("anonymised-"));
    assert!(h.wallets.all().is_empty());
    assert_eq!(
        h.vault.destroyed(),
        vec![credential.vault_secret_ref().to_string()]
    );
    assert!(h.credentials.all().is_empty());
    assert!(h
        .audit
        .actions()
        .contains(&"account.privacy.anonymised".to_string()));
}

#[tokio::test]
async fn anonymising_twice_returns_the_recorded_result() {
    let h = harness();
    seed_account(&h, AccountKind::Exchange, "binance", "Main");
    h.privacy_service
        .anonymise(&service("identity"), user_a(), "idem-1")
        .await
        .unwrap();
    let second = h
        .privacy_service
        .anonymise(&service("identity"), user_a(), "idem-2")
        .await
        .unwrap();
    assert!(second.already_applied);
    assert!(h
        .privacy
        .find_anonymisation(user_a())
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn anonymising_requires_an_idempotency_key() {
    let h = harness();
    let error = h
        .privacy_service
        .anonymise(&service("identity"), user_a(), "  ")
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Validation(_)));
}

#[tokio::test]
async fn anonymising_is_refused_for_a_service_without_the_operation() {
    let h = harness();
    let error = h
        .privacy_service
        .anonymise(&service("ledger"), user_a(), "idem-1")
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Forbidden(_)));
}

#[tokio::test]
async fn a_verified_claims_pair_builds_a_caller() {
    let claims = VerifiedClaims {
        user_id: user_a(),
        assurance: AssuranceLevel::Aal2,
    };
    let caller = Caller::from_claims(&claims, "corr");
    assert_eq!(caller.user_id, user_a());
    assert!(caller.require_mfa_for_exchange().is_ok());
    let domain = DomainError::SecretLike { field: "apiKey" };
    assert!(matches!(
        AppError::from_domain(domain),
        AppError::Validation(_)
    ));
}
