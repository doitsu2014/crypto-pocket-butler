//! WF5: export audit records (BR5.1, BR5.2).
//!
//! The export runs synchronously on the request that asked for it: with no queue
//! and no durable worker, an accepted-but-never-produced export is worse than a
//! slow one. Recording comes first, so an export that cannot be recorded is not
//! produced.

use std::sync::Arc;

use audit_domain::{
    AuditEvent, AuditExport, AuditExportId, Context, ExportFilter, Sensitivity, UserId,
    MAX_EXPORT_ROWS,
};
use chrono::Duration;

use crate::common::FieldError;
use crate::ports::{AuditEventRepository, Clock, Cursor, ExportRepository, Principal, Role};
use crate::trail::Trail;

/// The export batch size.
pub const EXPORT_BATCH_SIZE: u32 = 5_000;

/// The maximum number of log lines one export streams.
pub const EXPORT_ROW_CAP: u64 = MAX_EXPORT_ROWS as u64;

/// How old a `requested` export must be before startup marks it failed.
pub const STALE_EXPORT_AFTER_MINUTES: i64 = 10;

/// Why an export was refused or failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExportError {
    /// The caller lacks the admin role.
    #[error("admin role required")]
    Forbidden,
    /// The filter failed validation.
    #[error("the export filter is invalid")]
    Rejected(Vec<FieldError>),
    /// More rows matched than the cap; the operator must narrow the filter.
    #[error("the export matches more than {cap} rows; narrow the filter")]
    TooLarge {
        /// The cap in force.
        cap: u64,
    },
    /// Production failed after the request was recorded.
    #[error("the export could not be produced: {0}")]
    Failed(String),
    /// Storage failed.
    #[error("the export could not be stored: {0}")]
    Repository(String),
}

/// Records and produces one export.
pub struct ExportAudit {
    events: Arc<dyn AuditEventRepository>,
    exports: Arc<dyn ExportRepository>,
    clock: Arc<dyn Clock>,
    trail: Trail,
}

impl ExportAudit {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        events: Arc<dyn AuditEventRepository>,
        exports: Arc<dyn ExportRepository>,
        clock: Arc<dyn Clock>,
        trail: Trail,
    ) -> Self {
        Self {
            events,
            exports,
            clock,
            trail,
        }
    }

    /// Records the request, checks the cap, and returns a session that streams
    /// the rows.
    ///
    /// # Errors
    /// Returns [`ExportError::Forbidden`] for a non-admin caller,
    /// [`ExportError::Rejected`] for an invalid filter,
    /// [`ExportError::TooLarge`] when the cap is exceeded, and
    /// [`ExportError::Repository`] when recording fails.
    pub async fn start(
        &self,
        actor: &Principal,
        filter: ExportFilter,
    ) -> Result<ExportSession, ExportError> {
        if actor.role != Role::Admin {
            let _ = self
                .trail
                .record_denial(
                    UserId::parse(&actor.subject).ok(),
                    audit_domain::ActorKind::User,
                    "audit.export.denied",
                    "POST /v1/exports",
                    Context::empty(),
                )
                .await;
            return Err(ExportError::Forbidden);
        }
        let requested_by = UserId::parse(&actor.subject)
            .map_err(|error| ExportError::Rejected(vec![FieldError::from(error)]))?;

        let now = self.clock.now();
        let export =
            AuditExport::requested(AuditExportId::generate(), requested_by, filter.clone(), now);
        self.exports
            .insert(&export)
            .await
            .map_err(|error| ExportError::Repository(error.to_string()))?;

        let matching = self
            .events
            .count_for_export(&filter, EXPORT_ROW_CAP + 1)
            .await
            .map_err(|error| ExportError::Repository(error.to_string()))?;

        if matching > EXPORT_ROW_CAP {
            let failed = export
                .fail(self.clock.now())
                .map_err(|error| ExportError::Repository(error.to_string()))?;
            let _ = self.exports.update(&failed).await;
            let _ = self
                .trail
                .record_user(
                    requested_by,
                    "audit.export.failed",
                    &failed.id().to_string(),
                    audit_domain::Outcome::Failed,
                    Sensitivity::Routine,
                    Context::empty(),
                )
                .await;
            return Err(ExportError::TooLarge {
                cap: EXPORT_ROW_CAP,
            });
        }

        Ok(ExportSession {
            export,
            filter,
            events: self.events.clone(),
            exports: self.exports.clone(),
            clock: self.clock.clone(),
            trail: self.trail.clone(),
            cursor: None,
            streamed: 0,
            finished: false,
        })
    }

    /// Marks `requested` exports older than the recovery window as failed.
    ///
    /// # Errors
    /// Returns [`ExportError::Repository`] when the recovery update fails.
    pub async fn recover_stale(&self) -> Result<u64, ExportError> {
        let now = self.clock.now();
        let before = now - Duration::minutes(STALE_EXPORT_AFTER_MINUTES);
        let recovered = self
            .exports
            .mark_stale_failed(before, now)
            .await
            .map_err(|error| ExportError::Repository(error.to_string()))?;
        if recovered > 0 {
            let _ = self
                .trail
                .record_system(
                    "system.export.recovery",
                    "audit_exports",
                    audit_domain::Outcome::Success,
                    Sensitivity::Routine,
                    Context::empty(),
                )
                .await;
        }
        Ok(recovered)
    }
}

