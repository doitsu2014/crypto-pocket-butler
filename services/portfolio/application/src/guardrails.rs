//! WF7: guardrail evaluation (BR7.1). Portfolio owns the crossing state and
//! notifies once per crossing. A failed C9 notification leaves the state inside
//! so the next run retries it.

use std::sync::Arc;

use portfolio_domain::{GuardrailTransition, Portfolio, Valuation};
use serde_json::json;

use crate::common::AppError;
use crate::ports::{Clock, GuardrailRepository, NotificationClient, TargetRepository};

/// The outcome of one guardrail evaluation pass.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GuardrailReport {
    /// Crossings notified.
    pub crossings: u32,
    /// Crossings whose C9 notification failed and will be retried.
    pub alerts_failed: u32,
}

/// Evaluates guardrails after a valuation.
pub struct GuardrailService {
    targets: Arc<dyn TargetRepository>,
    guardrails: Arc<dyn GuardrailRepository>,
    notifications: Arc<dyn NotificationClient>,
    clock: Arc<dyn Clock>,
}

impl GuardrailService {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        targets: Arc<dyn TargetRepository>,
        guardrails: Arc<dyn GuardrailRepository>,
        notifications: Arc<dyn NotificationClient>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            targets,
            guardrails,
            notifications,
            clock,
        }
    }

    /// Compares each guardrail with the asset's drift and notifies once per
    /// crossing.
    ///
    /// # Errors
    /// Returns a repository refusal when the guardrail read or a state write
    /// fails. A C9 failure is reported in the result and leaves the state
    /// inside.
    pub async fn evaluate(
        &self,
        portfolio: &Portfolio,
        valuation: &Valuation,
    ) -> Result<GuardrailReport, AppError> {
        let guardrails = self
            .targets
            .list_guardrails(portfolio.id())
            .await
            .map_err(AppError::from_repository)?;
        let mut report = GuardrailReport::default();
        for guardrail in guardrails {
            let Some(drift) = drift_for(valuation, guardrail.asset_id()) else {
                continue;
            };
            let (updated, transition) =
                guardrail.evaluate(drift, portfolio.user_id(), self.clock.now());
            match transition {
                GuardrailTransition::Crossed { key } => {
                    let payload = json!({
                        // The drift alert's payload allow-list is
                        // portfolioId, assetId, and driftPoints only.
                        "portfolioId": portfolio.id().to_string(),
                        "assetId": guardrail.asset_id(),
                        "driftPoints": drift.to_string(),
                    });
                    match self
                        .notifications
                        .notify(portfolio.user_id(), "drift", &key, payload)
                        .await
                    {
                        Ok(()) => {
                            self.guardrails
                                .update(&updated)
                                .await
                                .map_err(AppError::from_repository)?;
                            report.crossings += 1;
                        }
                        Err(error) => {
                            tracing::warn!(%error, "the guardrail alert failed; the crossing stays inside for a retry");
                            report.alerts_failed += 1;
                        }
                    }
                }
                GuardrailTransition::Cleared => {
                    self.guardrails
                        .update(&updated)
                        .await
                        .map_err(AppError::from_repository)?;
                }
                GuardrailTransition::Retained { .. } | GuardrailTransition::None => {}
            }
        }
        Ok(report)
    }
}

fn drift_for(valuation: &Valuation, asset_id: &str) -> Option<rust_decimal::Decimal> {
    let line = valuation.line(asset_id)?;
    line.drift_points().or_else(|| {
        // An asset that is no longer held has zero actual allocation; the drift
        // is then the whole target. An unpriced held line has no defined
        // allocation and is skipped instead.
        if line.quantity().is_zero() {
            line.target_percent().map(|target| -target)
        } else {
            None
        }
    })
}
