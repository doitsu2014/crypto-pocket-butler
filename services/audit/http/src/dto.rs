//! Request and response DTOs. They live only at the HTTP boundary and map to and
//! from application commands here; no DTO type reaches application or domain.

use std::collections::BTreeMap;

use audit_application::append::{AppendCommand, AppendResult};
use audit_application::export::EXPORT_ROW_CAP;
use audit_application::privacy::RequestCommand;
use audit_application::query::QueryCommand;
use audit_application::retention::ReportCommand;
use audit_application::support::{CheckDecision, StartSessionCommand};
use audit_domain::{
    AuditEvent, AuditExport, ExportFilter, PrivacyDeletion, RetentionRun, Sensitivity,
    SupportScope, SupportSession, UserId,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// `POST /v1/events` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppendEventRequest {
    /// The acting user or support agent.
    #[serde(default)]
    pub actor_id: Option<String>,
    /// `user`, `system`, or `support_agent`.
    pub actor_kind: String,
    /// The dotted action name.
    pub action: String,
    /// The logical target.
    pub target: String,
    /// When the action happened.
    #[serde(default)]
    pub occurred_at: Option<DateTime<Utc>>,
    /// Correlation and identifier metadata.
    #[serde(default)]
    pub context: BTreeMap<String, String>,
    /// `success`, `denied`, or `failed`.
    pub outcome: String,
    /// `routine` or `security`.
    pub sensitivity: String,
    /// A caller-supplied class, checked and never trusted.
    #[serde(default)]
    pub event_class: Option<String>,
    /// The legal basis the record is kept under.
    #[serde(default)]
    pub legal_basis: Option<String>,
    /// The deletion request the record outlives.
    #[serde(default)]
    pub deletion_request_id: Option<String>,
}

impl AppendEventRequest {
    /// Maps the body plus the `Idempotency-Key` header to an application command.
    #[must_use]
    pub fn into_command(self, idempotency_key: Option<String>) -> AppendCommand {
        AppendCommand {
            actor_id: self.actor_id,
            actor_kind: self.actor_kind,
            action: self.action,
            target: self.target,
            occurred_at: self.occurred_at,
            context: self.context,
            outcome: self.outcome,
            sensitivity: self.sensitivity,
            event_class: self.event_class,
            legal_basis: self.legal_basis,
            deletion_request_id: self.deletion_request_id,
            idempotency_key,
        }
    }
}

/// `POST /v1/events` response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppendEventResponse {
    /// The event identifier.
    pub id: String,
    /// When the audit service stored it.
    pub recorded_at: DateTime<Utc>,
    /// Whether this call inserted the event.
    pub created: bool,
}

impl From<AppendResult> for AppendEventResponse {
    fn from(result: AppendResult) -> Self {
        Self {
            id: result.id.to_string(),
            recorded_at: result.recorded_at,
            created: result.created,
        }
    }
}

/// One audit event.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventDto {
    /// The event identifier.
    pub id: String,
    /// The acting user, when there is one.
    pub actor_id: Option<String>,
    /// Who the actor is.
    pub actor_kind: String,
    /// The dotted action name.
    pub action: String,
    /// The logical target.
    pub target: String,
    /// When the action happened.
    pub occurred_at: DateTime<Utc>,
    /// When the audit service stored it.
    pub recorded_at: DateTime<Utc>,
    /// Correlation and identifier metadata.
    pub context: BTreeMap<String, String>,
    /// How the action turned out.
    pub outcome: String,
    /// How important the event is.
    pub sensitivity: String,
    /// The derived failure class.
    pub event_class: String,
    /// The legal basis the record is kept under.
    pub legal_basis: Option<String>,
    /// The deletion request the record outlives.
    pub deletion_request_id: Option<String>,
}

impl From<&AuditEvent> for EventDto {
    fn from(event: &AuditEvent) -> Self {
        Self {
            id: event.id().to_string(),
            actor_id: event.actor_id().map(|id| id.to_string()),
            actor_kind: event.actor_kind().as_str().to_string(),
            action: event.action().to_string(),
            target: event.target().to_string(),
            occurred_at: event.occurred_at(),
            recorded_at: event.recorded_at(),
            context: event.context().as_map().clone(),
            outcome: event.outcome().as_str().to_string(),
            sensitivity: event.sensitivity().as_str().to_string(),
            event_class: event.event_class().as_str().to_string(),
            legal_basis: event.legal_basis().map(ToString::to_string),
            deletion_request_id: event.deletion_request_id().map(|id| id.to_string()),
        }
    }
}

