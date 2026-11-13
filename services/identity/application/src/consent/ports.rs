//! U4 consent ports, defined inward and implemented by `identity-infrastructure`.
//!
//! The withdrawal transaction is a port of its own: the use case must make the
//! state decision while the per-(user, kind) advisory lock is held, and the
//! consent event and its audit-outbox row must commit together (NFR19.1–19.3).
//! Handing the use case an explicit transaction keeps that decision inward and
//! still gives the adapter one atomic unit of work.

use std::collections::BTreeMap;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use identity_domain::{
    AuditOutcome, ConsentAction, ConsentEvent, ConsentEventDraft, ConsentKind, ConsentRecordId,
    UserId,
};

use crate::ports::RepositoryError;

/// The keyset position one history page continues after.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryCursor {
    /// The last event's database timestamp.
    pub occurred_at: DateTime<Utc>,
    /// The last event's monotonic sequence.
    pub seq: i64,
}

/// One audit intent waiting in `consent_audit_outbox` for C9 delivery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditIntent {
    /// The consent event the intent describes.
    pub event_id: ConsentRecordId,
    /// The acting user.
    pub actor_id: UserId,
    /// The dotted C9 action.
    pub action: String,
    /// The logical target, `consent:<kind>`.
    pub target: String,
    /// The recorded outcome.
    pub outcome: AuditOutcome,
    /// When the consent event was recorded.
    pub occurred_at: DateTime<Utc>,
    /// Identifier-only context.
    pub context: BTreeMap<String, String>,
    /// The request correlation id, carried so a delayed delivery still traces.
    pub correlation_id: String,
    /// The key C9 honours across retries.
    pub idempotency_key: String,
}

impl AuditIntent {
    /// The withdrawal intent. The payload carries the prior state and the
    /// grant's policy version, and nothing else (security design).
    #[must_use]
    pub fn consent_withdrawn(
        event: &ConsentEvent,
        previous_policy_version: &str,
        correlation_id: &str,
    ) -> Self {
        let mut context = BTreeMap::new();
        context.insert("previousState".to_string(), "granted".to_string());
        context.insert(
            "policyVersion".to_string(),
            previous_policy_version.to_string(),
        );
        context.insert("correlationId".to_string(), correlation_id.to_string());
        Self {
            event_id: event.id(),
            actor_id: event.user_id(),
            action: "identity.consent.withdrawn".to_string(),
            target: format!("consent:{}", event.kind().as_str()),
            outcome: AuditOutcome::Success,
            occurred_at: event.occurred_at(),
            context,
            correlation_id: correlation_id.to_string(),
            idempotency_key: format!("consent-withdrawn:{}", event.id()),
        }
    }
}

/// Reads and appends consent events (BR4.4, BR4.5).
#[async_trait]
pub trait ConsentRepository: Send + Sync {
    /// The newest event per kind, one `SELECT DISTINCT ON`.
    async fn newest_per_kind(&self, user_id: UserId) -> Result<Vec<ConsentEvent>, RepositoryError>;

    /// One page of the user's history, newest first.
    async fn history(
        &self,
        user_id: UserId,
        limit: u32,
        before: Option<HistoryCursor>,
    ) -> Result<Vec<ConsentEvent>, RepositoryError>;

    /// Appends one event and returns the stored row with its database facts.
    async fn insert(&self, draft: &ConsentEventDraft) -> Result<ConsentEvent, RepositoryError>;

    /// Opens the withdrawal transaction: the advisory lock is taken, the
    /// statement timeout is set, and the newest read happens through the
    /// returned handle.
    async fn begin_withdraw(
        &self,
        user_id: UserId,
        kind: ConsentKind,
    ) -> Result<Box<dyn ConsentTransaction>, RepositoryError>;
}

/// One open withdrawal transaction. The use case decides, then commits or
/// rolls back.
#[async_trait]
pub trait ConsentTransaction: Send {
    /// The newest event for the locked `(user, kind)`.
    async fn newest(&mut self, kind: ConsentKind) -> Result<Option<ConsentEvent>, RepositoryError>;

    /// Appends the withdrawal and returns its stored row.
    async fn insert_event(
        &mut self,
        draft: &ConsentEventDraft,
    ) -> Result<ConsentEvent, RepositoryError>;

    /// Enqueues the audit intent inside this transaction.
    async fn enqueue_audit(&mut self, intent: &AuditIntent) -> Result<(), RepositoryError>;

    /// Commits the event and the outbox row together.
    async fn commit(self: Box<Self>) -> Result<(), RepositoryError>;

    /// Rolls the transaction back, writing nothing.
    async fn rollback(self: Box<Self>) -> Result<(), RepositoryError>;
}

/// The consent-state query outcome, for `consent_state_queries_total`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateQueryOutcome {
    /// The query answered.
    Answered,
    /// The query failed; the gate fails closed.
    Error,
}

/// The preference-write outcome, for `preference_writes_total`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreferenceWriteOutcome {
    /// The write applied.
    Applied,
    /// The write was refused before reaching the store.
    Refused,
}

/// Why a withdrawal transaction rolled back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WithdrawalRollbackCause {
    /// The audit intent could not be enqueued.
    OutboxFailed,
    /// The event or the commit failed.
    InsertFailed,
}

/// One outbox delivery outcome, for `consent_outbox_deliveries_total`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryOutcome {
    /// C9 accepted the event.
    Delivered,
    /// A transient failure; the row will be retried.
    Retryable,
    /// A permanent failure; the row will not be retried.
    Dead,
}

/// Counters, histograms, and gauges this unit emits. Implemented at the `http`
/// boundary and injected into the use cases and the drainer.
pub trait ObservationSink: Send + Sync {
    /// One consent-state query.
    fn record_state_query(&self, outcome: StateQueryOutcome, has_missing: bool, elapsed: Duration);

    /// One recorded consent event.
    fn record_consent_event(&self, kind: ConsentKind, action: ConsentAction);

    /// One preference write.
    fn record_preference_write(&self, outcome: PreferenceWriteOutcome);

    /// One withdrawal rollback, with its cause.
    fn record_withdrawal_rollback(&self, cause: WithdrawalRollbackCause);
}

/// The drainer's gauges and delivery counter.
pub trait DrainerObservation: Send + Sync {
    /// One drainer pass: the pending backlog and the oldest row's age.
    fn record_pass(&self, pending: u64, oldest_age_seconds: i64);

    /// One delivery outcome.
    fn record_delivery(&self, outcome: DeliveryOutcome);

    /// One pass over the status audit outbox (U5). A default keeps the consent
    /// implementors that predate the status log compiling.
    fn record_status_pass(&self, _pending: u64, _oldest_age_seconds: i64) {}

    /// One status audit delivery outcome.
    fn record_status_delivery(&self, _outcome: DeliveryOutcome) {}
}
