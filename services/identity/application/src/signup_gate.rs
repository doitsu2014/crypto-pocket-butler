//! WF6 — the pre-sign-up jurisdiction and age gate (BR7.1–BR7.3).

use std::sync::Arc;

use identity_domain::{evaluate, AuditOutcome, GateOutcome, SignupDeclaration};

use crate::common::AppError;
use crate::ports::{Clock, GateTokenIssuer, JurisdictionRepository};
use crate::trail::{context, Trail};

/// The gate's answer to an allowed or disclosure-pending declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateResponse {
    gate_token: Option<String>,
    disclosures_required: bool,
}

impl GateResponse {
    /// The signed, short-lived gate token, or `None` while disclosures are
    /// outstanding.
    #[must_use]
    pub fn gate_token(&self) -> Option<&str> {
        self.gate_token.as_deref()
    }

    /// Whether the launch disclosures must be acknowledged first.
    #[must_use]
    pub fn disclosures_required(&self) -> bool {
        self.disclosures_required
    }
}

/// Applies the jurisdiction rule and issues the signed gate token.
pub struct SignupGateService {
    rules: Arc<dyn JurisdictionRepository>,
    issuer: Arc<dyn GateTokenIssuer>,
    trail: Arc<Trail>,
    clock: Arc<dyn Clock>,
}

impl SignupGateService {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        rules: Arc<dyn JurisdictionRepository>,
        issuer: Arc<dyn GateTokenIssuer>,
        trail: Arc<Trail>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            rules,
            issuer,
            trail,
            clock,
        }
    }

    /// Evaluates a declaration before any personal data is collected.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for a blocked or under-age declaration,
    /// [`AppError::Validation`] for a malformed declaration, and
    /// [`AppError::Unavailable`] when signing is unavailable.
    pub async fn evaluate(
        &self,
        jurisdiction: &str,
        declared_age: u32,
        disclosures_acknowledged: bool,
        correlation_id: &str,
    ) -> Result<GateResponse, AppError> {
        let declaration =
            SignupDeclaration::parse(jurisdiction, declared_age, disclosures_acknowledged)
                .map_err(AppError::from_domain)?;
        let rule = self
            .rules
            .find_by_code(declaration.jurisdiction.as_str())
            .await
            .map_err(AppError::from_repository)?;
        let now = self.clock.now();
        let outcome = evaluate(
            rule.as_ref(),
            declaration.declared_age,
            declaration.disclosures_acknowledged,
        );

        let (action, audit_outcome) = match &outcome {
            GateOutcome::Allowed => ("identity.signup_gate.allowed", AuditOutcome::Success),
            GateOutcome::DisclosuresRequired => (
                "identity.signup_gate.disclosures_required",
                AuditOutcome::Success,
            ),
            GateOutcome::Denied { .. } => ("identity.signup_gate.denied", AuditOutcome::Denied),
        };
        // The gate event is not on the refusal-critical path; a failed audit is
        // logged rather than blocking a visitor who collected no personal data.
        if let Err(error) = self
            .trail
            .record(
                None,
                action,
                declaration.jurisdiction.as_str().to_string(),
                audit_outcome,
                correlation_id,
                context(&[]),
            )
            .await
        {
            tracing::warn!(%error, "failed to audit the sign-up gate decision");
        }

        match outcome {
            GateOutcome::Denied { reason } => Err(AppError::Forbidden(reason)),
            GateOutcome::DisclosuresRequired => Ok(GateResponse {
                gate_token: None,
                disclosures_required: true,
            }),
            GateOutcome::Allowed => {
                let token = self
                    .issuer
                    .issue(declaration.jurisdiction.as_str(), now)
                    .map_err(AppError::Unavailable)?;
                Ok(GateResponse {
                    gate_token: Some(token),
                    disclosures_required: false,
                })
            }
        }
    }
}
