//! Port fakes for unit-level application and HTTP tests.
//!
//! Each fake is deterministic, records what the use case asked it to do, and can
//! be made to fail once so error paths are exercised without a database.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use audit_application::ports::{
    AppendOutcome, AuditEventRepository, AuthError, Clock, Cursor, EventQuery, ExportRepository,
    Pause, Principal, PrivacyDeletionRepository, ReadinessProbe, RepositoryError,
    RetentionRepository, ServiceTokenRegistry, SupportSessionRepository, TokenValidator,
};
use audit_domain::{
    AuditEvent, AuditEventId, AuditExport, DeletionRequestId, EventLabel, ExportFilter,
    PartitionSummary, PrivacyDeletion, RetentionRun, RetentionRunId, Sensitivity, SupportSession,
    SupportSessionId, UserId,
};
use chrono::{DateTime, Utc};

fn matches(
    event: &AuditEvent,
    action_prefix: Option<&str>,
    target: Option<&str>,
    sensitivity: Option<Sensitivity>,
    occurred_from: Option<DateTime<Utc>>,
    occurred_to: Option<DateTime<Utc>>,
    recorded_from: DateTime<Utc>,
    recorded_to: DateTime<Utc>,
) -> bool {
    event.recorded_at() >= recorded_from
        && event.recorded_at() < recorded_to
        && action_prefix.is_none_or(|prefix| event.action().starts_with(prefix))
        && target.is_none_or(|target| event.target() == target)
        && sensitivity.is_none_or(|value| event.sensitivity() == value)
        && occurred_from.is_none_or(|from| event.occurred_at() >= from)
        && occurred_to.is_none_or(|to| event.occurred_at() < to)
}

fn matches_export(event: &AuditEvent, filter: &ExportFilter) -> bool {
    event.recorded_at() >= filter.recorded_from
        && event.recorded_at() < filter.recorded_to
        && (filter.action_prefixes.is_empty()
            || filter
                .action_prefixes
                .iter()
                .any(|prefix| event.action().starts_with(prefix)))
        && filter
            .target
            .as_deref()
            .is_none_or(|target| event.target() == target)
        && filter
            .sensitivity
            .is_none_or(|value| event.sensitivity() == value)
        && filter
            .occurred_from
            .is_none_or(|from| event.occurred_at() >= from)
        && filter.occurred_to.is_none_or(|to| event.occurred_at() < to)
}

/// In-memory audit trail with idempotency.
#[derive(Default)]
pub struct FakeEvents {
    /// Stored events.
    pub events: Mutex<Vec<AuditEvent>>,
    /// `(emitting service, idempotency key) -> (event id, payload hash)`.
    pub idempotency: Mutex<HashMap<(String, String), (AuditEventId, String)>>,
    /// One-shot repository failure.
    pub failure: Mutex<Option<RepositoryError>>,
    /// One-shot failure on `append` only, so a read can succeed first.
    pub fail_append: Mutex<Option<RepositoryError>>,
}

impl FakeEvents {
    /// Makes the next call fail.
    pub fn fail_next(&self, error: RepositoryError) {
        *self.failure.lock().unwrap() = Some(error);
    }

    /// Makes the next append fail, leaving reads unaffected.
    pub fn fail_next_append(&self, error: RepositoryError) {
        *self.fail_append.lock().unwrap() = Some(error);
    }

    /// Stored events, oldest first.
    pub fn stored(&self) -> Vec<AuditEvent> {
        self.events.lock().unwrap().clone()
    }

    /// Seeds an event directly.
    pub fn seed(&self, event: AuditEvent) {
        self.events.lock().unwrap().push(event);
    }

