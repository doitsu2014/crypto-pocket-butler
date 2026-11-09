//! Price-provider placeholder for the walking skeleton.
//!
//! Market Data (U5) owns the real providers. Until it exists, every source
//! reports "no usable price", so the registry returns unpriced — never zero.

use asset_registry_application::ports::{PriceProvider, ProviderError};
use asset_registry_domain::{Asset, Price, PriceSource};
use async_trait::async_trait;

/// A provider that never produces a price.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoPriceProvider;

#[async_trait]
impl PriceProvider for NoPriceProvider {
    async fn fetch_price(
        &self,
        _asset: &Asset,
        _source: &PriceSource,
    ) -> Result<Option<Price>, ProviderError> {
        Ok(None)
    }
}
