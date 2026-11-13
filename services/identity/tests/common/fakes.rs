//! Port fakes for unit-level application and HTTP tests.
//!
//! Each fake is deterministic, records what the use case asked it to do, and can
//! be made to fail once so error paths are exercised without a database.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use async_trait::async_trait;
use chrono::{DateTime, Duration, TimeZone as _, Utc};
use identity_application::account_lifecycle::AccountObservation;
use identity_application::authorisation::ports::{
    AuditSink, AuditWriteContext, AuthorisationEvent, ObservationSink, RoleReader, RoleWriter,
    StatusFailure, StatusReadError, StatusReader,
};
use identity_application::consent::ports::{
    DeliveryOutcome, DrainerObservation, ObservationSink as ConsentObservationSink,
    PreferenceWriteOutcome, StateQueryOutcome, WithdrawalRollbackCause,
};
use identity_application::jurisdiction::{
    AuditSink as JurisdictionAuditSink, GeoLocator, ListRepository,
};
use identity_application::ports::{
    AccountSummary, PortfolioSummary, SupportAccountsSource, SupportObservation, SupportOutbox,
    SupportPortfoliosSource,
};
use identity_application::ports::{
    AuditClient, AuthAdmin, AuthError, Clock, ConsentRepository, ContributorGateway,
    DeletionRepository, DeviceSessionRepository, DueCandidate, DueUserRepository, ErasureStore,
    ExportRepository, ExportStore, ExternalError, GateGrant, GateRefusal, GateTokenIssuer,
    JurisdictionGate, JurisdictionRepository, ReadinessProbe, RepositoryError,
    ServiceTokenRegistry, ShreddingKeyRepository, StatusChange, StatusTransitionStore,
    SupportGrantRepository, TokenValidator, UserRepository, WakeTransport,
};
use identity_domain::preferences::{BaseCurrency, Locale, TimeZone};
use identity_domain::{
    AuditRecord, ConsentAction, ConsentEvent, ConsentEventDraft, ConsentKind, ConsentPurpose,
    ConsentRecord, ConsentRecordId, Contributor, CountryCode, DeletionProgress, DeletionRequest,
    DeletionRequestId, DeviceSession, DeviceSessionId, ExportRequest, ExportRequestId,
    JurisdictionListVersion, JurisdictionRule, PreferenceUpdate, ProviderProfile, RetainedCategory,
    Role, ShreddingKey, ShreddingKeyId, ShreddingKeyStatus, StatusTransition, StatusTransitionId,
    StoredPreferences, SupportAccessGrant, SupportGrantId, User, UserId, UserStatus,
    VerifiedClaims,
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
    /// Subject hash -> internal id of an erased account (the tombstone).
    pub erased: Mutex<BTreeMap<String, Uuid>>,
    /// Internal id -> revealed subject.
    pub revealed: Mutex<BTreeMap<Uuid, String>>,
    /// The key store the provision transaction writes through.
    pub keys: Mutex<Option<Arc<FakeKeys>>>,
    /// When set, the next subject lookup reports a hash collision.
    pub collide: Mutex<bool>,
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

    /// Shares the key store so `provision` can create the key reference.
    pub fn attach_keys(&self, keys: Arc<FakeKeys>) {
        *self.keys.lock().unwrap() = Some(keys);
    }

    /// Makes the next subject lookup report a collision.
    pub fn collide_next(&self) {
        *self.collide.lock().unwrap() = true;
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
        if std::mem::take(&mut *self.collide.lock().unwrap()) {
            return Err(RepositoryError::Validation(
                "the presented subject does not match the stored subject".to_string(),
            ));
        }
        let id = self.subjects.lock().unwrap().get(auth_subject).copied();
        if let Some(id) = id {
            return Ok(self.get(UserId::new(id)));
        }
        // The tombstone: an erased subject resolves to its deleted record, never
        // to `None`, so the principal denies it instead of provisioning it
        // again (BR5.8, AC4.5.1).
        let hash = identity_application::user_registry::subject_hash(auth_subject);
        let erased = self.erased.lock().unwrap().get(&hash).copied();
        Ok(erased.and_then(|id| self.get(UserId::new(id))))
    }

    async fn find_by_subject_hash(
        &self,
        subject_hash: &str,
    ) -> Result<Option<User>, RepositoryError> {
        maybe_fail(&self.failure)?;
        let id = self
            .subjects
            .lock()
            .unwrap()
            .iter()
            .find(|(subject, _)| {
                identity_application::user_registry::subject_hash(subject) == subject_hash
            })
            .map(|(_, id)| *id)
            .or_else(|| self.erased.lock().unwrap().get(subject_hash).copied());
        Ok(id.and_then(|id| self.get(UserId::new(id))))
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, RepositoryError> {
        maybe_fail(&self.failure)?;
        Ok(self
            .users
            .lock()
            .unwrap()
            .values()
            .filter(|user| user.email() == Some(email))
            .min_by_key(|user| (user.created_at(), user.id().as_uuid()))
            .cloned())
    }

    async fn provision(
        &self,
        user: &User,
        auth_subject: &str,
        now: DateTime<Utc>,
    ) -> Result<ShreddingKey, RepositoryError> {
        maybe_fail(&self.failure)?;
        let keys = self
            .keys
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| RepositoryError::Unavailable("no key store attached".to_string()))?;
        let key =
            <FakeKeys as ShreddingKeyRepository>::create_for_user(&keys, user.id(), now).await?;
        self.seed(user.clone(), auth_subject);
        Ok(key)
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
        // The hash moves to the tombstone before the personal row leaves.
        if let Some(subject) = self.revealed.lock().unwrap().remove(&id) {
            self.subjects.lock().unwrap().remove(&subject);
            self.erased.lock().unwrap().insert(
                identity_application::user_registry::subject_hash(&subject),
                id,
            );
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
    /// Events written in the same transaction as a grant change.
    pub events: Mutex<Vec<AuditRecord>>,
    /// When set, the conditional approve/revoke move no row.
    pub zero_rows: AtomicBool,
    /// One-shot repository failure.
    pub failure: Mutex<Option<RepositoryError>>,
    /// `(status, limit)` for each list read, so a use case's bound is asserted.
    pub list_calls: Mutex<Vec<(identity_domain::SupportGrantStatus, usize)>>,
    /// `(agent, limit)` for each own-grant read, so its scope and bound are
    /// asserted (S2).
    pub mine_calls: Mutex<Vec<(UserId, usize)>>,
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

    async fn find_live_by_agent_and_user(
        &self,
        agent_id: UserId,
        user_id: UserId,
    ) -> Result<Option<SupportAccessGrant>, RepositoryError> {
        maybe_fail(&self.failure)?;
        Ok(self
            .grants
            .lock()
            .unwrap()
            .iter()
            .filter(|grant| {
                grant.support_agent_id() == agent_id
                    && grant.user_id() == user_id
                    && matches!(
                        grant.status(),
                        identity_domain::support::SupportGrantStatus::PendingApproval
                            | identity_domain::support::SupportGrantStatus::Active
                    )
            })
            .max_by_key(|grant| (grant.requested_at(), grant.id().as_uuid()))
            .cloned())
    }

    async fn list_by_status(
        &self,
        status: identity_domain::SupportGrantStatus,
        limit: usize,
    ) -> Result<Vec<SupportAccessGrant>, RepositoryError> {
        maybe_fail(&self.failure)?;
        self.list_calls.lock().unwrap().push((status, limit));
        let mut rows: Vec<_> = self
            .grants
            .lock()
            .unwrap()
            .iter()
            .filter(|grant| grant.status() == status)
            .cloned()
            .collect();
        // Newest first, with the id as the stable tie-break the SQL uses.
        rows.sort_by(|left, right| {
            right
                .requested_at()
                .cmp(&left.requested_at())
                .then_with(|| right.id().as_uuid().cmp(&left.id().as_uuid()))
        });
        rows.truncate(limit);
        Ok(rows)
    }

    async fn list_by_support_agent(
        &self,
        agent_id: UserId,
        limit: usize,
    ) -> Result<Vec<SupportAccessGrant>, RepositoryError> {
        maybe_fail(&self.failure)?;
        self.mine_calls.lock().unwrap().push((agent_id, limit));
        let mut rows: Vec<_> = self
            .grants
            .lock()
            .unwrap()
            .iter()
            .filter(|grant| grant.support_agent_id() == agent_id)
            .cloned()
            .collect();
        // Newest first, with the id as the stable tie-break the SQL uses.
        rows.sort_by(|left, right| {
            right
                .requested_at()
                .cmp(&left.requested_at())
                .then_with(|| right.id().as_uuid().cmp(&left.id().as_uuid()))
        });
        rows.truncate(limit);
        Ok(rows)
    }

    async fn insert_with_event(
        &self,
        grant: &SupportAccessGrant,
        record: &AuditRecord,
        _correlation_id: &str,
    ) -> Result<(), RepositoryError> {
        maybe_fail(&self.failure)?;
        self.grants.lock().unwrap().push(grant.clone());
        self.events.lock().unwrap().push(record.clone());
        Ok(())
    }

    async fn approve_with_event(
        &self,
        id: SupportGrantId,
        approved_by: UserId,
        approved_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
        record: &AuditRecord,
        _correlation_id: &str,
    ) -> Result<u64, RepositoryError> {
        maybe_fail(&self.failure)?;
        if self.zero_rows.load(Ordering::SeqCst) {
            return Ok(0);
        }
        let mut grants = self.grants.lock().unwrap();
        let Some(existing) = grants.iter_mut().find(|grant| grant.id() == id) else {
            return Ok(0);
        };
        if existing.status() != identity_domain::support::SupportGrantStatus::PendingApproval {
            return Ok(0);
        }
        let next = SupportAccessGrant::from_stored(
            existing.id(),
            existing.user_id(),
            existing.support_agent_id(),
            existing.reason().to_string(),
            identity_domain::support::SupportGrantStatus::Active,
            existing.requested_at(),
            Some(approved_by),
            Some(approved_at),
            Some(expires_at),
            None,
        );
        *existing = next;
        self.events.lock().unwrap().push(record.clone());
        Ok(1)
    }

    async fn revoke_with_event(
        &self,
        id: SupportGrantId,
        revoked_at: DateTime<Utc>,
        record: &AuditRecord,
        _correlation_id: &str,
    ) -> Result<u64, RepositoryError> {
        maybe_fail(&self.failure)?;
        if self.zero_rows.load(Ordering::SeqCst) {
            return Ok(0);
        }
        let mut grants = self.grants.lock().unwrap();
        let Some(existing) = grants.iter_mut().find(|grant| grant.id() == id) else {
            return Ok(0);
        };
        if !matches!(
            existing.status(),
            identity_domain::support::SupportGrantStatus::PendingApproval
                | identity_domain::support::SupportGrantStatus::Active
        ) {
            return Ok(0);
        }
        let next = SupportAccessGrant::from_stored(
            existing.id(),
            existing.user_id(),
            existing.support_agent_id(),
            existing.reason().to_string(),
            identity_domain::support::SupportGrantStatus::Revoked,
            existing.requested_at(),
            existing.approved_by(),
            existing.approved_at(),
            existing.expires_at(),
            Some(revoked_at),
        );
        *existing = next;
        self.events.lock().unwrap().push(record.clone());
        Ok(1)
    }
}

/// In-memory support audit intents written outside a grant change.
#[derive(Default)]
pub struct FakeSupportOutbox {
    /// Refusal and failure events.
    pub events: Mutex<Vec<AuditRecord>>,
    /// One-shot append failure.
    pub failure: Mutex<Option<RepositoryError>>,
}

#[async_trait]
impl SupportOutbox for FakeSupportOutbox {
    async fn append(
        &self,
        record: &AuditRecord,
        _correlation_id: &str,
        _now: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        maybe_fail(&self.failure)?;
        self.events.lock().unwrap().push(record.clone());
        Ok(())
    }
}

/// The account side of the support metadata fan-out.
#[derive(Default)]
pub struct FakeAccountsSource {
    /// The answer.
    pub accounts: Mutex<Vec<AccountSummary>>,
    /// One-shot failure.
    pub failure: Mutex<Option<ExternalError>>,
    /// How long the call takes, in milliseconds.
    pub delay_ms: AtomicI64,
    /// When the call started, for the concurrency assertion.
    pub started: Mutex<Option<Instant>>,
    /// When the call returned.
    pub finished: Mutex<Option<Instant>>,
}

#[async_trait]
impl SupportAccountsSource for FakeAccountsSource {
    async fn accounts(&self, _user_id: UserId) -> Result<Vec<AccountSummary>, ExternalError> {
        *self.started.lock().unwrap() = Some(Instant::now());
        let delay = self.delay_ms.load(Ordering::SeqCst);
        if delay > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(delay as u64)).await;
        }
        let answer = match self.failure.lock().unwrap().take() {
            Some(error) => Err(error),
            None => Ok(self.accounts.lock().unwrap().clone()),
        };
        *self.finished.lock().unwrap() = Some(Instant::now());
        answer
    }
}

