//! Privacy operations over briefs: an internal export and an anonymisation.
//!
//! A brief is a financial record, so an erasure keeps the row and replaces the
//! subject with a random opaque reference. The caller's idempotency key makes a
//! retry apply once. Neither operation answers a user-facing request directly:
//! Identity orchestrates them under the user's own deletion request.

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use reporting_domain::{AuditOutcome, AuditRecord, Operation, UserId};
use serde_json::json;

use crate::common::{AppError, ServiceCaller};
use crate::ports::{
    AnonymisationRecord, AuditClient, BriefRepository, Clock, IdGenerator, PrivacyRepository,
};

/// The categories a brief keeps under a legal basis after an erasure.
pub const RETAINED_CATEGORIES: [&str; 2] = ["financial_record", "regulatory_retention"];

/// The outcome of an anonymisation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnonymiseResult {
    /// The random reference that replaced the subject.
    pub anonymised_ref: UserId,
    /// How many briefs were anonymised in this call (zero on a replay).
    pub briefs_changed: u64,
    /// When the anonymisation completed.
    pub completed_at: DateTime<Utc>,
    /// Whether the caller's key had already been applied.
    pub already_applied: bool,
}

/// Exports and anonymises briefs for Identity.
#[derive(Clone)]
pub struct PrivacyService {
    briefs: Arc<dyn BriefRepository>,
    ledger: Arc<dyn PrivacyRepository>,
    audit: Arc<dyn AuditClient>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
}

