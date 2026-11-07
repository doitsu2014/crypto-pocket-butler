//! WF4: the sync run, its lease, the fenced snapshot write, and the failure
//! path that keeps the previous holdings (BR4.1–BR4.4, NFR reliability).

use std::sync::Arc;

use account_domain::{
    Account, AccountKind, AuditOutcome, ErrorCategory, HoldingSnapshot, HoldingSnapshotId, SyncRun,
    SyncRunId, LEASE_SECONDS,
};

use crate::common::AppError;
use crate::ports::NotificationClient;
use crate::ports::{
    AccountRepository, AssetRegistry, ChainProvider, Clock, CredentialRepository, ExchangeProvider,
    Observation, ObservedBalance, RepositoryError, SyncRunRepository, VaultClient,
    WalletConnectionRepository,
};
use crate::trail::{context, Trail};

/// What one run produced.
#[derive(Debug, Clone)]
pub enum RunOutcome {
    /// The run wrote a snapshot.
    Succeeded(SyncRun),
    /// The run failed and kept the previous holdings.
    Failed(SyncRun),
    /// The run could not proceed (lease expired or superseded).
    Skipped(&'static str),
}

/// What one supervisor pass processed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RunReport {
    /// Runs attempted (succeeded or failed).
    pub processed: usize,
    /// Runs skipped because their lease was gone.
    pub skipped: usize,
    /// Runs that failed.
    pub failed: usize,
}

/// Claims, processes, renews, and sweeps sync runs.
pub struct SyncService {
    accounts: Arc<dyn AccountRepository>,
    credentials: Arc<dyn CredentialRepository>,
    wallets: Arc<dyn WalletConnectionRepository>,
    runs: Arc<dyn SyncRunRepository>,
    exchange: Arc<dyn ExchangeProvider>,
    chain: Arc<dyn ChainProvider>,
    registry: Arc<dyn AssetRegistry>,
    vault: Arc<dyn VaultClient>,
    notifier: Arc<dyn NotificationClient>,
    trail: Arc<Trail>,
    clock: Arc<dyn Clock>,
}