/// The portfolio side of the support metadata fan-out.
#[derive(Default)]
pub struct FakePortfoliosSource {
    /// The answer.
    pub portfolios: Mutex<Vec<PortfolioSummary>>,
    /// One-shot failure.
    pub failure: Mutex<Option<ExternalError>>,
    /// How long the call takes, in milliseconds.
    pub delay_ms: AtomicI64,
    /// When the call started, for the concurrency assertion.
    pub started: Mutex<Option<Instant>>,
    /// When the call returned.
    pub finished: Mutex<Option<Instant>>,
}

#[async_trait]
impl SupportPortfoliosSource for FakePortfoliosSource {
    async fn portfolios(&self, _user_id: UserId) -> Result<Vec<PortfolioSummary>, ExternalError> {
        *self.started.lock().unwrap() = Some(Instant::now());
        let delay = self.delay_ms.load(Ordering::SeqCst);
        if delay > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(delay as u64)).await;
        }
        let answer = match self.failure.lock().unwrap().take() {
            Some(error) => Err(error),
            None => Ok(self.portfolios.lock().unwrap().clone()),
        };
        *self.finished.lock().unwrap() = Some(Instant::now());
        answer
    }
}

/// The inline read-audit sink.
#[derive(Default)]
pub struct FakeSupportObservation {
    /// `identity.support_read.served` calls.
    pub served: Mutex<Vec<(UserId, UserId, SupportGrantId)>>,
    /// `identity.support_read.denied` calls.
    pub denied: Mutex<Vec<(UserId, String, String)>>,
    /// `identity.support_read.denied` calls with outcome `failed`.
    pub failed: Mutex<Vec<(UserId, UserId, String)>>,
    /// One-shot append failure.
    pub failure: Mutex<Option<RepositoryError>>,
}

