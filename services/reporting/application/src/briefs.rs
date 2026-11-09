//! BR3.1 — the owner-scoped brief reads: a list, one brief, and the newest one.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use reporting_domain::{Brief, BriefDelivery, BriefId};
use serde_json::Value;

use crate::common::{AppError, Caller};
use crate::ports::{BriefRepository, DeliveryRepository};

/// The default page size for the brief list.
pub const DEFAULT_PAGE: i64 = 20;
/// The largest page a caller may ask for.
pub const MAX_PAGE: i64 = 100;

/// A brief with its composed sections and delivery attempts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BriefView {
    /// The brief record.
    pub brief: Brief,
    /// The composed sections, absent until generated.
    pub sections: Option<Value>,
    /// The delivery attempts, oldest first.
    pub deliveries: Vec<BriefDelivery>,
}

/// Reads briefs for the authenticated user.
#[derive(Clone)]
pub struct BriefService {
    briefs: Arc<dyn BriefRepository>,
    deliveries: Arc<dyn DeliveryRepository>,
}

impl BriefService {
    /// Wires the repositories.
    #[must_use]
    pub fn new(briefs: Arc<dyn BriefRepository>, deliveries: Arc<dyn DeliveryRepository>) -> Self {
        Self { briefs, deliveries }
    }

    /// Lists the caller's briefs, newest first, bounded to [`MAX_PAGE`].
    ///
    /// # Errors
    /// Returns [`AppError::Validation`] for a non-positive page and
    /// [`AppError::Unavailable`] when the store cannot answer.
    pub async fn list(
        &self,
        caller: &Caller,
        period: Option<reporting_domain::Period>,
        limit: Option<i64>,
        before: Option<DateTime<Utc>>,
    ) -> Result<Vec<Brief>, AppError> {
        let limit = limit.unwrap_or(DEFAULT_PAGE);
        if limit <= 0 || limit > MAX_PAGE {
            return Err(AppError::Validation(vec![crate::common::FieldError::new(
                "limit",
                format!("must be between 1 and {MAX_PAGE}"),
            )]));
        }
        self.briefs
            .list_for_user(caller.user_id, period, limit, before)
            .await
            .map_err(AppError::from_repository)
    }