    fn maybe_fail(&self) -> Result<(), RepositoryError> {
        match self.failure.lock().unwrap().take() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

#[async_trait]
impl AuditEventRepository for FakeEvents {
    async fn append(
        &self,
        event: &AuditEvent,
        emitting_service: &str,
        payload_sha256: &str,
    ) -> Result<AppendOutcome, RepositoryError> {
        if let Some(error) = self.fail_append.lock().unwrap().take() {
            return Err(error);
        }
        self.maybe_fail()?;
        let mut idempotency = self.idempotency.lock().unwrap();
        if let Some(key) = event.idempotency_key() {
            let map_key = (emitting_service.to_string(), key.to_string());
            if let Some((event_id, existing)) = idempotency.get(&map_key) {
                return Ok(AppendOutcome::Duplicate {
                    event_id: *event_id,
                    same_payload: existing == payload_sha256,
                });
            }
            idempotency.insert(map_key, (event.id(), payload_sha256.to_string()));
        }
        drop(idempotency);
        self.events.lock().unwrap().push(event.clone());
        Ok(AppendOutcome::Appended)
    }

    async fn find_by_id(&self, id: AuditEventId) -> Result<Option<AuditEvent>, RepositoryError> {
        self.maybe_fail()?;
        Ok(self
            .events
            .lock()
            .unwrap()
            .iter()
            .find(|event| event.id() == id)
            .cloned())
    }

    async fn query(&self, query: &EventQuery) -> Result<Vec<AuditEvent>, RepositoryError> {
        self.maybe_fail()?;
        let mut rows: Vec<AuditEvent> = self
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|event| {
                matches(
                    event,
                    query.action_prefix.as_deref(),
                    query.target.as_deref(),
                    query.sensitivity,
                    query.occurred_from,
                    query.occurred_to,
                    query.recorded_from,
                    query.recorded_to,
                )
            })
            .cloned()
            .collect();
        rows.sort_by(|left, right| {
            right
                .occurred_at()
                .cmp(&left.occurred_at())
                .then(right.id().cmp(&left.id()))
        });
        if let Some(cursor) = query.cursor {
            rows.retain(|event| {
                (event.occurred_at(), event.id()) < (cursor.occurred_at, cursor.id)
            });
        }
        rows.truncate(query.limit as usize);
        Ok(rows)
    }

    async fn count_for_export(
        &self,
        filter: &ExportFilter,
        cap: u64,
    ) -> Result<u64, RepositoryError> {
        self.maybe_fail()?;
        let count = self
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|event| matches_export(event, filter))
            .count() as u64;
        Ok(count.min(cap))
    }

    async fn export_batch(
        &self,
        filter: &ExportFilter,
        after: Option<Cursor>,
        limit: u32,
    ) -> Result<Vec<AuditEvent>, RepositoryError> {
        self.maybe_fail()?;
        let mut rows: Vec<AuditEvent> = self
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|event| matches_export(event, filter))
            .cloned()
            .collect();
        rows.sort_by(|left, right| {
            left.occurred_at()
                .cmp(&right.occurred_at())
                .then(left.id().cmp(&right.id()))
        });
        if let Some(cursor) = after {
            rows.retain(|event| {
                (event.occurred_at(), event.id()) > (cursor.occurred_at, cursor.id)
            });
        }
        rows.truncate(limit as usize);
        Ok(rows)
    }
}

/// In-memory support sessions.
#[derive(Default)]
pub struct FakeSupport {
    /// Stored sessions.
    pub sessions: Mutex<Vec<SupportSession>>,
    /// One-shot repository failure.
    pub failure: Mutex<Option<RepositoryError>>,
}

impl FakeSupport {
    /// Stores a session directly.
    pub fn seed(&self, session: SupportSession) {
        self.sessions.lock().unwrap().push(session);
    }