/// `GET /v1/events` query string.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventQueryParams {
    /// A dotted action prefix.
    #[serde(default)]
    pub action_prefix: Option<String>,
    /// An exact target.
    #[serde(default)]
    pub target: Option<String>,
    /// `routine` or `security`.
    #[serde(default)]
    pub sensitivity: Option<String>,
    /// The lower bound on `occurredAt`.
    #[serde(default)]
    pub occurred_from: Option<DateTime<Utc>>,
    /// The upper bound on `occurredAt`.
    #[serde(default)]
    pub occurred_to: Option<DateTime<Utc>>,
    /// The mandatory lower bound on `recordedAt`.
    #[serde(default)]
    pub recorded_from: Option<DateTime<Utc>>,
    /// The mandatory upper bound on `recordedAt`.
    #[serde(default)]
    pub recorded_to: Option<DateTime<Utc>>,
    /// The opaque continuation token.
    #[serde(default)]
    pub cursor: Option<String>,
    /// The page size.
    #[serde(default)]
    pub limit: Option<u32>,
}

impl From<EventQueryParams> for QueryCommand {
    fn from(params: EventQueryParams) -> Self {
        Self {
            action_prefix: params.action_prefix,
            target: params.target,
            sensitivity: params.sensitivity,
            occurred_from: params.occurred_from,
            occurred_to: params.occurred_to,
            recorded_from: params.recorded_from,
            recorded_to: params.recorded_to,
            cursor: params.cursor,
            limit: params.limit,
        }
    }
}

/// `GET /v1/events` response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventPageResponse {
    /// The events, newest first.
    pub events: Vec<EventDto>,
    /// The token for the next page.
    pub next_cursor: Option<String>,
}

/// `POST /v1/exports` body: the filter and the mandatory `recordedAt` range.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRequest {
    /// Dotted action prefixes.
    #[serde(default)]
    pub action_prefixes: Vec<String>,
    /// An exact target.
    #[serde(default)]
    pub target: Option<String>,
    /// `routine` or `security`.
    #[serde(default)]
    pub sensitivity: Option<String>,
    /// The mandatory lower bound on `recordedAt`.
    pub recorded_from: DateTime<Utc>,
    /// The mandatory upper bound on `recordedAt`.
    pub recorded_to: DateTime<Utc>,
    /// The optional lower bound on `occurredAt`.
    #[serde(default)]
    pub occurred_from: Option<DateTime<Utc>>,
    /// The optional upper bound on `occurredAt`.
    #[serde(default)]
    pub occurred_to: Option<DateTime<Utc>>,
}

impl ExportRequest {
    /// Validates the body into an export filter.
    ///
    /// # Errors
    /// Returns the domain refusal, which the handler maps to a 422 naming the
    /// field path.
    pub fn into_filter(self) -> Result<ExportFilter, audit_domain::DomainError> {
        let sensitivity = match self.sensitivity.as_deref() {
            None => None,
            Some(raw) => Some(Sensitivity::parse(raw)?),
        };
        ExportFilter::new(
            self.action_prefixes,
            self.target,
            sensitivity,
            self.recorded_from,
            self.recorded_to,
            self.occurred_from,
            self.occurred_to,
        )
    }
}

/// The export cap, echoed in the refusal detail.
pub const EXPORT_CAP: u64 = EXPORT_ROW_CAP;

/// `POST /v1/support-sessions` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartSessionRequest {
    /// The agent; when supplied it must equal the authenticated subject.
    #[serde(default)]
    pub support_agent_id: Option<String>,
    /// The user whose data is in scope.
    pub subject_user_id: String,
    /// Why the access is needed.
    pub reason: String,
    /// The independent approver.
    pub second_approver_id: String,
    /// `account_metadata` or `tickets`.
    pub scope: String,
    /// The requested expiry.
    pub expires_at: DateTime<Utc>,
}

impl From<StartSessionRequest> for StartSessionCommand {
    fn from(request: StartSessionRequest) -> Self {
        Self {
            support_agent_id: request.support_agent_id,
            subject_user_id: request.subject_user_id,
            reason: request.reason,
            second_approver_id: request.second_approver_id,
            scope: request.scope,
            expires_at: request.expires_at,
        }
    }
}

/// `POST /v1/support-sessions` response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportSessionResponse {
    /// The session identifier.
    pub id: String,
    /// The stored state.
    pub state: String,
    /// What the session may read.
    pub scope: String,
    /// The user whose data is in scope.
    pub subject_user_id: String,
    /// The instant the session stops granting access.
    pub expires_at: DateTime<Utc>,
    /// When the second approver agreed.
    pub approved_at: Option<DateTime<Utc>>,
}

impl From<&SupportSession> for SupportSessionResponse {
    fn from(session: &SupportSession) -> Self {
        Self {
            id: session.id().to_string(),
            state: session.stored_state().as_str().to_string(),
            scope: session.scope().as_str().to_string(),
            subject_user_id: session.subject_user_id().to_string(),
            expires_at: session.expires_at(),
            approved_at: session.approved_at(),
        }
    }
}

