//! Audit exports: the filter, the record, and the 100 000-row cap (BR5.2, WF5).

use chrono::{DateTime, Utc};

use crate::error::DomainError;
use crate::event::{Sensitivity, MAX_ACTION_LEN, MAX_TARGET_LEN};
use crate::ids::{AuditExportId, UserId};
use crate::secrets::ensure_secret_free;

/// The hard cap on one export. Over the cap the request is refused, never
/// truncated silently.
pub const MAX_EXPORT_ROWS: i64 = 100_000;

/// The maximum number of action prefixes in one filter.
pub const MAX_ACTION_PREFIXES: usize = 10;

/// The longest content digest stored on an export.
pub const MAX_SHA256_LEN: usize = 64;

/// What an export selects. There is no free-text query: action prefixes, a
/// target, and time ranges only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportFilter {
    /// Dotted action prefixes, matched with the text-prefix operator.
    pub action_prefixes: Vec<String>,
    /// An exact target.
    pub target: Option<String>,
    /// Only events at this sensitivity.
    pub sensitivity: Option<Sensitivity>,
    /// The mandatory lower bound on `recordedAt`.
    pub recorded_from: DateTime<Utc>,
    /// The mandatory upper bound on `recordedAt`.
    pub recorded_to: DateTime<Utc>,
    /// An optional lower bound on `occurredAt`.
    pub occurred_from: Option<DateTime<Utc>>,
    /// An optional upper bound on `occurredAt`.
    pub occurred_to: Option<DateTime<Utc>>,
}

impl ExportFilter {
    /// Validates a filter.
    ///
    /// # Errors
    /// Returns [`DomainError`] when the mandatory `recordedAt` range is absent or
    /// inverted, a bound is out of order, a prefix is malformed, or the target is
    /// secret-shaped.
    pub fn new(
        action_prefixes: Vec<String>,
        target: Option<String>,
        sensitivity: Option<Sensitivity>,
        recorded_from: DateTime<Utc>,
        recorded_to: DateTime<Utc>,
        occurred_from: Option<DateTime<Utc>>,
        occurred_to: Option<DateTime<Utc>>,
    ) -> Result<Self, DomainError> {
        if recorded_from >= recorded_to {
            return Err(DomainError::Invalid {
                field: "recordedFrom",
                reason:
                    "the recordedAt range is mandatory and recordedFrom must precede recordedTo"
                        .to_string(),
            });
        }
        if let (Some(from), Some(to)) = (occurred_from, occurred_to) {
            if from >= to {
                return Err(DomainError::Invalid {
                    field: "occurredFrom",
                    reason: "occurredFrom must precede occurredTo".to_string(),
                });
            }
        }
        if action_prefixes.len() > MAX_ACTION_PREFIXES {
            return Err(DomainError::TooLarge {
                field: "actionPrefixes",
                limit: "10 prefixes",
            });
        }
        let action_prefixes = action_prefixes
            .into_iter()
            .map(|prefix| validate_prefix(&prefix))
            .collect::<Result<Vec<_>, _>>()?;
        let target = match target {
            None => None,
            Some(value) => {
                let value = value.trim().to_string();
                if value.is_empty() {
                    return Err(DomainError::Required { field: "target" });
                }
                if value.len() > MAX_TARGET_LEN {
                    return Err(DomainError::TooLarge {
                        field: "target",
                        limit: "240 characters",
                    });
                }
                ensure_secret_free("target", &value)?;
                Some(value)
            }
        };
        Ok(Self {
            action_prefixes,
            target,
            sensitivity,
            recorded_from,
            recorded_to,
            occurred_from,
            occurred_to,
        })
    }
}

fn validate_prefix(raw: &str) -> Result<String, DomainError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(DomainError::Required {
            field: "actionPrefixes",
        });
    }
    if value.len() > MAX_ACTION_LEN {
        return Err(DomainError::TooLarge {
            field: "actionPrefixes",
            limit: "120 characters per prefix",
        });
    }
    if !value.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
    }) {
        return Err(DomainError::Invalid {
            field: "actionPrefixes",
            reason: "prefixes are lowercase action names".to_string(),
        });
    }
    Ok(value.to_string())
}

/// Where an export request is in its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExportState {
    /// Recorded, not yet produced.
    Requested,
    /// Produced and hashed.
    Success,
    /// The caller lacked the admin role.
    Denied,
    /// Production failed.
    Failed,
}

impl ExportState {
    /// Parses the stored value.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for anything else.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "requested" => Ok(Self::Requested),
            "success" => Ok(Self::Success),
            "denied" => Ok(Self::Denied),
            "failed" => Ok(Self::Failed),
            _ => Err(DomainError::Invalid {
                field: "state",
                reason: "unknown export state".to_string(),
            }),
        }
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Requested => "requested",
            Self::Success => "success",
            Self::Denied => "denied",
            Self::Failed => "failed",
        }
    }
}

/// The terminal outcome of an export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExportOutcome {
    /// Produced.
    Success,
    /// Refused.
    Denied,
    /// Production failed.
    Failed,
}