    fn maybe_fail(&self) -> Result<(), RepositoryError> {
        match self.failure.lock().unwrap().take() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

#[async_trait]
impl SupportSessionRepository for FakeSupport {
    async fn insert(&self, session: &SupportSession) -> Result<(), RepositoryError> {
        self.maybe_fail()?;
        self.sessions.lock().unwrap().push(session.clone());
        Ok(())
    }

    async fn find_by_id(
        &self,
        id: SupportSessionId,
    ) -> Result<Option<SupportSession>, RepositoryError> {
        self.maybe_fail()?;
        Ok(self
            .sessions
            .lock()
            .unwrap()
            .iter()
            .find(|session| session.id() == id)
            .cloned())
    }

    async fn update(&self, session: &SupportSession) -> Result<(), RepositoryError> {
        self.maybe_fail()?;
        let mut sessions = self.sessions.lock().unwrap();
        let existing = sessions
            .iter_mut()
            .find(|existing| existing.id() == session.id())
            .ok_or_else(|| RepositoryError::Conflict("session not found".to_string()))?;
        *existing = session.clone();
        Ok(())
    }
}

/// In-memory export requests.
#[derive(Default)]
pub struct FakeExports {
    /// Stored exports.
    pub exports: Mutex<Vec<AuditExport>>,
    /// One-shot repository failure.
    pub failure: Mutex<Option<RepositoryError>>,
}

impl FakeExports {
    /// The stored exports.
    pub fn stored(&self) -> Vec<AuditExport> {
        self.exports.lock().unwrap().clone()
    }

    fn maybe_fail(&self) -> Result<(), RepositoryError> {
        match self.failure.lock().unwrap().take() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

#[async_trait]
impl ExportRepository for FakeExports {
    async fn insert(&self, export: &AuditExport) -> Result<(), RepositoryError> {
        self.maybe_fail()?;
        self.exports.lock().unwrap().push(export.clone());
        Ok(())
    }

    async fn update(&self, export: &AuditExport) -> Result<(), RepositoryError> {
        self.maybe_fail()?;
        let mut exports = self.exports.lock().unwrap();
        let existing = exports
            .iter_mut()
            .find(|existing| existing.id() == export.id())
            .ok_or_else(|| RepositoryError::Conflict("export not found".to_string()))?;
        *existing = export.clone();
        Ok(())
    }

    async fn mark_stale_failed(
        &self,
        before: DateTime<Utc>,
        at: DateTime<Utc>,
    ) -> Result<u64, RepositoryError> {
        self.maybe_fail()?;
        let mut exports = self.exports.lock().unwrap();
        let mut recovered = 0;
        for export in exports.iter_mut() {
            if export.requested_at() < before && export.state().as_str() == "requested" {
                *export = export.clone().fail(at).expect("requested export can fail");
                recovered += 1;
            }
        }
        Ok(recovered)
    }
}

/// In-memory retention state.
#[derive(Default)]
pub struct FakeRetention {
    /// Stored runs.
    pub runs: Mutex<Vec<RetentionRun>>,
    /// Partition summaries the sweep sees.
    pub partitions: Mutex<Vec<PartitionSummary>>,
    /// Partitions the sweep dropped.
    pub dropped: Mutex<Vec<String>>,
    /// The batch sizes the sweep's `delete_batch` calls returned.
    pub delete_batches: Mutex<Vec<u64>>,
    /// One-shot failure on insert.
    pub fail_insert: Mutex<Option<RepositoryError>>,
    /// One-shot failure on the partition listing.
    pub fail_partitions: Mutex<Option<RepositoryError>>,
    /// Months-ahead values passed to `ensure_partitions_ahead`.
    pub ensured: Mutex<Vec<u32>>,
}

impl FakeRetention {
    /// Sets the batch results, consumed one per call.
    pub fn with_batches(self, batches: Vec<u64>) -> Self {
        *self.delete_batches.lock().unwrap() = batches;
        self
    }

    /// Sets the partitions the sweep sees.
    pub fn with_partitions(self, partitions: Vec<PartitionSummary>) -> Self {
        *self.partitions.lock().unwrap() = partitions;
        self
    }

    /// The stored runs.
    pub fn stored(&self) -> Vec<RetentionRun> {
        self.runs.lock().unwrap().clone()
    }
}

#[async_trait]
impl RetentionRepository for FakeRetention {
    async fn insert_run(&self, run: &RetentionRun) -> Result<(), RepositoryError> {
        if let Some(error) = self.fail_insert.lock().unwrap().take() {
            return Err(error);
        }
        self.runs.lock().unwrap().push(run.clone());
        Ok(())
    }

    async fn update_run(&self, run: &RetentionRun) -> Result<(), RepositoryError> {
        let mut runs = self.runs.lock().unwrap();
        let existing = runs
            .iter_mut()
            .find(|existing| existing.id() == run.id())
            .ok_or_else(|| RepositoryError::Conflict("run not found".to_string()))?;
        *existing = run.clone();
        Ok(())
    }

    async fn find_run(&self, id: RetentionRunId) -> Result<Option<RetentionRun>, RepositoryError> {
        Ok(self
            .runs
            .lock()
            .unwrap()
            .iter()
            .find(|run| run.id() == id)
            .cloned())
    }

    async fn partition_summaries(&self) -> Result<Vec<PartitionSummary>, RepositoryError> {
        if let Some(error) = self.fail_partitions.lock().unwrap().take() {
            return Err(error);
        }
        Ok(self.partitions.lock().unwrap().clone())
    }

    async fn drop_partition(&self, name: &str) -> Result<(), RepositoryError> {
        self.dropped.lock().unwrap().push(name.to_string());
        Ok(())
    }

    async fn delete_batch(
        &self,
        _cutoff: DateTime<Utc>,
        _limit: u32,
    ) -> Result<u64, RepositoryError> {
        let mut batches = self.delete_batches.lock().unwrap();
        if batches.is_empty() {
            Ok(0)
        } else {
            Ok(batches.remove(0))
        }
    }

    async fn ensure_partitions_ahead(
        &self,
        months_ahead: u32,
    ) -> Result<Vec<String>, RepositoryError> {
        self.ensured.lock().unwrap().push(months_ahead);
        Ok(vec!["audit_events_202611".to_string()])
    }
}

/// In-memory privacy deletions.
#[derive(Default)]
pub struct FakePrivacy {
    /// Stored deletions.
    pub deletions: Mutex<Vec<PrivacyDeletion>>,
    /// Labels per subject, keyed by the subject's UUID string.
    pub labels: Mutex<HashMap<String, Vec<EventLabel>>>,
    /// The count `delete_unlabelled_for_subject` returns; `None` derives it.
    pub delete_count: Mutex<Option<u64>>,
    /// One-shot failure on insert.
    pub fail_insert: Mutex<Option<RepositoryError>>,
}

impl FakePrivacy {
    /// Seeds the subject's event labels.
    pub fn with_labels(self, subject: UserId, labels: Vec<EventLabel>) -> Self {
        self.labels
            .lock()
            .unwrap()
            .insert(subject.to_string(), labels);
        self
    }

    /// Overrides what the delete call returns.
    pub fn with_delete_count(self, count: u64) -> Self {
        *self.delete_count.lock().unwrap() = Some(count);
        self
    }

    /// The stored deletions.
    pub fn stored(&self) -> Vec<PrivacyDeletion> {
        self.deletions.lock().unwrap().clone()
    }
}

#[async_trait]
impl PrivacyDeletionRepository for FakePrivacy {
    async fn insert(&self, deletion: &PrivacyDeletion) -> Result<(), RepositoryError> {
        if let Some(error) = self.fail_insert.lock().unwrap().take() {
            return Err(error);
        }
        let mut deletions = self.deletions.lock().unwrap();
        if deletions
            .iter()
            .any(|existing| existing.deletion_request_id() == deletion.deletion_request_id())
        {
            return Err(RepositoryError::Conflict(
                "deletion request already recorded".to_string(),
            ));
        }
        deletions.push(deletion.clone());
        Ok(())
    }

    async fn update(&self, deletion: &PrivacyDeletion) -> Result<(), RepositoryError> {
        let mut deletions = self.deletions.lock().unwrap();
        let existing = deletions
            .iter_mut()
            .find(|existing| existing.id() == deletion.id())
            .ok_or_else(|| RepositoryError::Conflict("deletion not found".to_string()))?;
        *existing = deletion.clone();
        Ok(())
    }

    async fn find_by_request(
        &self,
        request: DeletionRequestId,
    ) -> Result<Option<PrivacyDeletion>, RepositoryError> {
        Ok(self
            .deletions
            .lock()
            .unwrap()
            .iter()
            .find(|deletion| deletion.deletion_request_id() == request)
            .cloned())
    }

    async fn next_requested(&self, limit: u32) -> Result<Vec<PrivacyDeletion>, RepositoryError> {
        Ok(self
            .deletions
            .lock()
            .unwrap()
            .iter()
            .filter(|deletion| deletion.state().as_str() == "requested")
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn labels_for_subject(
        &self,
        subject: UserId,
        limit: u32,
    ) -> Result<Vec<EventLabel>, RepositoryError> {
        Ok(self
            .labels
            .lock()
            .unwrap()
            .get(&subject.to_string())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .take(limit as usize)
            .collect())
    }

    async fn delete_unlabelled_for_subject(&self, subject: UserId) -> Result<u64, RepositoryError> {
        if let Some(count) = *self.delete_count.lock().unwrap() {
            return Ok(count);
        }
        let derived = self
            .labels
            .lock()
            .unwrap()
            .get(&subject.to_string())
            .map(|labels| {
                labels
                    .iter()
                    .filter(|label| label.legal_basis.is_none())
                    .count() as u64
            })
            .unwrap_or(0);
        Ok(derived)
    }
}

/// A clock frozen at one instant.
#[derive(Clone)]
pub struct FixedClock {
    /// The instant the clock reports.
    pub now: DateTime<Utc>,
}

impl FixedClock {
    /// Creates a clock at `now`.
    pub fn at(now: DateTime<Utc>) -> Self {
        Self { now }
    }
}

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.now
    }
}

/// A clock tests can move, so expiry boundaries are exercised without sleeping.
#[derive(Debug)]
pub struct MutableClock {
    now: Mutex<DateTime<Utc>>,
}

impl MutableClock {
    /// Creates a clock at `now`.
    pub fn at(now: DateTime<Utc>) -> Self {
        Self {
            now: Mutex::new(now),
        }
    }

    /// Moves the clock.
    pub fn set(&self, now: DateTime<Utc>) {
        *self.now.lock().unwrap() = now;
    }
}

impl Clock for MutableClock {
    fn now(&self) -> DateTime<Utc> {
        *self.now.lock().unwrap()
    }
}

/// A pause that never sleeps.
pub struct NoPause;

#[async_trait]
impl Pause for NoPause {
    async fn pause(&self) {}
}

/// Resolves test tokens to principals.
pub struct FakeTokenValidator {
    /// Token -> principal or failure.
    pub tokens: Mutex<HashMap<String, Result<Principal, AuthError>>>,
}

impl FakeTokenValidator {
    /// Creates a validator from pairs.
    pub fn with(entries: Vec<(&str, Result<Principal, AuthError>)>) -> Self {
        Self {
            tokens: Mutex::new(
                entries
                    .into_iter()
                    .map(|(token, principal)| (token.to_string(), principal))
                    .collect(),
            ),
        }
    }
}

#[async_trait]
impl TokenValidator for FakeTokenValidator {
    async fn validate(&self, token: &str) -> Result<Principal, AuthError> {
        match self.tokens.lock().unwrap().get(token) {
            Some(result) => result.clone(),
            None => Err(AuthError::Invalid("unknown test token".to_string())),
        }
    }
}

/// Resolves test service tokens to service names.
pub struct FakeServiceTokens {
    /// Token -> service name.
    pub tokens: Mutex<HashMap<String, String>>,
}

impl FakeServiceTokens {
    /// Creates a registry from pairs.
    pub fn with(pairs: &[(&str, &str)]) -> Self {
        Self {
            tokens: Mutex::new(
                pairs
                    .iter()
                    .map(|(token, service)| (token.to_string(), service.to_string()))
                    .collect(),
            ),
        }
    }
}

impl ServiceTokenRegistry for FakeServiceTokens {
    fn service_for(&self, token: &str) -> Option<String> {
        self.tokens.lock().unwrap().get(token.trim()).cloned()
    }
}

/// A readiness probe with a fixed answer.
pub struct Ready(pub bool);

#[async_trait]
impl ReadinessProbe for Ready {
    async fn check(&self) -> Result<(), String> {
        if self.0 {
            Ok(())
        } else {
            Err("database down".to_string())
        }
    }
}
