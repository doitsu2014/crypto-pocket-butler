//! Postgres implementation of [`RetentionRepository`].

use std::collections::BTreeSet;

use async_trait::async_trait;
use audit_application::ports::{RepositoryError, RetentionRepository};
use audit_domain::{
    PartitionSummary, RetentionCategory, RetentionOutcome, RetentionOwner, RetentionRun,
    RetentionRunId, RetentionState,
};
use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{invalid_data, map_sqlx_error};

/// The partition-name prefix. Names are created by this service, so their shape
/// is a contract and is validated before any identifier is interpolated.
const PARTITION_PREFIX: &str = "audit_events_";

const RUN_COLUMNS: &str = "id, category, owner, state, cutoff_at, started_at, finished_at, \
     deleted_count, privileged_role, outcome, failure_reason";

/// sqlx-backed retention runs and the audit-trail partitions.
#[derive(Debug, Clone)]
pub struct PgRetentionRepository {
    pool: PgPool,
}

impl PgRetentionRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct RunRow {
    id: Uuid,
    category: String,
    owner: String,
    state: String,
    cutoff_at: DateTime<Utc>,
    started_at: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
    deleted_count: i32,
    privileged_role: String,
    outcome: Option<String>,
    failure_reason: Option<String>,
}

impl RunRow {
    fn into_domain(self) -> Result<RetentionRun, RepositoryError> {
        let category = RetentionCategory::parse(&self.category)
            .map_err(|error| invalid_data("read retention run", error))?;
        let owner = RetentionOwner::parse(&self.owner)
            .map_err(|error| invalid_data("read retention run", error))?;
        let state = RetentionState::parse(&self.state)
            .map_err(|error| invalid_data("read retention run", error))?;
        let outcome = match self.outcome.as_deref() {
            None => None,
            Some(raw) => Some(
                RetentionOutcome::parse(raw)
                    .map_err(|error| invalid_data("read retention run", error))?,
            ),
        };
        Ok(RetentionRun::from_stored(
            RetentionRunId::new(self.id),
            category,
            owner,
            state,
            self.cutoff_at,
            self.started_at,
            self.finished_at,
            i64::from(self.deleted_count),
            self.privileged_role,
            outcome,
            self.failure_reason,
        ))
    }
}