impl PrivacyService {
    /// Wires the ports.
    #[must_use]
    pub fn new(
        briefs: Arc<dyn BriefRepository>,
        ledger: Arc<dyn PrivacyRepository>,
        audit: Arc<dyn AuditClient>,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            briefs,
            ledger,
            audit,
            clock,
            ids,
        }
    }

    /// Exports every brief the user holds, with its sections.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for a caller without the export
    /// operation and [`AppError::Unavailable`] when the store cannot answer.
    pub async fn export(
        &self,
        caller: &ServiceCaller,
        user_id: UserId,
    ) -> Result<serde_json::Value, AppError> {
        caller.require_operation(Operation::PrivacyExport)?;
        let briefs = self
            .briefs
            .list_for_user_export(user_id)
            .await
            .map_err(AppError::from_repository)?;
        let entries: Vec<serde_json::Value> = briefs
            .into_iter()
            .map(|stored| {
                json!({
                    "briefId": stored.brief.id().to_string(),
                    "period": stored.brief.period().as_str(),
                    "periodStart": stored.brief.period_start(),
                    "periodEnd": stored.brief.period_end(),
                    "status": stored.brief.status().as_str(),
                    "failureReason": stored.brief.failure_reason(),
                    "asOf": stored.brief.as_of(),
                    "dataStatus": stored.brief.data_status().map(|status| status.as_str()),
                    "createdAt": stored.brief.created_at(),
                    "sections": stored.sections,
                })
            })
            .collect();
        let record = AuditRecord::new(
            None,
            "reporting.privacy.export",
            format!("user:{user_id}"),
            AuditOutcome::Success,
            self.clock.now(),
            BTreeMap::from([("briefs".to_string(), entries.len().to_string())]),
        );
        self.audit_record(&record, &caller.correlation_id).await;
        Ok(json!({ "briefs": entries }))
    }

    /// Replaces the user's brief subjects with a random opaque reference. A
    /// replay of the same idempotency key returns the first result and changes
    /// nothing.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for a caller without the anonymise
    /// operation, [`AppError::Unavailable`] when the store cannot answer, and
    /// [`AppError::Validation`] for a blank key.
    pub async fn anonymise(
        &self,
        caller: &ServiceCaller,
        user_id: UserId,
        idempotency_key: &str,
    ) -> Result<AnonymiseResult, AppError> {
        caller.require_operation(Operation::PrivacyAnonymise)?;
        let key = idempotency_key.trim();
        if key.is_empty() || key.len() > 200 {
            return Err(AppError::Validation(vec![crate::common::FieldError::new(
                "idempotencyKey",
                "must be between 1 and 200 characters",
            )]));
        }
        if let Some(existing) = self
            .ledger
            .find_anonymisation(key)
            .await
            .map_err(AppError::from_repository)?
        {
            return Ok(AnonymiseResult {
                anonymised_ref: existing.anonymised_ref,
                briefs_changed: 0,
                completed_at: existing.completed_at,
                already_applied: true,
            });
        }
        let now = self.clock.now();
        let anonymised_ref = UserId::new(self.ids.next());
        let changed = self
            .briefs
            .anonymise_for_user(user_id, anonymised_ref, now)
            .await
            .map_err(AppError::from_repository)?;
        let record = AnonymisationRecord {
            anonymised_ref,
            idempotency_key: key.to_string(),
            retained_categories: RETAINED_CATEGORIES
                .iter()
                .map(|category| (*category).to_string())
                .collect(),
            completed_at: now,
        };
        self.ledger
            .record_anonymisation(&record)
            .await
            .map_err(AppError::from_repository)?;
        let audit = AuditRecord::new(
            None,
            "reporting.privacy.anonymise",
            format!("user:{user_id}"),
            AuditOutcome::Success,
            now,
            BTreeMap::from([("briefsChanged".to_string(), changed.to_string())]),
        );
        self.audit_record(&audit, &caller.correlation_id).await;
        Ok(AnonymiseResult {
            anonymised_ref,
            briefs_changed: changed,
            completed_at: now,
            already_applied: false,
        })
    }

    async fn audit_record(
        &self,
        record: &Result<AuditRecord, reporting_domain::DomainError>,
        correlation: &str,
    ) {
        match record {
            Ok(record) => {
                if let Err(error) = self.audit.append(record, correlation).await {
                    tracing::warn!(%error, "a privacy event was not audited");
                }
            }
            Err(error) => tracing::warn!(%error, "a privacy audit record was refused"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Duration;
    use reporting_domain::{Brief, BriefId, DataStatus, Period};
    use serde_json::Value;
    use std::sync::Mutex;

    use crate::ports::{RepositoryError, StoredBrief};

    #[derive(Default, Clone)]
    struct FakeBriefs {
        briefs: Arc<Mutex<Vec<StoredBrief>>>,
    }

    impl FakeBriefs {
        fn with(briefs: Vec<StoredBrief>) -> Self {
            Self {
                briefs: Arc::new(Mutex::new(briefs)),
            }
        }
    }

    #[async_trait]
    impl BriefRepository for FakeBriefs {
        async fn upsert_generated(
            &self,
            _brief: &Brief,
            _sections: &Value,
        ) -> Result<Brief, RepositoryError> {
            unreachable!()
        }

        async fn upsert_failed(&self, _brief: &Brief) -> Result<Brief, RepositoryError> {
            unreachable!()
        }

        async fn find_by_id(&self, _id: BriefId) -> Result<Option<StoredBrief>, RepositoryError> {
            Ok(None)
        }

        async fn find_window(
            &self,
            _user_id: UserId,
            _period: Period,
            _period_start: DateTime<Utc>,
        ) -> Result<Option<StoredBrief>, RepositoryError> {
            Ok(None)
        }

        async fn latest_for_user(
            &self,
            _user_id: UserId,
        ) -> Result<Option<StoredBrief>, RepositoryError> {
            Ok(None)
        }

        async fn list_for_user(
            &self,
            _user_id: UserId,
            _period: Option<Period>,
            _limit: i64,
            _before: Option<DateTime<Utc>>,
        ) -> Result<Vec<Brief>, RepositoryError> {
            Ok(Vec::new())
        }

        async fn list_for_user_export(
            &self,
            user_id: UserId,
        ) -> Result<Vec<StoredBrief>, RepositoryError> {
            Ok(self
                .briefs
                .lock()
                .expect("the lock is not poisoned")
                .iter()
                .filter(|stored| stored.brief.user_id() == user_id)
                .cloned()
                .collect())
        }

        async fn anonymise_for_user(
            &self,
            user_id: UserId,
            anonymised_ref: UserId,
            at: DateTime<Utc>,
        ) -> Result<u64, RepositoryError> {
            let mut briefs = self.briefs.lock().expect("the lock is not poisoned");
            let mut changed = 0;
            for stored in briefs.iter_mut() {
                if stored.brief.user_id() == user_id {
                    stored.brief = Brief::restore(
                        stored.brief.id(),
                        anonymised_ref,
                        stored.brief.period(),
                        stored.brief.period_start(),
                        stored.brief.period_end(),
                        stored.brief.status(),
                        stored.brief.failure_reason().map(ToString::to_string),
                        stored.brief.as_of(),
                        stored.brief.data_status(),
                        stored.brief.created_at(),
                        at,
                    )
                    .expect("the row stays well formed");
                    changed += 1;
                }
            }
            Ok(changed)
        }

        async fn delete_generated_before(
            &self,
            _cutoff: DateTime<Utc>,
        ) -> Result<u64, RepositoryError> {
            Ok(0)
        }
    }

    #[derive(Default, Clone)]
    struct FakeLedger {
        records: Arc<Mutex<Vec<AnonymisationRecord>>>,
    }

    #[async_trait]
    impl PrivacyRepository for FakeLedger {
        async fn find_anonymisation(
            &self,
            idempotency_key: &str,
        ) -> Result<Option<AnonymisationRecord>, RepositoryError> {
            Ok(self
                .records
                .lock()
                .expect("the lock is not poisoned")
                .iter()
                .find(|record| record.idempotency_key == idempotency_key)
                .cloned())
        }

        async fn record_anonymisation(
            &self,
            record: &AnonymisationRecord,
        ) -> Result<(), RepositoryError> {
            self.records
                .lock()
                .expect("the lock is not poisoned")
                .push(record.clone());
            Ok(())
        }
    }

    struct FakeAudit;
    #[async_trait]
    impl AuditClient for FakeAudit {
        async fn append(
            &self,
            _record: &AuditRecord,
            _correlation_id: &str,
        ) -> Result<(), RepositoryError> {
            Ok(())
        }
    }

    struct FixedClock(DateTime<Utc>);
    impl Clock for FixedClock {
        fn now(&self) -> DateTime<Utc> {
            self.0
        }
    }

    struct SequentialIds(Mutex<u32>);
    impl IdGenerator for SequentialIds {
        fn next(&self) -> uuid::Uuid {
            let mut next = self.0.lock().expect("the lock is not poisoned");
            *next += 1;
            uuid::Uuid::from_u128(u128::from(*next))
        }
    }

    fn now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_800_000_000, 0).expect("a valid instant")
    }

    fn stored(user: UserId) -> StoredBrief {
        let mut brief = Brief::new(
            BriefId::generate(),
            user,
            Period::Daily,
            now() - Duration::hours(24),
            now(),
            now(),
        )
        .expect("a valid window");
        brief
            .mark_generated(now(), DataStatus::Complete, now())
            .expect("a pending brief can be generated");
        StoredBrief {
            brief,
            sections: Some(serde_json::json!({ "dataStatus": "complete" })),
        }
    }

    fn service(briefs: FakeBriefs, ledger: FakeLedger) -> PrivacyService {
        PrivacyService::new(
            Arc::new(briefs),
            Arc::new(ledger),
            Arc::new(FakeAudit),
            Arc::new(FixedClock(now())),
            Arc::new(SequentialIds(Mutex::new(0))),
        )
    }

    #[tokio::test]
    async fn export_returns_the_users_briefs_only() {
        let user = UserId::generate();
        let service = service(
            FakeBriefs::with(vec![stored(user), stored(UserId::generate())]),
            FakeLedger::default(),
        );
        let caller = ServiceCaller::new("identity", "c-1");
        let exported = service
            .export(&caller, user)
            .await
            .expect("the export answers");
        assert_eq!(exported["briefs"].as_array().expect("an array").len(), 1);
    }

    #[tokio::test]
    async fn export_is_refused_to_a_caller_without_the_operation() {
        let service = service(FakeBriefs::default(), FakeLedger::default());
        let caller = ServiceCaller::new("job-orchestration", "c-1");
        let error = service
            .export(&caller, UserId::generate())
            .await
            .expect_err("the caller may not export");
        assert!(matches!(error, AppError::Forbidden(_)));
    }

    #[tokio::test]
    async fn anonymise_replaces_the_subject_and_keeps_the_brief() {
        let user = UserId::generate();
        let briefs = FakeBriefs::with(vec![stored(user)]);
        let service = service(briefs.clone(), FakeLedger::default());
        let caller = ServiceCaller::new("identity", "c-1");
        let result = service
            .anonymise(&caller, user, "erase-1")
            .await
            .expect("the erasure applies");
        assert_eq!(result.briefs_changed, 1);
        assert!(!result.already_applied);
        let remaining = briefs
            .briefs
            .lock()
            .expect("the lock is not poisoned")
            .len();
        assert_eq!(remaining, 1, "the financial record is kept");
        let exported = service
            .export(&caller, user)
            .await
            .expect("the export answers");
        assert!(exported["briefs"].as_array().expect("an array").is_empty());
    }

    #[tokio::test]
    async fn a_replayed_key_applies_once() {
        let user = UserId::generate();
        let service = service(FakeBriefs::with(vec![stored(user)]), FakeLedger::default());
        let caller = ServiceCaller::new("identity", "c-1");
        let first = service
            .anonymise(&caller, user, "erase-1")
            .await
            .expect("the erasure applies");
        let second = service
            .anonymise(&caller, user, "erase-1")
            .await
            .expect("the replay answers");
        assert_eq!(second.anonymised_ref, first.anonymised_ref);
        assert!(second.already_applied);
        assert_eq!(second.briefs_changed, 0);
    }

    #[tokio::test]
    async fn a_blank_key_is_refused() {
        let service = service(FakeBriefs::default(), FakeLedger::default());
        let caller = ServiceCaller::new("identity", "c-1");
        let error = service
            .anonymise(&caller, UserId::generate(), "  ")
            .await
            .expect_err("a blank key is refused");
        assert!(matches!(error, AppError::Validation(_)));
    }

    #[tokio::test]
    async fn an_anonymise_caller_without_the_operation_is_refused() {
        let service = service(FakeBriefs::default(), FakeLedger::default());
        let caller = ServiceCaller::new("smoke-reader", "c-1");
        let error = service
            .anonymise(&caller, UserId::generate(), "erase-2")
            .await
            .expect_err("the caller may not anonymise");
        assert!(matches!(error, AppError::Forbidden(_)));
    }

    #[test]
    fn the_retained_categories_are_the_financial_record_basis() {
        assert_eq!(RETAINED_CATEGORIES[0], "financial_record");
        assert_eq!(RETAINED_CATEGORIES[1], "regulatory_retention");
    }
}