#[async_trait]
impl SupportObservation for FakeSupportObservation {
    async fn served(
        &self,
        actor: UserId,
        target: UserId,
        grant_id: SupportGrantId,
        _correlation_id: &str,
    ) -> Result<(), RepositoryError> {
        maybe_fail(&self.failure)?;
        self.served.lock().unwrap().push((actor, target, grant_id));
        Ok(())
    }

    async fn denied(
        &self,
        actor: UserId,
        target: String,
        code: &str,
        _correlation_id: &str,
    ) -> Result<(), RepositoryError> {
        maybe_fail(&self.failure)?;
        self.denied
            .lock()
            .unwrap()
            .push((actor, target, code.to_string()));
        Ok(())
    }

    async fn failed(
        &self,
        actor: UserId,
        target: UserId,
        code: &str,
        _correlation_id: &str,
    ) -> Result<(), RepositoryError> {
        maybe_fail(&self.failure)?;
        self.failed
            .lock()
            .unwrap()
            .push((actor, target, code.to_string()));
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
            .filter(|deletion| deletion.user_id() == user_id && deletion.is_open())
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
            .filter(|deletion| {
                deletion.is_open()
                    && deletion.next_attempt_at() <= now
                    && deletion.recoverable_until() <= now
                    && deletion
                        .lease_until()
                        .is_none_or(|lease_until| now >= lease_until)
            })
            .min_by_key(|deletion| deletion.recoverable_until())
        else {
            return Ok(None);
        };
        deletion.claim(worker, now, lease).map_err(|error| {
            RepositoryError::Conflict(format!("deletion claim refused: {error}"))
        })?;
        Ok(Some(deletion.clone()))
    }
}

/// In-memory status transitions and the audit intents they carry (BR5.9).
#[derive(Default)]
pub struct FakeStatusTransitions {
    /// Appended transitions.
    pub transitions: Mutex<Vec<StatusTransition>>,
    /// The next `seq`.
    pub next_seq: AtomicI64,
    /// Where the status write lands.
    pub users: Mutex<Option<Arc<FakeUsers>>>,
    /// Where an opened or cancelled request lands.
    pub deletions: Mutex<Option<Arc<FakeDeletions>>>,
    /// One-shot failure.
    pub failure: Mutex<Option<RepositoryError>>,
}

impl FakeStatusTransitions {
    /// Shares the user store the status write updates.
    pub fn attach_users(&self, users: Arc<FakeUsers>) {
        *self.users.lock().unwrap() = Some(users);
    }

    /// Shares the request store the open/cancel writes update.
    pub fn attach_deletions(&self, deletions: Arc<FakeDeletions>) {
        *self.deletions.lock().unwrap() = Some(deletions);
    }

    /// The transitions appended so far.
    pub fn stored(&self) -> Vec<StatusTransition> {
        self.transitions.lock().unwrap().clone()
    }

    fn record(&self, change: &StatusChange) -> Result<StatusTransition, RepositoryError> {
        let transition = StatusTransition::new(
            StatusTransitionId::generate(),
            change.user_id,
            change.from,
            change.to,
            &change.actor,
            change.reason.as_deref(),
            change.occurred_at,
        )
        .map_err(|error| RepositoryError::Conflict(error.to_string()))?;
        let seq = self.next_seq.fetch_add(1, Ordering::SeqCst) + 1;
        let stored = StatusTransition::from_stored(
            transition.id(),
            transition.user_id(),
            transition.from_status(),
            transition.to_status(),
            transition.actor().to_string(),
            transition.reason().map(ToString::to_string),
            transition.occurred_at(),
            seq,
        );
        self.transitions.lock().unwrap().push(stored.clone());
        Ok(stored)
    }

