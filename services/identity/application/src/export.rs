//! WF8 — asynchronous data export and its assembly worker (BR5.1–BR5.4).

use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use identity_domain::{
    find_secret_field, AuditOutcome, Contributor, ExportRequest, ExportRequestId,
    DEFAULT_ASSEMBLY_MINUTES, DOWNLOAD_WINDOW_HOURS, LEASE_SECONDS,
};

use crate::common::{AppError, Caller};
use crate::ports::{Clock, ContributorGateway, ExportRepository, ExportStore, WakeTransport};
use crate::trail::{context, Trail};

/// The export status plus the signed download when one is available.
#[derive(Debug, Clone)]
pub struct ExportStatusView {
    export: ExportRequest,
    signed_download: Option<String>,
}

impl ExportStatusView {
    /// The stored request.
    #[must_use]
    pub fn export(&self) -> &ExportRequest {
        &self.export
    }

    /// The owner-only signed URL, while the download window is open.
    #[must_use]
    pub fn signed_download(&self) -> Option<&str> {
        self.signed_download.as_deref()
    }
}

/// Creates export requests and answers status polls.
pub struct ExportService {
    exports: Arc<dyn ExportRepository>,
    store: Arc<dyn ExportStore>,
    trail: Arc<Trail>,
    wake: Arc<dyn WakeTransport>,
    clock: Arc<dyn Clock>,
}

impl ExportService {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        exports: Arc<dyn ExportRepository>,
        store: Arc<dyn ExportStore>,
        trail: Arc<Trail>,
        wake: Arc<dyn WakeTransport>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            exports,
            store,
            trail,
            wake,
            clock,
        }
    }

    /// Requests an export, returning the user's active request when one exists.
    ///
    /// # Errors
    /// Returns [`AppError::Unavailable`] when the request or its audit event
    /// cannot be written. A failed wake only degrades to the periodic sweep.
    pub async fn request(&self, caller: &Caller) -> Result<ExportRequest, AppError> {
        if let Some(active) = self
            .exports
            .find_active_for_user(caller.user_id())
            .await
            .map_err(AppError::from_repository)?
        {
            return Ok(active);
        }
        let now = self.clock.now();
        let export = ExportRequest::request(
            ExportRequestId::generate(),
            caller.user_id(),
            now,
            now + Duration::minutes(DEFAULT_ASSEMBLY_MINUTES),
        );
        self.exports
            .insert(&export)
            .await
            .map_err(AppError::from_repository)?;
        self.trail
            .record(
                Some(caller.user_id()),
                "identity.export.requested",
                export.id().to_string(),
                AuditOutcome::Success,
                &caller.correlation_id,
                context(&[]),
            )
            .await?;
        if let Err(error) = self.wake.wake().await {
            tracing::warn!(%error, "export wake failed; the sweep will pick the request up");
        }
        Ok(export)
    }

    /// Returns the progress and, while the download window is open, a signed
    /// link. Only the owner may read the request (BR5.3).
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for another user and
    /// [`AppError::Unavailable`] when the link or its audit event cannot be
    /// produced.
    pub async fn status(
        &self,
        caller: &Caller,
        export_id: ExportRequestId,
    ) -> Result<ExportStatusView, AppError> {
        let mut export = self
            .exports
            .find_by_id(export_id)
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("export"))?;
        if export.user_id() != caller.user_id() {
            return Err(AppError::Forbidden(
                "only the owner may read an export".to_string(),
            ));
        }
        let now = self.clock.now();
        if export.expire_if_due(now) {
            self.exports
                .update(&export)
                .await
                .map_err(AppError::from_repository)?;
        }

        let mut signed_download = None;
        if export.is_ready() {
            if let (Some(key), Some(expires_at)) =
                (export.file_reference(), export.download_expires_at())
            {
                if now < expires_at {
                    let url = self
                        .store
                        .signed_url(key, expires_at - now)
                        .await
                        .map_err(AppError::from_external)?;
                    self.trail
                        .record(
                            Some(caller.user_id()),
                            "identity.export.downloaded",
                            export.id().to_string(),
                            AuditOutcome::Success,
                            &caller.correlation_id,
                            context(&[]),
                        )
                        .await?;
                    signed_download = Some(url);
                }
            }
        }
        Ok(ExportStatusView {
            export,
            signed_download,
        })
    }
}

