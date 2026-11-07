//! Request and response DTOs. They live only at the HTTP boundary and map to and
//! from domain and application types here; no DTO type reaches inward.

use account_application::holdings::AccountHoldings;
use account_application::privacy::PrivacyExport;
use account_domain::{Account, HoldingSnapshot, ImportedTransaction, SyncRun, WalletConnection};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// `POST /v1/accounts/exchange` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddExchangeRequest {
    /// `binance` or `okx`.
    pub venue: String,
    /// The user's label.
    pub label: String,
    /// The public key identifier.
    pub api_key: String,
    /// The private secret.
    pub api_secret: String,
    /// The OKX passphrase, when the venue requires one.
    #[serde(default)]
    pub passphrase: Option<String>,
}

/// `POST /v1/accounts/wallet` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddWalletRequest {
    /// `evm:<chainId>` or `solana`.
    pub chain: String,
    /// The public address.
    pub address: String,
    /// The user's label.
    pub label: String,
}

/// `POST /v1/accounts/walletconnect` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddWalletConnectRequest {
    /// `evm:<chainId>` or `solana`.
    pub chain: String,
    /// The verified public address.
    pub address: String,
    /// The user's label.
    pub label: String,
    /// The opaque WalletConnect session reference the SPA holds.
    pub session_ref: String,
}

/// `GET /v1/users/{userId}/accounts` response item.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountResponse {
    /// The account identifier.
    pub id: String,
    /// The owner.
    pub user_id: String,
    /// The source kind.
    pub kind: String,
    /// The venue or chain.
    pub venue: String,
    /// The display label.
    pub label: String,
    /// The lifecycle state.
    pub status: String,
    /// The last successful sync.
    pub last_synced_at: Option<DateTime<Utc>>,
    /// When the account was created.
    pub created_at: DateTime<Utc>,
}

impl From<&Account> for AccountResponse {
    fn from(account: &Account) -> Self {
        Self {
            id: account.id().to_string(),
            user_id: account.user_id().to_string(),
            kind: account.kind().as_str().to_string(),
            venue: account.venue().as_str().to_string(),
            label: account.label().to_string(),
            status: account.status().as_str().to_string(),
            last_synced_at: account.last_synced_at(),
            created_at: account.created_at(),
        }
    }
}

/// A manual re-sync response (C1 returns 202 with the active run id).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResyncResponse {
    /// The account.
    pub account_id: String,
    /// The active or newly claimed run.
    pub run_id: String,
    /// Always `syncing` at the boundary.
    pub status: String,
    /// When the run started.
    pub started_at: DateTime<Utc>,
}

impl ResyncResponse {
    /// Builds the response from the run.
    #[must_use]
    pub fn from_run(run: &SyncRun) -> Self {
        Self {
            account_id: run.account_id().to_string(),
            run_id: run.id().to_string(),
            status: "syncing".to_string(),
            started_at: run.started_at(),
        }
    }
}

/// `POST /v1/accounts/{accountId}/imports` response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportAcceptedResponse {
    /// The staged batch.
    pub batch_id: String,
    /// The batch status.
    pub status: String,
}

/// One C5 holding.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HoldingDto {
    /// The account.
    pub account_id: String,
    /// The canonical asset id, absent for a spam holding.
    pub asset_id: Option<String>,
    /// The observed token.
    pub observed_token: String,
    /// The decimal quantity as a string.
    pub quantity: String,
    /// When the holding was observed.
    pub observed_at: DateTime<Utc>,
    /// Whether the asset is absent from the canonical registry.
    pub spam: bool,
    /// The account's last successful sync.
    pub last_synced_at: Option<DateTime<Utc>>,
}

impl HoldingDto {
    /// Flattens one account's holdings.
    #[must_use]
    pub fn flatten(account: &AccountHoldings) -> Vec<Self> {
        account
            .holdings
            .iter()
            .map(|snapshot| Self::from_snapshot(snapshot, account.last_synced_at))
            .collect()
    }

    fn from_snapshot(snapshot: &HoldingSnapshot, last_synced_at: Option<DateTime<Utc>>) -> Self {
        Self {
            account_id: snapshot.account_id().to_string(),
            asset_id: snapshot.asset_id().map(ToString::to_string),
            observed_token: snapshot.observed_token().to_string(),
            quantity: snapshot.quantity().to_string(),
            observed_at: snapshot.observed_at(),
            spam: snapshot.spam(),
            last_synced_at,
        }
    }
}