    fn write_status(&self, change: &StatusChange) -> Result<(), RepositoryError> {
        let users = self.users.lock().unwrap().clone();
        let Some(users) = users else {
            return Err(RepositoryError::Unavailable(
                "no user store attached".to_string(),
            ));
        };
        let mut stored = users
            .get(change.user_id)
            .ok_or_else(|| RepositoryError::Conflict("the account does not exist".to_string()))?;
        if stored.status() != change.from {
            return Err(RepositoryError::Conflict(format!(
                "the account is no longer '{}'",
                change.from.as_str()
            )));
        }
        stored
            .transition(change.to, change.occurred_at)
            .map_err(|error| RepositoryError::Conflict(error.to_string()))?;
        users
            .users
            .lock()
            .unwrap()
            .insert(change.user_id.as_uuid(), stored);
        Ok(())
    }

    fn request_store(&self) -> Result<Arc<FakeDeletions>, RepositoryError> {
        self.deletions
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| RepositoryError::Unavailable("no request store attached".to_string()))
    }
}

#[async_trait]
impl StatusTransitionStore for FakeStatusTransitions {
    async fn apply(&self, change: &StatusChange) -> Result<StatusTransition, RepositoryError> {
        maybe_fail(&self.failure)?;
        self.write_status(change)?;
        self.record(change)
    }

    async fn open_deletion(
        &self,
        change: &StatusChange,
        request: &DeletionRequest,
    ) -> Result<StatusTransition, RepositoryError> {
        maybe_fail(&self.failure)?;
        let deletions = self.request_store()?;
        if deletions
            .find_active_for_user(change.user_id)
            .await?
            .is_some()
        {
            return Err(RepositoryError::Conflict(
                "a deletion request is already open".to_string(),
            ));
        }
        self.write_status(change)?;
        deletions.insert(request).await?;
        self.record(change)
    }

    async fn cancel_deletion(
        &self,
        change: &StatusChange,
        request: &DeletionRequest,
    ) -> Result<StatusTransition, RepositoryError> {
        maybe_fail(&self.failure)?;
        self.write_status(change)?;
        self.request_store()?.update(request).await?;
        self.record(change)
    }

    async fn newest_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Option<StatusTransition>, RepositoryError> {
        maybe_fail(&self.failure)?;
        Ok(self
            .transitions
            .lock()
            .unwrap()
            .iter()
            .filter(|transition| transition.user_id() == user_id)
            .max_by_key(|transition| transition.seq())
            .cloned())
    }
}

/// Records the account-lifecycle metrics the tests assert on.
#[derive(Default)]
pub struct FakeAccountObservation {
    /// `(from, to, actor_kind)` per accepted transition.
    pub transitions: Mutex<Vec<(String, String, String)>>,
    /// Refusal reasons.
    pub refusals: Mutex<Vec<String>>,
    /// Deletion events.
    pub deletion_events: Mutex<Vec<String>>,
    /// Failed erasure steps.
    pub erasure_failures: Mutex<Vec<String>>,
}

impl AccountObservation for FakeAccountObservation {
    fn record_transition(&self, from: UserStatus, to: UserStatus, actor_kind: &str) {
        self.transitions.lock().unwrap().push((
            from.as_str().to_string(),
            to.as_str().to_string(),
            actor_kind.to_string(),
        ));
    }

    fn record_refusal(&self, reason: &str) {
        self.refusals.lock().unwrap().push(reason.to_string());
    }

    fn record_deletion_event(&self, event: &str) {
        self.deletion_events.lock().unwrap().push(event.to_string());
    }

