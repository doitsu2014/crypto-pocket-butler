use crate::entities::{assets, asset_prices};
use crate::jobs::runner::{JobRunner, JobMetrics};
use chrono::Utc;
use coinpaprika_api::client::Client as CoinPaprikaClient;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    sea_query::OnConflict, Insert,
};
use std::error::Error;
use std::str::FromStr;
use tracing;
use uuid::Uuid;

/// Result of fetching all coins
#[derive(Debug)]
pub struct CollectionResult {
    pub success: bool,
    pub coins_fetched: usize,
    pub assets_created: usize,
    pub assets_updated: usize,
    pub prices_stored: usize,
    pub error: Option<String>,
}

/// Helper function to parse decimal from JSON value
fn parse_decimal_from_quote(quote: &serde_json::Value, key: &str) -> Option<Decimal> {
    quote.get(key)
        .and_then(|v| v.as_f64())
        .and_then(|v| Decimal::from_str(&v.to_string()).ok())
}

/// Helper function to convert supply value to optional decimal
fn optional_supply(value: i64) -> Option<Decimal> {
    if value > 0 {
        Decimal::from_str(&value.to_string()).ok()
    } else {
        None
    }
}

/// Fetch all active coins from CoinPaprika in one request and store in database
/// 
/// This function uses the CoinPaprika SDK to:
/// 1. Fetch all active coins via /tickers endpoint in one request
/// 2. Upsert asset metadata (creates new or updates existing)
/// 3. Store comprehensive price data with rank, supply, and market info
/// 
/// This replaces the previous approach of separate top_coins_collection, 
/// price_collection, and contract_addresses_collection jobs.
/// 
/// # Arguments
/// * `db` - Database connection
/// 
/// # Returns
/// CollectionResult with statistics about the operation
pub async fn fetch_all_coins(
    db: &DatabaseConnection,
) -> Result<CollectionResult, Box<dyn Error + Send + Sync>> {
    let runner = JobRunner::new("fetch_all_coins".to_string());

    let result = runner.execute(|| async {
        // Create CoinPaprika SDK client
        let api_key = std::env::var("COINPAPRIKA_API_KEY").ok();
        let client = if let Some(key) = api_key {
            tracing::info!("Using CoinPaprika Pro API with API key");
            CoinPaprikaClient::with_key(&key)
        } else {
            tracing::info!("Using CoinPaprika free API (rate limited)");
            CoinPaprikaClient::new()
        };

        // Fetch all tickers in one request (with USD quote)
        tracing::info!("Fetching all coins from CoinPaprika via SDK");
        let tickers = client.tickers()
            .quotes(vec!["USD"])
            .send()
            .await
            .map_err(|e| format!("Failed to fetch tickers from CoinPaprika: {}", e))?;

        let coins_fetched = tickers.len();
        tracing::info!("Successfully fetched {} coins from CoinPaprika", coins_fetched);

        let mut assets_created = 0;
        let mut assets_updated = 0;
        let mut prices_to_store = Vec::new();

        let source = "coinpaprika";
        let current_timestamp = Utc::now();

        for ticker in tickers {
            // Get the USD quote (price data)
            let quote_usd = match ticker.quotes.get("USD") {
                Some(q) => q,
                None => {
                    tracing::warn!("No USD quote for {}, skipping", ticker.symbol);
                    continue;
                }
            };

            // Check if asset already exists by symbol or coinpaprika_id
            let existing_asset = assets::Entity::find()
                .filter(
                    assets::Column::Symbol.eq(&ticker.symbol.to_uppercase())
                        .or(assets::Column::CoingeckoId.eq(&ticker.id))
                )
                .one(db)
                .await
                .map_err(|e| format!("Failed to query assets: {}", e))?;

            let asset_id = match existing_asset {
                Some(existing) => {
                    // Update existing asset
                    let mut asset_update: assets::ActiveModel = existing.into();
                    asset_update.name = ActiveValue::Set(ticker.name.clone());
                    asset_update.symbol = ActiveValue::Set(ticker.symbol.to_uppercase());
                    asset_update.coingecko_id = ActiveValue::Set(Some(ticker.id.clone()));
                    asset_update.is_active = ActiveValue::Set(true);
                    asset_update.updated_at = ActiveValue::Set(current_timestamp.into());
                    
                    let updated = asset_update.update(db).await
                        .map_err(|e| format!("Failed to update asset: {}", e))?;
                    assets_updated += 1;
                    tracing::debug!("Updated asset: {} ({})", updated.symbol, updated.id);
                    updated.id
                }
                None => {
                    // Create new asset
                    let new_asset = assets::ActiveModel {
                        id: ActiveValue::Set(Uuid::new_v4()),
                        symbol: ActiveValue::Set(ticker.symbol.to_uppercase()),
                        name: ActiveValue::Set(ticker.name.clone()),
                        asset_type: ActiveValue::Set("cryptocurrency".to_string()),
                        coingecko_id: ActiveValue::Set(Some(ticker.id.clone())),
                        coinmarketcap_id: ActiveValue::NotSet,
                        logo_url: ActiveValue::NotSet,
                        description: ActiveValue::NotSet,
                        decimals: ActiveValue::NotSet,
                        is_active: ActiveValue::Set(true),
                        created_at: ActiveValue::Set(current_timestamp.into()),
                        updated_at: ActiveValue::Set(current_timestamp.into()),
                    };
                    
                    let inserted = new_asset.insert(db).await
                        .map_err(|e| format!("Failed to insert asset: {}", e))?;
                    assets_created += 1;
                    tracing::debug!("Created asset: {} ({})", inserted.symbol, inserted.id);
                    inserted.id
                }
            };

            // Parse price from USD quote
            let price_usd = parse_decimal_from_quote(&quote_usd, "price")
                .unwrap_or_else(|| Decimal::ZERO);
            let market_cap_usd = parse_decimal_from_quote(&quote_usd, "market_cap");
            let volume_24h_usd = parse_decimal_from_quote(&quote_usd, "volume_24h");
            let change_percent_24h = parse_decimal_from_quote(&quote_usd, "percent_change_24h");

            // Extended fields from CoinPaprika
            let rank = Some(ticker.rank as i32);
            let circulating_supply = optional_supply(ticker.circulating_supply);
            let total_supply = optional_supply(ticker.total_supply);
            let max_supply = optional_supply(ticker.max_supply);
            
            let beta_value = if ticker.beta_value != 0.0 {
                Decimal::from_str(&ticker.beta_value.to_string()).ok()
            } else {
                None
            };
            
            let percent_change_1h = parse_decimal_from_quote(&quote_usd, "percent_change_1h");
            let percent_change_7d = parse_decimal_from_quote(&quote_usd, "percent_change_7d");
            let percent_change_30d = parse_decimal_from_quote(&quote_usd, "percent_change_30d");
            let ath_price = parse_decimal_from_quote(&quote_usd, "ath_price");
            
            let ath_date = quote_usd.get("ath_date")
                .and_then(|v| v.as_str())
                .and_then(|d| chrono::DateTime::parse_from_rfc3339(d).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc));
            
            let percent_from_price_ath = parse_decimal_from_quote(&quote_usd, "percent_from_price_ath");

            let new_price = asset_prices::ActiveModel {
                id: ActiveValue::Set(Uuid::new_v4()),
                asset_id: ActiveValue::Set(asset_id),
                timestamp: ActiveValue::Set(current_timestamp.into()),
                price_usd: ActiveValue::Set(price_usd),
                volume_24h_usd: ActiveValue::Set(volume_24h_usd),
                market_cap_usd: ActiveValue::Set(market_cap_usd),
                change_percent_24h: ActiveValue::Set(change_percent_24h),
                source: ActiveValue::Set(source.to_string()),
                // Extended fields from CoinPaprika refactoring
                rank: ActiveValue::Set(rank),
                circulating_supply: ActiveValue::Set(circulating_supply),
                total_supply: ActiveValue::Set(total_supply),
                max_supply: ActiveValue::Set(max_supply),
                beta_value: ActiveValue::Set(beta_value),
                percent_change_1h: ActiveValue::Set(percent_change_1h),
                percent_change_7d: ActiveValue::Set(percent_change_7d),
                percent_change_30d: ActiveValue::Set(percent_change_30d),
                ath_price: ActiveValue::Set(ath_price),
                ath_date: ActiveValue::Set(ath_date.map(Into::into)),
                percent_from_price_ath: ActiveValue::Set(percent_from_price_ath),
                created_at: ActiveValue::Set(current_timestamp.into()),
            };

            prices_to_store.push(new_price);

            // Batch insert every 500 prices to avoid too large transactions
            if prices_to_store.len() >= 500 {
                let count = prices_to_store.len();
                match Insert::many(prices_to_store)
                    .on_conflict(
                        OnConflict::columns([
                            asset_prices::Column::AssetId,
                            asset_prices::Column::Timestamp,
                            asset_prices::Column::Source,
                        ])
                        .update_columns([
                            asset_prices::Column::PriceUsd,
                            asset_prices::Column::Volume24hUsd,
                            asset_prices::Column::MarketCapUsd,
                            asset_prices::Column::ChangePercent24h,
                            asset_prices::Column::Rank,
                            asset_prices::Column::CirculatingSupply,
                            asset_prices::Column::TotalSupply,
                            asset_prices::Column::MaxSupply,
                            asset_prices::Column::BetaValue,
                            asset_prices::Column::PercentChange1h,
                            asset_prices::Column::PercentChange7d,
                            asset_prices::Column::PercentChange30d,
                            asset_prices::Column::AthPrice,
                            asset_prices::Column::AthDate,
                            asset_prices::Column::PercentFromPriceAth,
                        ])
                        .to_owned(),
                    )
                    .exec(db)
                    .await
                {
                    Ok(_) => {
                        tracing::info!("Batch stored {} prices", count);
                    }
                    Err(e) => {
                        tracing::error!("Failed to batch store prices: {}", e);
                    }
                }
                prices_to_store = Vec::new();
            }
        }

        // Insert remaining prices
        let prices_stored = if !prices_to_store.is_empty() {
            let count = prices_to_store.len();
            match Insert::many(prices_to_store)
                .on_conflict(
                    OnConflict::columns([
                        asset_prices::Column::AssetId,
                        asset_prices::Column::Timestamp,
                        asset_prices::Column::Source,
                    ])
                    .update_columns([
                        asset_prices::Column::PriceUsd,
                        asset_prices::Column::Volume24hUsd,
                        asset_prices::Column::MarketCapUsd,
                        asset_prices::Column::ChangePercent24h,
                        asset_prices::Column::Rank,
                        asset_prices::Column::CirculatingSupply,
                        asset_prices::Column::TotalSupply,
                        asset_prices::Column::MaxSupply,
                        asset_prices::Column::BetaValue,
                        asset_prices::Column::PercentChange1h,
                        asset_prices::Column::PercentChange7d,
                        asset_prices::Column::PercentChange30d,
                        asset_prices::Column::AthPrice,
                        asset_prices::Column::AthDate,
                        asset_prices::Column::PercentFromPriceAth,
                    ])
                    .to_owned(),
                )
                .exec(db)
                .await
            {
                Ok(_) => {
                    tracing::info!("Batch stored {} prices", count);
                    count
                }
                Err(e) => {
                    tracing::error!("Failed to batch store prices: {}", e);
                    0
                }
            }
        } else {
            0
        };

        tracing::info!(
            "Fetch all coins completed: {} coins fetched, {} assets created, {} updated, {} prices stored",
            coins_fetched, assets_created, assets_updated, prices_stored
        );

        Ok(JobMetrics {
            items_processed: coins_fetched,
            items_created: assets_created,
            items_updated: assets_updated,
            items_skipped: 0,
            custom: serde_json::json!({
                "coins_fetched": coins_fetched,
                "assets_created": assets_created,
                "assets_updated": assets_updated,
                "prices_stored": prices_stored,
            }),
        })
    }).await;

    // Convert JobResult to CollectionResult
    Ok(CollectionResult {
        success: result.success,
        coins_fetched: result.metrics.custom["coins_fetched"].as_u64().unwrap_or(0) as usize,
        assets_created: result.metrics.custom["assets_created"].as_u64().unwrap_or(0) as usize,
        assets_updated: result.metrics.custom["assets_updated"].as_u64().unwrap_or(0) as usize,
        prices_stored: result.metrics.custom["prices_stored"].as_u64().unwrap_or(0) as usize,
        error: result.error,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collection_result_creation() {
        let result = CollectionResult {
            success: true,
            coins_fetched: 1000,
            assets_created: 50,
            assets_updated: 950,
            prices_stored: 1000,
            error: None,
        };
        assert!(result.success);
        assert_eq!(result.coins_fetched, 1000);
    }
}
