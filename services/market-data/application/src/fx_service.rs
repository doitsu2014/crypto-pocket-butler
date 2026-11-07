//! WF2: serve one FX rate (C6).
//!
//! A direct pair is served only while it is fresh; a missing or stale direct
//! pair may be derived through USD from two fresh legs. An unavailable
//! conversion is surfaced as a problem, never defaulted to a ratio of one
//! (BR3.1–BR3.2).

use std::sync::Arc;

use market_data_domain::{normalise_currency, FxRate, ServedFxRate, FX_BOUND_SECONDS};

use crate::common::{AppError, FieldError, ServiceCaller};
use crate::ports::{Clock, FxRepository, ServiceMetrics};
use market_data_domain::Operation;

/// Serves C6 FX rates.
pub struct FxService {
    fx: Arc<dyn FxRepository>,
    metrics: Arc<dyn ServiceMetrics>,
    clock: Arc<dyn Clock>,
}

impl FxService {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        fx: Arc<dyn FxRepository>,
        metrics: Arc<dyn ServiceMetrics>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { fx, metrics, clock }
    }

    /// Serves `base → quote`.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for a caller without the FX operation,
    /// [`AppError::Validation`] for a malformed or identical pair, and
    /// [`AppError::NotFound`] when neither a fresh direct pair nor two fresh
    /// USD legs exist.
    pub async fn get_rate(
        &self,
        caller: &ServiceCaller,
        base: &str,
        quote: &str,
    ) -> Result<ServedFxRate, AppError> {
        caller.require_operation(Operation::FxRead)?;
        let base = normalise_currency(base).map_err(AppError::from_domain)?;
        let quote = normalise_currency(quote).map_err(AppError::from_domain)?;
        if base == quote {
            return Err(AppError::Validation(vec![FieldError::new(
                "base",
                "must differ from the quote currency",
            )]));
        }
        let now = self.clock.now();

        if let Some(direct) = self.fresh_leg(&base, &quote, now).await? {
            let age = direct.age_seconds(now);
            self.metrics.record_fx(&format!("{base}/{quote}"), age);
            return Ok(ServedFxRate::from_direct(&direct));
        }

        // A cross rate is resolved through USD. Either leg may be stored in the
        // provider's own direction and inverted for the requested direction.
        let Some(first) = self.fresh_leg(&base, "USD", now).await? else {
            return Err(AppError::NotFound("fx rate"));
        };
        let Some(second) = self.fresh_leg("USD", &quote, now).await? else {
            return Err(AppError::NotFound("fx rate"));
        };
        let served =
            ServedFxRate::derived(&base, &quote, &first, &second).map_err(AppError::from_domain)?;
        let age = (now - served.observed_at()).num_seconds();
        self.metrics.record_fx(&format!("{base}/{quote}"), age);
        Ok(served)
    }

    /// Resolves `from → to` using the exact stored pair when present, otherwise
    /// the reverse stored pair inverted. The reading must be within the one-hour
    /// FX bound.
    async fn fresh_leg(
        &self,
        from: &str,
        to: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Option<FxRate>, AppError> {
        if let Some(rate) = self.fresh_exact(from, to, now).await? {
            return Ok(Some(rate));
        }
        match self.fresh_exact(to, from, now).await? {
            Some(reverse) => reverse.inverted().map(Some).map_err(AppError::from_domain),
            None => Ok(None),
        }
    }

    async fn fresh_exact(
        &self,
        base: &str,
        quote: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Option<FxRate>, AppError> {
        let Some(rate) = self
            .fx
            .latest(base, quote)
            .await
            .map_err(AppError::from_repository)?
        else {
            return Ok(None);
        };
        if rate.age_seconds(now) <= FX_BOUND_SECONDS {
            Ok(Some(rate))
        } else {
            Ok(None)
        }
    }
}
