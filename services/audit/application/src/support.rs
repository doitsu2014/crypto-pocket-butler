//! WF2 and WF3: start a support session and enforce it (BR3.1–BR3.3, BR5.1).

use std::sync::Arc;

use audit_domain::{
    ActorKind, Context, Outcome, Sensitivity, SupportDenial, SupportScope, SupportSession,
    SupportSessionId, SupportSessionRequest, UserId,
};
use chrono::{DateTime, Utc};

use crate::common::FieldError;
use crate::ports::{Clock, Principal, Role, SupportSessionRepository};
use crate::trail::Trail;

/// The raw request a support agent's console sends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartSessionCommand {
    /// The agent; when supplied it must equal the authenticated subject.
    pub support_agent_id: Option<String>,
    /// The user whose data is in scope.
    pub subject_user_id: String,
    /// Why the access is needed.
    pub reason: String,
    /// A different operator who approved the session.
    pub second_approver_id: String,
    /// `account_metadata` or `tickets`.
    pub scope: String,
    /// The requested expiry.
    pub expires_at: DateTime<Utc>,
}

/// The session-check decision. A check that cannot be answered is a deny.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckDecision {
    /// The read may proceed.
    Allow,
    /// The read must be refused.
    Deny {
        /// A stable, non-sensitive reason code.
        reason: &'static str,
    },
}

impl CheckDecision {
    /// The wire value.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny { .. } => "deny",
        }
    }
}

/// Why a support-session operation was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SupportError {
    /// The caller lacks the support (or admin) role.
    #[error("support role required")]
    Forbidden,
    /// The request failed validation.
    #[error("the request is invalid")]
    Rejected(Vec<FieldError>),
    /// BR3.1 refused the session; a `denied` record was stored.
    #[error("the session was denied: {denial}")]
    Denied {
        /// The rule that refused it.
        denial: SupportDenial,
        /// The id of the stored `denied` record.
        session_id: SupportSessionId,
    },
    /// The session does not exist.
    #[error("support session not found")]
    NotFound,
    /// The session cannot make the requested transition.
    #[error("the session cannot be changed: {0}")]
    Conflict(String),
    /// Storage failed.
    #[error("the session could not be stored: {0}")]
    Repository(String),
}

/// Owns the support-session lifecycle and the per-read record.
pub struct SupportSessions {
    sessions: Arc<dyn SupportSessionRepository>,
    clock: Arc<dyn Clock>,
    trail: Trail,
}

