//! WF7: periodic key re-verification. A key that gained trading or withdrawal
//! permission is revoked, its secret destroyed, the account disconnected, the
//! claim cleared, and the user notified (BR2.4, BR2.5, BR4.5, BR7.1).

use std::sync::Arc;

use account_domain::{AccountStatus, AuditOutcome};

use crate::common::AppError;
use crate::ports::{
    AccountRepository, AuthAdmin, Clock, CredentialRepository, ExchangeProvider, ExternalError,
    NotificationClient, VaultClient,
};
use crate::trail::{context, Trail};

/// What one re-verification pass found.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VerificationReport {
    /// Credentials confirmed read-only.
    pub verified: usize,
    /// Credentials revoked.
    pub rejected: usize,
    /// Credentials left stale after a transient failure.
    pub stale: usize,
}

/// Re-checks every verified credential against its venue.
pub struct VerificationService {
    credentials: Arc<dyn CredentialRepository>,
    accounts: Arc<dyn AccountRepository>,
    exchange: Arc<dyn ExchangeProvider>,
    vault: Arc<dyn VaultClient>,
    auth_admin: Arc<dyn AuthAdmin>,
    notifier: Arc<dyn NotificationClient>,
    trail: Arc<Trail>,
    clock: Arc<dyn Clock>,
}

impl VerificationService {
    /// Creates the use case.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        credentials: Arc<dyn CredentialRepository>,
        accounts: Arc<dyn AccountRepository>,
        exchange: Arc<dyn ExchangeProvider>,
        vault: Arc<dyn VaultClient>,
        auth_admin: Arc<dyn AuthAdmin>,
        notifier: Arc<dyn NotificationClient>,
        trail: Arc<Trail>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            credentials,
            accounts,
            exchange,
            vault,
            auth_admin,
            notifier,
            trail,
            clock,
        }
    }

    /// Re-verifies every verified or stale credential once.
    ///
    /// # Errors
    /// Returns a repository refusal when the credential list cannot be read.
    pub async fn reverify_all(&self, correlation_id: &str) -> Result<VerificationReport, AppError> {
        let now = self.clock.now();
        let credentials = self
            .credentials
            .list_verified()
            .await
            .map_err(AppError::from_repository)?;
        let mut report = VerificationReport::default();

        for mut credential in credentials {
            let Some(mut account) = self
                .accounts
                .find_by_id(credential.account_id())
                .await
                .map_err(AppError::from_repository)?
            else {
                continue;
            };
            let Some(exchange) = account.venue().exchange() else {
                continue;
            };
            let exchange = exchange.to_string();

            let secret = match self.vault.reveal(credential.vault_secret_ref()).await {
                Ok(secret) => secret,
                Err(error) => {
                    self.mark_stale(&mut credential, now, correlation_id, &error.to_string())
                        .await?;
                    report.stale += 1;
                    continue;
                }
            };
            let Some(credentials) = parse_secret(&secret) else {
                self.revoke(
                    &mut credential,
                    &mut account,
                    now,
                    correlation_id,
                    "the stored credential is unreadable",
                )
                .await?;
                report.rejected += 1;
                continue;
            };

            match self
                .exchange
                .verify_read_only(&exchange, &credentials)
                .await
            {
                Ok(permissions) if permissions.read_only => {
                    credential.verify(now);
                    self.credentials
                        .update(&credential)
                        .await
                        .map_err(AppError::from_repository)?;
                    if account.status() == AccountStatus::Disconnected {
                        account.reactivate(now).map_err(AppError::from_domain)?;
                        self.accounts
                            .update(&account)
                            .await
                            .map_err(AppError::from_repository)?;
                    }
                    report.verified += 1;
                    self.audit(
                        &account,
                        "account.credential.verified",
                        AuditOutcome::Success,
                        correlation_id,
                        &[("accountId", &account.id().to_string())],
                    )
                    .await;
                }
                Ok(_) => {
                    self.revoke(
                        &mut credential,
                        &mut account,
                        now,
                        correlation_id,
                        "the key now carries trading or withdrawal permission",
                    )
                    .await?;
                    report.rejected += 1;
                }
                Err(ExternalError::Unavailable(detail)) => {
                    self.mark_stale(&mut credential, now, correlation_id, &detail)
                        .await?;
                    report.stale += 1;
                }
                Err(ExternalError::Refused(detail)) => {
                    self.revoke(&mut credential, &mut account, now, correlation_id, &detail)
                        .await?;
                    report.rejected += 1;
                }
            }
        }
        Ok(report)
    }

    async fn mark_stale(
        &self,
        credential: &mut account_domain::Credential,
        now: chrono::DateTime<chrono::Utc>,
        correlation_id: &str,
        reason: &str,
    ) -> Result<(), AppError> {
        credential.mark_stale(now);
        self.credentials
            .update(credential)
            .await
            .map_err(AppError::from_repository)?;
        if let Some(account) = self
            .accounts
            .find_by_id(credential.account_id())
            .await
            .map_err(AppError::from_repository)?
        {
            self.audit(
                &account,
                "account.credential.stale",
                AuditOutcome::Failed,
                correlation_id,
                &[("accountId", &account.id().to_string()), ("reason", reason)],
            )
            .await;
        }
        Ok(())
    }

    async fn revoke(
        &self,
        credential: &mut account_domain::Credential,
        account: &mut account_domain::Account,
        now: chrono::DateTime<chrono::Utc>,
        correlation_id: &str,
        reason: &str,
    ) -> Result<(), AppError> {
        let reference = credential.vault_secret_ref().to_string();
        credential.reject(now);
        self.credentials
            .update(credential)
            .await
            .map_err(AppError::from_repository)?;
        if let Err(error) = self.vault.destroy(&reference).await {
            tracing::warn!(%error, "the revoked credential secret could not be destroyed");
        }

        account.disconnect(now);
        self.accounts
            .update(account)
            .await
            .map_err(AppError::from_repository)?;
        if let Err(error) = self
            .auth_admin
            .set_has_exchange_credentials(account.user_id(), false)
            .await
        {
            tracing::warn!(%error, "the has_exchange_credentials claim could not be cleared");
        }

        let idempotency = format!("key_revoked:{}:{}", account.id(), credential.id());
        let payload = serde_json::json!({
            "accountId": account.id().to_string(),
            "credentialId": credential.id().to_string(),
        });
        if let Err(error) = self
            .notifier
            .notify(account.user_id(), "key_revoked", &idempotency, payload)
            .await
        {
            tracing::warn!(%error, "the key-revoked notification could not be sent");
        }

        self.audit(
            account,
            "account.credential.rejected",
            AuditOutcome::Success,
            correlation_id,
            &[("accountId", &account.id().to_string()), ("reason", reason)],
        )
        .await;
        self.audit(
            account,
            "account.disconnected",
            AuditOutcome::Success,
            correlation_id,
            &[("accountId", &account.id().to_string())],
        )
        .await;
        Ok(())
    }

    async fn audit(
        &self,
        account: &account_domain::Account,
        action: &str,
        outcome: AuditOutcome,
        correlation_id: &str,
        pairs: &[(&str, &str)],
    ) {
        if let Err(error) = self
            .trail
            .record(
                Some(account.user_id()),
                action,
                account.id().to_string(),
                outcome,
                correlation_id,
                context(pairs),
            )
            .await
        {
            tracing::warn!(%error, action, "the credential audit event could not be recorded");
        }
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
