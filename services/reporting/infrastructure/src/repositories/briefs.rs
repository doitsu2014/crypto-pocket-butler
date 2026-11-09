//! Postgres implementation of [`BriefRepository`].

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reporting_application::ports::{BriefRepository, RepositoryError, StoredBrief};
use reporting_domain::{Brief, BriefId, DataStatus, Period, UserId};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use super::{invalid_data, map_error};

const BRIEF_COLUMNS: &str =
    "id, user_id, period, period_start, period_end, status, failure_reason, as_of, \
     data_status, created_at, updated_at";

/// sqlx-backed brief store.
#[derive(Debug, Clone)]
pub struct PgBriefRepository {
    pool: PgPool,
}

impl PgBriefRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct BriefRow {
    id: Uuid,
    user_id: Uuid,
    period: String,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
    status: String,
    failure_reason: Option<String>,
    as_of: Option<DateTime<Utc>>,
    data_status: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    #[sqlx(default)]
    sections: Option<Value>,
}

impl BriefRow {
    fn into_brief(self) -> Result<Brief, RepositoryError> {
        let period =
            Period::parse(&self.period).map_err(|error| invalid_data("read brief", error))?;
        let status = reporting_domain::BriefStatus::parse(&self.status)
            .map_err(|error| invalid_data("read brief", error))?;
        let data_status = match self.data_status.as_deref() {
            None => None,
            Some(raw) => {
                Some(DataStatus::parse(raw).map_err(|error| invalid_data("read brief", error))?)
            }
        };
        Brief::restore(
            BriefId::new(self.id),
            UserId::new(self.user_id),
            period,
            self.period_start,
            self.period_end,
            status,
            self.failure_reason,
            self.as_of,
            data_status,
            self.created_at,
            self.updated_at,
        )
        .map_err(|error| invalid_data("read brief", error))
    }
}

