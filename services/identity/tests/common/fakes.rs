//! Port fakes for unit-level application and HTTP tests.
//!
//! Each fake is deterministic, records what the use case asked it to do, and can
//! be made to fail once so error paths are exercised without a database.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use identity_application::ports::{
    AuditClient, AuthAdmin, AuthError, Clock, ConsentRepository, ContributorGateway,
    DeletionRepository, DeviceSessionRepository, ExportRepository, ExportStore, ExternalError,
    GateGrant, GateTokenIssuer, JurisdictionRepository, ReadinessProbe, RepositoryError,
    ServiceTokenRegistry, ShreddingKeyRepository, SupportGrantRepository, TokenValidator,
    UserRepository, WakeTransport,
};
use identity_domain::{
    AuditRecord, ConsentPurpose, ConsentRecord, Contributor, DeletionProgress, DeletionRequest,
    DeletionRequestId, DeviceSession, DeviceSessionId, ExportRequest, ExportRequestId,
    JurisdictionRule, RetainedCategory, ShreddingKey, ShreddingKeyId, ShreddingKeyStatus,
    SupportAccessGrant, SupportGrantId, User, UserId, VerifiedClaims,
};
use uuid::Uuid;

fn maybe_fail(failure: &Mutex<Option<RepositoryError>>) -> Result<(), RepositoryError> {
    match failure.lock().unwrap().take() {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// In-memory users with the subject mapping.
#[derive(Default)]
pub struct FakeUsers {
    /// Stored users.
    pub users: Mutex<BTreeMap<Uuid, User>>,
    /// Supabase subject -> internal id.
    pub subjects: Mutex<BTreeMap<String, Uuid>>,
    /// Internal id -> revealed subject.
    pub revealed: Mutex<BTreeMap<Uuid, String>>,
    /// One-shot repository failure.
    pub failure: Mutex<Option<RepositoryError>>,
}

impl FakeUsers {
    /// Stores a user and its subject directly.
    pub fn seed(&self, user: User, subject: &str) {
        self.users
            .lock()
            .unwrap()
            .insert(user.id().as_uuid(), user.clone());
        self.subjects
            .lock()
            .unwrap()
            .insert(subject.to_string(), user.id().as_uuid());
        self.revealed
            .lock()
            .unwrap()
            .insert(user.id().as_uuid(), subject.to_string());
    }

    /// Reads a stored user.
    pub fn get(&self, id: UserId) -> Option<User> {
        self.users.lock().unwrap().get(&id.as_uuid()).cloned()
    }

    /// The number of stored users.
    pub fn len(&self) -> usize {
        self.users.lock().unwrap().len()
    }

    /// Makes the next call fail.
    pub fn fail_next(&self, error: RepositoryError) {
        *self.failure.lock().unwrap() = Some(error);
    }
}

#[async_trait]
impl UserRepository for FakeUsers {
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, RepositoryError> {
        maybe_fail(&self.failure)?;
        Ok(self.get(id))
    }

    async fn find_by_auth_subject(
        &self,
        auth_subject: &str,
    ) -> Result<Option<User>, RepositoryError> {
        maybe_fail(&self.failure)?;
        let id = self.subjects.lock().unwrap().get(auth_subject).copied();
        Ok(id.and_then(|id| self.get(UserId::new(id))))
    }

    async fn insert(
        &self,
        user: &User,
        auth_subject: &str,
        _key: &ShreddingKey,
    ) -> Result<(), RepositoryError> {
        maybe_fail(&self.failure)?;
        self.seed(user.clone(), auth_subject);
        Ok(())
    }

    async fn update(&self, user: &User) -> Result<(), RepositoryError> {
        maybe_fail(&self.failure)?;
        self.users
            .lock()
            .unwrap()
            .insert(user.id().as_uuid(), user.clone());
        Ok(())
    }

    async fn reveal_auth_subject(
        &self,
        user_id: UserId,
    ) -> Result<Option<String>, RepositoryError> {
        maybe_fail(&self.failure)?;
        Ok(self
            .revealed
            .lock()
            .unwrap()
            .get(&user_id.as_uuid())
            .cloned())
    }

    async fn anonymise(&self, user: &User) -> Result<(), RepositoryError> {
        maybe_fail(&self.failure)?;
        let id = user.id().as_uuid();
        if let Some(subject) = self.revealed.lock().unwrap().remove(&id) {
            self.subjects.lock().unwrap().remove(&subject);
        }
        self.users.lock().unwrap().insert(id, user.clone());
        Ok(())
    }
}

/// In-memory shredding keys.
#[derive(Default)]
pub struct FakeKeys {
    /// Stored keys.
    pub keys: Mutex<BTreeMap<Uuid, ShreddingKey>>,
    /// The next pgsodium key id.
    pub next_key: AtomicI64,
    /// One-shot repository failure.
    pub failure: Mutex<Option<RepositoryError>>,
}

impl FakeKeys {
    /// Reads a stored key.
    pub fn get(&self, id: ShreddingKeyId) -> Option<ShreddingKey> {
        self.keys.lock().unwrap().get(&id.as_uuid()).cloned()
    }

    /// The key id handed out to a user.
    pub fn key_for(&self, user_id: UserId) -> Option<ShreddingKey> {
        self.keys
            .lock()
            .unwrap()
            .values()
            .find(|key| key.user_id() == user_id)
            .cloned()
    }
}

#[async_trait]
impl ShreddingKeyRepository for FakeKeys {
    async fn create_for_user(
        &self,
        user_id: UserId,
        now: DateTime<Utc>,
    ) -> Result<ShreddingKey, RepositoryError> {
        maybe_fail(&self.failure)?;
        let key_id = self.next_key.fetch_add(1, Ordering::Relaxed) + 1;
        let key = ShreddingKey::from_stored(
            ShreddingKeyId::generate(),
            user_id,
            format!("identity/user/{user_id}"),
            key_id,
            ShreddingKeyStatus::Active,
            now,
            None,
        );
        self.keys
            .lock()
            .unwrap()
            .insert(key.id().as_uuid(), key.clone());
        Ok(key)
    }

    async fn active_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Option<ShreddingKey>, RepositoryError> {
        maybe_fail(&self.failure)?;
        Ok(self
            .keys
            .lock()
            .unwrap()
            .values()
            .find(|key| key.user_id() == user_id && key.status() == ShreddingKeyStatus::Active)
            .cloned())
    }

    async fn destroy(
        &self,
        key: ShreddingKeyId,
        now: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        maybe_fail(&self.failure)?;
        let mut keys = self.keys.lock().unwrap();
        let existing = keys
            .get(&key.as_uuid())
            .cloned()
            .ok_or_else(|| RepositoryError::Conflict("key not found".to_string()))?;
        let destroyed = ShreddingKey::from_stored(
            existing.id(),
            existing.user_id(),
            existing.vault_reference().to_string(),
            existing.key_id(),
            ShreddingKeyStatus::Destroyed,
            existing.created_at(),
            Some(now),
        );
        keys.insert(key.as_uuid(), destroyed);
        Ok(())
    }
}

/// In-memory consent history.
#[derive(Default)]
pub struct FakeConsent {
    /// Stored records.
    pub records: Mutex<Vec<ConsentRecord>>,
    /// One-shot repository failure.
    pub failure: Mutex<Option<RepositoryError>>,
}

#[async_trait]
impl ConsentRepository for FakeConsent {
    async fn append(&self, record: &ConsentRecord) -> Result<(), RepositoryError> {
        maybe_fail(&self.failure)?;
        self.records.lock().unwrap().push(record.clone());
        Ok(())
    }

    async fn latest(
        &self,
        user_id: UserId,
        purpose: ConsentPurpose,
    ) -> Result<Option<ConsentRecord>, RepositoryError> {
        maybe_fail(&self.failure)?;
        Ok(self
            .records
            .lock()
            .unwrap()
            .iter()
            .filter(|record| record.user_id() == user_id && record.purpose() == purpose)
            .max_by_key(|record| record.recorded_at())
            .cloned())
    }

    async fn delete_for_user(&self, user_id: UserId) -> Result<u64, RepositoryError> {
        maybe_fail(&self.failure)?;
        let mut records = self.records.lock().unwrap();
        let before = records.len();
        records.retain(|record| record.user_id() != user_id);
        Ok((before - records.len()) as u64)
    }
}

/// In-memory support grants.
#[derive(Default)]
pub struct FakeGrants {
    /// Stored grants.
    pub grants: Mutex<Vec<SupportAccessGrant>>,
    /// One-shot repository failure.
    pub failure: Mutex<Option<RepositoryError>>,
}

#[async_trait]
impl SupportGrantRepository for FakeGrants {
    async fn insert(&self, grant: &SupportAccessGrant) -> Result<(), RepositoryError> {
        maybe_fail(&self.failure)?;
        self.grants.lock().unwrap().push(grant.clone());
        Ok(())
    }

    async fn find_by_id(
        &self,
        id: SupportGrantId,
    ) -> Result<Option<SupportAccessGrant>, RepositoryError> {
        maybe_fail(&self.failure)?;
        Ok(self
            .grants
            .lock()
            .unwrap()
            .iter()
            .find(|grant| grant.id() == id)
            .cloned())
    }

    async fn update(&self, grant: &SupportAccessGrant) -> Result<(), RepositoryError> {
        maybe_fail(&self.failure)?;
        let mut grants = self.grants.lock().unwrap();
        let existing = grants
            .iter_mut()
            .find(|existing| existing.id() == grant.id())
            .ok_or_else(|| RepositoryError::Conflict("grant not found".to_string()))?;
        *existing = grant.clone();
        Ok(())
    }
}

/// In-memory device sessions.
#[derive(Default)]
pub struct FakeSessions {
    /// Stored sessions.
    pub sessions: Mutex<Vec<DeviceSession>>,
    /// One-shot repository failure.
    pub failure: Mutex<Option<RepositoryError>>,
}

#[async_trait]
impl DeviceSessionRepository for FakeSessions {
    async fn upsert(&self, session: &DeviceSession) -> Result<(), RepositoryError> {
        maybe_fail(&self.failure)?;
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(existing) = sessions.iter_mut().find(|existing| {
            existing.user_id() == session.user_id()
                && existing.auth_session_id() == session.auth_session_id()
        }) {
            // Mirror the Postgres upsert: the row keeps its id, the location is
            // replaced, and an existing revocation is never cleared.
            *existing = DeviceSession::from_stored(
                existing.id(),
                session.user_id(),
                session.auth_session_id().to_string(),
                session
                    .device()
                    .map(ToString::to_string)
                    .or_else(|| existing.device().map(ToString::to_string)),
                session.location().map(ToString::to_string),
                existing.last_activity_at().max(session.last_activity_at()),
                existing.revoked_at().or_else(|| session.revoked_at()),
            );
        } else {
            sessions.push(session.clone());
        }
        Ok(())
    }

    async fn list_for_user(&self, user_id: UserId) -> Result<Vec<DeviceSession>, RepositoryError> {
        maybe_fail(&self.failure)?;
        let mut sessions: Vec<DeviceSession> = self
            .sessions
            .lock()
            .unwrap()
            .iter()
            .filter(|session| session.user_id() == user_id)
            .cloned()
            .collect();
        sessions.sort_by_key(|session| std::cmp::Reverse(session.last_activity_at()));
        Ok(sessions)
    }

    async fn find(&self, id: DeviceSessionId) -> Result<Option<DeviceSession>, RepositoryError> {
        maybe_fail(&self.failure)?;
        Ok(self
            .sessions
            .lock()
            .unwrap()
            .iter()
            .find(|session| session.id() == id)
            .cloned())
    }

    async fn update(&self, session: &DeviceSession) -> Result<(), RepositoryError> {
        maybe_fail(&self.failure)?;
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
    pub exports: Mutex<Vec<ExportRequest>>,
    /// One-shot repository failure.
    pub failure: Mutex<Option<RepositoryError>>,
}

impl FakeExports {
    /// The stored exports.
    pub fn stored(&self) -> Vec<ExportRequest> {
        self.exports.lock().unwrap().clone()
    }
}

#[async_trait]
impl ExportRepository for FakeExports {
    async fn insert(&self, export: &ExportRequest) -> Result<(), RepositoryError> {
        maybe_fail(&self.failure)?;
        self.exports.lock().unwrap().push(export.clone());
        Ok(())
    }

    async fn find_by_id(
        &self,
        id: ExportRequestId,
    ) -> Result<Option<ExportRequest>, RepositoryError> {
        maybe_fail(&self.failure)?;
        Ok(self
            .exports
            .lock()
            .unwrap()
            .iter()
            .find(|export| export.id() == id)
            .cloned())
    }

    async fn update(&self, export: &ExportRequest) -> Result<(), RepositoryError> {
        maybe_fail(&self.failure)?;
        let mut exports = self.exports.lock().unwrap();
        let existing = exports
            .iter_mut()
            .find(|existing| existing.id() == export.id())
            .ok_or_else(|| RepositoryError::Conflict("export not found".to_string()))?;
        *existing = export.clone();
        Ok(())
    }

    async fn find_active_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Option<ExportRequest>, RepositoryError> {
        maybe_fail(&self.failure)?;
        Ok(self
            .exports
            .lock()
            .unwrap()
            .iter()
            .filter(|export| {
                export.user_id() == user_id
                    && matches!(
                        export.status(),
                        identity_domain::ExportStatus::Pending
                            | identity_domain::ExportStatus::Building
                            | identity_domain::ExportStatus::Ready
                    )
            })
            .max_by_key(|export| export.requested_at())
            .cloned())
    }

    async fn claim_next(
        &self,
        worker: &str,
        now: DateTime<Utc>,
        lease: Duration,
    ) -> Result<Option<ExportRequest>, RepositoryError> {
        maybe_fail(&self.failure)?;
        let mut exports = self.exports.lock().unwrap();
        let Some(export) = exports
            .iter_mut()
            .filter(|export| export.is_claimable(now))
            .min_by_key(|export| export.requested_at())
        else {
            return Ok(None);
        };
        export
            .claim(worker, now, lease)
            .map_err(|error| RepositoryError::Conflict(format!("export claim refused: {error}")))?;
        Ok(Some(export.clone()))
    }

    async fn list_expired_ready(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<ExportRequest>, RepositoryError> {
        maybe_fail(&self.failure)?;
        Ok(self
            .exports
            .lock()
            .unwrap()
            .iter()
            .filter(|export| {
                export.status() == identity_domain::ExportStatus::Ready
                    && export
                        .download_expires_at()
                        .is_some_and(|expires| now >= expires)
            })
            .cloned()
            .collect())
    }
}

/// In-memory deletion requests.
#[derive(Default)]
pub struct FakeDeletions {
    /// Stored deletions.
    pub deletions: Mutex<Vec<DeletionRequest>>,
    /// One-shot repository failure.
    pub failure: Mutex<Option<RepositoryError>>,
}

impl FakeDeletions {
    /// The stored deletions.
    pub fn stored(&self) -> Vec<DeletionRequest> {
        self.deletions.lock().unwrap().clone()
    }
}

#[async_trait]
impl DeletionRepository for FakeDeletions {
    async fn insert(&self, deletion: &DeletionRequest) -> Result<(), RepositoryError> {
        maybe_fail(&self.failure)?;
        self.deletions.lock().unwrap().push(deletion.clone());
        Ok(())
    }

    async fn find_by_id(
        &self,
        id: DeletionRequestId,
    ) -> Result<Option<DeletionRequest>, RepositoryError> {
        maybe_fail(&self.failure)?;
        Ok(self
            .deletions
            .lock()
            .unwrap()
            .iter()
            .find(|deletion| deletion.id() == id)
            .cloned())
    }

    async fn update(&self, deletion: &DeletionRequest) -> Result<(), RepositoryError> {
        maybe_fail(&self.failure)?;
        let mut deletions = self.deletions.lock().unwrap();
        let existing = deletions
            .iter_mut()
            .find(|existing| existing.id() == deletion.id())
            .ok_or_else(|| RepositoryError::Conflict("deletion not found".to_string()))?;
        *existing = deletion.clone();
        Ok(())
    }

    async fn find_active_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Option<DeletionRequest>, RepositoryError> {
        maybe_fail(&self.failure)?;
        Ok(self
            .deletions
            .lock()
            .unwrap()
            .iter()
            .filter(|deletion| {
                deletion.user_id() == user_id
                    && matches!(
                        deletion.status(),
                        identity_domain::DeletionStatus::Requested
                            | identity_domain::DeletionStatus::InProgress
                    )
            })
            .max_by_key(|deletion| deletion.requested_at())
            .cloned())
    }

    async fn claim_next(
        &self,
        worker: &str,
        now: DateTime<Utc>,
        lease: Duration,
    ) -> Result<Option<DeletionRequest>, RepositoryError> {
        maybe_fail(&self.failure)?;
        let mut deletions = self.deletions.lock().unwrap();
        let Some(deletion) = deletions
            .iter_mut()
            .filter(|deletion| deletion.is_claimable(now))
            .min_by_key(|deletion| deletion.requested_at())
        else {
            return Ok(None);
        };
        deletion.claim(worker, now, lease).map_err(|error| {
            RepositoryError::Conflict(format!("deletion claim refused: {error}"))
        })?;
        Ok(Some(deletion.clone()))
    }
}

/// In-memory jurisdiction rules.
#[derive(Default)]
pub struct FakeRules {
    /// Stored rules.
    pub rules: Mutex<Vec<JurisdictionRule>>,
}

impl FakeRules {
    /// Seeds a rule.
    pub fn seed(&self, rule: JurisdictionRule) {
        self.rules.lock().unwrap().push(rule);
    }
}

#[async_trait]
impl JurisdictionRepository for FakeRules {
    async fn find_by_code(&self, code: &str) -> Result<Option<JurisdictionRule>, RepositoryError> {
        Ok(self
            .rules
            .lock()
            .unwrap()
            .iter()
            .find(|rule| rule.code.as_str() == code)
            .cloned())
    }
}

/// In-memory audit sink.
#[derive(Default)]
pub struct FakeAudit {
    /// Recorded events with their correlation id.
    pub records: Mutex<Vec<(AuditRecord, String)>>,
    /// One-shot failure.
    pub failure: Mutex<Option<RepositoryError>>,
}

impl FakeAudit {
    /// The recorded actions.
    pub fn actions(&self) -> Vec<String> {
        self.records
            .lock()
            .unwrap()
            .iter()
            .map(|(record, _)| record.action().to_string())
            .collect()
    }

    /// Makes the next append fail.
    pub fn fail_next(&self) {
        *self.failure.lock().unwrap() =
            Some(RepositoryError::Unavailable("audit timed out".to_string()));
    }
}

#[async_trait]
impl AuditClient for FakeAudit {
    async fn append(
        &self,
        record: &AuditRecord,
        correlation_id: &str,
    ) -> Result<(), RepositoryError> {
        maybe_fail(&self.failure)?;
        self.records
            .lock()
            .unwrap()
            .push((record.clone(), correlation_id.to_string()));
        Ok(())
    }
}

/// In-memory Supabase Auth admin.
#[derive(Default)]
pub struct FakeAuthAdmin {
    /// Revoked `(subject, session)` pairs.
    pub revoked: Mutex<Vec<(String, String)>>,
    /// Deleted subjects.
    pub deleted: Mutex<Vec<String>>,
    /// One-shot failure.
    pub failure: Mutex<Option<ExternalError>>,
}

#[async_trait]
impl AuthAdmin for FakeAuthAdmin {
    async fn revoke_session(
        &self,
        auth_subject: &str,
        session_id: &str,
    ) -> Result<(), ExternalError> {
        if let Some(error) = self.failure.lock().unwrap().take() {
            return Err(error);
        }
        self.revoked
            .lock()
            .unwrap()
            .push((auth_subject.to_string(), session_id.to_string()));
        Ok(())
    }

    async fn delete_user(&self, auth_subject: &str) -> Result<(), ExternalError> {
        if let Some(error) = self.failure.lock().unwrap().take() {
            return Err(error);
        }
        self.deleted.lock().unwrap().push(auth_subject.to_string());
        Ok(())
    }
}

/// In-memory privacy contributors.
#[derive(Default)]
pub struct FakeContributors {
    /// Export payload per contributor.
    pub exports: Mutex<BTreeMap<String, serde_json::Value>>,
    /// The contributor whose export fails next.
    pub fail_export: Mutex<Option<Contributor>>,
    /// The contributor whose anonymise fails next.
    pub fail_anonymise: Mutex<Option<Contributor>>,
    /// Export call order.
    pub export_calls: Mutex<Vec<Contributor>>,
    /// Anonymise call order.
    pub anonymise_calls: Mutex<Vec<Contributor>>,
    /// Retained categories per contributor.
    pub retained: Mutex<BTreeMap<String, Vec<RetainedCategory>>>,
}

impl FakeContributors {
    /// Seeds one contributor's export payload.
    pub fn with_export(&self, contributor: Contributor, value: serde_json::Value) {
        self.exports
            .lock()
            .unwrap()
            .insert(contributor.as_str().to_string(), value);
    }

    /// Seeds one contributor's retained categories.
    pub fn with_retained(&self, contributor: Contributor, categories: Vec<RetainedCategory>) {
        self.retained
            .lock()
            .unwrap()
            .insert(contributor.as_str().to_string(), categories);
    }
}

#[async_trait]
impl ContributorGateway for FakeContributors {
    async fn export(
        &self,
        contributor: Contributor,
        _user_id: UserId,
    ) -> Result<serde_json::Value, ExternalError> {
        self.export_calls.lock().unwrap().push(contributor);
        if *self.fail_export.lock().unwrap() == Some(contributor) {
            return Err(ExternalError::Unavailable(
                "contributor timed out".to_string(),
            ));
        }
        Ok(self
            .exports
            .lock()
            .unwrap()
            .get(contributor.as_str())
            .cloned()
            .unwrap_or(serde_json::json!({})))
    }

    async fn anonymise(
        &self,
        contributor: Contributor,
        _user_id: UserId,
        _idempotency_key: &str,
    ) -> Result<Vec<RetainedCategory>, ExternalError> {
        self.anonymise_calls.lock().unwrap().push(contributor);
        if *self.fail_anonymise.lock().unwrap() == Some(contributor) {
            return Err(ExternalError::Unavailable(
                "contributor timed out".to_string(),
            ));
        }
        Ok(self
            .retained
            .lock()
            .unwrap()
            .get(contributor.as_str())
            .cloned()
            .unwrap_or_default())
    }
}

/// In-memory export archive store.
#[derive(Default)]
pub struct FakeStore {
    /// Stored objects `(key, body)`.
    pub objects: Mutex<Vec<(String, Vec<u8>)>>,
    /// Deleted keys.
    pub deleted: Mutex<Vec<String>>,
    /// One-shot failure.
    pub failure: Mutex<Option<ExternalError>>,
}

#[async_trait]
impl ExportStore for FakeStore {
    async fn put(&self, key: &str, body: Vec<u8>) -> Result<(), ExternalError> {
        if let Some(error) = self.failure.lock().unwrap().take() {
            return Err(error);
        }
        self.objects.lock().unwrap().push((key.to_string(), body));
        Ok(())
    }

    async fn signed_url(&self, key: &str, _valid_for: Duration) -> Result<String, ExternalError> {
        if let Some(error) = self.failure.lock().unwrap().take() {
            return Err(error);
        }
        Ok(format!("https://storage.test/{key}?signature=fake"))
    }

    async fn delete(&self, key: &str) -> Result<(), ExternalError> {
        if let Some(error) = self.failure.lock().unwrap().take() {
            return Err(error);
        }
        self.deleted.lock().unwrap().push(key.to_string());
        Ok(())
    }
}

/// A wake transport that records calls.
#[derive(Default)]
pub struct FakeWake {
    /// Number of wake calls.
    pub calls: Mutex<usize>,
    /// Whether the next wake fails.
    pub fail: Mutex<bool>,
}

#[async_trait]
impl WakeTransport for FakeWake {
    async fn wake(&self) -> Result<(), String> {
        *self.calls.lock().unwrap() += 1;
        if *self.fail.lock().unwrap() {
            return Err("worker unavailable".to_string());
        }
        Ok(())
    }
}

/// A gate-token issuer with a deterministic token.
pub struct FakeGateIssuer {
    /// Issued jurisdictions.
    pub issued: Mutex<Vec<String>>,
    /// Whether issuing fails.
    pub fail: Mutex<bool>,
}

impl Default for FakeGateIssuer {
    fn default() -> Self {
        Self {
            issued: Mutex::new(Vec::new()),
            fail: Mutex::new(false),
        }
    }
}

impl GateTokenIssuer for FakeGateIssuer {
    fn issue(&self, jurisdiction: &str, now: DateTime<Utc>) -> Result<String, String> {
        if *self.fail.lock().unwrap() {
            return Err("gate signing key unavailable".to_string());
        }
        self.issued.lock().unwrap().push(jurisdiction.to_string());
        Ok(format!("gate-token-{jurisdiction}-{}", now.timestamp()))
    }

    fn verify(&self, token: &str, now: DateTime<Utc>) -> Result<GateGrant, String> {
        let jurisdiction = token
            .strip_prefix("gate-token-")
            .and_then(|rest| rest.split('-').next())
            .ok_or_else(|| "invalid gate token".to_string())?;
        Ok(GateGrant {
            jurisdiction: jurisdiction.to_string(),
            issued_at: now,
            expires_at: now + Duration::minutes(10),
        })
    }
}

/// A clock frozen at one instant and movable by a test.
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

/// Resolves test tokens to validated claims.
pub struct FakeTokenValidator {
    /// Token -> claims or failure.
    pub tokens: Mutex<BTreeMap<String, Result<VerifiedClaims, AuthError>>>,
}

impl FakeTokenValidator {
    /// Creates a validator from pairs.
    pub fn with(entries: Vec<(&str, Result<VerifiedClaims, AuthError>)>) -> Self {
        Self {
            tokens: Mutex::new(
                entries
                    .into_iter()
                    .map(|(token, claims)| (token.to_string(), claims))
                    .collect(),
            ),
        }
    }
}

#[async_trait]
impl TokenValidator for FakeTokenValidator {
    async fn validate(&self, token: &str) -> Result<VerifiedClaims, AuthError> {
        match self.tokens.lock().unwrap().get(token) {
            Some(result) => result.clone(),
            None => Err(AuthError::Invalid("unknown test token".to_string())),
        }
    }
}

/// Resolves test service tokens.
pub struct FakeServiceTokens {
    /// Token -> service name.
    pub tokens: Mutex<BTreeMap<String, String>>,
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

    fn token_for(&self, service: &str) -> Option<String> {
        self.tokens
            .lock()
            .unwrap()
            .iter()
            .find(|(_, name)| name.as_str() == service)
            .map(|(token, _)| token.clone())
    }
}

/// A helper for building a deletion progress in a test.
#[must_use]
pub fn progress_with_all_anonymised() -> DeletionProgress {
    let mut progress = DeletionProgress::default();
    for contributor in Contributor::ALL {
        progress.record_anonymised(contributor);
    }
    progress
}
