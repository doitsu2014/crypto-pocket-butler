//! WF1: serve prices (C6).
//!
//! The source list is read from C4 once per request, the newest accepted
//! observation per candidate source is read from Postgres, and selection is by
//! source priority and age. No provider is called here (NFR2.1). When C4 is
//! unavailable the service degrades to each asset's newest stored observation
//! as `stale` (NFR1.11).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use market_data_domain::{
    classify_freshness, validate_asset_id, Freshness, PriceObservation, PriceQuote, PriceTier,
    MAX_ASSET_IDS, TAIL_BOUND_SECONDS, TOP100_BOUND_SECONDS,
};

use crate::common::{AppError, FieldError, ServiceCaller};
use crate::ports::{
    AssetRegistry, Clock, ObservationRepository, PriceSource, ServiceMetrics, TierRepository,
};
use market_data_domain::Operation;

/// Serves C6 prices.
pub struct QuoteService {
    registry: Arc<dyn AssetRegistry>,
    observations: Arc<dyn ObservationRepository>,
    tiers: Arc<dyn TierRepository>,
    metrics: Arc<dyn ServiceMetrics>,
    clock: Arc<dyn Clock>,
}

impl QuoteService {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        registry: Arc<dyn AssetRegistry>,
        observations: Arc<dyn ObservationRepository>,
        tiers: Arc<dyn TierRepository>,
        metrics: Arc<dyn ServiceMetrics>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            registry,
            observations,
            tiers,
            metrics,
            clock,
        }
    }

    /// Serves the requested assets in request order (WF1).
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for a caller without the read operation,
    /// [`AppError::Validation`] for an empty, oversized, or malformed list, and
    /// [`AppError::Unavailable`] when Postgres cannot be read.
    pub async fn get_prices(
        &self,
        caller: &ServiceCaller,
        requested: &[String],
    ) -> Result<Vec<PriceQuote>, AppError> {
        caller.require_operation(Operation::PricesRead)?;
        let asset_ids = validate_request(requested)?;
        let now = self.clock.now();
        match self.registry.sources().await {
            Ok(sources) => self.serve_prioritised(&asset_ids, &sources, now).await,
            Err(error) => {
                tracing::warn!(%error, "C4 source list unavailable; serving stored observations as stale");
                self.serve_degraded(&asset_ids, now).await
            }
        }
    }

    async fn serve_prioritised(
        &self,
        asset_ids: &[String],
        sources: &[PriceSource],
        now: DateTime<Utc>,
    ) -> Result<Vec<PriceQuote>, AppError> {
        let bounds = self.bounds(asset_ids).await?;
        let mut enabled: Vec<PriceSource> = sources
            .iter()
            .filter(|source| source.enabled)
            .cloned()
            .collect();
        enabled.sort_by_key(|source| source.priority);

        let mut per_source: Vec<(PriceSource, HashMap<String, PriceObservation>)> =
            Vec::with_capacity(enabled.len());
        for source in &enabled {
            let observations = self
                .observations
                .latest_accepted_for_source(&source.name, asset_ids)
                .await
                .map_err(AppError::from_repository)?;
            per_source.push((
                source.clone(),
                observations
                    .into_iter()
                    .map(|observation| (observation.asset_id().to_string(), observation))
                    .collect(),
            ));
        }

        let mut quotes = Vec::with_capacity(asset_ids.len());
        for asset_id in asset_ids {
            let bound = *bounds.get(asset_id).unwrap_or(&TAIL_BOUND_SECONDS);
            let tier = tier_for_bound(bound);
            let mut selected: Option<(&PriceObservation, Freshness)> = None;
            let mut newest: Option<&PriceObservation> = None;

            for (source, observations) in &per_source {
                let Some(observation) = observations.get(asset_id) else {
                    continue;
                };
                if newest.is_none_or(|current| observation.observed_at() > current.observed_at()) {
                    newest = Some(observation);
                }
                if selected.is_none() {
                    let age = observation.age_seconds(now);
                    if age <= bound {
                        selected = Some((
                            observation,
                            classify_freshness(age, bound, source.delayed_feed, true),
                        ));
                    }
                }
            }

            let quote = match (selected, newest) {
                (Some((observation, freshness)), _) => Some((observation, freshness)),
                (None, Some(observation)) => {
                    let age = observation.age_seconds(now);
                    let freshness = classify_freshness(age, bound, false, true);
                    (freshness != Freshness::Unpriced).then_some((observation, freshness))
                }
                (None, None) => None,
            };

            match quote {
                Some((observation, freshness)) => {
                    let age = observation.age_seconds(now);
                    self.metrics.record_quote(freshness, tier, age);
                    quotes.push(
                        PriceQuote::priced(asset_id, observation, freshness)
                            .map_err(AppError::from_domain)?,
                    );
                }
                None => {
                    self.metrics.record_quote(Freshness::Unpriced, tier, 0);
                    quotes.push(PriceQuote::unpriced(asset_id).map_err(AppError::from_domain)?);
                }
            }
        }
        Ok(quotes)
    }

    async fn serve_degraded(
        &self,
        asset_ids: &[String],
        now: DateTime<Utc>,
    ) -> Result<Vec<PriceQuote>, AppError> {
        let observations = self
            .observations
            .latest_accepted_for_assets(asset_ids)
            .await
            .map_err(AppError::from_repository)?;
        let by_asset: HashMap<&str, &PriceObservation> = observations
            .iter()
            .map(|observation| (observation.asset_id(), observation))
            .collect();

        let mut quotes = Vec::with_capacity(asset_ids.len());
        for asset_id in asset_ids {
            match by_asset.get(asset_id.as_str()) {
                Some(observation)
                    if observation.age_seconds(now) <= market_data_domain::HARD_BOUND_SECONDS =>
                {
                    self.metrics.record_quote(
                        Freshness::Stale,
                        PriceTier::Tail,
                        observation.age_seconds(now),
                    );
                    quotes.push(
                        PriceQuote::priced(asset_id, observation, Freshness::Stale)
                            .map_err(AppError::from_domain)?,
                    );
                }
                _ => {
                    self.metrics
                        .record_quote(Freshness::Unpriced, PriceTier::Tail, 0);
                    quotes.push(PriceQuote::unpriced(asset_id).map_err(AppError::from_domain)?);
                }
            }
        }
        Ok(quotes)
    }

    async fn bounds(&self, asset_ids: &[String]) -> Result<HashMap<String, i64>, AppError> {
        let rows = self
            .tiers
            .bounds_for(asset_ids)
            .await
            .map_err(AppError::from_repository)?;
        Ok(rows.into_iter().collect())
    }
}

fn tier_for_bound(bound: i64) -> PriceTier {
    if bound <= TOP100_BOUND_SECONDS {
        PriceTier::Top100
    } else {
        PriceTier::Tail
    }
}

fn validate_request(requested: &[String]) -> Result<Vec<String>, AppError> {
    if requested.is_empty() {
        return Err(AppError::Validation(vec![FieldError::new(
            "assetIds",
            "must not be empty",
        )]));
    }
    if requested.len() > MAX_ASSET_IDS {
        return Err(AppError::Validation(vec![FieldError::new(
            "assetIds",
            format!("accepts at most {MAX_ASSET_IDS} ids"),
        )]));
    }
    let mut seen = HashSet::with_capacity(requested.len());
    let mut asset_ids = Vec::with_capacity(requested.len());
    for raw in requested {
        let asset_id = validate_asset_id(raw).map_err(AppError::from_domain)?;
        if seen.insert(asset_id.clone()) {
            asset_ids.push(asset_id);
        }
    }
    Ok(asset_ids)
}
