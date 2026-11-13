//! WF7 — time-bound, second-approved support access (BR3.1–BR3.13,
//! FR16.1–FR16.6).
//!
//! Three paths, three audit mechanisms: an accepted mutation writes its event in
//! the same transaction as the grant, a refused mutation writes its event in its
//! own transaction before the refusal is returned, and a read appends its event
//! inline before any data leaves. All three fail closed.

use std::sync::Arc;

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use identity_domain::{
    AuditOutcome, AuditRecord, SupportAccessGrant, SupportGrantId, SupportGrantStatus, UserId,
    DEFAULT_GRANT_MINUTES,
};

use crate::common::{AppError, Caller};
use crate::ports::{
    AccountSummary, Clock, PortfolioSummary, SupportAccountsSource, SupportGrantRepository,
    SupportObservation, SupportOutbox, SupportPortfoliosSource, UserRepository,
};
use crate::trail::context;

/// The uniform answer an unknown grant id and a foreign grant both get
/// (BR3.7, BR3.8).
pub const SUPPORT_ACCESS_REFUSED: &str = "support-access-refused";
/// The answer when a record could not be written: the action fails instead of
/// completing without a record (BR3.13).
pub const AUDIT_UNAVAILABLE: &str = "audit-unavailable";
/// The answer when Account or Portfolio could not serve its projection.
pub const METADATA_SOURCE_UNAVAILABLE: &str = "metadata-source-unavailable";
/// How long the two owner calls may take together. Both run concurrently, and
/// the slower of the two is the read's latency.
const FANOUT_BUDGET_MS: u64 = 500;
/// The most pending grants one approvals read returns (U10, NFR2.5).
pub const PENDING_GRANT_LIMIT: usize = 100;
/// The most of its own grants one support agent's read returns (S2, NFR2.3).
pub const MY_GRANTS_LIMIT: usize = 20;

/// The closed metadata projection a grant may read (BR3.9). Every field is
/// allowlisted; nothing else is ever requested from its owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportMetadata {
    /// The user record the grant names.
    pub user_id: UserId,
    /// The display name; null when genuinely absent.
    pub display_name: Option<String>,
    /// Verification status only — never the email address.
    pub email_verified: bool,
    /// The allowlisted account summaries.
    pub accounts: Vec<AccountSummary>,
    /// The allowlisted portfolio summaries.
    pub portfolios: Vec<PortfolioSummary>,
}

/// Requests, approves, revokes, and reads under support grants.
pub struct SupportService {
    grants: Arc<dyn SupportGrantRepository>,
    users: Arc<dyn UserRepository>,
    outbox: Arc<dyn SupportOutbox>,
    accounts: Arc<dyn SupportAccountsSource>,
    portfolios: Arc<dyn SupportPortfoliosSource>,
    observation: Arc<dyn SupportObservation>,
    clock: Arc<dyn Clock>,
}