/// `GET /v1/support-sessions/{id}/check` query string.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckParams {
    /// The scope the caller wants to read.
    pub scope: String,
}

impl CheckParams {
    /// Parses the requested scope.
    ///
    /// # Errors
    /// Returns the domain refusal for an unknown scope.
    pub fn into_scope(self) -> Result<SupportScope, audit_domain::DomainError> {
        SupportScope::parse(&self.scope)
    }
}

/// `GET /v1/support-sessions/{id}/check` response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckResponse {
    /// `allow` or `deny`.
    pub decision: String,
    /// A non-sensitive reason code on a deny.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl From<CheckDecision> for CheckResponse {
    fn from(decision: CheckDecision) -> Self {
        match decision {
            CheckDecision::Allow => Self {
                decision: "allow".to_string(),
                reason: None,
            },
            CheckDecision::Deny { reason } => Self {
                decision: "deny".to_string(),
                reason: Some(reason.to_string()),
            },
        }
    }
}

/// `POST /v1/privacy/deletions` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyDeletionRequest {
    /// Identity's deletion request id.
    pub deletion_request_id: String,
    /// The user being deleted.
    pub subject_user_id: String,
}

impl From<PrivacyDeletionRequest> for RequestCommand {
    fn from(request: PrivacyDeletionRequest) -> Self {
        Self {
            deletion_request_id: request.deletion_request_id,
            subject_user_id: request.subject_user_id,
        }
    }
}

/// `POST /v1/privacy/deletions` response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyDeletionResponse {
    /// The deletion-request identifier.
    pub deletion_request_id: String,
    /// Where the request is in its lifecycle.
    pub state: String,
    /// Records removed.
    pub deleted_count: i64,
    /// Records kept under a stated legal basis.
    pub kept_count: i64,
    /// Kept records carrying no legal basis — a defect.
    pub defects: i64,
}

impl From<&PrivacyDeletion> for PrivacyDeletionResponse {
    fn from(deletion: &PrivacyDeletion) -> Self {
        Self {
            deletion_request_id: deletion.deletion_request_id().to_string(),
            state: deletion.state().as_str().to_string(),
            deleted_count: deletion.deleted_count(),
            kept_count: deletion.kept_with_legal_basis(),
            defects: deletion.defects(),
        }
    }
}

/// `POST /v1/runs` body: another service's completed retention run (C11).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionReportRequest {
    /// Must be `reported`.
    pub category: String,
    /// The service that ran the sweep.
    pub owner: String,
    /// The cutoff it used.
    pub cutoff_at: DateTime<Utc>,
    /// When its sweep started.
    pub started_at: DateTime<Utc>,
    /// When its sweep finished.
    pub finished_at: DateTime<Utc>,
    /// How many records it deleted.
    pub deleted_count: i64,
    /// The audited role it ran under.
    pub privileged_role: String,
    /// `success` or `failed`.
    pub outcome: String,
    /// Why it failed, when it did.
    #[serde(default)]
    pub failure_reason: Option<String>,
}

impl From<RetentionReportRequest> for ReportCommand {
    fn from(request: RetentionReportRequest) -> Self {
        Self {
            category: request.category,
            owner: request.owner,
            cutoff_at: request.cutoff_at,
            started_at: request.started_at,
            finished_at: request.finished_at,
            deleted_count: request.deleted_count,
            privileged_role: request.privileged_role,
            outcome: request.outcome,
            failure_reason: request.failure_reason,
        }
    }
}

/// `POST /v1/runs` response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionRunResponse {
    /// The run identifier.
    pub id: String,
    /// `audit_logs` or `reported`.
    pub category: String,
    /// The service that executed the sweep.
    pub owner: String,
    /// Where the run is in its lifecycle.
    pub state: String,
    /// How many records it deleted.
    pub deleted_count: i64,
    /// The audited role the sweep ran under.
    pub privileged_role: String,
    /// The terminal outcome.
    pub outcome: Option<String>,
}

impl From<&RetentionRun> for RetentionRunResponse {
    fn from(run: &RetentionRun) -> Self {
        Self {
            id: run.id().to_string(),
            category: run.category().as_str().to_string(),
            owner: run.owner().as_str().to_string(),
            state: run.state().as_str().to_string(),
            deleted_count: run.deleted_count(),
            privileged_role: run.privileged_role().to_string(),
            outcome: run.outcome().map(|outcome| outcome.as_str().to_string()),
        }
    }
}

/// The export record, echoed in the `x-export-id` response header.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportHeader {
    /// The export identifier.
    pub id: String,
}

impl From<&AuditExport> for ExportHeader {
    fn from(export: &AuditExport) -> Self {
        Self {
            id: export.id().to_string(),
        }
    }
}

/// A subject identifier, echoed on the deletion response.
#[must_use]
pub fn subject_of(id: UserId) -> String {
    id.to_string()
}