/// One C5 transaction.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionDto {
    /// The venue's own row id.
    pub external_id: Option<String>,
    /// The canonical asset id.
    pub asset_id: String,
    /// The transaction kind.
    pub kind: String,
    /// The decimal quantity as a string.
    pub quantity: String,
    /// The decimal unit price as a string.
    pub price: Option<String>,
    /// When the transaction executed.
    pub executed_at: DateTime<Utc>,
}

impl From<&ImportedTransaction> for TransactionDto {
    fn from(transaction: &ImportedTransaction) -> Self {
        Self {
            external_id: transaction.external_id().map(ToString::to_string),
            asset_id: transaction.asset_id().to_string(),
            kind: transaction.kind().as_str().to_string(),
            quantity: transaction.quantity().to_string(),
            price: transaction.price().map(|price| price.to_string()),
            executed_at: transaction.executed_at(),
        }
    }
}

/// One wallet address or WalletConnect reference in the privacy export.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletExportDto {
    /// The account.
    pub account_id: String,
    /// The public address.
    pub address: String,
    /// The chain the address was validated against.
    pub observed_chain: String,
    /// The opaque WalletConnect reference, when one exists.
    pub session_ref: Option<String>,
    /// When the connection was established.
    pub connected_at: DateTime<Utc>,
}

impl From<&WalletConnection> for WalletExportDto {
    fn from(connection: &WalletConnection) -> Self {
        Self {
            account_id: connection.account_id().to_string(),
            address: connection.address().to_string(),
            observed_chain: connection.observed_chain().to_string(),
            session_ref: connection.session_ref().map(ToString::to_string),
            connected_at: connection.connected_at(),
        }
    }
}

/// One account row in the privacy export.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountExportDto {
    /// The account.
    pub id: String,
    /// The source kind.
    pub kind: String,
    /// The venue or chain.
    pub venue: String,
    /// The display label.
    pub label: String,
    /// The lifecycle state.
    pub status: String,
    /// The last successful sync.
    pub last_synced_at: Option<DateTime<Utc>>,
}

/// `GET /v1/internal/privacy/export` response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyExportResponse {
    /// The user.
    pub user_id: String,
    /// The account rows.
    pub accounts: Vec<AccountExportDto>,
    /// The public wallet addresses.
    pub wallets: Vec<WalletExportDto>,
    /// The observed holdings.
    pub holdings: Vec<HoldingDto>,
    /// The imported transactions.
    pub transactions: Vec<TransactionDto>,
}

impl From<PrivacyExport> for PrivacyExportResponse {
    fn from(export: PrivacyExport) -> Self {
        Self {
            user_id: export.user_id.to_string(),
            accounts: export
                .accounts
                .iter()
                .map(|account| AccountExportDto {
                    id: account.id.to_string(),
                    kind: account.kind.clone(),
                    venue: account.venue.clone(),
                    label: account.label.clone(),
                    status: account.status.clone(),
                    last_synced_at: account.last_synced_at,
                })
                .collect(),
            wallets: export.wallets.iter().map(WalletExportDto::from).collect(),
            holdings: export
                .holdings
                .iter()
                .map(|snapshot| HoldingDto::from_snapshot(snapshot, None))
                .collect(),
            transactions: export
                .transactions
                .iter()
                .map(TransactionDto::from)
                .collect(),
        }
    }
}

/// `POST /v1/internal/privacy/anonymise` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnonymiseRequest {
    /// The user.
    pub user_id: String,
    /// The caller's idempotency key.
    pub idempotency_key: String,
}

/// `POST /v1/internal/privacy/anonymise` response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnonymiseResponse {
    /// The user.
    pub user_id: String,
    /// The categories kept under a legal basis.
    pub retained_categories: Vec<String>,
    /// Whether an earlier request already applied it.
    pub already_applied: bool,
}

/// `POST /v1/internal/run/account-sync` body.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRequest {
    /// The accounts to sync; empty runs every pending run.
    #[serde(default)]
    pub account_ids: Vec<String>,
}

/// The C11 run result.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunResultResponse {
    /// The job name.
    pub job_name: String,
    /// `succeeded`, `failed`, or `skipped`.
    pub status: String,
    /// When the run started.
    pub started_at: DateTime<Utc>,
    /// A short non-secret detail.
    pub detail: Option<String>,
}

impl RunResultResponse {
    /// Builds the result.
    #[must_use]
    pub fn new(status: &str, started_at: DateTime<Utc>, detail: Option<String>) -> Self {
        Self {
            job_name: "account-sync".to_string(),
            status: status.to_string(),
            started_at,
            detail,
        }
    }
}