/// Assembles one export per run. A dead worker's lease is re-claimed by the next
/// sweep, and a failed contribution marks the request failed.
pub struct ExportWorker {
    exports: Arc<dyn ExportRepository>,
    contributors: Arc<dyn ContributorGateway>,
    store: Arc<dyn ExportStore>,
    trail: Arc<Trail>,
}

impl ExportWorker {
    /// Creates the worker.
    #[must_use]
    pub fn new(
        exports: Arc<dyn ExportRepository>,
        contributors: Arc<dyn ContributorGateway>,
        store: Arc<dyn ExportStore>,
        trail: Arc<Trail>,
    ) -> Self {
        Self {
            exports,
            contributors,
            store,
            trail,
        }
    }

    /// Runs one export assembly pass. Returns how many requests it processed.
    ///
    /// # Errors
    /// Returns [`AppError`] when the store cannot be reached. A failed
    /// contribution is recorded on the request rather than returned.
    pub async fn run_once(&self, worker: &str, now: DateTime<Utc>) -> Result<usize, AppError> {
        self.expire_archives(now).await?;

        let Some(mut export) = self
            .exports
            .claim_next(worker, now, Duration::seconds(LEASE_SECONDS))
            .await
            .map_err(AppError::from_repository)?
        else {
            return Ok(0);
        };
        let user_id = export.user_id();
        let export_label = export.id().to_string();
        let correlation = format!("export-{export_label}");

        let mut data = serde_json::Map::new();
        for contributor in Contributor::ALL {
            match self.contributors.export(contributor, user_id).await {
                Ok(value) => {
                    export.record_success(contributor, now);
                    data.insert(contributor.as_str().to_string(), value);
                    self.exports
                        .update(&export)
                        .await
                        .map_err(AppError::from_repository)?;
                }
                Err(error) => {
                    export.record_failure(contributor, error.to_string(), now);
                    self.exports
                        .update(&export)
                        .await
                        .map_err(AppError::from_repository)?;
                    self.audit_failure(&export, &correlation).await;
                    return Ok(1);
                }
            }
        }

        let archive = serde_json::json!({
            "exportId": export_label,
            "generatedAt": now,
            "data": data,
        });
        let body = serde_json::to_vec_pretty(&archive).map_err(|error| {
            AppError::Internal(format!("archive serialization failed: {error}"))
        })?;
        let text = String::from_utf8_lossy(&body);
        if let Some(field) = find_secret_field(&text) {
            export.fail(
                format!("the archive contained a credential-shaped field '{field}'"),
                now,
            );
            self.exports
                .update(&export)
                .await
                .map_err(AppError::from_repository)?;
            self.audit_failure(&export, &correlation).await;
            return Ok(1);
        }

        let key = format!("{export_label}.json");
        if let Err(error) = self.store.put(&key, body).await {
            export.fail(error.to_string(), now);
            self.exports
                .update(&export)
                .await
                .map_err(AppError::from_repository)?;
            self.audit_failure(&export, &correlation).await;
            return Ok(1);
        }

        export
            .mark_ready(key, now + Duration::hours(DOWNLOAD_WINDOW_HOURS), now)
            .map_err(AppError::from_domain)?;
        self.exports
            .update(&export)
            .await
            .map_err(AppError::from_repository)?;
        if let Err(error) = self
            .trail
            .record(
                Some(user_id),
                "identity.export.ready",
                export_label,
                AuditOutcome::Success,
                &correlation,
                context(&[]),
            )
            .await
        {
            tracing::warn!(%error, "failed to audit a ready export");
        }
        Ok(1)
    }

    async fn expire_archives(&self, now: DateTime<Utc>) -> Result<(), AppError> {
        for mut export in self
            .exports
            .list_expired_ready(now)
            .await
            .map_err(AppError::from_repository)?
        {
            if let Some(key) = export.file_reference().map(str::to_string) {
                if let Err(error) = self.store.delete(&key).await {
                    tracing::warn!(
                        %error,
                        export_id = %export.id(),
                        "failed to delete an expired export archive"
                    );
                }
            }
            export.expire_if_due(now);
            self.exports
                .update(&export)
                .await
                .map_err(AppError::from_repository)?;
        }
        Ok(())
    }

    async fn audit_failure(&self, export: &ExportRequest, correlation: &str) {
        if let Err(error) = self
            .trail
            .record(
                Some(export.user_id()),
                "identity.export.failed",
                export.id().to_string(),
                AuditOutcome::Failed,
                correlation,
                context(&[]),
            )
            .await
        {
            tracing::warn!(%error, "failed to audit a failed export");
        }
    }
}