impl SyncService {
    /// Creates the use case.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        accounts: Arc<dyn AccountRepository>,
        credentials: Arc<dyn CredentialRepository>,
        wallets: Arc<dyn WalletConnectionRepository>,
        runs: Arc<dyn SyncRunRepository>,
        exchange: Arc<dyn ExchangeProvider>,
        chain: Arc<dyn ChainProvider>,
        registry: Arc<dyn AssetRegistry>,
        vault: Arc<dyn VaultClient>,
        notifier: Arc<dyn NotificationClient>,
        trail: Arc<Trail>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            accounts,
            credentials,
            wallets,
            runs,
            exchange,
            chain,
            registry,
            vault,
            notifier,
            trail,
            clock,
        }
    }

    /// Claims and processes one account's run immediately (the C11 path for a
    /// named account). Returns [`AppError::Conflict`] while a live lease exists.
    ///
    /// # Errors
    /// Returns [`AppError::NotFound`] for an unknown account,
    /// [`AppError::Validation`] for a CSV account, and [`AppError::Conflict`]
    /// while another run owns the lease.
    pub async fn run_account(
        &self,
        account_id: account_domain::AccountId,
        correlation_id: &str,
    ) -> Result<RunOutcome, AppError> {
        let account = self
            .accounts
            .find_by_id(account_id)
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("account"))?;
        if !account.is_syncable() {
            return Err(AppError::Validation(vec![crate::common::FieldError::new(
                "accountId",
                "is not a syncable account",
            )]));
        }
        let now = self.clock.now();
        let run = SyncRun::claim(SyncRunId::generate(), account_id, now, LEASE_SECONDS);
        self.runs
            .claim(&run)
            .await
            .map_err(AppError::from_repository)?;
        self.process(run.id(), correlation_id).await
    }

    /// Processes one claimed run.
    ///
    /// # Errors
    /// Returns [`AppError::NotFound`] for an unknown run and a repository refusal
    /// when the run or account cannot be read or written.
    pub async fn process(
        &self,
        run_id: SyncRunId,
        correlation_id: &str,
    ) -> Result<RunOutcome, AppError> {
        let Some(mut run) = self
            .runs
            .find_by_id(run_id)
            .await
            .map_err(AppError::from_repository)?
        else {
            return Err(AppError::NotFound("sync run"));
        };
        let now = self.clock.now();
        if !run.is_live(now) {
            return Ok(RunOutcome::Skipped("the run lease expired"));
        }
        let Some(mut account) = self
            .accounts
            .find_by_id(run.account_id())
            .await
            .map_err(AppError::from_repository)?
        else {
            run.fail(now, ErrorCategory::Data, "the account no longer exists")
                .map_err(AppError::from_domain)?;
            self.runs
                .finish(&run)
                .await
                .map_err(AppError::from_repository)?;
            return Ok(RunOutcome::Failed(run));
        };
        if !account.is_syncable() {
            run.fail(now, ErrorCategory::Data, "the account is not syncable")
                .map_err(AppError::from_domain)?;
            self.runs
                .finish(&run)
                .await
                .map_err(AppError::from_repository)?;
            return Ok(RunOutcome::Failed(run));
        }
        if let Err(error) = account.begin_sync(now) {
            return self
                .fail_run(
                    &mut run,
                    &mut account,
                    ErrorCategory::Auth,
                    &error.to_string(),
                    correlation_id,
                )
                .await;
        }
        self.accounts
            .update(&account)
            .await
            .map_err(AppError::from_repository)?;

        let balances = match self.balances(&account).await {
            Ok(balances) => balances,
            Err((category, message)) => {
                return self
                    .fail_run(&mut run, &mut account, category, &message, correlation_id)
                    .await;
            }
        };

        let mut holdings = Vec::with_capacity(balances.len());
        for balance in balances {
            let observation = observation_for(&account, &balance.observed_token);
            let asset = match self.registry.resolve(&observation).await {
                Ok(asset) => asset,
                Err(error) => {
                    return self
                        .fail_run(
                            &mut run,
                            &mut account,
                            ErrorCategory::Network,
                            &error.to_string(),
                            correlation_id,
                        )
                        .await;
                }
            };
            match HoldingSnapshot::record(
                HoldingSnapshotId::generate(),
                run.id(),
                account.id(),
                &balance.observed_token,
                asset.as_deref(),
                balance.quantity,
                now,
            ) {
                Ok(snapshot) => holdings.push(snapshot),
                Err(error) => {
                    return self
                        .fail_run(
                            &mut run,
                            &mut account,
                            ErrorCategory::Data,
                            &error.to_string(),
                            correlation_id,
                        )
                        .await;
                }
            }
        }

        match self.runs.write_snapshot(&run, &holdings, now, now).await {
            Ok(()) => {}
            Err(RepositoryError::Conflict(_)) => {
                return Ok(RunOutcome::Skipped("the run lease was superseded"));
            }
            Err(error) => return Err(AppError::from_repository(error)),
        }
        run.succeed(now).map_err(AppError::from_domain)?;
        self.runs
            .finish(&run)
            .await
            .map_err(AppError::from_repository)?;
        account
            .finish_sync_success(now)
            .map_err(AppError::from_domain)?;
        self.accounts
            .update(&account)
            .await
            .map_err(AppError::from_repository)?;

        self.trail
            .record(
                Some(account.user_id()),
                "account.sync.succeeded",
                account.id().to_string(),
                AuditOutcome::Success,
                correlation_id,
                context(&[
                    ("accountId", &account.id().to_string()),
                    ("runId", &run.id().to_string()),
                    ("holdings", &holdings.len().to_string()),
                ]),
            )
            .await?;
        Ok(RunOutcome::Succeeded(run))
    }

    /// Marks every expired run failed, then processes every live run.
    ///
    /// # Errors
    /// Returns a repository refusal when the run list cannot be read.
    pub async fn process_pending(&self, correlation_id: &str) -> Result<RunReport, AppError> {
        let _ = self.sweep().await?;
        let runs = self
            .runs
            .list_running(self.clock.now())
            .await
            .map_err(AppError::from_repository)?;
        let mut report = RunReport::default();
        for run in runs {
            match self.process(run.id(), correlation_id).await? {
                RunOutcome::Succeeded(_) => report.processed += 1,
                RunOutcome::Failed(_) => {
                    report.processed += 1;
                    report.failed += 1;
                }
                RunOutcome::Skipped(_) => report.skipped += 1,
            }
        }
        Ok(report)
    }

    /// Marks expired runs failed (BR4.2). Returns how many were swept.
    ///
    /// # Errors
    /// Returns a repository refusal when the sweep cannot run.
    pub async fn sweep(&self) -> Result<usize, AppError> {
        let swept = self
            .runs
            .sweep_expired(self.clock.now())
            .await
            .map_err(AppError::from_repository)?;
        Ok(swept.len())
    }

    /// Renews a live lease (the worker calls this every 20 seconds).
    ///
    /// # Errors
    /// Returns [`AppError::NotFound`] for an unknown run and
    /// [`AppError::Conflict`] when the lease is no longer live.
    pub async fn renew(&self, run_id: SyncRunId) -> Result<(), AppError> {
        let now = self.clock.now();
        let run = self
            .runs
            .find_by_id(run_id)
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("sync run"))?;
        if !run.is_live(now) {
            return Err(AppError::Conflict(
                "the run lease has expired; the worker must stop".to_string(),
            ));
        }
        let renewed = self
            .runs
            .renew(run_id, now + chrono::Duration::seconds(LEASE_SECONDS), now)
            .await
            .map_err(AppError::from_repository)?;
        if renewed {
            Ok(())
        } else {
            Err(AppError::Conflict(
                "the run lease was superseded".to_string(),
            ))
        }
    }

    async fn balances(
        &self,
        account: &Account,
    ) -> Result<Vec<ObservedBalance>, (ErrorCategory, String)> {
        match account.kind() {
            AccountKind::Exchange => {
                let credential = match self.credentials.find_for_account(account.id()).await {
                    Ok(Some(credential)) if credential.is_usable() => credential,
                    Ok(_) => {
                        return Err((
                            ErrorCategory::Auth,
                            "the account has no usable credential".to_string(),
                        ));
                    }
                    Err(error) => {
                        return Err((ErrorCategory::Data, error.to_string()));
                    }
                };
                let credentials = self
                    .vault
                    .reveal(credential.vault_secret_ref())
                    .await
                    .map_err(|error| (ErrorCategory::Auth, error.to_string()))
                    .and_then(|secret| {
                        parse_secret(&secret).ok_or_else(|| {
                            (
                                ErrorCategory::Auth,
                                "the stored credential is unreadable".to_string(),
                            )
                        })
                    })?;
                let venue = account.venue().exchange().unwrap_or_default();
                self.exchange
                    .balances(venue, &credentials)
                    .await
                    .map_err(classify_balance_error)
            }
            AccountKind::Wallet | AccountKind::WalletConnect => {
                let connection = match self.wallets.find_for_account(account.id()).await {
                    Ok(Some(connection)) => connection,
                    Ok(None) => {
                        return Err((
                            ErrorCategory::Data,
                            "the wallet has no connection".to_string(),
                        ));
                    }
                    Err(error) => {
                        return Err((ErrorCategory::Data, error.to_string()));
                    }
                };
                let chain = account
                    .venue()
                    .chain()
                    .unwrap_or_else(|| account.venue().as_str());
                self.chain
                    .balances(chain, connection.address())
                    .await
                    .map_err(classify_balance_error)
            }
            AccountKind::Csv => Err((
                ErrorCategory::Data,
                "a CSV account has no holdings sync".to_string(),
            )),
        }
    }

    async fn fail_run(
        &self,
        run: &mut SyncRun,
        account: &mut Account,
        category: ErrorCategory,
        message: &str,
        correlation_id: &str,
    ) -> Result<RunOutcome, AppError> {
        let now = self.clock.now();
        run.fail(now, category, message)
            .map_err(AppError::from_domain)?;
        self.runs
            .finish(run)
            .await
            .map_err(AppError::from_repository)?;
        let _ = account.finish_sync_failure(now);
        if let Err(error) = self.accounts.update(account).await {
            tracing::warn!(%error, "the failed account status could not be stored");
        }
        self.notify_failure(account, run).await;
        if let Err(error) = self
            .trail
            .record(
                Some(account.user_id()),
                "account.sync.failed",
                account.id().to_string(),
                AuditOutcome::Failed,
                correlation_id,
                context(&[
                    ("accountId", &account.id().to_string()),
                    ("runId", &run.id().to_string()),
                    ("category", category.as_str()),
                ]),
            )
            .await
        {
            tracing::warn!(%error, "the sync-failure audit event could not be recorded");
        }
        Ok(RunOutcome::Failed(run.clone()))
    }

    async fn notify_failure(&self, account: &Account, run: &SyncRun) {
        let idempotency = format!("sync_failed:{}:{}", account.id(), run.id());
        let payload = serde_json::json!({
            "accountId": account.id().to_string(),
            "runId": run.id().to_string(),
            "category": run.error_category().map(ErrorCategory::as_str),
        });
        if let Err(error) = self
            .notifier
            .notify(account.user_id(), "sync_failed", &idempotency, payload)
            .await
        {
            tracing::warn!(%error, "the sync-failed notification could not be sent");
        }
    }
}

fn observation_for(account: &Account, observed_token: &str) -> Observation {
    let contract_address = observed_token
        .starts_with("0x")
        .then(|| observed_token.to_string());
    let symbol = contract_address
        .is_none()
        .then(|| observed_token.to_string());
    Observation {
        chain: account.venue().chain().map(ToString::to_string),
        contract_address,
        symbol,
    }
}

fn classify_balance_error(error: crate::ports::ExternalError) -> (ErrorCategory, String) {
    match error {
        crate::ports::ExternalError::Unavailable(detail)
        | crate::ports::ExternalError::Refused(detail) => (ErrorCategory::Network, detail),
    }
}

fn parse_secret(secret: &str) -> Option<crate::ports::ExchangeCredentials> {
    let value: serde_json::Value = serde_json::from_str(secret).ok()?;
    let api_key = value.get("apiKey")?.as_str()?.to_string();
    let api_secret = value.get("apiSecret")?.as_str()?.to_string();
    if api_key.is_empty() || api_secret.is_empty() {
        return None;
    }
    let passphrase = value
        .get("passphrase")
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string);
    Some(crate::ports::ExchangeCredentials {
        api_key,
        api_secret,
        passphrase,
    })
}
