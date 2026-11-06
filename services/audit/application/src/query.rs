//! WF4: query the audit trail (BR5.1).

use std::sync::Arc;

use audit_domain::{AuditEvent, Context, Sensitivity};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{DateTime, Utc};

use crate::common::FieldError;
use crate::ports::{AuditEventRepository, Cursor, EventQuery, Principal, Role};
use crate::trail::Trail;

/// The default page size.
pub const DEFAULT_PAGE_SIZE: u32 = 100;
/// The largest page an operator may request.
pub const MAX_PAGE_SIZE: u32 = 100;

/// The raw query, exactly as the boundary received it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QueryCommand {
    /// A dotted action prefix.
    pub action_prefix: Option<String>,
    /// An exact target.
    pub target: Option<String>,
    /// `routine` or `security`.
    pub sensitivity: Option<String>,
    /// The lower bound on `occurredAt`.
    pub occurred_from: Option<DateTime<Utc>>,
    /// The upper bound on `occurredAt`.
    pub occurred_to: Option<DateTime<Utc>>,
    /// The mandatory lower bound on `recordedAt`.
    pub recorded_from: Option<DateTime<Utc>>,
    /// The mandatory upper bound on `recordedAt`.
    pub recorded_to: Option<DateTime<Utc>>,
    /// The opaque continuation token.
    pub cursor: Option<String>,
    /// The page size.
    pub limit: Option<u32>,
}

/// One page of events, newest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryResult {
    /// The events.
    pub events: Vec<AuditEvent>,
    /// The token for the next page, when there is one.
    pub next_cursor: Option<String>,
}

/// Why a query was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum QueryError {
    /// The caller lacks the admin role.
    #[error("admin role required")]
    Forbidden,
    /// The query failed validation.
    #[error("the query is invalid")]
    Rejected(Vec<FieldError>),
    /// The read failed.
    #[error("the trail could not be read: {0}")]
    Repository(String),
}

/// Queries the trail for an admin.
pub struct QueryEvents {
    events: Arc<dyn AuditEventRepository>,
    trail: Trail,
}

impl QueryEvents {
    /// Creates the use case.
    #[must_use]
    pub fn new(events: Arc<dyn AuditEventRepository>, trail: Trail) -> Self {
        Self { events, trail }
    }

    /// Runs the query, auditing a denial before it goes out.
    ///
    /// # Errors
    /// Returns [`QueryError::Forbidden`] for a non-admin caller,
    /// [`QueryError::Rejected`] for a bad range or cursor, and
    /// [`QueryError::Repository`] when the read fails.
    pub async fn execute(
        &self,
        actor: &Principal,
        command: QueryCommand,
    ) -> Result<QueryResult, QueryError> {
        if actor.role != Role::Admin {
            let _ = self
                .trail
                .record_denial(
                    audit_domain::UserId::parse(&actor.subject).ok(),
                    audit_domain::ActorKind::User,
                    "audit.query.denied",
                    "GET /v1/events",
                    Context::empty(),
                )
                .await;
            return Err(QueryError::Forbidden);
        }

        let (query, page_size) = self.validate(command)?;
        let mut rows = self
            .events
            .query(&query)
            .await
            .map_err(|error| QueryError::Repository(error.to_string()))?;

        let has_more = rows.len() > page_size as usize;
        rows.truncate(page_size as usize);
        let next_cursor = if has_more {
            rows.last().map(|event| {
                encode_cursor(&Cursor {
                    occurred_at: event.occurred_at(),
                    id: event.id(),
                })
            })
        } else {
            None
        };

        Ok(QueryResult {
            events: rows,
            next_cursor,
        })
    }

    fn validate(&self, command: QueryCommand) -> Result<(EventQuery, u32), QueryError> {
        let mut errors = Vec::new();

        let recorded_from = command.recorded_from;
        let recorded_to = command.recorded_to;
        let (Some(recorded_from), Some(recorded_to)) = (recorded_from, recorded_to) else {
            return Err(QueryError::Rejected(vec![FieldError::new(
                "recordedFrom",
                "the recordedAt range is mandatory",
            )]));
        };
        if recorded_from >= recorded_to {
            return Err(QueryError::Rejected(vec![FieldError::new(
                "recordedFrom",
                "recordedFrom must precede recordedTo",
            )]));
        }
        if let (Some(from), Some(to)) = (command.occurred_from, command.occurred_to) {
            if from >= to {
                errors.push(FieldError::new(
                    "occurredFrom",
                    "occurredFrom must precede occurredTo",
                ));
            }
        }

        let sensitivity = match command.sensitivity.as_deref() {
            None => None,
            Some(raw) => match Sensitivity::parse(raw) {
                Ok(value) => Some(value),
                Err(error) => {
                    errors.push(FieldError::from(error));
                    None
                }
            },
        };

        let cursor = match command.cursor.as_deref() {
            None => None,
            Some(raw) => match decode_cursor(raw) {
                Ok(value) => Some(value),
                Err(error) => {
                    errors.push(error);
                    None
                }
            },
        };

        let limit = command.limit.unwrap_or(DEFAULT_PAGE_SIZE);
        if limit == 0 || limit > MAX_PAGE_SIZE {
            errors.push(FieldError::new(
                "limit",
                format!("must be between 1 and {MAX_PAGE_SIZE}"),
            ));
        }

        if !errors.is_empty() {
            return Err(QueryError::Rejected(errors));
        }

        Ok((
            EventQuery {
                action_prefix: command.action_prefix,
                target: command.target,
                sensitivity,
                occurred_from: command.occurred_from,
                occurred_to: command.occurred_to,
                recorded_from,
                recorded_to,
                cursor,
                // Ask for one extra row so the next-cursor is exact.
                limit: limit + 1,
            },
            limit,
        ))
    }
}

/// Encodes a cursor as an opaque token.
#[must_use]
pub fn encode_cursor(cursor: &Cursor) -> String {
    let raw = format!("{}|{}", cursor.occurred_at.timestamp_millis(), cursor.id);
    URL_SAFE_NO_PAD.encode(raw)
}

/// Decodes an opaque cursor token.
///
/// # Errors
/// Returns a [`FieldError`] for a token that is not one this service issued.
pub fn decode_cursor(raw: &str) -> Result<Cursor, FieldError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(raw.trim())
        .map_err(|_| FieldError::new("cursor", "the cursor is not a token this service issued"))?;
    let text = String::from_utf8(bytes)
        .map_err(|_| FieldError::new("cursor", "the cursor is not a token this service issued"))?;
    let (instant, id) = text.split_once('|').ok_or_else(|| {
        FieldError::new("cursor", "the cursor is not a token this service issued")
    })?;
    let millis: i64 = instant
        .parse()
        .map_err(|_| FieldError::new("cursor", "the cursor is not a token this service issued"))?;
    let occurred_at = DateTime::<Utc>::from_timestamp_millis(millis).ok_or_else(|| {
        FieldError::new("cursor", "the cursor is not a token this service issued")
    })?;
    let id = audit_domain::AuditEventId::parse(id)
        .map_err(|_| FieldError::new("cursor", "the cursor is not a token this service issued"))?;
    Ok(Cursor { occurred_at, id })
}
