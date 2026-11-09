//! WF5 (identity side): the C3 reporting read.
//!
//! Identity owns user enumeration and timezones, so it answers which users a
//! reporting run should consider. A local period boundary always has a most
//! recent occurrence, so every active user is due; Reporting applies its own
//! skip/retry rule per (user, window) and never produces a brief twice.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use identity_domain::UserId;

use crate::common::{AppError, FieldError};
use crate::ports::{DueUserRepository, RepositoryError};

/// How many users one page holds.
pub const PAGE_SIZE: i64 = 1_000;
/// The most pages one read will walk, bounding a run's enumeration.
pub const MAX_PAGES: i64 = 100;

/// One due user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DueUserView {
    /// The user identifier.
    pub user_id: UserId,
    /// The IANA timezone name.
    pub timezone: String,
}

/// Answers the C3 due-user read.
#[derive(Clone)]
pub struct DueUsersService {
    users: Arc<dyn DueUserRepository>,
}

impl DueUsersService {
    /// Wires the port.
    #[must_use]
    pub fn new(users: Arc<dyn DueUserRepository>) -> Self {
        Self { users }
    }

    /// Lists the active users due at `at` for `period`.
    ///
    /// # Errors
    /// Returns [`AppError::Validation`] for an unknown period and
    /// [`AppError::Unavailable`] when the store cannot answer.
    pub async fn due(
        &self,
        period: &str,
        _at: DateTime<Utc>,
    ) -> Result<Vec<DueUserView>, AppError> {
        let period = period.trim().to_ascii_lowercase();
        if period != "daily" && period != "weekly" {
            return Err(AppError::Validation(vec![FieldError::new(
                "period",
                "must be daily or weekly",
            )]));
        }
        let mut due = Vec::new();
        for page in 0..MAX_PAGES {
            let offset = page * PAGE_SIZE;
            let rows = self
                .users
                .list_active(PAGE_SIZE, offset)
                .await
                .map_err(map_repository)?;
            let count = rows.len();
            due.extend(rows.into_iter().map(|row| DueUserView {
                user_id: row.user_id,
                timezone: row.timezone,
            }));
            if count < usize::try_from(PAGE_SIZE).unwrap_or(usize::MAX) {
                break;
            }
        }
        Ok(due)
    }
}

fn map_repository(error: RepositoryError) -> AppError {
    match error {
        RepositoryError::Unavailable(detail) => AppError::Unavailable(detail),
        RepositoryError::InvalidData(detail) => {
            AppError::Internal(format!("stored data is invalid: {detail}"))
        }
        RepositoryError::Conflict(detail) => AppError::Conflict(detail),
        RepositoryError::Forbidden(detail) => AppError::Forbidden(detail),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeUsers {
        rows: Mutex<Vec<crate::ports::DueCandidate>>,
        unavailable: bool,
        calls: Mutex<usize>,
    }

    #[async_trait]
    impl DueUserRepository for FakeUsers {
        async fn list_active(
            &self,
            limit: i64,
            offset: i64,
        ) -> Result<Vec<crate::ports::DueCandidate>, RepositoryError> {
            *self.calls.lock().expect("the lock is not poisoned") += 1;
            if self.unavailable {
                return Err(RepositoryError::Unavailable("down".to_string()));
            }
            let rows = self.rows.lock().expect("the lock is not poisoned");
            Ok(rows
                .iter()
                .skip(usize::try_from(offset).unwrap_or(usize::MAX))
                .take(usize::try_from(limit).unwrap_or(usize::MAX))
                .cloned()
                .collect())
        }
    }

    fn now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_800_000_000, 0).expect("a valid instant")
    }

    fn service(users: Arc<FakeUsers>) -> DueUsersService {
        DueUsersService::new(users)
    }

    #[tokio::test]
    async fn every_active_user_is_returned_with_its_timezone() {
        let users = Arc::new(FakeUsers {
            rows: Mutex::new(vec![crate::ports::DueCandidate {
                user_id: UserId::generate(),
                timezone: "Asia/Ho_Chi_Minh".to_string(),
            }]),
            unavailable: false,
            calls: Mutex::new(0),
        });
        let due = service(users)
            .due("daily", now())
            .await
            .expect("the read answers");
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].timezone, "Asia/Ho_Chi_Minh");
    }

    #[tokio::test]
    async fn an_unknown_period_is_refused() {
        let error = service(Arc::new(FakeUsers::default()))
            .due("yearly", now())
            .await
            .expect_err("an unknown period is refused");
        assert!(matches!(error, AppError::Validation(_)));
    }

    #[tokio::test]
    async fn a_store_outage_is_surfaced() {
        let users = Arc::new(FakeUsers {
            rows: Mutex::new(Vec::new()),
            unavailable: true,
            calls: Mutex::new(0),
        });
        let error = service(users)
            .due("weekly", now())
            .await
            .expect_err("an outage is surfaced");
        assert!(matches!(error, AppError::Unavailable(_)));
    }

    #[tokio::test]
    async fn a_short_page_stops_the_walk_after_one_read() {
        let users = Arc::new(FakeUsers::default());
        service(users.clone())
            .due("daily", now())
            .await
            .expect("the read answers");
        assert_eq!(*users.calls.lock().expect("the lock is not poisoned"), 1);
    }
}