/// One in-flight export. The HTTP handler drains [`ExportSession::next_batch`]
/// into the response body and hashes the exact bytes it wrote.
pub struct ExportSession {
    export: AuditExport,
    filter: ExportFilter,
    events: Arc<dyn AuditEventRepository>,
    exports: Arc<dyn ExportRepository>,
    clock: Arc<dyn Clock>,
    trail: Trail,
    cursor: Option<Cursor>,
    streamed: u64,
    finished: bool,
}

impl std::fmt::Debug for ExportSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExportSession")
            .field("export", &self.export.id())
            .field("streamed", &self.streamed)
            .field("finished", &self.finished)
            .finish_non_exhaustive()
    }
}

impl ExportSession {
    /// The export record id.
    #[must_use]
    pub fn id(&self) -> AuditExportId {
        self.export.id()
    }

    /// The number of rows streamed so far.
    #[must_use]
    pub fn streamed(&self) -> u64 {
        self.streamed
    }

    /// Returns the next batch, or `None` when the stream is complete.
    ///
    /// # Errors
    /// Returns [`ExportError::TooLarge`] if the stream would pass the cap and
    /// [`ExportError::Repository`] on a read failure.
    pub async fn next_batch(&mut self) -> Result<Option<Vec<AuditEvent>>, ExportError> {
        if self.finished {
            return Ok(None);
        }
        let batch = self
            .events
            .export_batch(&self.filter, self.cursor, EXPORT_BATCH_SIZE)
            .await
            .map_err(|error| ExportError::Repository(error.to_string()))?;
        if batch.is_empty() {
            return Ok(None);
        }
        if let Some(last) = batch.last() {
            self.cursor = Some(Cursor {
                occurred_at: last.occurred_at(),
                id: last.id(),
            });
        }
        self.streamed += batch.len() as u64;
        if self.streamed > EXPORT_ROW_CAP {
            return Err(ExportError::TooLarge {
                cap: EXPORT_ROW_CAP,
            });
        }
        Ok(Some(batch))
    }

    /// Records the export as produced, with the digest of the body sent.
    ///
    /// # Errors
    /// Returns [`ExportError::Repository`] when the update or the audit write fails.
    pub async fn finish(mut self, content_sha256: &str) -> Result<AuditExport, ExportError> {
        let completed = self
            .export
            .clone()
            .succeed(self.streamed as i64, content_sha256, self.clock.now())
            .map_err(|error| ExportError::Repository(error.to_string()))?;
        self.exports
            .update(&completed)
            .await
            .map_err(|error| ExportError::Repository(error.to_string()))?;
        let _ = self
            .trail
            .record_user(
                completed.requested_by(),
                "audit.export.completed",
                &completed.id().to_string(),
                audit_domain::Outcome::Success,
                Sensitivity::Routine,
                Context::empty(),
            )
            .await;
        self.finished = true;
        Ok(completed)
    }

    /// Records the export as failed and returns the reason for the caller.
    ///
    /// # Errors
    /// Returns [`ExportError::Repository`] when the update fails.
    pub async fn fail(mut self, reason: &str) -> Result<(), ExportError> {
        let failed = self
            .export
            .clone()
            .fail(self.clock.now())
            .map_err(|error| ExportError::Repository(error.to_string()))?;
        self.exports
            .update(&failed)
            .await
            .map_err(|error| ExportError::Repository(error.to_string()))?;
        tracing::error!(
            export_id = %failed.id(),
            reason = %reason,
            "audit export failed after the request was recorded"
        );
        let _ = self
            .trail
            .record_user(
                failed.requested_by(),
                "audit.export.failed",
                &failed.id().to_string(),
                audit_domain::Outcome::Failed,
                Sensitivity::Routine,
                Context::empty(),
            )
            .await;
        self.finished = true;
        Ok(())
    }
}