#[async_trait]
impl RetentionRepository for PgRetentionRepository {
    async fn insert_run(&self, run: &RetentionRun) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO retention_runs
                 (id, category, owner, state, cutoff_at, started_at, finished_at, deleted_count,
                  privileged_role, outcome, failure_reason)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(run.id().as_uuid())
        .bind(run.category().as_str())
        .bind(run.owner().as_str())
        .bind(run.state().as_str())
        .bind(run.cutoff_at())
        .bind(run.started_at())
        .bind(run.finished_at())
        .bind(i32::try_from(run.deleted_count()).unwrap_or(i32::MAX))
        .bind(run.privileged_role())
        .bind(run.outcome().map(RetentionOutcome::as_str))
        .bind(run.failure_reason())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("insert retention run", error))?;
        Ok(())
    }

    async fn update_run(&self, run: &RetentionRun) -> Result<(), RepositoryError> {
        let affected = sqlx::query(
            "UPDATE retention_runs
             SET state = $2, finished_at = $3, deleted_count = $4, outcome = $5, failure_reason = $6
             WHERE id = $1",
        )
        .bind(run.id().as_uuid())
        .bind(run.state().as_str())
        .bind(run.finished_at())
        .bind(i32::try_from(run.deleted_count()).unwrap_or(i32::MAX))
        .bind(run.outcome().map(RetentionOutcome::as_str))
        .bind(run.failure_reason())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("update retention run", error))?
        .rows_affected();
        if affected == 0 {
            return Err(RepositoryError::Conflict(format!(
                "retention run '{}' does not exist",
                run.id()
            )));
        }
        Ok(())
    }

    async fn find_run(&self, id: RetentionRunId) -> Result<Option<RetentionRun>, RepositoryError> {
        let row = sqlx::query_as::<_, RunRow>(&format!(
            "SELECT {RUN_COLUMNS} FROM retention_runs WHERE id = $1"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find retention run", error))?;
        row.map(RunRow::into_domain).transpose()
    }

    async fn partition_summaries(&self) -> Result<Vec<PartitionSummary>, RepositoryError> {
        let names: Vec<String> = sqlx::query_scalar(
            "SELECT child.relname
             FROM pg_inherits AS inheritance
             JOIN pg_class AS child ON child.oid = inheritance.inhrelid
             JOIN pg_class AS parent ON parent.oid = inheritance.inhparent
             WHERE parent.relname = 'audit_events'
               AND parent.relnamespace = current_schema()::regnamespace
             ORDER BY child.relname",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("list audit partitions", error))?;

        let mut summaries = Vec::with_capacity(names.len());
        for name in names {
            let Some((from, to)) = partition_bounds(&name) else {
                tracing::warn!(partition = %name, "ignoring an unrecognised audit partition name");
                continue;
            };
            let quoted = quote_partition(&name)?;
            let row_count: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {quoted}"))
                .fetch_one(&self.pool)
                .await
                .map_err(|error| map_sqlx_error("count audit partition", error))?;
            let holds_legal_hold: bool = sqlx::query_scalar(&format!(
                "SELECT EXISTS (SELECT 1 FROM {quoted} WHERE legal_basis IS NOT NULL)"
            ))
            .fetch_one(&self.pool)
            .await
            .map_err(|error| map_sqlx_error("check legal holds", error))?;
            summaries.push(PartitionSummary {
                name,
                from,
                to,
                holds_legal_hold,
                row_count,
            });
        }
        Ok(summaries)
    }

    async fn drop_partition(&self, name: &str) -> Result<(), RepositoryError> {
        let quoted = quote_partition(name)?;
        sqlx::query(&format!("DROP TABLE IF EXISTS {quoted}"))
            .execute(&self.pool)
            .await
            .map_err(|error| map_sqlx_error("drop audit partition", error))?;
        Ok(())
    }

    async fn delete_batch(
        &self,
        cutoff: DateTime<Utc>,
        limit: u32,
    ) -> Result<u64, RepositoryError> {
        let affected = sqlx::query(
            "DELETE FROM audit_events
             WHERE id IN (
                 SELECT id FROM audit_events
                 WHERE recorded_at < $1 AND legal_basis IS NULL
                 LIMIT $2
             )",
        )
        .bind(cutoff)
        .bind(i64::from(limit))
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("delete audit batch", error))?
        .rows_affected();
        Ok(affected)
    }

    async fn ensure_partitions_ahead(
        &self,
        months_ahead: u32,
    ) -> Result<Vec<String>, RepositoryError> {
        let today = Utc::now().date_naive();
        let mut created = BTreeSet::new();
        for offset in 0..=months_ahead {
            let Some(first) = first_of_month(today, offset) else {
                continue;
            };
            let name: String = sqlx::query_scalar("SELECT ensure_audit_partition($1)")
                .bind(first)
                .fetch_one(&self.pool)
                .await
                .map_err(|error| map_sqlx_error("ensure audit partition", error))?;
            created.insert(name);
        }
        Ok(created.into_iter().collect())
    }
}

/// Parses `audit_events_YYYYMM` into the month's UTC range.
fn partition_bounds(name: &str) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
    let suffix = name.strip_prefix(PARTITION_PREFIX)?;
    if suffix.len() != 6 || !suffix.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let year: i32 = suffix[..4].parse().ok()?;
    let month: u32 = suffix[4..].parse().ok()?;
    let from = NaiveDate::from_ymd_opt(year, month, 1)?;
    let next = first_of_month(from, 1)?;
    Some((
        Utc.from_utc_datetime(&from.and_hms_opt(0, 0, 0)?),
        Utc.from_utc_datetime(&next.and_hms_opt(0, 0, 0)?),
    ))
}

fn first_of_month(date: NaiveDate, months_ahead: u32) -> Option<NaiveDate> {
    let mut year = date.year();
    let mut month = date.month() + months_ahead;
    while month > 12 {
        month -= 12;
        year += 1;
    }
    NaiveDate::from_ymd_opt(year, month, 1)
}

fn quote_partition(name: &str) -> Result<String, RepositoryError> {
    if partition_bounds(name).is_none() {
        return Err(invalid_data(
            "partition name",
            "the name is not an audit_events_YYYYMM partition",
        ));
    }
    Ok(format!("\"{name}\""))
}
