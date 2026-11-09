//! WF3: the user's in-app notification feed (C1), keyset paginated.

use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};
use notification_domain::{Alert, AlertId, UserId, DEFAULT_FEED_LIMIT, MAX_FEED_LIMIT};

use crate::common::FieldError;
use crate::ports::{AlertRepository, RepositoryError};

/// A keyset cursor: the last row of the previous page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeedCursor {
    /// The last row's `createdAt`.
    pub created_at: DateTime<Utc>,
    /// The last row's id, breaking ties inside one instant.
    pub id: AlertId,
}

impl FeedCursor {
    /// Renders the cursor for the response. Both halves are URL-safe: Postgres
    /// stores microseconds, so the epoch-microsecond form is lossless.
    #[must_use]
    pub fn render(&self) -> String {
        format!("{}~{}", self.created_at.timestamp_micros(), self.id)
    }

    /// Parses a cursor from the query string.
    ///
    /// # Errors
    /// Returns a field error naming `cursor` when either half is malformed.
    pub fn parse(raw: &str) -> Result<Self, FieldError> {
        let invalid = || {
            FieldError::new(
                "cursor",
                "must be the `nextCursor` value from the last page",
            )
        };
        let (micros, id) = raw.split_once('~').ok_or_else(invalid)?;
        let micros: i64 = micros.parse().map_err(|_| invalid())?;
        let seconds = micros.div_euclid(1_000_000);
        let nanos = u32::try_from(micros.rem_euclid(1_000_000))
            .map_err(|_| invalid())?
            .saturating_mul(1_000);
        let created_at = Utc
            .timestamp_opt(seconds, nanos)
            .single()
            .ok_or_else(invalid)?;
        let id = AlertId::parse(id).map_err(|_| invalid())?;
        Ok(Self { created_at, id })
    }
}

/// One feed page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedPage {
    /// The alerts, newest first.
    pub alerts: Vec<Alert>,
    /// Present when another page may exist.
    pub next_cursor: Option<String>,
    /// The page size actually used.
    pub limit: u32,
}

/// Why a feed read failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FeedError {
    /// The request failed validation.
    #[error("the feed request is invalid")]
    Rejected(Vec<FieldError>),
    /// A storage failure.
    #[error("storage failure")]
    Repository(#[from] RepositoryError),
}

/// Reads a user's feed.
pub struct FeedService {
    alerts: Arc<dyn AlertRepository>,
}

impl FeedService {
    /// Creates the use case.
    #[must_use]
    pub fn new(alerts: Arc<dyn AlertRepository>) -> Self {
        Self { alerts }
    }

    /// One keyset page, newest first.
    ///
    /// # Errors
    /// Returns [`FeedError`] for an invalid user or cursor, or a storage failure.
    pub async fn list(
        &self,
        user_id: &str,
        cursor: Option<&str>,
        limit: Option<u32>,
    ) -> Result<FeedPage, FeedError> {
        let user = UserId::parse(user_id)
            .map_err(|error| FeedError::Rejected(vec![FieldError::from(error)]))?;
        let before = match cursor.filter(|value| !value.trim().is_empty()) {
            None => None,
            Some(raw) => {
                let cursor =
                    FeedCursor::parse(raw).map_err(|error| FeedError::Rejected(vec![error]))?;
                Some((cursor.created_at, cursor.id))
            }
        };
        let limit = limit.unwrap_or(DEFAULT_FEED_LIMIT).clamp(1, MAX_FEED_LIMIT);
        let alerts = self.alerts.list_for_user(user, before, limit).await?;
        let next_cursor = if u32::try_from(alerts.len()).unwrap_or(u32::MAX) == limit {
            alerts.last().map(|alert| {
                FeedCursor {
                    created_at: alert.created_at(),
                    id: alert.id(),
                }
                .render()
            })
        } else {
            None
        };
        Ok(FeedPage {
            alerts,
            next_cursor,
            limit,
        })
    }
}
