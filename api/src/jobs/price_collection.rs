use crate::connectors::coinpaprika::CoinPaprikaConnector;
use crate::entities::{asset_prices, assets, accounts};
use crate::jobs::runner::{JobRunner, JobMetrics};
use chrono::{Timelike, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, 
    QueryOrder, QuerySelect, sea_query::OnConflict, Insert,
};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::str::FromStr;
use tracing;
use uuid::Uuid;

/// Result of collecting prices
#[derive(Debug)]
pub struct CollectionResult {
    pub success: bool,
    pub assets_tracked: usize,
    pub assets_created: usize,
    pub assets_updated: usize,
    pub prices_collected: usize,
    pub prices_stored: usize,
    pub error: Option<String>,
}

/// Holding data structure from accounts JSON
#[derive(Debug, Deserialize)]
struct Holding {
    pub symbol: Option<String>,
    pub asset_id: Option<Uuid>,
}

/// Collect prices for tracked assets and store in database
/// 
/// This function is the unified job that:
/// 1. Fetches top N coins from CoinPaprika (by market cap)
/// 2. Creates/updates asset records for these coins
/// 3. Fetches prices for tracked assets (including portfolio holdings)
/// 4. Stores all price data with rank and market info in asset_prices table
/// 
/// This replaces the need for separate top_coins_collection and price_collection jobs.
/// 
/// # Arguments
/// * `db` - Database connection
/// * `top_n_limit` - Number of top coins by market cap to include (default: 100)
/// 
/// # Returns
/// CollectionResult with statistics about the operation
pub async fn collect_prices(
    db: &DatabaseConnection,
    top_n_limit: usize,
) -> Result<CollectionResult, Box<dyn Error + Send + Sync>> {
    let runner = JobRunner::new(format!("price_collection(top_n={})", top_n_limit));

    let result = runner.execute(|| async {
        // Step 1: Fetch top N coins from CoinPaprika to discover/update assets
        let connector = CoinPaprikaConnector::new();
        let top_coins = connector.fetch_top_coins(top_n_limit).await
            .map_err(|e| format!("Failed to fetch top coins: {}", e))?;
        
        let mut assets_created = 0;
        let mut assets_updated = 0;
        
        // Step 2: Upsert asset records for top coins
        for coin in &top_coins {
            match upsert_asset(db, coin).await {
                Ok((created, _asset_id)) => {
                    if created {
                        assets_created += 1;
                    } else {
                        assets_updated += 1;
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to upsert asset {}: {}", coin.symbol, e);
                }
            }
        }
        
        tracing::info!(
            "Asset upserts completed: {} created, {} updated",
            assets_created, assets_updated
        );
        
        // Step 3: Get list of all tracked assets (including portfolio holdings)
        let tracked_assets = get_tracked_assets(db, top_n_limit).await
            .map_err(|e| format!("Failed to get tracked assets: {}", e))?;
        
        let assets_tracked = tracked_assets.len();
        
        if tracked_assets.is_empty() {
            return Ok(JobMetrics {
                items_processed: 0,
                items_created: 0,
                items_updated: 0,
                items_skipped: 0,
                custom: serde_json::json!({
                    "assets_tracked": 0,
                    "assets_created": assets_created,
                    "assets_updated": assets_updated,
                    "prices_collected": 0,
                    "prices_stored": 0,
                }),
            });
        }

        tracing::info!("Found {} unique assets to collect prices for", assets_tracked);

        // Step 4: Fetch prices from CoinPaprika (reuses top_coins data + fetches additional)
        let price_data = fetch_prices_for_assets(&connector, &tracked_assets, top_n_limit).await
            .map_err(|e| format!("Failed to fetch prices: {}", e))?;

        let prices_collected = price_data.len();
        tracing::info!("Fetched {} prices from CoinPaprika", prices_collected);

        // Step 5: Store prices in database using upserts
        let prices_stored = store_prices(db, &price_data).await
            .map_err(|e| format!("Failed to store prices: {}", e))?;

        Ok(JobMetrics {
            items_processed: assets_tracked,
            items_created: prices_stored,
            items_updated: 0, // Upserts treated as creates for metrics simplicity
            items_skipped: 0, // All prices are upserted (inserted or updated)
            custom: serde_json::json!({
                "assets_tracked": assets_tracked,
                "assets_created": assets_created,
                "assets_updated": assets_updated,
                "prices_collected": prices_collected,
                "prices_stored": prices_stored,
            }),
        })
    }).await;

    // Convert JobResult to CollectionResult for backwards compatibility
    Ok(CollectionResult {
        success: result.success,
        assets_tracked: result.metrics.items_processed,
        assets_created: result.metrics.custom["assets_created"].as_u64().unwrap_or(0) as usize,
        assets_updated: result.metrics.custom["assets_updated"].as_u64().unwrap_or(0) as usize,
        prices_collected: result.metrics.custom["prices_collected"].as_u64().unwrap_or(0) as usize,
        prices_stored: result.metrics.items_created,
        error: result.error,
    })
}

/// Upsert an asset record from CoinPaprika data
/// Returns (created: bool, asset_id: Uuid)
async fn upsert_asset(
    db: &DatabaseConnection,
    coin: &crate::connectors::coinpaprika::CoinMarketData,
) -> Result<(bool, Uuid), Box<dyn Error + Send + Sync>> {
    use crate::entities::assets;
    use sea_orm::{ActiveModelTrait, Condition};
    
    // Check if asset already exists by (symbol AND name) OR coingecko_id (legacy field name, stores coinpaprika_id)
    // The new uniqueness constraint requires both symbol and name to match
    let existing_asset = assets::Entity::find()
        .filter(
            Condition::any()
                .add(
                    Condition::all()
                        .add(assets::Column::Symbol.eq(&coin.symbol.to_uppercase()))
                        .add(assets::Column::Name.eq(&coin.name))
                )
                .add(assets::Column::CoingeckoId.eq(&coin.id))
        )
        .one(db)
        .await?;
    
    match existing_asset {
        Some(existing) => {
            // Update existing asset
            let mut asset_update: assets::ActiveModel = existing.into();
            asset_update.name = ActiveValue::Set(coin.name.clone());
            asset_update.symbol = ActiveValue::Set(coin.symbol.to_uppercase());
            asset_update.coingecko_id = ActiveValue::Set(Some(coin.id.clone()));
            asset_update.is_active = ActiveValue::Set(true);
            asset_update.updated_at = ActiveValue::Set(Utc::now().into());
            
            let updated = asset_update.update(db).await?;
            tracing::debug!("Updated asset: {} ({})", updated.symbol, updated.id);
            Ok((false, updated.id))
        }
        None => {
            // Create new asset
            let new_asset = assets::ActiveModel {
                id: ActiveValue::Set(Uuid::new_v4()),
                symbol: ActiveValue::Set(coin.symbol.to_uppercase()),
                name: ActiveValue::Set(coin.name.clone()),
                asset_type: ActiveValue::Set("cryptocurrency".to_string()),
                coingecko_id: ActiveValue::Set(Some(coin.id.clone())),
                coinmarketcap_id: ActiveValue::NotSet,
                logo_url: ActiveValue::NotSet,
                description: ActiveValue::NotSet,
                decimals: ActiveValue::NotSet,
                is_active: ActiveValue::Set(true),
                created_at: ActiveValue::Set(Utc::now().into()),
                updated_at: ActiveValue::Set(Utc::now().into()),
            };
            
            let inserted = new_asset.insert(db).await?;
            tracing::debug!("Created asset: {} ({})", inserted.symbol, inserted.id);
            Ok((true, inserted.id))
        }
    }
}

/// Get list of tracked assets (Top N + portfolio assets)
async fn get_tracked_assets(
    db: &DatabaseConnection,
    top_n_limit: usize,
) -> Result<Vec<assets::Model>, Box<dyn Error + Send + Sync>> {
    let mut tracked_asset_ids: HashSet<Uuid> = HashSet::new();

    // Get up to N active assets with CoinPaprika IDs from database
    // Note: The actual "top N by market cap" is determined by CoinPaprika API in fetch_prices_for_assets().
    // This query just ensures we have asset records in our DB to match against.
    // The limit here helps reduce unnecessary lookups when we have many assets in the DB.
    // Note: The field is named CoingeckoId for historical reasons but now stores CoinPaprika IDs
    let top_assets = assets::Entity::find()
        .filter(assets::Column::IsActive.eq(true))
        .filter(assets::Column::CoingeckoId.is_not_null())
        .order_by_asc(assets::Column::Symbol) // Order by symbol for consistent results
        .limit(top_n_limit as u64)
        .all(db)
        .await?;

    for asset in &top_assets {
        tracked_asset_ids.insert(asset.id);
    }

    tracing::debug!("Found {} assets from database", tracked_asset_ids.len());

    // Get assets from portfolio holdings
    let accounts_with_holdings = accounts::Entity::find()
        .filter(accounts::Column::IsActive.eq(true))
        .filter(accounts::Column::Holdings.is_not_null())
        .all(db)
        .await?;

    for account in accounts_with_holdings {
        if let Some(holdings_json) = account.holdings {
            // Parse holdings JSON to extract asset_ids
            if let Ok(holdings) = serde_json::from_value::<Vec<Holding>>(holdings_json) {
                for holding in holdings {
                    if let Some(asset_id) = holding.asset_id {
                        tracked_asset_ids.insert(asset_id);
                    } else if let Some(symbol) = holding.symbol {
                        // If only symbol is available, lookup asset_id
                        if let Ok(Some(asset)) = assets::Entity::find()
                            .filter(assets::Column::Symbol.eq(symbol.to_uppercase()))
                            .one(db)
                            .await
                        {
                            tracked_asset_ids.insert(asset.id);
                        }
                    }
                }
            }
        }
    }

    tracing::debug!(
        "Total unique tracked assets (including portfolio holdings): {}",
        tracked_asset_ids.len()
    );

    // Fetch full asset models for all tracked asset IDs
    let tracked_assets = assets::Entity::find()
        .filter(assets::Column::Id.is_in(tracked_asset_ids))
        .filter(assets::Column::CoingeckoId.is_not_null()) // Only assets with CoinPaprika ID (field name is legacy)
        .all(db)
        .await?;

    Ok(tracked_assets)
}

/// Fetch prices from CoinPaprika for tracked assets
/// 
/// This function ensures we get prices for:
/// 1. Top N coins by market cap (from CoinPaprika's tickers API, sorted by market cap)
/// 2. All portfolio assets (even if not in top N)
async fn fetch_prices_for_assets(
    connector: &CoinPaprikaConnector,
    tracked_assets: &[assets::Model],
    top_n_limit: usize,
) -> Result<Vec<PriceData>, Box<dyn Error + Send + Sync>> {
    // Fetch top N coins by market cap from CoinPaprika
    // CoinPaprika returns these pre-sorted by market cap rank, so this IS the true top N
    let mut all_coins = connector.fetch_top_coins(top_n_limit).await?;
    
    // Build a set of coinpaprika_ids we already have
    let fetched_ids: HashSet<String> = all_coins.iter().map(|c| c.id.clone()).collect();
    
    // Identify assets not in the top N that we still need prices for
    let mut missing_coin_ids: Vec<String> = Vec::new();
    for asset in tracked_assets {
        if let Some(coinpaprika_id) = &asset.coingecko_id {
            if !fetched_ids.contains(coinpaprika_id) {
                missing_coin_ids.push(coinpaprika_id.clone());
            }
        }
    }
    
    // Fetch prices for missing coins (portfolio assets not in top N)
    if !missing_coin_ids.is_empty() {
        tracing::info!(
            "Fetching prices for {} additional portfolio assets not in top {}",
            missing_coin_ids.len(),
            top_n_limit
        );
        
        let mut additional_coins = connector.fetch_coins_by_ids(&missing_coin_ids).await?;
        
        // Add them to our all_coins vec
        all_coins.append(&mut additional_coins);
    }
    
    // Build a map of coinpaprika_id to coin data for quick lookup
    let coins_map: HashMap<String, &crate::connectors::coinpaprika::CoinMarketData> = 
        all_coins.iter().map(|c| (c.id.clone(), c)).collect();
    
    // Map CoinPaprika coin data to our tracked assets
    let mut price_data = Vec::new();
    
    for asset in tracked_assets {
        if let Some(coinpaprika_id) = &asset.coingecko_id {
            // Find matching coin data
            if let Some(coin) = coins_map.get(coinpaprika_id) {
                price_data.push(PriceData {
                    asset_id: asset.id,
                    price_usd: coin.quotes.usd.price,
                    volume_24h_usd: coin.quotes.usd.volume_24h,
                    market_cap_usd: Some(coin.quotes.usd.market_cap),
                    change_percent_24h: coin.quotes.usd.percent_change_24h,
                    // New fields from CoinPaprika
                    rank: Some(coin.rank),
                    circulating_supply: coin.circulating_supply,
                    total_supply: coin.total_supply,
                    max_supply: coin.max_supply,
                    beta_value: coin.beta_value,
                    percent_change_1h: coin.quotes.usd.percent_change_1h,
                    percent_change_7d: coin.quotes.usd.percent_change_7d,
                    percent_change_30d: coin.quotes.usd.percent_change_30d,
                    ath_price: coin.quotes.usd.ath_price,
                    ath_date: coin.quotes.usd.ath_date.clone(),
                    percent_from_price_ath: coin.quotes.usd.percent_from_price_ath,
                });
            } else {
                tracing::warn!(
                    "No price data found for asset {} ({})",
                    asset.symbol,
                    coinpaprika_id
                );
            }
        }
    }

    Ok(price_data)
}

/// Price data to be stored
#[derive(Debug, Clone)]
struct PriceData {
    asset_id: Uuid,
    price_usd: f64,
    volume_24h_usd: Option<f64>,
    market_cap_usd: Option<f64>,
    change_percent_24h: Option<f64>,
    // New fields from CoinPaprika
    rank: Option<u32>,
    circulating_supply: Option<f64>,
    total_supply: Option<f64>,
    max_supply: Option<f64>,
    beta_value: Option<f64>,
    percent_change_1h: Option<f64>,
    percent_change_7d: Option<f64>,
    percent_change_30d: Option<f64>,
    ath_price: Option<f64>,
    ath_date: Option<String>,
    percent_from_price_ath: Option<f64>,
}

/// Store prices in the database using ON CONFLICT for idempotency
/// Uses batched inserts for better performance
async fn store_prices(
    db: &DatabaseConnection,
    price_data: &[PriceData],
) -> Result<usize, Box<dyn Error + Send + Sync>> {
    let timestamp = Utc::now();
    let source = "coinpaprika";
    
    // Prepare all price records in a batch
    let mut price_models = Vec::new();

    for data in price_data {
        // Round timestamp to the nearest minute for consistent time buckets
        let rounded_timestamp = match timestamp
            .date_naive()
            .and_hms_opt(timestamp.hour(), timestamp.minute(), 0)
        {
            Some(dt) => dt.and_utc(),
            None => {
                tracing::error!(
                    "Failed to create rounded timestamp from {} for asset {}",
                    timestamp,
                    data.asset_id
                );
                continue;
            }
        };

        // Convert price data to Decimal
        let price_usd = match Decimal::from_str(&data.price_usd.to_string()) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(
                    "Failed to convert price {} to Decimal for asset {}: {}. Skipping.",
                    data.price_usd,
                    data.asset_id,
                    e
                );
                continue;
            }
        };
        
        let volume_24h_usd = data.volume_24h_usd
            .and_then(|v| {
                Decimal::from_str(&v.to_string())
                    .map_err(|e| {
                        tracing::warn!("Failed to convert volume {} to Decimal: {}", v, e);
                        e
                    })
                    .ok()
            });
        
        let market_cap_usd = data.market_cap_usd
            .and_then(|v| {
                Decimal::from_str(&v.to_string())
                    .map_err(|e| {
                        tracing::warn!("Failed to convert market cap {} to Decimal: {}", v, e);
                        e
                    })
                    .ok()
            });
        
        let change_percent_24h = data.change_percent_24h
            .and_then(|v| {
                Decimal::from_str(&v.to_string())
                    .map_err(|e| {
                        tracing::warn!("Failed to convert change percent {} to Decimal: {}", v, e);
                        e
                    })
                    .ok()
            });
        
        // Convert new fields to Decimal
        let rank = data.rank.map(|rank_value| rank_value as i32);
        
        let circulating_supply = data.circulating_supply
            .and_then(|v| Decimal::from_str(&v.to_string()).ok());
        
        let total_supply = data.total_supply
            .and_then(|v| Decimal::from_str(&v.to_string()).ok());
        
        let max_supply = data.max_supply
            .and_then(|v| Decimal::from_str(&v.to_string()).ok());
        
        let beta_value = data.beta_value
            .and_then(|v| Decimal::from_str(&v.to_string()).ok());
        
        let percent_change_1h = data.percent_change_1h
            .and_then(|v| Decimal::from_str(&v.to_string()).ok());
        
        let percent_change_7d = data.percent_change_7d
            .and_then(|v| Decimal::from_str(&v.to_string()).ok());
        
        let percent_change_30d = data.percent_change_30d
            .and_then(|v| Decimal::from_str(&v.to_string()).ok());
        
        let ath_price = data.ath_price
            .and_then(|v| Decimal::from_str(&v.to_string()).ok());
        
        let percent_from_price_ath = data.percent_from_price_ath
            .and_then(|v| Decimal::from_str(&v.to_string()).ok());
        
        // Parse ATH date string to DateTimeWithTimeZone
        let ath_date = data.ath_date.as_ref().and_then(|date_str| {
            chrono::DateTime::parse_from_rfc3339(date_str)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc).into())
        });

        let new_price = asset_prices::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            asset_id: ActiveValue::Set(data.asset_id),
            timestamp: ActiveValue::Set(rounded_timestamp.into()),
            price_usd: ActiveValue::Set(price_usd),
            volume_24h_usd: ActiveValue::Set(volume_24h_usd),
            market_cap_usd: ActiveValue::Set(market_cap_usd),
            change_percent_24h: ActiveValue::Set(change_percent_24h),
            source: ActiveValue::Set(source.to_string()),
            created_at: ActiveValue::Set(timestamp.into()),
            // New fields
            rank: ActiveValue::Set(rank),
            circulating_supply: ActiveValue::Set(circulating_supply),
            total_supply: ActiveValue::Set(total_supply),
            max_supply: ActiveValue::Set(max_supply),
            beta_value: ActiveValue::Set(beta_value),
            percent_change_1h: ActiveValue::Set(percent_change_1h),
            percent_change_7d: ActiveValue::Set(percent_change_7d),
            percent_change_30d: ActiveValue::Set(percent_change_30d),
            ath_price: ActiveValue::Set(ath_price),
            ath_date: ActiveValue::Set(ath_date),
            percent_from_price_ath: ActiveValue::Set(percent_from_price_ath),
        };

        price_models.push(new_price);
    }

    if price_models.is_empty() {
        return Ok(0);
    }

    // Batch insert with ON CONFLICT for idempotency
    // The unique constraint on (asset_id, timestamp, source) ensures idempotency
    match Insert::many(price_models)
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
                // Update new fields on conflict
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
            tracing::info!("Batch upserted {} prices", price_data.len());
            Ok(price_data.len())
        }
        Err(e) => {
            tracing::error!("Failed to batch upsert prices: {}", e);
            Err(Box::new(e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collection_result_creation() {
        let result = CollectionResult {
            success: true,
            assets_tracked: 150,
            assets_created: 10,
            assets_updated: 140,
            prices_collected: 150,
            prices_stored: 150,
            error: None,
        };
        assert!(result.success);
        assert_eq!(result.assets_tracked, 150);
        assert_eq!(result.prices_collected, 150);
        assert_eq!(result.prices_stored, 150);
    }
}