impl SupportService {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        grants: Arc<dyn SupportGrantRepository>,
        users: Arc<dyn UserRepository>,
        outbox: Arc<dyn SupportOutbox>,
        accounts: Arc<dyn SupportAccountsSource>,
        portfolios: Arc<dyn SupportPortfoliosSource>,
        observation: Arc<dyn SupportObservation>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            grants,
            users,
            outbox,
            accounts,
            portfolios,
            observation,
            clock,
        }
    }

    /// A support agent requests a grant for one target user (W1). The grant and
    /// its event commit together; every refusal is audited before it returns.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for a non-support caller or an
    /// inaccessible target, [`AppError::Conflict`] when a live grant already
    /// covers the pair, [`AppError::NotFound`] for an unknown target,
    /// [`AppError::Validation`] for a malformed request, and
    /// [`AppError::Unavailable`] when the audit row cannot be written.
    pub async fn request(
        &self,
        caller: &Caller,
        user_id: UserId,
        reason: &str,
    ) -> Result<SupportAccessGrant, AppError> {
        let now = self.clock.now();
        const ACTION: &str = "identity.support_grant.requested";
        if !caller.is_support() {
            return self
                .refuse_mutation(
                    caller,
                    ACTION,
                    user_id.to_string(),
                    "support-role-required",
                    AppError::Forbidden("support-role-required".to_string()),
                    &[],
                )
                .await;
        }
        let target = self
            .users
            .find_by_id(user_id)
            .await
            .map_err(AppError::from_repository)?;
        let Some(target) = target else {
            return Err(AppError::NotFound("user"));
        };
        if target.is_deleted() {
            return Err(AppError::NotFound("user"));
        }
        if target.status() == identity_domain::UserStatus::PendingDeletion {
            return self
                .refuse_mutation(
                    caller,
                    ACTION,
                    user_id.to_string(),
                    "target-not-accessible",
                    AppError::Forbidden("target-not-accessible".to_string()),
                    &[],
                )
                .await;
        }
        let grant = match SupportAccessGrant::request(
            SupportGrantId::generate(),
            user_id,
            caller.user_id(),
            reason,
            now,
        ) {
            Ok(grant) => grant,
            Err(error) => {
                return self
                    .refuse_mutation(
                        caller,
                        ACTION,
                        user_id.to_string(),
                        "validation",
                        AppError::from_domain(error),
                        &[],
                    )
                    .await;
            }
        };
        if let Some(existing) = self
            .grants
            .find_live_by_agent_and_user(caller.user_id(), user_id)
            .await
            .map_err(AppError::from_repository)?
        {
            if existing.approval_window_closed(now) {
                // A stale pending request is expired before the new one is
                // written, so the partial unique index sees no live row
                // (BR3.3).
                let mut stale = existing;
                if stale.expire_if_due(now) {
                    self.grants
                        .update(&stale)
                        .await
                        .map_err(AppError::from_repository)?;
                }
            } else {
                let existing_id = existing.id().to_string();
                return self
                    .refuse_mutation(
                        caller,
                        ACTION,
                        user_id.to_string(),
                        "grant-already-live",
                        AppError::Conflict("grant-already-live".to_string()),
                        &[("existingGrantId", existing_id.as_str())],
                    )
                    .await;
            }
        }
        let grant_id = grant.id().to_string();
        let record = self.record(
            Some(caller.user_id()),
            ACTION,
            user_id.to_string(),
            AuditOutcome::Success,
            now,
            &[("grantId", grant_id.as_str())],
        )?;
        self.grants
            .insert_with_event(&grant, &record, &caller.correlation_id)
            .await
            .map_err(AppError::from_repository)?;
        Ok(grant)
    }

    /// A distinct admin approves a pending grant (W2).
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for a non-admin or self-approval,
    /// [`AppError::Conflict`] when the request expired or another writer moved
    /// the grant first, and [`AppError::NotFound`] for an unknown grant.
    pub async fn approve(
        &self,
        caller: &Caller,
        grant_id: SupportGrantId,
    ) -> Result<SupportAccessGrant, AppError> {
        let now = self.clock.now();
        const ACTION: &str = "identity.support_grant.approved";
        let target = grant_id.to_string();
        let grant_label = target.clone();
        if !caller.is_admin() {
            return self
                .refuse_mutation(
                    caller,
                    ACTION,
                    target,
                    "admin-role-required",
                    AppError::Forbidden("admin-role-required".to_string()),
                    &[],
                )
                .await;
        }
        let Some(mut grant) = self.load(grant_id).await? else {
            return self
                .refuse_mutation(
                    caller,
                    ACTION,
                    target,
                    "not-found",
                    AppError::NotFound("support grant"),
                    &[],
                )
                .await;
        };
        if grant.is_held_by(caller.user_id()) {
            return self
                .refuse_mutation(
                    caller,
                    ACTION,
                    target,
                    "self-approval",
                    AppError::Forbidden("self-approval".to_string()),
                    &[("grantId", grant_label.as_str())],
                )
                .await;
        }
        if grant.approval_window_closed(now) {
            if grant.expire_if_due(now) {
                self.grants
                    .update(&grant)
                    .await
                    .map_err(AppError::from_repository)?;
            }
            return self
                .refuse_mutation(
                    caller,
                    ACTION,
                    target,
                    "request-expired",
                    AppError::Conflict("request-expired".to_string()),
                    &[("grantId", grant_label.as_str())],
                )
                .await;
        }
        if grant.status() != SupportGrantStatus::PendingApproval {
            return self
                .refuse_mutation(
                    caller,
                    ACTION,
                    target,
                    "grant-not-pending",
                    AppError::Conflict("grant-not-pending".to_string()),
                    &[("grantId", grant_label.as_str())],
                )
                .await;
        }
        grant
            .approve(
                caller.user_id(),
                caller.role(),
                now,
                ChronoDuration::minutes(DEFAULT_GRANT_MINUTES),
            )
            .map_err(AppError::from_domain)?;
        let expires_at = grant
            .expires_at()
            .ok_or_else(|| AppError::Internal("an approval without a window".to_string()))?;
        let record = self.record(
            Some(caller.user_id()),
            ACTION,
            target.clone(),
            AuditOutcome::Success,
            now,
            &[("grantId", target.as_str())],
        )?;
        let affected = self
            .grants
            .approve_with_event(
                grant.id(),
                caller.user_id(),
                now,
                expires_at,
                &record,
                &caller.correlation_id,
            )
            .await
            .map_err(AppError::from_repository)?;
        if affected == 0 {
            return self
                .refuse_mutation(
                    caller,
                    ACTION,
                    target,
                    "grant-not-pending",
                    AppError::Conflict("grant-not-pending".to_string()),
                    &[("grantId", grant_label.as_str())],
                )
                .await;
        }
        Ok(grant)
    }

    /// An admin revokes a pending or active grant (W3). The row is kept as
    /// evidence and is never deleted.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for a non-admin,
    /// [`AppError::Conflict`] for a terminal grant or a concurrent move, and
    /// [`AppError::NotFound`] for an unknown grant.
    pub async fn revoke(
        &self,
        caller: &Caller,
        grant_id: SupportGrantId,
    ) -> Result<SupportAccessGrant, AppError> {
        let now = self.clock.now();
        const ACTION: &str = "identity.support_grant.revoked";
        let target = grant_id.to_string();
        let grant_label = target.clone();
        if !caller.is_admin() {
            return self
                .refuse_mutation(
                    caller,
                    ACTION,
                    target,
                    "admin-role-required",
                    AppError::Forbidden("admin-role-required".to_string()),
                    &[],
                )
                .await;
        }
        let Some(mut grant) = self.load(grant_id).await? else {
            return self
                .refuse_mutation(
                    caller,
                    ACTION,
                    target,
                    "not-found",
                    AppError::NotFound("support grant"),
                    &[],
                )
                .await;
        };
        if grant.status().is_terminal() {
            return self
                .refuse_mutation(
                    caller,
                    ACTION,
                    target,
                    "grant-not-pending",
                    AppError::Conflict("grant-not-pending".to_string()),
                    &[("grantId", grant_label.as_str())],
                )
                .await;
        }
        grant.revoke(now).map_err(AppError::from_domain)?;
        let record = self.record(
            Some(caller.user_id()),
            ACTION,
            target.clone(),
            AuditOutcome::Success,
            now,
            &[("grantId", target.as_str())],
        )?;
        let affected = self
            .grants
            .revoke_with_event(grant.id(), now, &record, &caller.correlation_id)
            .await
            .map_err(AppError::from_repository)?;
        if affected == 0 {
            return self
                .refuse_mutation(
                    caller,
                    ACTION,
                    target,
                    "grant-not-pending",
                    AppError::Conflict("grant-not-pending".to_string()),
                    &[("grantId", grant_label.as_str())],
                )
                .await;
        }
        Ok(grant)
    }

    /// The bounded pending-grant list an admin reviews (U10, FR20.1, NFR2.5).
    /// One read, newest first, at most [`PENDING_GRANT_LIMIT`] rows.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for a caller without the admin role and
    /// [`AppError::Unavailable`] when the store cannot answer.
    pub async fn pending_grants(
        &self,
        caller: &Caller,
    ) -> Result<Vec<SupportAccessGrant>, AppError> {
        if !caller.is_admin() {
            return Err(AppError::Forbidden(
                "only an administrator may review support grants".to_string(),
            ));
        }
        self.grants
            .list_by_status(SupportGrantStatus::PendingApproval, PENDING_GRANT_LIMIT)
            .await
            .map_err(AppError::from_repository)
    }

    /// The support agent's own grants, newest first, at most
    /// [`MY_GRANTS_LIMIT`] rows (S2, NFR2.3). Self-scoped: the agent id comes
    /// from the validated token, never the request.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for a caller without the support role and
    /// [`AppError::Unavailable`] when the store cannot answer.
    pub async fn my_grants(&self, caller: &Caller) -> Result<Vec<SupportAccessGrant>, AppError> {
        if !caller.is_support() {
            return Err(AppError::Forbidden("support-role-required".to_string()));
        }
        self.grants
            .list_by_support_agent(caller.user_id(), MY_GRANTS_LIMIT)
            .await
            .map_err(AppError::from_repository)
    }

    /// The grant holder reads the target's closed metadata projection (W4).
    /// Validity is decided on every call from the stored state and the clock,
    /// and an observed expiry is persisted before the refusal.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for every refused read and
    /// [`AppError::Unavailable`] when an owner cannot answer or an audit row
    /// cannot be written.
    pub async fn metadata(
        &self,
        caller: &Caller,
        grant_id: SupportGrantId,
    ) -> Result<SupportMetadata, AppError> {
        let now = self.clock.now();
        let actor = caller.user_id();
        let correlation_id = &caller.correlation_id;
        if !caller.is_support() {
            return self
                .refuse_read(
                    actor,
                    grant_id.to_string(),
                    "support-role-required",
                    AppError::Forbidden("support-role-required".to_string()),
                    correlation_id,
                )
                .await;
        }
        let Some(mut grant) = self.load(grant_id).await? else {
            return self
                .refuse_read(
                    actor,
                    grant_id.to_string(),
                    SUPPORT_ACCESS_REFUSED,
                    AppError::Forbidden(SUPPORT_ACCESS_REFUSED.to_string()),
                    correlation_id,
                )
                .await;
        };
        if !grant.is_held_by(actor) {
            return self
                .refuse_read(
                    actor,
                    grant.user_id().to_string(),
                    SUPPORT_ACCESS_REFUSED,
                    AppError::Forbidden(SUPPORT_ACCESS_REFUSED.to_string()),
                    correlation_id,
                )
                .await;
        }
        if grant.expire_if_due(now) {
            self.grants
                .update(&grant)
                .await
                .map_err(AppError::from_repository)?;
        }
        let denial = match grant.status() {
            SupportGrantStatus::Active => None,
            SupportGrantStatus::Expired => Some("grant-expired"),
            SupportGrantStatus::Revoked => Some("grant-revoked"),
            SupportGrantStatus::PendingApproval => Some("grant-pending"),
        };
        if let Some(code) = denial {
            return self
                .refuse_read(
                    actor,
                    grant.user_id().to_string(),
                    code,
                    AppError::Forbidden(code.to_string()),
                    correlation_id,
                )
                .await;
        }
        let Some(target) = self
            .users
            .find_by_id(grant.user_id())
            .await
            .map_err(AppError::from_repository)?
        else {
            return self
                .refuse_read(
                    actor,
                    grant.user_id().to_string(),
                    "target-not-accessible",
                    AppError::Forbidden("target-not-accessible".to_string()),
                    correlation_id,
                )
                .await;
        };
        if target.is_deleted() || target.status() == identity_domain::UserStatus::PendingDeletion {
            return self
                .refuse_read(
                    actor,
                    grant.user_id().to_string(),
                    "target-not-accessible",
                    AppError::Forbidden("target-not-accessible".to_string()),
                    correlation_id,
                )
                .await;
        }
        let target_id = target.id();
        let accounts_call = self.accounts.accounts(target_id);
        let portfolios_call = self.portfolios.portfolios(target_id);
        let joined =
            tokio::time::timeout(std::time::Duration::from_millis(FANOUT_BUDGET_MS), async {
                tokio::join!(accounts_call, portfolios_call)
            })
            .await;
        let (accounts, portfolios) = match joined {
            Ok((Ok(accounts), Ok(portfolios))) => (accounts, portfolios),
            other => {
                // Name the owner that failed for the monitoring series, then
                // keep the uniform refusal and its audit record unchanged.
                match &other {
                    Ok((Err(_), Err(_))) => {
                        self.observation.source_failure("account", "unreachable");
                        self.observation.source_failure("portfolio", "unreachable");
                    }
                    Ok((Err(_), Ok(_))) => {
                        self.observation.source_failure("account", "unreachable");
                    }
                    Ok((Ok(_), Err(_))) => {
                        self.observation.source_failure("portfolio", "unreachable");
                    }
                    Ok((Ok(_), Ok(_))) => {}
                    Err(_) => {
                        self.observation.source_failure("account", "timeout");
                        self.observation.source_failure("portfolio", "timeout");
                    }
                }
                self.observation
                    .failed(
                        actor,
                        target_id,
                        METADATA_SOURCE_UNAVAILABLE,
                        correlation_id,
                    )
                    .await
                    .map_err(|_| AppError::Unavailable(AUDIT_UNAVAILABLE.to_string()))?;
                return Err(AppError::Unavailable(
                    METADATA_SOURCE_UNAVAILABLE.to_string(),
                ));
            }
        };
        self.observation
            .served(actor, target_id, grant.id(), correlation_id)
            .await
            .map_err(|_| AppError::Unavailable(AUDIT_UNAVAILABLE.to_string()))?;
        Ok(SupportMetadata {
            user_id: target_id,
            display_name: target.display_name().map(str::to_owned),
            email_verified: target.email_verified(),
            accounts,
            portfolios,
        })
    }

    /// Builds one audited mutation refusal and returns the refusal it could not
    /// suppress: a failure to record replaces the clean refusal with
    /// `audit-unavailable` (BR3.13).
    async fn refuse_mutation<T>(
        &self,
        caller: &Caller,
        action: &str,
        target: String,
        code: &str,
        error: AppError,
        extra: &[(&str, &str)],
    ) -> Result<T, AppError> {
        let mut pairs = extra.to_vec();
        pairs.push(("code", code));
        let record = self.record(
            Some(caller.user_id()),
            action,
            target,
            AuditOutcome::Denied,
            self.clock.now(),
            &pairs,
        )?;
        self.outbox
            .append(&record, &caller.correlation_id, self.clock.now())
            .await
            .map_err(|_| AppError::Unavailable(AUDIT_UNAVAILABLE.to_string()))?;
        Err(error)
    }

    /// Builds one audited read refusal and returns it, or `audit-unavailable`
    /// when the event cannot be appended.
    async fn refuse_read<T>(
        &self,
        actor: UserId,
        target: String,
        code: &str,
        error: AppError,
        correlation_id: &str,
    ) -> Result<T, AppError> {
        self.observation
            .denied(actor, target, code, correlation_id)
            .await
            .map_err(|_| AppError::Unavailable(AUDIT_UNAVAILABLE.to_string()))?;
        Err(error)
    }

    fn record(
        &self,
        actor: Option<UserId>,
        action: &str,
        target: String,
        outcome: AuditOutcome,
        at: DateTime<Utc>,
        extra: &[(&str, &str)],
    ) -> Result<AuditRecord, AppError> {
        AuditRecord::new(actor, action, target, outcome, at, context(extra))
            .map_err(|error| AppError::Internal(format!("invalid support audit event: {error}")))
    }

    async fn load(&self, grant_id: SupportGrantId) -> Result<Option<SupportAccessGrant>, AppError> {
        self.grants
            .find_by_id(grant_id)
            .await
            .map_err(AppError::from_repository)
    }
}