impl ExportOutcome {
    /// Parses the stored value.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for anything else.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "success" => Ok(Self::Success),
            "denied" => Ok(Self::Denied),
            "failed" => Ok(Self::Failed),
            _ => Err(DomainError::Invalid {
                field: "outcome",
                reason: "must be success, denied, or failed".to_string(),
            }),
        }
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Denied => "denied",
            Self::Failed => "failed",
        }
    }

    /// The state the outcome moves the export to.
    #[must_use]
    pub fn state(self) -> ExportState {
        match self {
            Self::Success => ExportState::Success,
            Self::Denied => ExportState::Denied,
            Self::Failed => ExportState::Failed,
        }
    }
}

/// One audited export request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditExport {
    id: AuditExportId,
    requested_by: UserId,
    filter: ExportFilter,
    requested_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    row_count: i64,
    state: ExportState,
    outcome: Option<ExportOutcome>,
    content_sha256: Option<String>,
}

impl AuditExport {
    /// Records a request in the `requested` state.
    #[must_use]
    pub fn requested(
        id: AuditExportId,
        requested_by: UserId,
        filter: ExportFilter,
        at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            requested_by,
            filter,
            requested_at: at,
            completed_at: None,
            row_count: 0,
            state: ExportState::Requested,
            outcome: None,
            content_sha256: None,
        }
    }

    /// Marks a request as produced.
    ///
    /// # Errors
    /// Returns [`DomainError::InvalidTransition`] when the export is not
    /// requested, or [`DomainError::Invalid`] for a bad count or digest.
    pub fn succeed(
        mut self,
        row_count: i64,
        content_sha256: &str,
        at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        self.ensure_requested("success")?;
        if !(0..=MAX_EXPORT_ROWS).contains(&row_count) {
            return Err(DomainError::Invalid {
                field: "rowCount",
                reason: "must be between 0 and 100000".to_string(),
            });
        }
        let digest = content_sha256.trim();
        if digest.len() != MAX_SHA256_LEN || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(DomainError::Invalid {
                field: "contentSha256",
                reason: "must be a 64-character hex digest".to_string(),
            });
        }
        self.state = ExportState::Success;
        self.outcome = Some(ExportOutcome::Success);
        self.row_count = row_count;
        self.content_sha256 = Some(digest.to_ascii_lowercase());
        self.completed_at = Some(at);
        Ok(self)
    }

    /// Marks a request as denied.
    ///
    /// # Errors
    /// Returns [`DomainError::InvalidTransition`] when the export is not requested.
    pub fn deny(mut self, at: DateTime<Utc>) -> Result<Self, DomainError> {
        self.ensure_requested("denied")?;
        self.state = ExportState::Denied;
        self.outcome = Some(ExportOutcome::Denied);
        self.completed_at = Some(at);
        Ok(self)
    }

    /// Marks a request as failed.
    ///
    /// # Errors
    /// Returns [`DomainError::InvalidTransition`] when the export is not requested.
    pub fn fail(mut self, at: DateTime<Utc>) -> Result<Self, DomainError> {
        self.ensure_requested("failed")?;
        self.state = ExportState::Failed;
        self.outcome = Some(ExportOutcome::Failed);
        self.completed_at = Some(at);
        Ok(self)
    }

    fn ensure_requested(&self, to: &'static str) -> Result<(), DomainError> {
        if self.state == ExportState::Requested {
            Ok(())
        } else {
            Err(DomainError::InvalidTransition {
                entity: "audit export",
                from: self.state.as_str(),
                to,
            })
        }
    }

    /// The export identifier.
    #[must_use]
    pub fn id(&self) -> AuditExportId {
        self.id
    }

    /// The admin who asked.
    #[must_use]
    pub fn requested_by(&self) -> UserId {
        self.requested_by
    }

    /// The filter used.
    #[must_use]
    pub fn filter(&self) -> &ExportFilter {
        &self.filter
    }

    /// When the request was recorded.
    #[must_use]
    pub fn requested_at(&self) -> DateTime<Utc> {
        self.requested_at
    }

    /// When the export reached a terminal state.
    #[must_use]
    pub fn completed_at(&self) -> Option<DateTime<Utc>> {
        self.completed_at
    }

    /// How many rows were streamed.
    #[must_use]
    pub fn row_count(&self) -> i64 {
        self.row_count
    }

    /// Where the request is in its lifecycle.
    #[must_use]
    pub fn state(&self) -> ExportState {
        self.state
    }

    /// The terminal outcome.
    #[must_use]
    pub fn outcome(&self) -> Option<ExportOutcome> {
        self.outcome
    }

    /// The digest of the body that was streamed.
    #[must_use]
    pub fn content_sha256(&self) -> Option<&str> {
        self.content_sha256.as_deref()
    }

    /// Rebuilds an export from storage.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: AuditExportId,
        requested_by: UserId,
        filter: ExportFilter,
        requested_at: DateTime<Utc>,
        completed_at: Option<DateTime<Utc>>,
        row_count: i64,
        state: ExportState,
        outcome: Option<ExportOutcome>,
        content_sha256: Option<String>,
    ) -> Self {
        Self {
            id,
            requested_by,
            filter,
            requested_at,
            completed_at,
            row_count,
            state,
            outcome,
            content_sha256,
        }
    }
}
