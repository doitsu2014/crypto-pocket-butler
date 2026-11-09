//! WF8: the C5 privacy export and anonymise Identity calls (BR8.1, BR8.2).

use std::sync::Arc;

use account_domain::{
    Account, AuditOutcome, ExportedAccount, HoldingSnapshot, ImportedTransaction, Operation,
    UserId, WalletConnection,
};

use crate::common::{AppError, FieldError, ServiceCaller};
use crate::ports::{
    AccountRepository, AnonymisationRecord, Clock, CredentialRepository, HoldingRepository,
    ImportRepository, PrivacyRepository, VaultClient, WalletConnectionRepository,
};
use crate::trail::{context, Trail};

/// The account data Identity assembles into the user's export.
#[derive(Debug, Clone, PartialEq)]
pub struct PrivacyExport {
    /// The user.
    pub user_id: UserId,
    /// The accounts, labels, and status.
    pub accounts: Vec<ExportedAccount>,
    /// The public wallet addresses and WalletConnect references.
    pub wallets: Vec<WalletConnection>,
    /// The observed holdings.
    pub holdings: Vec<HoldingSnapshot>,
    /// The imported transactions.
    pub transactions: Vec<ImportedTransaction>,
}

/// The result of one anonymisation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnonymiseResult {
    /// The user.
    pub user_id: UserId,
    /// The categories kept under a legal basis.
    pub retained_categories: Vec<String>,
    /// Whether an earlier request had already applied it.
    pub already_applied: bool,
}

/// Exports and anonymises a user's account data for Identity.
pub struct PrivacyService {
    accounts: Arc<dyn AccountRepository>,
    credentials: Arc<dyn CredentialRepository>,
    wallets: Arc<dyn WalletConnectionRepository>,
    holdings: Arc<dyn HoldingRepository>,
    imports: Arc<dyn ImportRepository>,
    privacy: Arc<dyn PrivacyRepository>,
    vault: Arc<dyn VaultClient>,
    trail: Arc<Trail>,
    clock: Arc<dyn Clock>,
}

impl PrivacyService {
    /// Creates the use case.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        accounts: Arc<dyn AccountRepository>,
        credentials: Arc<dyn CredentialRepository>,
        wallets: Arc<dyn WalletConnectionRepository>,
        holdings: Arc<dyn HoldingRepository>,
        imports: Arc<dyn ImportRepository>,
        privacy: Arc<dyn PrivacyRepository>,
        vault: Arc<dyn VaultClient>,
        trail: Arc<Trail>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            accounts,
            credentials,
            wallets,
            holdings,
            imports,
            privacy,
            vault,
            trail,
            clock,
        }
    }

    /// Returns the account rows, labels, wallet addresses, holdings, and imported
    /// transactions for the user (BR8.1).
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for a caller without the privacy-export
    /// operation, and a repository refusal when the read fails.
    pub async fn export(
        &self,
        caller: &ServiceCaller,
        user_id: UserId,
    ) -> Result<PrivacyExport, AppError> {
        caller.require_operation(Operation::PrivacyExport)?;
        let accounts = self
            .accounts
            .find_for_user(user_id)
            .await
            .map_err(AppError::from_repository)?;
        Ok(PrivacyExport {
            user_id,
            accounts: accounts.iter().map(exported_account).collect(),
            wallets: self
                .wallets
                .list_for_user(user_id)
                .await
                .map_err(AppError::from_repository)?,
            holdings: self
                .holdings
                .snapshots_for_user(user_id)
                .await
                .map_err(AppError::from_repository)?,
            transactions: self
                .imports
                .transactions_for_user(user_id)
                .await
                .map_err(AppError::from_repository)?,
        })
    }

    /// Removes the labels and wallet addresses, destroys exchange secrets, keeps
    /// the labelled financial-retention rows, and records the retained categories
    /// (BR8.2). A retry applies once.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for a caller without the anonymise
    /// operation, a validation refusal for a blank idempotency key, and an
    /// unavailable refusal when a step fails so the deletion stays retryable.
    pub async fn anonymise(
        &self,
        caller: &ServiceCaller,
        user_id: UserId,
        idempotency_key: &str,
    ) -> Result<AnonymiseResult, AppError> {
        caller.require_operation(Operation::PrivacyAnonymise)?;
        let idempotency_key = idempotency_key.trim();
        if idempotency_key.is_empty() {
            return Err(AppError::Validation(vec![FieldError::new(
                "idempotencyKey",
                "is required",
            )]));
        }
        if let Some(existing) = self
            .privacy
            .find_anonymisation(user_id)
            .await
            .map_err(AppError::from_repository)?
        {
            return Ok(AnonymiseResult {
                user_id,
                retained_categories: existing.retained_categories,
                already_applied: true,
            });
        }

        let now = self.clock.now();
        self.accounts
            .anonymise_for_user(user_id, now)
            .await
            .map_err(AppError::from_repository)?;

        let credentials = self
            .credentials
            .list_for_user(user_id)
            .await
            .map_err(AppError::from_repository)?;
        for credential in &credentials {
            if let Err(error) = self.vault.destroy(credential.vault_secret_ref()).await {
                tracing::warn!(%error, "a credential secret could not be destroyed");
                return Err(AppError::Unavailable(
                    "the credential secret could not be destroyed".to_string(),
                ));
            }
        }
        self.credentials
            .delete_for_user(user_id)
            .await
            .map_err(AppError::from_repository)?;
        self.wallets
            .delete_for_user(user_id)
            .await
            .map_err(AppError::from_repository)?;

        let retained_categories = vec![
            "accounts".to_string(),
            "holdings".to_string(),
            "sync_runs".to_string(),
            "import_batches".to_string(),
            "imported_transactions".to_string(),
        ];
        let record = AnonymisationRecord {
            user_id,
            idempotency_key: idempotency_key.to_string(),
            retained_categories: retained_categories.clone(),
            completed_at: now,
        };
        self.privacy
            .record_anonymisation(&record)
            .await
            .map_err(AppError::from_repository)?;

        self.trail
            .record(
                None,
                "account.privacy.anonymised",
                user_id.to_string(),
                AuditOutcome::Success,
                &caller.correlation_id,
                context(&[
                    ("userId", &user_id.to_string()),
                    ("credentials", &credentials.len().to_string()),
                ]),
            )
            .await?;
        Ok(AnonymiseResult {
            user_id,
            retained_categories,
            already_applied: false,
        })
    }
}

fn exported_account(account: &Account) -> ExportedAccount {
    ExportedAccount {
        id: account.id(),
        user_id: account.user_id(),
        kind: account.kind().as_str().to_string(),
        venue: account.venue().as_str().to_string(),
        label: account.label().to_string(),
        status: account.status().as_str().to_string(),
        last_synced_at: account.last_synced_at(),
    }
}
