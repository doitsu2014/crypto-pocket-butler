//! WF1–WF3: account onboarding, credential replacement, and manual re-sync
//! (BR1.1–BR1.4, BR2.1–BR2.5, BR3.1–BR3.3, BR4.1).
//!
//! `support_metadata` returns the closed account projection for a support grant
//! (BR3.9): an id, label, source type, sync status and last-sync time.

use std::sync::Arc;

use account_domain::{
    ensure_not_secret, validate_label, Account, AccountId, AccountKind, AuditOutcome, Credential,
    CredentialId, Operation, SyncRun, SyncRunId, UserId, Venue, WalletConnection,
    WalletConnectionId, LEASE_SECONDS,
};

use crate::common::{AppError, Caller, FieldError, ServiceCaller};
use crate::ports::{
    AccountRepository, AssetRegistry, AuthAdmin, Clock, CredentialRepository, ExchangeCredentials,
    ExchangeProvider, RepositoryError, SyncRunRepository, VaultClient, WakeTransport,
    WalletConnectionRepository,
};
use crate::trail::{context, Trail};

/// A read-only exchange key submitted by the user. It is never logged and is
/// stored only inside Vault.
pub type ExchangeKey = ExchangeCredentials;

/// The result of a manual re-sync request.
#[derive(Debug, Clone)]
pub enum ResyncOutcome {
    /// A new run was claimed.
    Started(SyncRun),
    /// A live run already owns the account (C1 returns 202 with its id).
    AlreadyRunning(SyncRun),
}

/// One account's allowlisted support-metadata projection (BR3.9). It carries no
/// venue, provider account identifier, credential reference, or holding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportAccountSummary {
    /// The account identifier.
    pub id: AccountId,
    /// The account label.
    pub label: String,
    /// `exchange`, `wallet`, `walletconnect`, or `csv`.
    pub source_type: String,
    /// `active`, `syncing`, `error`, or `disconnected`.
    pub sync_status: String,
    /// When the account last synced; null when it never has.
    pub last_synced_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Onboards accounts, replaces credentials, and starts manual runs.
pub struct AccountService {
    accounts: Arc<dyn AccountRepository>,
    credentials: Arc<dyn CredentialRepository>,
    wallets: Arc<dyn WalletConnectionRepository>,
    runs: Arc<dyn SyncRunRepository>,
    exchange: Arc<dyn ExchangeProvider>,
    registry: Arc<dyn AssetRegistry>,
    vault: Arc<dyn VaultClient>,
    auth_admin: Arc<dyn AuthAdmin>,
    wake: Arc<dyn WakeTransport>,
    trail: Arc<Trail>,
    clock: Arc<dyn Clock>,
}