    fn record_erasure_failure(&self, step: &str) {
        self.erasure_failures.lock().unwrap().push(step.to_string());
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

#[async_trait]
impl DueUserRepository for FakeUsers {
    async fn list_active(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DueCandidate>, RepositoryError> {
        maybe_fail(&self.failure)?;
        let mut rows: Vec<DueCandidate> = self
            .users
            .lock()
            .unwrap()
            .values()
            .filter(|user| user.status() == identity_domain::UserStatus::Active)
            .map(|user| DueCandidate {
                user_id: user.id(),
                timezone: user.timezone().as_str().to_string(),
            })
            .collect();
        rows.sort_by_key(|row| row.user_id.as_uuid());
        Ok(rows
            .into_iter()
            .skip(usize::try_from(offset).unwrap_or(usize::MAX))
            .take(usize::try_from(limit).unwrap_or(usize::MAX))
            .collect())
    }
}

// ---------------------------------------------------------------------------
// U6 — the jurisdiction gate's ports
// ---------------------------------------------------------------------------

/// The in-process jurisdiction gate.
#[derive(Default)]
pub struct FakeGate {
    /// Whether the next clearance is denied.
    pub deny: Mutex<Option<String>>,
    /// Whether the next clearance is unavailable.
    pub unavailable: Mutex<Option<String>>,
    /// The `(country, ip)` declarations presented.
    pub calls: Mutex<Vec<(String, String)>>,
}

impl FakeGate {
    /// Denies the next clearance with `reason`.
    pub fn deny_next(&self, reason: &str) {
        *self.deny.lock().unwrap() = Some(reason.to_string());
    }

    /// Makes the next clearance unavailable.
    pub fn fail_next(&self, detail: &str) {
        *self.unavailable.lock().unwrap() = Some(detail.to_string());
    }

    /// The declarations the gate was asked about.
    pub fn calls(&self) -> Vec<(String, String)> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl JurisdictionGate for FakeGate {
    async fn clear(
        &self,
        country_code: &str,
        ip: &str,
        _correlation_id: &str,
    ) -> Result<(), GateRefusal> {
        self.calls
            .lock()
            .unwrap()
            .push((country_code.to_string(), ip.to_string()));
        if let Some(detail) = self.unavailable.lock().unwrap().take() {
            return Err(GateRefusal::Unavailable(detail));
        }
        if let Some(reason) = self.deny.lock().unwrap().take() {
            return Err(GateRefusal::Denied(reason));
        }
        Ok(())
    }
}

/// In-memory jurisdiction list store.
#[derive(Default)]
pub struct FakeListStore {
    /// The active version, when one is published.
    pub active: Mutex<Option<JurisdictionListVersion>>,
    /// Every published version, in publish order.
    pub published: Mutex<Vec<JurisdictionListVersion>>,
    /// One-shot failure.
    pub failure: Mutex<Option<RepositoryError>>,
}

impl FakeListStore {
    /// Seeds the active version without recording a publish.
    pub fn seed(&self, list: JurisdictionListVersion) {
        *self.active.lock().unwrap() = Some(list);
    }

    /// Makes the next call fail.
    pub fn fail_next(&self, error: RepositoryError) {
        *self.failure.lock().unwrap() = Some(error);
    }

    /// The published version labels, in order.
    pub fn published_versions(&self) -> Vec<String> {
        self.published
            .lock()
            .unwrap()
            .iter()
            .map(|list| list.version().to_string())
            .collect()
    }
}

#[async_trait]
impl ListRepository for FakeListStore {
    async fn active(&self) -> Result<Option<JurisdictionListVersion>, RepositoryError> {
        maybe_fail(&self.failure)?;
        Ok(self.active.lock().unwrap().clone())
    }

    async fn publish(&self, list: &JurisdictionListVersion) -> Result<(), RepositoryError> {
        maybe_fail(&self.failure)?;
        self.published.lock().unwrap().push(list.clone());
        *self.active.lock().unwrap() = Some(list.clone());
        Ok(())
    }
}

/// In-memory geolocation provider.
#[derive(Default)]
pub struct FakeGeoLocator {
    /// Country per IP.
    pub countries: Mutex<BTreeMap<String, String>>,
    /// Country returned for any unlisted IP.
    pub default_country: Mutex<Option<String>>,
    /// Queued failures, consumed in order.
    pub failures: Mutex<Vec<ExternalError>>,
    /// Every IP the provider was asked about.
    pub calls: Mutex<Vec<String>>,
}

impl FakeGeoLocator {
    /// Maps one IP to a country.
    pub fn set(&self, ip: &str, country: &str) {
        self.countries
            .lock()
            .unwrap()
            .insert(ip.to_string(), country.to_string());
    }

    /// Sets the fallback country for unlisted IPs.
    pub fn set_default(&self, country: &str) {
        *self.default_country.lock().unwrap() = Some(country.to_string());
    }

    /// Queues one provider failure.
    pub fn fail_next(&self, error: ExternalError) {
        self.failures.lock().unwrap().push(error);
    }

    /// How many provider calls were made.
    pub fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
}

#[async_trait]
impl GeoLocator for FakeGeoLocator {
    async fn country_for(&self, ip: &str) -> Result<CountryCode, ExternalError> {
        self.calls.lock().unwrap().push(ip.to_string());
        let queued = {
            let mut failures = self.failures.lock().unwrap();
            if failures.is_empty() {
                None
            } else {
                Some(failures.remove(0))
            }
        };
        if let Some(error) = queued {
            return Err(error);
        }
        let country = self
            .countries
            .lock()
            .unwrap()
            .get(ip)
            .cloned()
            .or_else(|| self.default_country.lock().unwrap().clone())
            .ok_or_else(|| ExternalError::Unavailable("no country for this address".to_string()))?;
        CountryCode::parse(&country).map_err(|error| ExternalError::Refused(error.to_string()))
    }
}

/// In-memory decision audit sink.
#[derive(Default)]
pub struct FakeDecisionAudit {
    /// Recorded events with their correlation id and idempotency key.
    pub records: Mutex<Vec<(AuditRecord, String, String)>>,
    /// Every idempotency key presented, including failed attempts.
    pub attempts: Mutex<Vec<String>>,
    /// Queued failures, consumed in order.
    pub failures: Mutex<Vec<RepositoryError>>,
}

impl FakeDecisionAudit {
    /// Queues one audit failure.
    pub fn fail_next(&self, error: RepositoryError) {
        self.failures.lock().unwrap().push(error);
    }

    /// The recorded actions.
    pub fn actions(&self) -> Vec<String> {
        self.records
            .lock()
            .unwrap()
            .iter()
            .map(|(record, _, _)| record.action().to_string())
            .collect()
    }

    /// The idempotency keys that reached the sink.
    pub fn attempted_keys(&self) -> Vec<String> {
        self.attempts.lock().unwrap().clone()
    }
}

#[async_trait]
impl JurisdictionAuditSink for FakeDecisionAudit {
    async fn record(
        &self,
        record: &AuditRecord,
        correlation_id: &str,
        idempotency_key: &str,
    ) -> Result<(), RepositoryError> {
        self.attempts
            .lock()
            .unwrap()
            .push(idempotency_key.to_string());
        let queued = {
            let mut failures = self.failures.lock().unwrap();
            if failures.is_empty() {
                None
            } else {
                Some(failures.remove(0))
            }
        };
        if let Some(error) = queued {
            return Err(error);
        }
        self.records.lock().unwrap().push((
            record.clone(),
            correlation_id.to_string(),
            idempotency_key.to_string(),
        ));
        Ok(())
    }
}

/// The status reader fake: serves a stored status and can fail once or always.
#[derive(Default)]
pub struct FakeStatusReader {
    statuses: Mutex<BTreeMap<Uuid, UserStatus>>,
    failure: Mutex<Option<StatusReadError>>,
    always_fails: Mutex<BTreeMap<Uuid, StatusReadError>>,
    calls: AtomicUsize,
}

impl FakeStatusReader {
    /// Serves `status` for `id`.
    pub fn set(&self, id: UserId, status: UserStatus) {
        self.statuses.lock().unwrap().insert(id.as_uuid(), status);
    }

    /// Makes the next read fail.
    pub fn fail_next(&self, error: StatusReadError) {
        *self.failure.lock().unwrap() = Some(error);
    }

    /// Makes every read for `id` fail.
    pub fn fail_for(&self, id: UserId, error: StatusReadError) {
        self.always_fails
            .lock()
            .unwrap()
            .insert(id.as_uuid(), error);
    }

    /// How many times the reader was asked.
    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl StatusReader for FakeStatusReader {
    async fn status_of(&self, user_id: UserId) -> Result<Option<UserStatus>, StatusReadError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        if let Some(error) = self.failure.lock().unwrap().take() {
            return Err(error);
        }
        if let Some(error) = self
            .always_fails
            .lock()
            .unwrap()
            .get(&user_id.as_uuid())
            .cloned()
        {
            return Err(error);
        }
        Ok(self
            .statuses
            .lock()
            .unwrap()
            .get(&user_id.as_uuid())
            .copied())
    }
}

/// The authorisation audit sink fake: records every event, fails on demand.
#[derive(Default)]
pub struct FakeAuthorisationAudit {
    /// Recorded events with their correlation ids.
    pub events: Mutex<Vec<(AuthorisationEvent, String)>>,
    failure: Mutex<Option<RepositoryError>>,
}

impl FakeAuthorisationAudit {
    /// Makes the next write fail.
    pub fn fail_next(&self, error: RepositoryError) {
        *self.failure.lock().unwrap() = Some(error);
    }

    /// The recorded events.
    pub fn events(&self) -> Vec<(AuthorisationEvent, String)> {
        self.events.lock().unwrap().clone()
    }
}

#[async_trait]
impl AuditSink for FakeAuthorisationAudit {
    async fn record(
        &self,
        event: &AuthorisationEvent,
        correlation_id: &str,
    ) -> Result<(), RepositoryError> {
        if let Some(error) = self.failure.lock().unwrap().take() {
            return Err(error);
        }
        self.events
            .lock()
            .unwrap()
            .push((event.clone(), correlation_id.to_string()));
        Ok(())
    }
}

/// The provider role writer fake: records writes, fails on demand.
#[derive(Default)]
pub struct FakeRoleWriter {
    writes: Mutex<Vec<(String, Role)>>,
    failure: Mutex<Option<ExternalError>>,
}

impl FakeRoleWriter {
    /// Makes the next write fail.
    pub fn fail_next(&self, error: ExternalError) {
        *self.failure.lock().unwrap() = Some(error);
    }

    /// The recorded `(subject, role)` writes.
    pub fn writes(&self) -> Vec<(String, Role)> {
        self.writes.lock().unwrap().clone()
    }
}

#[async_trait]
impl RoleWriter for FakeRoleWriter {
    async fn set_role(&self, subject: &str, role: Role) -> Result<(), ExternalError> {
        if let Some(error) = self.failure.lock().unwrap().take() {
            return Err(error);
        }
        self.writes
            .lock()
            .unwrap()
            .push((subject.to_string(), role));
        Ok(())
    }
}

/// A provider role reader that answers from memory (U10 admin lookup).
#[derive(Default)]
pub struct FakeRoleReader {
    roles: Mutex<BTreeMap<String, Role>>,
    failure: Mutex<Option<ExternalError>>,
}

impl FakeRoleReader {
    /// Sets the role the provider reports for a subject.
    pub fn set(&self, subject: &str, role: Role) {
        self.roles.lock().unwrap().insert(subject.to_string(), role);
    }

    /// Makes the next read fail.
    pub fn fail_next(&self, error: ExternalError) {
        *self.failure.lock().unwrap() = Some(error);
    }
}

#[async_trait]
impl RoleReader for FakeRoleReader {
    async fn read_role(&self, subject: &str) -> Result<Role, ExternalError> {
        if let Some(error) = self.failure.lock().unwrap().take() {
            return Err(error);
        }
        self.roles
            .lock()
            .unwrap()
            .get(subject)
            .copied()
            .ok_or_else(|| ExternalError::Refused(format!("no provider user for {subject}")))
    }
}

/// An observation sink that records what the decision and cache reported.
#[derive(Default)]
pub struct FakeObservation {
    /// `(operation class, reason)` for each decision.
    pub decisions: Mutex<Vec<(String, String)>>,
    /// Status lookup failure causes.
    pub status_failures: Mutex<Vec<String>>,
    /// Audit write failure contexts.
    pub audit_failures: Mutex<Vec<String>>,
    /// Role-change outcomes.
    pub role_changes: Mutex<Vec<String>>,
    /// The last reported `(age seconds, entries)`.
    pub cache: Mutex<Option<(i64, usize)>>,
}

impl ObservationSink for FakeObservation {
    fn record_decision(
        &self,
        class: identity_domain::OperationClass,
        reason: identity_domain::DenialReason,
        _elapsed: std::time::Duration,
    ) {
        self.decisions
            .lock()
            .unwrap()
            .push((class.as_str().to_string(), reason.as_str().to_string()));
    }

    fn record_status_lookup_failure(&self, cause: StatusFailure) {
        self.status_failures
            .lock()
            .unwrap()
            .push(cause.as_str().to_string());
    }

    fn record_audit_write_failure(&self, context: AuditWriteContext) {
        self.audit_failures
            .lock()
            .unwrap()
            .push(context.as_str().to_string());
    }

    fn record_cache(&self, age_seconds: i64, entries: usize) {
        *self.cache.lock().unwrap() = Some((age_seconds, entries));
    }

    fn record_role_change(&self, success: bool) {
        self.role_changes.lock().unwrap().push(if success {
            "success".to_string()
        } else {
            "failed".to_string()
        });
    }
}

/// U4's observation sink: records every metric the consent and preference use
/// cases and the outbox drainer emit.
#[derive(Default)]
pub struct FakeObservationSink {
    /// `(outcome, has_missing)` for each consent-state query.
    pub state_queries: Mutex<Vec<(StateQueryOutcome, bool)>>,
    /// `(kind, action)` for each recorded consent event.
    pub events: Mutex<Vec<(ConsentKind, ConsentAction)>>,
    /// The outcome of each preference write.
    pub preference_writes: Mutex<Vec<PreferenceWriteOutcome>>,
    /// The cause of each withdrawal rollback.
    pub rollbacks: Mutex<Vec<WithdrawalRollbackCause>>,
    /// `(pending, oldest age)` for each drainer pass.
    pub passes: Mutex<Vec<(u64, i64)>>,
    /// The outcome of each delivery.
    pub deliveries: Mutex<Vec<DeliveryOutcome>>,
}

impl ConsentObservationSink for FakeObservationSink {
    fn record_state_query(
        &self,
        outcome: StateQueryOutcome,
        has_missing: bool,
        _elapsed: std::time::Duration,
    ) {
        self.state_queries
            .lock()
            .unwrap()
            .push((outcome, has_missing));
    }

    fn record_consent_event(&self, kind: ConsentKind, action: ConsentAction) {
        self.events.lock().unwrap().push((kind, action));
    }

    fn record_preference_write(&self, outcome: PreferenceWriteOutcome) {
        self.preference_writes.lock().unwrap().push(outcome);
    }

    fn record_withdrawal_rollback(&self, cause: WithdrawalRollbackCause) {
        self.rollbacks.lock().unwrap().push(cause);
    }
}

impl DrainerObservation for FakeObservationSink {
    fn record_pass(&self, pending: u64, oldest_age_seconds: i64) {
        self.passes
            .lock()
            .unwrap()
            .push((pending, oldest_age_seconds));
    }

    fn record_delivery(&self, outcome: DeliveryOutcome) {
        self.deliveries.lock().unwrap().push(outcome);
    }
}

/// U4's erasure role fake: records the deletes the worker routed through the
/// dedicated erasure connection.
#[derive(Default)]
pub struct FakeErasureStore {
    /// The users whose consent-scoped rows were erased.
    pub erased: Mutex<Vec<UserId>>,
    /// One-shot erasure failure.
    pub failure: Mutex<Option<RepositoryError>>,
}

#[async_trait]
impl ErasureStore for FakeErasureStore {
    async fn erase_consent(&self, user_id: UserId) -> Result<u64, RepositoryError> {
        maybe_fail(&self.failure)?;
        self.erased.lock().unwrap().push(user_id);
        Ok(0)
    }
}

/// U4's consent store. A withdrawal stages its event and audit intent and moves
/// them only on commit, so the atomicity the transaction port promises is
/// observable.
#[derive(Default)]
pub struct FakeConsentRepository {
    inner: Arc<FakeConsentInner>,
}

#[derive(Default)]
struct FakeConsentInner {
    events: Mutex<Vec<ConsentEvent>>,
    staged_events: Mutex<Vec<ConsentEvent>>,
    outbox: Mutex<Vec<identity_application::consent::ports::AuditIntent>>,
    staged_outbox: Mutex<Vec<identity_application::consent::ports::AuditIntent>>,
    failure: Mutex<Option<RepositoryError>>,
    outbox_failure: Mutex<Option<RepositoryError>>,
    commits: AtomicUsize,
    rollbacks: AtomicUsize,
    next_seq: AtomicI64,
}

impl FakeConsentRepository {
    /// Every committed event.
    pub fn events(&self) -> Vec<ConsentEvent> {
        self.inner.events.lock().unwrap().clone()
    }

    /// Every committed outbox intent.
    pub fn outbox(&self) -> Vec<identity_application::consent::ports::AuditIntent> {
        self.inner.outbox.lock().unwrap().clone()
    }

    /// The commit count.
    pub fn commits(&self) -> usize {
        self.inner.commits.load(Ordering::Relaxed)
    }

    /// The rollback count.
    pub fn rollbacks(&self) -> usize {
        self.inner.rollbacks.load(Ordering::Relaxed)
    }

    /// Seeds a committed event.
    pub fn seed(&self, event: ConsentEvent) {
        self.inner.events.lock().unwrap().push(event);
    }

    /// Makes the next read fail.
    pub fn fail_next(&self, error: RepositoryError) {
        *self.inner.failure.lock().unwrap() = Some(error);
    }

    /// Makes the next audit enqueue fail (the outbox insert fails).
    pub fn fail_outbox_next(&self, error: RepositoryError) {
        *self.inner.outbox_failure.lock().unwrap() = Some(error);
    }

    fn next_event(
        &self,
        user_id: UserId,
        kind: ConsentKind,
        action: ConsentAction,
        policy_version: &str,
    ) -> ConsentEvent {
        let seq = self.inner.next_seq.fetch_add(1, Ordering::Relaxed) + 1;
        ConsentEvent::from_stored(
            ConsentRecordId::generate(),
            seq,
            user_id,
            kind,
            action,
            Utc.with_ymd_and_hms(2026, 11, 10, 12, 0, 0).unwrap() + Duration::seconds(seq),
            policy_version,
        )
    }

    fn newest_for(
        events: &[ConsentEvent],
        user_id: UserId,
        kind: ConsentKind,
    ) -> Option<ConsentEvent> {
        events
            .iter()
            .filter(|event| event.user_id() == user_id && event.kind() == kind)
            .max_by_key(|event| (event.occurred_at(), event.seq()))
            .cloned()
    }
}

#[async_trait]
impl identity_application::consent::ports::ConsentRepository for FakeConsentRepository {
    async fn newest_per_kind(&self, user_id: UserId) -> Result<Vec<ConsentEvent>, RepositoryError> {
        maybe_fail(&self.inner.failure)?;
        let events = self.inner.events.lock().unwrap().clone();
        Ok(ConsentKind::ALL
            .into_iter()
            .filter_map(|kind| Self::newest_for(&events, user_id, kind))
            .collect())
    }

    async fn history(
        &self,
        user_id: UserId,
        limit: u32,
        before: Option<identity_application::consent::ports::HistoryCursor>,
    ) -> Result<Vec<ConsentEvent>, RepositoryError> {
        maybe_fail(&self.inner.failure)?;
        let mut events: Vec<ConsentEvent> = self
            .inner
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|event| event.user_id() == user_id)
            .cloned()
            .collect();
        events.sort_by_key(|event| std::cmp::Reverse((event.occurred_at(), event.seq())));
        if let Some(cursor) = before {
            events.retain(|event| {
                event.occurred_at() < cursor.occurred_at
                    || (event.occurred_at() == cursor.occurred_at && event.seq() < cursor.seq)
            });
        }
        events.truncate(limit as usize);
        Ok(events)
    }

    async fn insert(&self, draft: &ConsentEventDraft) -> Result<ConsentEvent, RepositoryError> {
        maybe_fail(&self.inner.failure)?;
        let event = self.next_event(
            draft.user_id(),
            draft.kind(),
            draft.action(),
            draft.policy_version().as_str(),
        );
        self.inner.events.lock().unwrap().push(event.clone());
        Ok(event)
    }

    async fn begin_withdraw(
        &self,
        user_id: UserId,
        kind: ConsentKind,
    ) -> Result<Box<dyn identity_application::consent::ports::ConsentTransaction>, RepositoryError>
    {
        maybe_fail(&self.inner.failure)?;
        Ok(Box::new(FakeConsentTransaction {
            inner: self.inner.clone(),
            user_id,
            kind,
        }))
    }
}

struct FakeConsentTransaction {
    inner: Arc<FakeConsentInner>,
    user_id: UserId,
    kind: ConsentKind,
}

#[async_trait]
impl identity_application::consent::ports::ConsentTransaction for FakeConsentTransaction {
    async fn newest(&mut self, kind: ConsentKind) -> Result<Option<ConsentEvent>, RepositoryError> {
        maybe_fail(&self.inner.failure)?;
        let events = self.inner.events.lock().unwrap();
        Ok(FakeConsentRepository::newest_for(
            &events,
            self.user_id,
            kind,
        ))
    }

    async fn insert_event(
        &mut self,
        draft: &ConsentEventDraft,
    ) -> Result<ConsentEvent, RepositoryError> {
        let event = self.next_staged(draft);
        self.inner.staged_events.lock().unwrap().push(event.clone());
        Ok(event)
    }

    async fn enqueue_audit(
        &mut self,
        intent: &identity_application::consent::ports::AuditIntent,
    ) -> Result<(), RepositoryError> {
        maybe_fail(&self.inner.outbox_failure)?;
        self.inner
            .staged_outbox
            .lock()
            .unwrap()
            .push(intent.clone());
        Ok(())
    }

    async fn commit(self: Box<Self>) -> Result<(), RepositoryError> {
        let mut staged = self.inner.staged_events.lock().unwrap();
        self.inner.events.lock().unwrap().append(&mut staged);
        let mut staged_outbox = self.inner.staged_outbox.lock().unwrap();
        self.inner.outbox.lock().unwrap().append(&mut staged_outbox);
        self.inner.commits.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    async fn rollback(self: Box<Self>) -> Result<(), RepositoryError> {
        self.inner.staged_events.lock().unwrap().clear();
        self.inner.staged_outbox.lock().unwrap().clear();
        self.inner.rollbacks.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

impl FakeConsentTransaction {
    fn next_staged(&self, draft: &ConsentEventDraft) -> ConsentEvent {
        let seq = self.inner.next_seq.fetch_add(1, Ordering::Relaxed) + 1;
        ConsentEvent::from_stored(
            ConsentRecordId::generate(),
            seq,
            self.user_id,
            draft.kind(),
            draft.action(),
            Utc.with_ymd_and_hms(2026, 11, 10, 12, 0, 0).unwrap() + Duration::seconds(seq),
            draft.policy_version().as_str(),
        )
    }
}

/// U4's preferences store: one row per user, only the supplied fields change.
#[derive(Default)]
pub struct FakePreferencesRepository {
    /// Stored rows keyed by user id.
    pub rows: Mutex<BTreeMap<Uuid, StoredPreferences>>,
    /// One-shot read or write failure.
    pub failure: Mutex<Option<RepositoryError>>,
}

impl FakePreferencesRepository {
    /// The stored row for one user.
    pub fn row(&self, user_id: UserId) -> Option<StoredPreferences> {
        self.rows.lock().unwrap().get(&user_id.as_uuid()).cloned()
    }
}

#[async_trait]
impl identity_application::preferences::ports::PreferencesRepository for FakePreferencesRepository {
    async fn load(&self, user_id: UserId) -> Result<Option<StoredPreferences>, RepositoryError> {
        maybe_fail(&self.failure)?;
        Ok(self.row(user_id))
    }

    async fn upsert(
        &self,
        user_id: UserId,
        update: &PreferenceUpdate,
        _now: DateTime<Utc>,
    ) -> Result<StoredPreferences, RepositoryError> {
        maybe_fail(&self.failure)?;
        let mut rows = self.rows.lock().unwrap();
        let stored = rows.entry(user_id.as_uuid()).or_default();
        if update.locale.is_some() {
            stored.locale = update.locale;
        }
        if update.base_currency.is_some() {
            stored.base_currency = update.base_currency;
        }
        if update.timezone.is_some() {
            stored.timezone = update.timezone.clone();
        }
        if update.notification_email.is_some() {
            stored.notification_email = update.notification_email;
        }
        if update.notification_push.is_some() {
            stored.notification_push = update.notification_push;
        }
        Ok(stored.clone())
    }
}

/// U4's provider-profile lookup.
#[derive(Default)]
pub struct FakeUserLookup {
    /// Stored provider profiles keyed by user id.
    pub profiles: Mutex<BTreeMap<Uuid, ProviderProfile>>,
    /// One-shot lookup failure.
    pub failure: Mutex<Option<RepositoryError>>,
}

impl FakeUserLookup {
    /// Seeds one provider profile.
    pub fn seed(&self, user_id: UserId, profile: ProviderProfile) {
        self.profiles
            .lock()
            .unwrap()
            .insert(user_id.as_uuid(), profile);
    }
}

#[async_trait]
impl identity_application::preferences::ports::UserLookup for FakeUserLookup {
    async fn provider_profile(
        &self,
        user_id: UserId,
    ) -> Result<Option<ProviderProfile>, RepositoryError> {
        maybe_fail(&self.failure)?;
        // A provisioned user always has a profile in the real adapter. The fake
        // resolves an unseeded id to the launch defaults so a test that only
        // provisions through the principal path still gets an effective read.
        Ok(Some(
            self.profiles
                .lock()
                .unwrap()
                .get(&user_id.as_uuid())
                .cloned()
                .unwrap_or_else(|| {
                    ProviderProfile::new(
                        Locale::En,
                        BaseCurrency::parse("USD").expect("USD is valid"),
                        TimeZone::parse("UTC").expect("UTC is valid"),
                    )
                }),
        ))
    }
}
