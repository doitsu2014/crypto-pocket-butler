//! Postgres implementation of [`AuditEventRepository`].
//!
//! The trail is partitioned by `recorded_at`, so every read carries that range
//! and every insert names its partition. Uniqueness cannot live on the parent, so
//! the idempotency key is held in the unpartitioned `append_idempotency` table
//! and written in the same transaction as the event.

use std::collections::BTreeMap;

use async_trait::async_trait;
use audit_application::ports::{
    AppendOutcome, AuditEventRepository, Cursor, EventQuery, ExportFilter, RepositoryError,
};
use audit_domain::{
    from_stored, ActorKind, AuditEvent, AuditEventId, DeletionRequestId, EventClass, Outcome,
    Sensitivity, UserId,
};
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::{invalid_data, map_sqlx_error};

const EVENT_COLUMNS: &str = "id, actor_id, actor_kind, action, target, occurred_at, recorded_at, \
     context, outcome, sensitivity, event_class, idempotency_key, legal_basis, deletion_request_id";

/// sqlx-backed audit trail.
#[derive(Debug, Clone)]
pub struct PgAuditEventRepository {
    pool: PgPool,
}

impl PgAuditEventRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct EventRow {
    id: Uuid,
    actor_id: Option<Uuid>,
    actor_kind: String,
    action: String,
    target: String,
    occurred_at: DateTime<Utc>,
    recorded_at: DateTime<Utc>,
    context: Option<serde_json::Value>,
    outcome: String,
    sensitivity: String,
    event_class: String,
    idempotency_key: Option<String>,
    legal_basis: Option<String>,
    deletion_request_id: Option<Uuid>,
}

impl EventRow {
    fn into_domain(self) -> Result<AuditEvent, RepositoryError> {
        let context: BTreeMap<String, String> = match self.context {
            None => BTreeMap::new(),
            Some(serde_json::Value::Object(map)) => {
                let mut entries = BTreeMap::new();
                for (key, value) in map {
                    let value = match value {
                        serde_json::Value::String(text) => text,
                        other => other.to_string(),
                    };
                    entries.insert(key, value);
                }
                entries
            }
            Some(_) => {
                return Err(invalid_data(
                    "read audit event",
                    "context is not a JSON object",
                ));
            }
        };
        from_stored(
            AuditEventId::new(self.id),
            self.actor_id.map(UserId::new),
            ActorKind::parse(&self.actor_kind)
                .map_err(|error| invalid_data("read event", error))?,
            self.action,
            self.target,
            self.occurred_at,
            self.recorded_at,
            context,
            Outcome::parse(&self.outcome).map_err(|error| invalid_data("read event", error))?,
            Sensitivity::parse(&self.sensitivity)
                .map_err(|error| invalid_data("read event", error))?,
            EventClass::parse(&self.event_class)
                .map_err(|error| invalid_data("read event", error))?,
            self.idempotency_key,
            self.legal_basis,
            self.deletion_request_id.map(DeletionRequestId::new),
        )
        .map_err(|error| invalid_data("read event", error))
    }
}

#[async_trait]
impl AuditEventRepository for PgAuditEventRepository {
    async fn append(
        &self,
        event: &AuditEvent,
        emitting_service: &str,
        payload_sha256: &str,
    ) -> Result<AppendOutcome, RepositoryError> {
        let context = serde_json::to_value(event.context().as_map())
            .map_err(|error| invalid_data("append event", error))?;
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| map_sqlx_error("begin append", error))?;

