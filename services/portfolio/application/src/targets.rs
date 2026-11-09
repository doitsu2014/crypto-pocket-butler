//! WF3: targets and drift (BR3.1–BR3.4). A save persists the targets and their
//! guardrails; a preview returns the sum and difference without saving.

use std::collections::BTreeSet;
use std::sync::Arc;

use portfolio_domain::{
    preview_targets, require_target_for_guardrail, validate_target, AuditOutcome, Guardrail,
    GuardrailId, Portfolio, PortfolioId, TargetAllocation, TargetAllocationId, TargetPreview,
};
use rust_decimal::Decimal;

use crate::common::{AppError, Caller, FieldError};
use crate::ports::{Clock, IdGenerator, PortfolioRepository, TargetRepository};
use crate::trail::Trail;

/// One requested target, with an optional guardrail for the same asset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetInput {
    /// The asset.
    pub asset_id: String,
    /// The target percentage.
    pub target_percent: Decimal,
    /// The guardrail threshold in percentage points, when one is requested.
    pub guardrail_points: Option<Decimal>,
}

/// The saved target set and its preview.
#[derive(Debug, Clone)]
pub struct TargetSave {
    /// The preview the user saw.
    pub preview: TargetPreview,
    /// The saved targets.
    pub targets: Vec<TargetAllocation>,
    /// The saved guardrails.
    pub guardrails: Vec<Guardrail>,
}

/// Validates, previews, and saves targets and guardrails.
pub struct TargetService {
    portfolios: Arc<dyn PortfolioRepository>,
    targets: Arc<dyn TargetRepository>,
    trail: Arc<Trail>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
}

impl TargetService {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        portfolios: Arc<dyn PortfolioRepository>,
        targets: Arc<dyn TargetRepository>,
        trail: Arc<Trail>,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            portfolios,
            targets,
            trail,
            clock,
            ids,
        }
    }

    /// Previews a target set: the sum and its difference from 100 (BR3.2).
    ///
    /// # Errors
    /// Returns a validation refusal for a bad asset, percentage, or duplicate
    /// asset.
    pub fn preview(
        &self,
        portfolio_id: PortfolioId,
        inputs: &[TargetInput],
    ) -> Result<TargetPreview, AppError> {
        let targets = self.validate(portfolio_id, inputs)?;
        Ok(preview_targets(&targets))
    }

    /// Saves the targets and guardrails and returns what the user saw.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for another user's portfolio, a
    /// validation refusal for a bad set, and a repository refusal when the write
    /// fails.
    pub async fn save(
        &self,
        caller: &Caller,
        portfolio_id: PortfolioId,
        inputs: &[TargetInput],
    ) -> Result<TargetSave, AppError> {
        let portfolio = self.owned(caller, portfolio_id).await?;
        let targets = self.validate(portfolio.id(), inputs)?;
        let now = self.clock.now();
        let target_assets: Vec<String> = targets
            .iter()
            .map(|target| target.asset_id().to_string())
            .collect();
        let existing = self
            .targets
            .list_guardrails(portfolio.id())
            .await
            .map_err(AppError::from_repository)?;
        let mut guardrails = Vec::new();
        for input in inputs {
            let Some(threshold) = input.guardrail_points else {
                continue;
            };
            require_target_for_guardrail(input.asset_id.trim(), &target_assets)
                .map_err(AppError::from_domain)?;
            let mut guardrail = Guardrail::new(
                GuardrailId::new(self.ids.next()),
                portfolio.id(),
                &input.asset_id,
                threshold,
                now,
            )
            .map_err(AppError::from_domain)?;
            // A save preserves the crossing state of an unchanged guardrail so a
            // target edit does not reset the one-alert-per-crossing dedupe.
            if let Some(previous) = existing
                .iter()
                .find(|candidate| candidate.asset_id() == input.asset_id.trim())
            {
                if previous.threshold_points() == threshold {
                    guardrail = Guardrail::from_stored(
                        previous.id(),
                        previous.portfolio_id(),
                        previous.asset_id().to_string(),
                        previous.threshold_points(),
                        previous.is_outside(),
                        previous.last_crossing_key().map(ToString::to_string),
                        previous.crossed_at(),
                        now,
                    );
                }
            }
            guardrails.push(guardrail);
        }
        let preview = preview_targets(&targets);
        self.targets
            .replace_for_portfolio(portfolio.id(), &targets, &guardrails)
            .await
            .map_err(AppError::from_repository)?;
        self.trail
            .record_action(
                Some(caller.user_id),
                "portfolio.targets.changed",
                portfolio.id().to_string(),
                AuditOutcome::Success,
                &caller.correlation_id,
                "BR3.2",
            )
            .await?;
        Ok(TargetSave {
            preview,
            targets,
            guardrails,
        })
    }

    fn validate(
        &self,
        portfolio_id: PortfolioId,
        inputs: &[TargetInput],
    ) -> Result<Vec<TargetAllocation>, AppError> {
        let now = self.clock.now();
        let mut seen = BTreeSet::new();
        let mut targets = Vec::with_capacity(inputs.len());
        let mut errors = Vec::new();
        for input in inputs {
            let asset = input.asset_id.trim().to_string();
            if !seen.insert(asset.clone()) {
                errors.push(FieldError::new(
                    "assetId",
                    format!("{asset} appears more than once"),
                ));
                continue;
            }
            match validate_target(&asset, input.target_percent) {
                Ok(()) => match TargetAllocation::new(
                    TargetAllocationId::new(self.ids.next()),
                    portfolio_id,
                    &asset,
                    input.target_percent,
                    now,
                ) {
                    Ok(target) => targets.push(target),
                    Err(error) => errors.push(FieldError::new(error.field(), error.to_string())),
                },
                Err(error) => errors.push(FieldError::new(error.field(), error.to_string())),
            }
        }
        if errors.is_empty() {
            Ok(targets)
        } else {
            Err(AppError::Validation(errors))
        }
    }

    async fn owned(
        &self,
        caller: &Caller,
        portfolio_id: PortfolioId,
    ) -> Result<Portfolio, AppError> {
        let portfolio = self
            .portfolios
            .find_by_id(portfolio_id)
            .await
            .map_err(AppError::from_repository)?
            .ok_or(AppError::NotFound("portfolio"))?;
        caller.require_owner(portfolio.user_id())?;
        Ok(portfolio)
    }
}