impl SupportSessions {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        sessions: Arc<dyn SupportSessionRepository>,
        clock: Arc<dyn Clock>,
        trail: Trail,
    ) -> Self {
        Self {
            sessions,
            clock,
            trail,
        }
    }

    /// Starts a session, or stores a `denied` record and refuses it.
    ///
    /// # Errors
    /// Returns [`SupportError::Forbidden`] for a non-support caller,
    /// [`SupportError::Rejected`] for a malformed request, and
    /// [`SupportError::Denied`] when BR3.1 refuses the session after storing the
    /// denial.
    pub async fn request(
        &self,
        actor: &Principal,
        command: StartSessionCommand,
    ) -> Result<SupportSession, SupportError> {
        if actor.role != Role::Support {
            let _ = self
                .trail
                .record_denial(
                    UserId::parse(&actor.subject).ok(),
                    ActorKind::User,
                    "support.session.denied",
                    "POST /v1/support-sessions",
                    Context::empty(),
                )
                .await;
            return Err(SupportError::Forbidden);
        }

        let mut errors = Vec::new();
        let agent = match UserId::parse(&actor.subject) {
            Ok(value) => Some(value),
            Err(error) => {
                errors.push(FieldError::from(error));
                None
            }
        };
        if let Some(supplied) = command.support_agent_id.as_deref() {
            if supplied.trim() != actor.subject {
                errors.push(FieldError::new(
                    "supportAgentId",
                    "must equal the authenticated subject",
                ));
            }
        }
        let subject = match UserId::parse(&command.subject_user_id) {
            Ok(value) => Some(value),
            Err(error) => {
                errors.push(FieldError::from(error));
                None
            }
        };
        let approver = match UserId::parse(&command.second_approver_id) {
            Ok(value) => Some(value),
            Err(error) => {
                errors.push(FieldError::from(error));
                None
            }
        };
        let scope = match SupportScope::parse(&command.scope) {
            Ok(value) => Some(value),
            Err(error) => {
                errors.push(FieldError::from(error));
                None
            }
        };
        if !errors.is_empty() {
            return Err(SupportError::Rejected(errors));
        }

        let now = self.clock.now();
        let request = SupportSessionRequest::new(
            agent.expect("validated agent"),
            subject.expect("validated subject"),
            &command.reason,
            approver.expect("validated approver"),
            scope.expect("validated scope"),
            now,
            command.expires_at,
        )
        .map_err(|error| SupportError::Rejected(vec![FieldError::from(error)]))?;

        let id = SupportSessionId::generate();
        match SupportSession::approve(request, id, now) {
            Ok(session) => {
                self.sessions
                    .insert(&session)
                    .await
                    .map_err(|error| SupportError::Repository(error.to_string()))?;
                let _ = self
                    .trail
                    .record_agent(
                        session.support_agent_id(),
                        "support.session.started",
                        &session.id().to_string(),
                        Sensitivity::Security,
                        session_context(session.id()),
                    )
                    .await;
                Ok(session)
            }
            Err((request, denial)) => {
                let session = SupportSession::deny(request, id);
                self.sessions
                    .insert(&session)
                    .await
                    .map_err(|error| SupportError::Repository(error.to_string()))?;
                let _ = self
                    .trail
                    .record_as(
                        Some(session.support_agent_id()),
                        ActorKind::SupportAgent,
                        "support.session.denied",
                        &session.id().to_string(),
                        Outcome::Denied,
                        Sensitivity::Security,
                        denial_context(session.id(), denial),
                    )
                    .await;
                Err(SupportError::Denied {
                    denial,
                    session_id: session.id(),
                })
            }
        }
    }

    /// Answers a session check and writes the read event before the allow.
    ///
    /// A check that cannot be answered is a deny, never an allow.
    pub async fn check(
        &self,
        session_id: SupportSessionId,
        scope: SupportScope,
        source_service: &str,
    ) -> CheckDecision {
        let now = self.clock.now();
        let session = match self.sessions.find_by_id(session_id).await {
            Ok(Some(session)) => session,
            Ok(None) => {
                self.record_check_denial(None, session_id, source_service, "unknown-session")
                    .await;
                return CheckDecision::Deny {
                    reason: "unknown-session",
                };
            }
            Err(error) => {
                tracing::error!(%error, %session_id, "support-session check could not be read");
                return CheckDecision::Deny {
                    reason: "check-unavailable",
                };
            }
        };

        if !session.is_active_at(now) {
            self.record_check_denial(
                Some(&session),
                session_id,
                source_service,
                "session-inactive",
            )
            .await;
            return CheckDecision::Deny {
                reason: "session-inactive",
            };
        }
        if !session.covers(scope) {
            self.record_check_denial(Some(&session), session_id, source_service, "out-of-scope")
                .await;
            return CheckDecision::Deny {
                reason: "out-of-scope",
            };
        }

        let action = match scope {
            SupportScope::AccountMetadata => "support.account_metadata.read",
            SupportScope::Tickets => "support.tickets.read",
        };
        let mut context = Context::empty();
        let _ = context.insert("sessionId", &session.id().to_string());
        let _ = context.insert("sourceService", source_service);
        match self
            .trail
            .record_as(
                Some(session.support_agent_id()),
                ActorKind::SupportAgent,
                action,
                &session.subject_user_id().to_string(),
                Outcome::Success,
                Sensitivity::Security,
                context,
            )
            .await
        {
            Ok(_) => CheckDecision::Allow,
            // BR3.3: a read that is not recorded does not happen.
            Err(_) => CheckDecision::Deny {
                reason: "not-recorded",
            },
        }
    }

    /// Revokes an active session.
    ///
    /// # Errors
    /// Returns [`SupportError::Forbidden`] for a non-admin caller,
    /// [`SupportError::NotFound`], [`SupportError::Conflict`] when the session is
    /// no longer active, and [`SupportError::Repository`] on a storage failure.
    pub async fn revoke(
        &self,
        actor: &Principal,
        session_id: SupportSessionId,
    ) -> Result<SupportSession, SupportError> {
        if actor.role != Role::Admin {
            let _ = self
                .trail
                .record_denial(
                    UserId::parse(&actor.subject).ok(),
                    ActorKind::User,
                    "support.session.revoke_denied",
                    &session_id.to_string(),
                    Context::empty(),
                )
                .await;
            return Err(SupportError::Forbidden);
        }
        let now = self.clock.now();
        let session = self
            .sessions
            .find_by_id(session_id)
            .await
            .map_err(|error| SupportError::Repository(error.to_string()))?
            .ok_or(SupportError::NotFound)?;
        let revoked = session
            .revoke(now)
            .map_err(|error| SupportError::Conflict(error.to_string()))?;
        self.sessions
            .update(&revoked)
            .await
            .map_err(|error| SupportError::Repository(error.to_string()))?;
        let admin = UserId::parse(&actor.subject)
            .map_err(|error| SupportError::Rejected(vec![FieldError::from(error)]))?;
        let _ = self
            .trail
            .record_user(
                admin,
                "support.session.revoked",
                &revoked.id().to_string(),
                Outcome::Success,
                Sensitivity::Security,
                session_context(revoked.id()),
            )
            .await;
        Ok(revoked)
    }

    async fn record_check_denial(
        &self,
        session: Option<&SupportSession>,
        session_id: SupportSessionId,
        source_service: &str,
        reason: &'static str,
    ) {
        let mut context = Context::empty();
        let _ = context.insert("sessionId", &session_id.to_string());
        let _ = context.insert("sourceService", source_service);
        let _ = context.insert("ruleId", reason);
        let actor = session.map(SupportSession::support_agent_id);
        let target = session
            .map(|value| value.subject_user_id().to_string())
            .unwrap_or_else(|| session_id.to_string());
        let _ = self
            .trail
            .record_denial(
                actor,
                ActorKind::SupportAgent,
                "support.read.denied",
                &target,
                context,
            )
            .await;
    }
}

fn session_context(session_id: SupportSessionId) -> Context {
    let mut context = Context::empty();
    let _ = context.insert("sessionId", &session_id.to_string());
    context
}

fn denial_context(session_id: SupportSessionId, denial: SupportDenial) -> Context {
    let mut context = session_context(session_id);
    let _ = context.insert("ruleId", denial.code());
    context
}