    /// Opens one brief for its owner.
    ///
    /// # Errors
    /// Returns [`AppError::NotFound`] when the brief does not exist and
    /// [`AppError::NotFound`] when it belongs to another user (BR3.1).
    /// Opens one brief for its owner. A brief that belongs to another user is
    /// reported as not found, so the path never confirms that an id exists
    /// (BR3.1).
    ///
    /// # Errors
    /// Returns [`AppError::NotFound`] when the brief does not exist or belongs to
    /// another user.
    pub async fn open(&self, caller: &Caller, id: BriefId) -> Result<BriefView, AppError> {
        let stored = self
            .briefs
            .find_by_id(id)
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("brief"))?;
        if stored.brief.user_id() != caller.user_id {
            return Err(AppError::NotFound("brief"));
        }
        let deliveries = self
            .deliveries
            .list_for_brief(id)
            .await
            .map_err(AppError::from_repository)?;
        Ok(BriefView {
            brief: stored.brief,
            sections: stored.sections,
            deliveries,
        })
    }

    /// The caller's newest brief, when one exists.
    ///
    /// # Errors
    /// Returns [`AppError::Unavailable`] when the store cannot answer.
    pub async fn latest(&self, caller: &Caller) -> Result<Option<BriefView>, AppError> {
        let Some(stored) = self
            .briefs
            .latest_for_user(caller.user_id)
            .await
            .map_err(AppError::from_repository)?
        else {
            return Ok(None);
        };
        let deliveries = self
            .deliveries
            .list_for_brief(stored.brief.id())
            .await
            .map_err(AppError::from_repository)?;
        Ok(Some(BriefView {
            brief: stored.brief,
            sections: stored.sections,
            deliveries,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Duration;
    use reporting_domain::{BriefStatus, DataStatus, Period, UserId};
    use std::sync::Mutex;

    use crate::common::AssuranceLevel;
    use crate::ports::{RepositoryError, StoredBrief};

    #[derive(Default)]
    struct FakeBriefs {
        briefs: Mutex<Vec<StoredBrief>>,
        unavailable: bool,
    }

    impl FakeBriefs {
        fn with(briefs: Vec<StoredBrief>) -> Self {
            Self {
                briefs: Mutex::new(briefs),
                unavailable: false,
            }
        }

        fn down() -> Self {
            Self {
                briefs: Mutex::new(Vec::new()),
                unavailable: true,
            }
        }

        fn guard(&self) -> Result<(), RepositoryError> {
            if self.unavailable {
                Err(RepositoryError::Unavailable("down".to_string()))
            } else {
                Ok(())
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
            unreachable!("the read service never writes")
        }

        async fn upsert_failed(&self, _brief: &Brief) -> Result<Brief, RepositoryError> {
            unreachable!("the read service never writes")
        }

        async fn find_by_id(&self, id: BriefId) -> Result<Option<StoredBrief>, RepositoryError> {
            self.guard()?;
            Ok(self
                .briefs
                .lock()
                .expect("the lock is not poisoned")
                .iter()
                .find(|stored| stored.brief.id() == id)
                .cloned())
        }

        async fn find_window(
            &self,
            _user_id: UserId,
            _period: Period,
            _period_start: DateTime<Utc>,
        ) -> Result<Option<StoredBrief>, RepositoryError> {
            self.guard()?;
            Ok(None)
        }

        async fn latest_for_user(
            &self,
            user_id: UserId,
        ) -> Result<Option<StoredBrief>, RepositoryError> {
            self.guard()?;
            let briefs = self.briefs.lock().expect("the lock is not poisoned");
            let mut owned: Vec<&StoredBrief> = briefs
                .iter()
                .filter(|stored| stored.brief.user_id() == user_id)
                .filter(|stored| stored.brief.status() == BriefStatus::Generated)
                .collect();
            owned.sort_by_key(|left| std::cmp::Reverse(left.brief.period_start()));
            Ok(owned.first().map(|stored| (*stored).clone()))
        }

        async fn list_for_user(
            &self,
            user_id: UserId,
            period: Option<Period>,
            limit: i64,
            before: Option<DateTime<Utc>>,
        ) -> Result<Vec<Brief>, RepositoryError> {
            self.guard()?;
            let briefs = self.briefs.lock().expect("the lock is not poisoned");
            let mut owned: Vec<Brief> = briefs
                .iter()
                .filter(|stored| stored.brief.user_id() == user_id)
                .filter(|stored| period.is_none_or(|wanted| stored.brief.period() == wanted))
                .filter(|stored| before.is_none_or(|cutoff| stored.brief.updated_at() < cutoff))
                .map(|stored| stored.brief.clone())
                .collect();
            owned.sort_by_key(|stored| std::cmp::Reverse(stored.updated_at()));
            owned.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
            Ok(owned)
        }

        async fn list_for_user_export(
            &self,
            user_id: UserId,
        ) -> Result<Vec<StoredBrief>, RepositoryError> {
            self.guard()?;
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
            _user_id: UserId,
            _anonymised_ref: UserId,
            _at: DateTime<Utc>,
        ) -> Result<u64, RepositoryError> {
            unreachable!("the read service never anonymises")
        }

        async fn delete_generated_before(
            &self,
            _cutoff: DateTime<Utc>,
        ) -> Result<u64, RepositoryError> {
            unreachable!("the read service never prunes")
        }
    }

    #[derive(Default)]
    struct FakeDeliveries {
        deliveries: Mutex<Vec<BriefDelivery>>,
    }

    #[async_trait]
    impl DeliveryRepository for FakeDeliveries {
        async fn upsert(&self, _delivery: &BriefDelivery) -> Result<(), RepositoryError> {
            unreachable!("the read service never writes deliveries")
        }

        async fn list_for_brief(
            &self,
            brief_id: BriefId,
        ) -> Result<Vec<BriefDelivery>, RepositoryError> {
            Ok(self
                .deliveries
                .lock()
                .expect("the lock is not poisoned")
                .iter()
                .filter(|delivery| delivery.brief_id() == brief_id)
                .cloned()
                .collect())
        }
    }

    fn now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_800_000_000, 0).expect("a valid instant")
    }

    fn generated(user: UserId, start_offset_hours: i64) -> StoredBrief {
        let start = now() + Duration::hours(start_offset_hours);
        let mut brief = Brief::new(
            BriefId::generate(),
            user,
            Period::Daily,
            start,
            start + Duration::hours(24),
            start,
        )
        .expect("a valid window");
        brief
            .mark_generated(start, DataStatus::Complete, start)
            .expect("a pending brief can be generated");
        StoredBrief {
            brief,
            sections: Some(serde_json::json!({ "dataStatus": "complete" })),
        }
    }

    fn caller(user: UserId) -> Caller {
        Caller {
            user_id: user,
            assurance: AssuranceLevel::Aal1,
            correlation_id: "c-1".to_string(),
        }
    }

    fn service(briefs: FakeBriefs, deliveries: FakeDeliveries) -> BriefService {
        BriefService::new(Arc::new(briefs), Arc::new(deliveries))
    }

    #[tokio::test]
    async fn a_list_returns_only_the_callers_briefs() {
        let user = UserId::generate();
        let other = UserId::generate();
        let service = service(
            FakeBriefs::with(vec![generated(user, 0), generated(other, 0)]),
            FakeDeliveries::default(),
        );
        let listed = service
            .list(&caller(user), None, None, None)
            .await
            .expect("the list answers");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].user_id(), user);
    }

    #[tokio::test]
    async fn a_list_page_is_bounded() {
        let user = UserId::generate();
        let service = service(FakeBriefs::with(vec![]), FakeDeliveries::default());
        assert!(service
            .list(&caller(user), None, Some(0), None)
            .await
            .is_err());
        assert!(service
            .list(&caller(user), None, Some(MAX_PAGE + 1), None)
            .await
            .is_err());
        assert!(service
            .list(&caller(user), None, Some(MAX_PAGE), None)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn opening_another_users_brief_is_forbidden() {
        let owner = UserId::generate();
        let stored = generated(owner, 0);
        let id = stored.brief.id();
        let service = service(FakeBriefs::with(vec![stored]), FakeDeliveries::default());
        let error = service
            .open(&caller(UserId::generate()), id)
            .await
            .expect_err("another user's brief is refused");
        assert!(matches!(error, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn opening_a_missing_brief_is_not_found() {
        let user = UserId::generate();
        let service = service(FakeBriefs::with(vec![]), FakeDeliveries::default());
        let error = service
            .open(&caller(user), BriefId::generate())
            .await
            .expect_err("a missing brief is not found");
        assert!(matches!(error, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn opening_your_own_brief_returns_its_sections_and_deliveries() {
        let user = UserId::generate();
        let stored = generated(user, 0);
        let id = stored.brief.id();
        let service = service(FakeBriefs::with(vec![stored]), FakeDeliveries::default());
        let view = service
            .open(&caller(user), id)
            .await
            .expect("the owner reads it");
        assert!(view.sections.is_some());
        assert!(view.deliveries.is_empty());
    }

    #[tokio::test]
    async fn the_latest_brief_is_the_newest_generated_one() {
        let user = UserId::generate();
        let service = service(
            FakeBriefs::with(vec![generated(user, 0), generated(user, 24)]),
            FakeDeliveries::default(),
        );
        let view = service
            .latest(&caller(user))
            .await
            .expect("the latest answers")
            .expect("a brief exists");
        assert_eq!(view.brief.period_start(), now() + Duration::hours(24));
    }

    #[tokio::test]
    async fn a_user_without_briefs_has_no_latest() {
        let user = UserId::generate();
        let service = service(FakeBriefs::with(vec![]), FakeDeliveries::default());
        assert!(service
            .latest(&caller(user))
            .await
            .expect("the read answers")
            .is_none());
    }

    #[tokio::test]
    async fn a_store_outage_is_surfaced_as_unavailable() {
        let user = UserId::generate();
        let service = service(FakeBriefs::down(), FakeDeliveries::default());
        let error = service
            .list(&caller(user), None, None, None)
            .await
            .expect_err("an outage is surfaced");
        assert!(matches!(error, AppError::Unavailable(_)));
    }
}