        sqlx::query(
            "INSERT INTO audit_events
                 (id, actor_id, actor_kind, action, target, occurred_at, recorded_at,
                  context, outcome, sensitivity, event_class, idempotency_key,
                  legal_basis, deletion_request_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
        )
        .bind(event.id().as_uuid())
        .bind(event.actor_id().map(|id| id.as_uuid()))
        .bind(event.actor_kind().as_str())
        .bind(event.action())
        .bind(event.target())
        .bind(event.occurred_at())
        .bind(event.recorded_at())
        .bind(context)
        .bind(event.outcome().as_str())
        .bind(event.sensitivity().as_str())
        .bind(event.event_class().as_str())
        .bind(event.idempotency_key())
        .bind(event.legal_basis())
        .bind(event.deletion_request_id().map(|id| id.as_uuid()))
        .execute(&mut *transaction)
        .await
        .map_err(|error| map_sqlx_error("insert audit event", error))?;

        let Some(key) = event.idempotency_key() else {
            transaction
                .commit()
                .await
                .map_err(|error| map_sqlx_error("commit append", error))?;
            return Ok(AppendOutcome::Appended);
        };

        let inserted = sqlx::query(
            "INSERT INTO append_idempotency (emitting_service, idempotency_key, event_id, payload_sha256)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(emitting_service)
        .bind(key)
        .bind(event.id().as_uuid())
        .bind(payload_sha256)
        .execute(&mut *transaction)
        .await;

        match inserted {
            Ok(_) => {
                transaction
                    .commit()
                    .await
                    .map_err(|error| map_sqlx_error("commit append", error))?;
                Ok(AppendOutcome::Appended)
            }
            Err(error) if is_unique_violation(&error) => {
                transaction
                    .rollback()
                    .await
                    .map_err(|error| map_sqlx_error("rollback append", error))?;
                let existing = sqlx::query_as::<_, (Uuid, String)>(
                    "SELECT event_id, payload_sha256
                     FROM append_idempotency
                     WHERE emitting_service = $1 AND idempotency_key = $2",
                )
                .bind(emitting_service)
                .bind(key)
                .fetch_optional(&self.pool)
                .await
                .map_err(|error| map_sqlx_error("read idempotency row", error))?
                .ok_or_else(|| {
                    repository_conflict("the idempotency conflict had no visible winner")
                })?;
                Ok(AppendOutcome::Duplicate {
                    event_id: AuditEventId::new(existing.0),
                    same_payload: existing.1 == payload_sha256,
                })
            }
            Err(error) => Err(map_sqlx_error("insert idempotency key", error)),
        }
    }

    async fn find_by_id(&self, id: AuditEventId) -> Result<Option<AuditEvent>, RepositoryError> {
        let row = sqlx::query_as::<_, EventRow>(&format!(
            "SELECT {EVENT_COLUMNS} FROM audit_events WHERE id = $1"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find audit event", error))?;
        row.map(EventRow::into_domain).transpose()
    }

    async fn query(&self, query: &EventQuery) -> Result<Vec<AuditEvent>, RepositoryError> {
        let mut builder = QueryBuilder::<Postgres>::new(format!(
            "SELECT {EVENT_COLUMNS} FROM audit_events WHERE recorded_at >= "
        ));
        builder.push_bind(query.recorded_from);
        builder.push(" AND recorded_at < ");
        builder.push_bind(query.recorded_to);
        push_filters(
            &mut builder,
            query.action_prefix.as_deref(),
            query.target.as_deref(),
            query.sensitivity,
            query.occurred_from,
            query.occurred_to,
        );
        if let Some(cursor) = query.cursor {
            builder.push(" AND (occurred_at, id) < (");
            builder.push_bind(cursor.occurred_at);
            builder.push(", ");
            builder.push_bind(cursor.id.as_uuid());
            builder.push(")");
        }
        builder.push(" ORDER BY occurred_at DESC, id DESC LIMIT ");
        builder.push_bind(i64::from(query.limit));

        let rows = builder
            .build_query_as::<EventRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(|error| map_sqlx_error("query audit events", error))?;
        rows.into_iter().map(EventRow::into_domain).collect()
    }

    async fn count_for_export(
        &self,
        filter: &ExportFilter,
        cap: u64,
    ) -> Result<u64, RepositoryError> {
        let mut builder = QueryBuilder::<Postgres>::new(
            "SELECT count(*) FROM (SELECT 1 FROM audit_events WHERE recorded_at >= ",
        );
        builder.push_bind(filter.recorded_from);
        builder.push(" AND recorded_at < ");
        builder.push_bind(filter.recorded_to);
        push_export_filters(&mut builder, filter);
        builder.push(" LIMIT ");
        builder.push_bind(i64::try_from(cap).unwrap_or(i64::MAX));
        builder.push(") AS capped");

        builder
            .build_query_scalar::<i64>()
            .fetch_one(&self.pool)
            .await
            .map(|count| count.max(0) as u64)
            .map_err(|error| map_sqlx_error("count export rows", error))
    }

    async fn export_batch(
        &self,
        filter: &ExportFilter,
        after: Option<Cursor>,
        limit: u32,
    ) -> Result<Vec<AuditEvent>, RepositoryError> {
        let mut builder = QueryBuilder::<Postgres>::new(format!(
            "SELECT {EVENT_COLUMNS} FROM audit_events WHERE recorded_at >= "
        ));
        builder.push_bind(filter.recorded_from);
        builder.push(" AND recorded_at < ");
        builder.push_bind(filter.recorded_to);
        push_export_filters(&mut builder, filter);
        if let Some(cursor) = after {
            builder.push(" AND (occurred_at, id) > (");
            builder.push_bind(cursor.occurred_at);
            builder.push(", ");
            builder.push_bind(cursor.id.as_uuid());
            builder.push(")");
        }
        builder.push(" ORDER BY occurred_at ASC, id ASC LIMIT ");
        builder.push_bind(i64::from(limit));

        let rows = builder
            .build_query_as::<EventRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(|error| map_sqlx_error("export audit events", error))?;
        rows.into_iter().map(EventRow::into_domain).collect()
    }
}

fn push_filters(
    builder: &mut QueryBuilder<'_, Postgres>,
    action_prefix: Option<&str>,
    target: Option<&str>,
    sensitivity: Option<Sensitivity>,
    occurred_from: Option<DateTime<Utc>>,
    occurred_to: Option<DateTime<Utc>>,
) {
    if let Some(prefix) = action_prefix {
        builder.push(" AND action LIKE ");
        builder.push_bind(prefix_pattern(prefix));
    }
    if let Some(target) = target {
        builder.push(" AND target = ");
        builder.push_bind(target.to_string());
    }
    if let Some(sensitivity) = sensitivity {
        builder.push(" AND sensitivity = ");
        builder.push_bind(sensitivity.as_str());
    }
    if let Some(from) = occurred_from {
        builder.push(" AND occurred_at >= ");
        builder.push_bind(from);
    }
    if let Some(to) = occurred_to {
        builder.push(" AND occurred_at < ");
        builder.push_bind(to);
    }
}

fn push_export_filters(builder: &mut QueryBuilder<'_, Postgres>, filter: &ExportFilter) {
    if !filter.action_prefixes.is_empty() {
        builder.push(" AND (");
        for (index, prefix) in filter.action_prefixes.iter().enumerate() {
            if index > 0 {
                builder.push(" OR ");
            }
            builder.push("action LIKE ");
            builder.push_bind(prefix_pattern(prefix));
        }
        builder.push(")");
    }
    if let Some(target) = &filter.target {
        builder.push(" AND target = ");
        builder.push_bind(target.clone());
    }
    if let Some(sensitivity) = filter.sensitivity {
        builder.push(" AND sensitivity = ");
        builder.push_bind(sensitivity.as_str());
    }
    if let Some(from) = filter.occurred_from {
        builder.push(" AND occurred_at >= ");
        builder.push_bind(from);
    }
    if let Some(to) = filter.occurred_to {
        builder.push(" AND occurred_at < ");
        builder.push_bind(to);
    }
}

/// A `LIKE` prefix with the wildcard characters escaped, so an action prefix is
/// always a literal prefix.
fn prefix_pattern(prefix: &str) -> String {
    let escaped = prefix
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("{escaped}%")
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    matches!(error, sqlx::Error::Database(database) if database.code().as_deref() == Some("23505"))
}

fn repository_conflict(message: &str) -> RepositoryError {
    RepositoryError::Conflict(message.to_string())
}