impl AccountService {
    /// Creates the use case.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        accounts: Arc<dyn AccountRepository>,
        credentials: Arc<dyn CredentialRepository>,
        wallets: Arc<dyn WalletConnectionRepository>,
        runs: Arc<dyn SyncRunRepository>,
        exchange: Arc<dyn ExchangeProvider>,
        registry: Arc<dyn AssetRegistry>,
        vault: Arc<dyn VaultClient>,
        auth_admin: Arc<dyn AuthAdmin>,
        wake: Arc<dyn WakeTransport>,
        trail: Arc<Trail>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            accounts,
            credentials,
            wallets,
            runs,
            exchange,
            registry,
            vault,
            auth_admin,
            wake,
            trail,
            clock,
        }
    }

    /// WF1: adds an exchange account. The key is verified before anything is
    /// created; a rejected key leaves no account, credential, or secret (BR1.4).
    ///
    /// # Errors
    /// Returns a validation refusal for a bad label or key, a forbidden refusal
    /// below AAL2 or for a key with trading permission, and an unavailable
    /// refusal when the provider cannot be reached.
    pub async fn add_exchange(
        &self,
        caller: &Caller,
        venue: &str,
        label: &str,
        key: ExchangeKey,
    ) -> Result<Account, AppError> {
        caller.require_mfa_for_exchange()?;
        let venue =
            Venue::parse_for(AccountKind::Exchange, venue).map_err(AppError::from_domain)?;
        let label = validate_label(label).map_err(AppError::from_domain)?;
        ensure_not_secret("label", &label).map_err(AppError::from_domain)?;
        ensure_not_secret("apiKey", &key.api_key).map_err(AppError::from_domain)?;
        ensure_not_secret("apiSecret", &key.api_secret).map_err(AppError::from_domain)?;
        if let Some(passphrase) = key.passphrase.as_deref() {
            ensure_not_secret("passphrase", passphrase).map_err(AppError::from_domain)?;
        }
        self.ensure_label_free(caller.user_id, &label, None).await?;

        let now = self.clock.now();
        let mut secret_fields = serde_json::Map::new();
        secret_fields.insert("apiKey".to_string(), key.api_key.clone().into());
        secret_fields.insert("apiSecret".to_string(), key.api_secret.clone().into());
        if let Some(passphrase) = key.passphrase.clone() {
            secret_fields.insert("passphrase".to_string(), passphrase.into());
        }
        let secret = serde_json::Value::Object(secret_fields).to_string();
        let reference = self
            .vault
            .store(&format!("account:{}", caller.user_id), &secret)
            .await
            .map_err(AppError::from_external)?;

        let permissions = match self.exchange.verify_read_only(venue.as_str(), &key).await {
            Ok(permissions) => permissions,
            Err(error) => {
                let _ = self.vault.destroy(&reference).await;
                return Err(AppError::from_external(error));
            }
        };
        if !permissions.read_only {
            let _ = self.vault.destroy(&reference).await;
            return Err(AppError::Forbidden(
                "the key carries trading or withdrawal permission".to_string(),
            ));
        }

        let account = Account::open(
            AccountId::generate(),
            caller.user_id,
            AccountKind::Exchange,
            venue.as_str(),
            &label,
            now,
        )
        .map_err(AppError::from_domain)?;
        if let Err(error) = self.accounts.insert(&account).await {
            let _ = self.vault.destroy(&reference).await;
            return Err(AppError::from_repository(error));
        }
        let mut credential =
            Credential::pending(CredentialId::generate(), account.id(), &reference, now)
                .map_err(AppError::from_domain)?;
        credential.verify(now);
        if let Err(error) = self.credentials.insert(&credential).await {
            let _ = self.vault.destroy(&reference).await;
            return Err(AppError::from_repository(error));
        }

        self.start_first_run(&account, now).await;
        self.maintain_claim(caller.user_id, true).await;
        let account_label = account.id().to_string();
        self.trail
            .record(
                Some(caller.user_id),
                "account.added",
                account_label.clone(),
                AuditOutcome::Success,
                &caller.correlation_id,
                context(&[("kind", "exchange"), ("venue", venue.as_str())]),
            )
            .await?;
        self.trail
            .record(
                Some(caller.user_id),
                "account.credential.added",
                account_label,
                AuditOutcome::Success,
                &caller.correlation_id,
                context(&[("accountId", &account.id().to_string())]),
            )
            .await?;
        Ok(account)
    }

    /// WF1 edge: verifies a replacement key, then swaps it in and destroys the
    /// old secret (BR2.2). A failed replacement leaves the old credential usable.
    ///
    /// # Errors
    /// Returns the ownership, validation, or provider refusal.
    pub async fn replace_key(
        &self,
        caller: &Caller,
        account_id: AccountId,
        key: ExchangeKey,
    ) -> Result<Credential, AppError> {
        caller.require_mfa_for_exchange()?;
        let account = self.load_owned(caller, account_id).await?;
        if account.kind() != AccountKind::Exchange {
            return Err(AppError::Validation(vec![FieldError::new(
                "accountId",
                "is not an exchange account",
            )]));
        }
        ensure_not_secret("apiKey", &key.api_key).map_err(AppError::from_domain)?;
        ensure_not_secret("apiSecret", &key.api_secret).map_err(AppError::from_domain)?;
        if let Some(passphrase) = key.passphrase.as_deref() {
            ensure_not_secret("passphrase", passphrase).map_err(AppError::from_domain)?;
        }
        let mut credential = self
            .credentials
            .find_for_account(account_id)
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("credential"))?;
        let exchange = account
            .venue()
            .exchange()
            .ok_or(AppError::NotFound("exchange venue"))?
            .to_string();

        let now = self.clock.now();
        let old_reference = credential.vault_secret_ref().to_string();
        let candidate = self
            .vault
            .store(&format!("account:{}", caller.user_id), &secret_json(&key))
            .await
            .map_err(AppError::from_external)?;

        let permissions = match self.exchange.verify_read_only(&exchange, &key).await {
            Ok(permissions) => permissions,
            Err(error) => {
                let _ = self.vault.destroy(&candidate).await;
                return Err(AppError::from_external(error));
            }
        };
        if !permissions.read_only {
            let _ = self.vault.destroy(&candidate).await;
            return Err(AppError::Forbidden(
                "the key carries trading or withdrawal permission".to_string(),
            ));
        }

        credential
            .replace_reference(&candidate, now)
            .map_err(AppError::from_domain)?;
        self.credentials
            .update(&credential)
            .await
            .map_err(AppError::from_repository)?;
        if let Err(error) = self.vault.destroy(&old_reference).await {
            tracing::warn!(%error, "the old credential secret could not be destroyed");
        }

        if account.status() == account_domain::AccountStatus::Disconnected {
            let mut account = account;
            account.reactivate(now).map_err(AppError::from_domain)?;
            self.accounts
                .update(&account)
                .await
                .map_err(AppError::from_repository)?;
            self.start_first_run(&account, now).await;
        }
        self.maintain_claim(caller.user_id, true).await;
        self.trail
            .record(
                Some(caller.user_id),
                "account.credential.replaced",
                account_id.to_string(),
                AuditOutcome::Success,
                &caller.correlation_id,
                context(&[("accountId", &account_id.to_string())]),
            )
            .await?;
        Ok(credential)
    }

    /// WF2: adds a public wallet address after checking the chain against C4 and
    /// validating the address shape (BR3.1, BR3.2).
    ///
    /// # Errors
    /// Returns a validation refusal for an unknown chain, a malformed address, or
    /// a duplicate label.
    pub async fn add_wallet(
        &self,
        caller: &Caller,
        chain: &str,
        address: &str,
        label: &str,
    ) -> Result<Account, AppError> {
        self.ensure_chain_supported(chain).await?;
        let label = validate_label(label).map_err(AppError::from_domain)?;
        ensure_not_secret("label", &label).map_err(AppError::from_domain)?;
        self.ensure_label_free(caller.user_id, &label, None).await?;
        let now = self.clock.now();
        let account = Account::open(
            AccountId::generate(),
            caller.user_id,
            AccountKind::Wallet,
            chain,
            &label,
            now,
        )
        .map_err(AppError::from_domain)?;
        let connection = WalletConnection::new_address(
            WalletConnectionId::generate(),
            account.id(),
            chain,
            address,
            now,
        )
        .map_err(AppError::from_domain)?;

        self.accounts
            .insert(&account)
            .await
            .map_err(AppError::from_repository)?;
        self.wallets
            .insert(&connection)
            .await
            .map_err(AppError::from_repository)?;
        self.start_first_run(&account, now).await;
        self.trail
            .record(
                Some(caller.user_id),
                "account.added",
                account.id().to_string(),
                AuditOutcome::Success,
                &caller.correlation_id,
                context(&[("kind", "wallet"), ("venue", chain)]),
            )
            .await?;
        Ok(account)
    }

    /// WF3: attaches a WalletConnect read-only connection. Only the verified
    /// address and an opaque session reference are stored (BR3.3).
    ///
    /// # Errors
    /// Returns a validation refusal for an unknown chain, a malformed address, a
    /// missing session reference, or a duplicate label.
    pub async fn add_walletconnect(
        &self,
        caller: &Caller,
        chain: &str,
        address: &str,
        label: &str,
        session_ref: &str,
    ) -> Result<Account, AppError> {
        self.ensure_chain_supported(chain).await?;
        let label = validate_label(label).map_err(AppError::from_domain)?;
        ensure_not_secret("label", &label).map_err(AppError::from_domain)?;
        ensure_not_secret("sessionRef", session_ref).map_err(AppError::from_domain)?;
        self.ensure_label_free(caller.user_id, &label, None).await?;
        let now = self.clock.now();
        let account = Account::open(
            AccountId::generate(),
            caller.user_id,
            AccountKind::WalletConnect,
            chain,
            &label,
            now,
        )
        .map_err(AppError::from_domain)?;
        let connection = WalletConnection::walletconnect(
            WalletConnectionId::generate(),
            account.id(),
            chain,
            address,
            session_ref,
            now,
        )
        .map_err(AppError::from_domain)?;
        self.accounts
            .insert(&account)
            .await
            .map_err(AppError::from_repository)?;
        self.wallets
            .insert(&connection)
            .await
            .map_err(AppError::from_repository)?;
        self.start_first_run(&account, now).await;
        self.trail
            .record(
                Some(caller.user_id),
                "account.added",
                account.id().to_string(),
                AuditOutcome::Success,
                &caller.correlation_id,
                context(&[("kind", "walletconnect"), ("venue", chain)]),
            )
            .await?;
        Ok(account)
    }

    /// Lists the user's accounts with status and last-synced time.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] when the caller is not the owner.
    pub async fn list(&self, caller: &Caller, user_id: UserId) -> Result<Vec<Account>, AppError> {
        caller.require_owner(user_id)?;
        self.accounts
            .find_for_user(user_id)
            .await
            .map_err(AppError::from_repository)
    }

    /// C1 manual re-sync. A live run returns its id; otherwise a new run is
    /// claimed (BR4.1).
    ///
    /// # Errors
    /// Returns a forbidden refusal for another user's account, a validation
    /// refusal for a CSV account, and a conflict when the run cannot be claimed.
    pub async fn resync(
        &self,
        caller: &Caller,
        account_id: AccountId,
    ) -> Result<ResyncOutcome, AppError> {
        let account = self.load_owned(caller, account_id).await?;
        if account.is_csv() {
            return Err(AppError::Validation(vec![FieldError::new(
                "accountId",
                "is a CSV history account and has no holdings sync",
            )]));
        }
        let now = self.clock.now();
        if let Some(run) = self
            .runs
            .running_for_account(account_id)
            .await
            .map_err(AppError::from_repository)?
        {
            if run.is_live(now) {
                return Ok(ResyncOutcome::AlreadyRunning(run));
            }
        }
        self.runs
            .sweep_expired(now)
            .await
            .map_err(AppError::from_repository)?;
        let run = SyncRun::claim(SyncRunId::generate(), account_id, now, LEASE_SECONDS);
        self.runs
            .claim(&run)
            .await
            .map_err(AppError::from_repository)?;
        self.wake_worker().await;
        self.trail
            .record(
                Some(caller.user_id),
                "account.resync.requested",
                account_id.to_string(),
                AuditOutcome::Success,
                &caller.correlation_id,
                context(&[
                    ("accountId", &account_id.to_string()),
                    ("runId", &run.id().to_string()),
                ]),
            )
            .await?;
        Ok(ResyncOutcome::Started(run))
    }

    async fn load_owned(
        &self,
        caller: &Caller,
        account_id: AccountId,
    ) -> Result<Account, AppError> {
        let account = self
            .accounts
            .find_by_id(account_id)
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("account"))?;
        caller.require_owner(account.user_id())?;
        Ok(account)
    }

    async fn ensure_label_free(
        &self,
        user_id: UserId,
        label: &str,
        except: Option<AccountId>,
    ) -> Result<(), AppError> {
        if self
            .accounts
            .label_exists(user_id, label, except)
            .await
            .map_err(AppError::from_repository)?
        {
            return Err(AppError::Validation(vec![FieldError::new(
                "label",
                "is already used by another account",
            )]));
        }
        Ok(())
    }

    async fn ensure_chain_supported(&self, chain: &str) -> Result<(), AppError> {
        let supported = self
            .registry
            .supported_chains()
            .await
            .map_err(AppError::from_external)?;
        if supported
            .iter()
            .any(|candidate| chain_matches(chain, &candidate.id))
        {
            return Ok(());
        }
        let list = supported
            .iter()
            .map(|candidate| candidate.id.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        Err(AppError::Validation(vec![FieldError::new(
            "chain",
            format!("is not supported; supported chains are {list}"),
        )]))
    }

    async fn start_first_run(&self, account: &Account, now: chrono::DateTime<chrono::Utc>) {
        let run = SyncRun::claim(SyncRunId::generate(), account.id(), now, LEASE_SECONDS);
        if let Err(error) = self.runs.claim(&run).await {
            if !matches!(error, RepositoryError::Conflict(_)) {
                tracing::warn!(%error, "the first sync run could not be claimed");
            }
        }
        self.wake_worker().await;
    }

    async fn wake_worker(&self) {
        if let Err(error) = self.wake.wake().await {
            tracing::warn!(%error, "the sync worker could not be woken; the sweep will retry");
        }
    }

    async fn maintain_claim(&self, user_id: UserId, value: bool) {
        if let Err(error) = self
            .auth_admin
            .set_has_exchange_credentials(user_id, value)
            .await
        {
            tracing::warn!(
                %error,
                "the has_exchange_credentials claim could not be updated; the next credential event corrects it"
            );
        }
    }

    /// Reads the allowlisted support-metadata projection for one user (BR3.9).
    /// The caller must be the Identity service carrying `support_metadata`;
    /// the owner answers from its own store and returns no venue, provider
    /// account identifier, credential, or holding.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] when the caller may not call the
    /// operation and [`AppError::Internal`] when the store fails.
    pub async fn support_metadata(
        &self,
        caller: &ServiceCaller,
        user_id: UserId,
    ) -> Result<Vec<SupportAccountSummary>, AppError> {
        caller.require_operation(Operation::SupportMetadata)?;
        let accounts = self
            .accounts
            .find_for_user(user_id)
            .await
            .map_err(AppError::from_repository)?;
        Ok(accounts
            .iter()
            .map(|account| SupportAccountSummary {
                id: account.id(),
                label: account.label().to_string(),
                source_type: account.kind().as_str().to_string(),
                sync_status: account.status().as_str().to_string(),
                last_synced_at: account.last_synced_at(),
            })
            .collect())
    }
}

fn secret_json(key: &ExchangeKey) -> String {
    serde_json::json!({
        "apiKey": key.api_key,
        "apiSecret": key.api_secret,
        "passphrase": key.passphrase,
    })
    .to_string()
}

fn chain_matches(chain: &str, supported: &str) -> bool {
    if chain.eq_ignore_ascii_case(supported) {
        return true;
    }
    match chain.strip_prefix("evm:") {
        Some(chain_id) => supported == chain_id || supported.strip_prefix("evm:") == Some(chain_id),
        None => false,
    }
}