#[async_trait]
impl BriefRepository for PgBriefRepository {
    async fn upsert_generated(
        &self,
        brief: &Brief,
        sections: &Value,
    ) -> Result<Brief, RepositoryError> {
        let row = sqlx::query_as::<_, BriefRow>(&format!(
            "INSERT INTO briefs
                 ({BRIEF_COLUMNS}, sections, anonymised_at)
             VALUES ($1, $2, $3, $4, $5, 'generated', NULL, $6, $7, $8, $9, $10, NULL)
             ON CONFLICT (user_id, period, period_start) DO UPDATE
             SET status = 'generated',
                 failure_reason = NULL,
                 as_of = EXCLUDED.as_of,
                 data_status = EXCLUDED.data_status,
                 sections = EXCLUDED.sections,
                 updated_at = EXCLUDED.updated_at
             WHERE briefs.status <> 'generated'
             RETURNING {BRIEF_COLUMNS}, sections"
        ))
        .bind(brief.id().as_uuid())
        .bind(brief.user_id().as_uuid())
        .bind(brief.period().as_str())
        .bind(brief.period_start())
        .bind(brief.period_end())
        .bind(brief.as_of())
        .bind(brief.data_status().map(DataStatus::as_str))
        .bind(brief.created_at())
        .bind(brief.updated_at())
        .bind(sections)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_error)?;
        match row {
            Some(row) => Ok(row.into_brief()?),
            None => {
                // A concurrent run already generated this window (BR1.1).
                let existing = self
                    .find_window(brief.user_id(), brief.period(), brief.period_start())
                    .await?
                    .ok_or_else(|| {
                        RepositoryError::Conflict(
                            "the window conflicted but no brief is readable".to_string(),
                        )
                    })?;
                Ok(existing.brief)
            }
        }
    }

    async fn upsert_failed(&self, brief: &Brief) -> Result<Brief, RepositoryError> {
        let reason = brief.failure_reason().ok_or_else(|| {
            RepositoryError::InvalidData("a failed brief needs a reason".to_string())
        })?;
        let row = sqlx::query_as::<_, BriefRow>(&format!(
            "INSERT INTO briefs
                 ({BRIEF_COLUMNS}, sections, anonymised_at)
             VALUES ($1, $2, $3, $4, $5, 'failed', $6, NULL, NULL, $7, $8, NULL, NULL)
             ON CONFLICT (user_id, period, period_start) DO UPDATE
             SET status = 'failed',
                 failure_reason = EXCLUDED.failure_reason,
                 as_of = NULL,
                 data_status = NULL,
                 sections = NULL,
                 updated_at = EXCLUDED.updated_at
             WHERE briefs.status <> 'generated'
             RETURNING {BRIEF_COLUMNS}, sections"
        ))
        .bind(brief.id().as_uuid())
        .bind(brief.user_id().as_uuid())
        .bind(brief.period().as_str())
        .bind(brief.period_start())
        .bind(brief.period_end())
        .bind(reason)
        .bind(brief.created_at())
        .bind(brief.updated_at())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_error)?;
        match row {
            Some(row) => Ok(row.into_brief()?),
            None => {
                let existing = self
                    .find_window(brief.user_id(), brief.period(), brief.period_start())
                    .await?
                    .ok_or_else(|| {
                        RepositoryError::Conflict(
                            "the window conflicted but no brief is readable".to_string(),
                        )
                    })?;
                Ok(existing.brief)
            }
        }
    }

    async fn find_by_id(&self, id: BriefId) -> Result<Option<StoredBrief>, RepositoryError> {
        let row = sqlx::query_as::<_, BriefRow>(&format!(
            "SELECT {BRIEF_COLUMNS}, sections FROM briefs WHERE id = $1"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_error)?;
        row.map(stored).transpose()
    }

    async fn find_window(
        &self,
        user_id: UserId,
        period: Period,
        period_start: DateTime<Utc>,
    ) -> Result<Option<StoredBrief>, RepositoryError> {
        let row = sqlx::query_as::<_, BriefRow>(&format!(
            "SELECT {BRIEF_COLUMNS}, sections FROM briefs
             WHERE user_id = $1 AND period = $2 AND period_start = $3"
        ))
        .bind(user_id.as_uuid())
        .bind(period.as_str())
        .bind(period_start)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_error)?;
        row.map(stored).transpose()
    }

    async fn latest_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Option<StoredBrief>, RepositoryError> {
        let row = sqlx::query_as::<_, BriefRow>(&format!(
            "SELECT {BRIEF_COLUMNS}, sections FROM briefs
             WHERE user_id = $1 AND status = 'generated'
             ORDER BY period_start DESC
             LIMIT 1"
        ))
        .bind(user_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_error)?;
        row.map(stored).transpose()
    }

    async fn list_for_user(
        &self,
        user_id: UserId,
        period: Option<Period>,
        limit: i64,
        before: Option<DateTime<Utc>>,
    ) -> Result<Vec<Brief>, RepositoryError> {
        let rows = sqlx::query_as::<_, BriefRow>(&format!(
            "SELECT {BRIEF_COLUMNS} FROM briefs
             WHERE user_id = $1
               AND ($2::text IS NULL OR period = $2::text)
               AND ($3::timestamptz IS NULL OR updated_at < $3::timestamptz)
             ORDER BY updated_at DESC, id DESC
             LIMIT $4"
        ))
        .bind(user_id.as_uuid())
        .bind(period.map(Period::as_str))
        .bind(before)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_error)?;
        rows.into_iter().map(BriefRow::into_brief).collect()
    }

    async fn list_for_user_export(
        &self,
        user_id: UserId,
    ) -> Result<Vec<StoredBrief>, RepositoryError> {
        let rows = sqlx::query_as::<_, BriefRow>(&format!(
            "SELECT {BRIEF_COLUMNS}, sections FROM briefs
             WHERE user_id = $1
             ORDER BY period_start DESC"
        ))
        .bind(user_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(map_error)?;
        rows.into_iter().map(stored).collect()
    }

    async fn anonymise_for_user(
        &self,
        user_id: UserId,
        anonymised_ref: UserId,
        at: DateTime<Utc>,
    ) -> Result<u64, RepositoryError> {
        let result = sqlx::query(
            "UPDATE briefs
             SET user_id = $2, anonymised_at = $3, updated_at = $3
             WHERE user_id = $1",
        )
        .bind(user_id.as_uuid())
        .bind(anonymised_ref.as_uuid())
        .bind(at)
        .execute(&self.pool)
        .await
        .map_err(map_error)?;
        Ok(result.rows_affected())
    }

    async fn delete_generated_before(&self, cutoff: DateTime<Utc>) -> Result<u64, RepositoryError> {
        let result = sqlx::query("DELETE FROM briefs WHERE period_end < $1")
            .bind(cutoff)
            .execute(&self.pool)
            .await
            .map_err(map_error)?;
        Ok(result.rows_affected())
    }
}

fn stored(row: BriefRow) -> Result<StoredBrief, RepositoryError> {
    let sections = row.sections.clone();
    Ok(StoredBrief {
        brief: row.into_brief()?,
        sections,
    })
}
