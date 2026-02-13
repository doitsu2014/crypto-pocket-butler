use crate::connectors::coingecko::CoinGeckoConnector;
use crate::entities::{asset_prices, assets, accounts};
use chrono::{Timelike, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect,
};
use serde::Deserialize;
use std::collections::HashSet;
use std::error::Error;
use std::str::FromStr;
use tracing;
use uuid::Uuid;

/// Result of collecting prices
#[derive(Debug)]
pub struct CollectionResult {
    pub success: bool,
    pub assets_tracked: usize,
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
/// This function:
/// 1. Identifies tracked assets (Top 100 by market cap + portfolio assets)
/// 2. Fetches current prices from CoinGecko
/// 3. Stores prices in asset_prices table with deduplication
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
    tracing::info!("Starting price collection job for top {} coins + portfolio assets", top_n_limit);

    // Step 1: Get list of tracked assets
    let tracked_assets = get_tracked_assets(db, top_n_limit).await?;
    let assets_tracked = tracked_assets.len();
    
    if tracked_assets.is_empty() {
        tracing::warn!("No tracked assets found to collect prices for");
        return Ok(CollectionResult {
            success: true,
            assets_tracked: 0,
            prices_collected: 0,
            prices_stored: 0,
            error: None,
        });
    }

    tracing::info!("Found {} unique assets to collect prices for", assets_tracked);

    // Step 2: Fetch prices from CoinGecko
    let connector = CoinGeckoConnector::new();
    let price_data = match fetch_prices_for_assets(&connector, &tracked_assets, top_n_limit).await {
        Ok(data) => data,
        Err(e) => {
            tracing::error!("Failed to fetch prices from CoinGecko: {}", e);
            return Ok(CollectionResult {
                success: false,
                assets_tracked,
                prices_collected: 0,
                prices_stored: 0,
                error: Some(format!("Failed to fetch prices: {}", e)),
            });
        }
    };

    let prices_collected = price_data.len();
    tracing::info!("Fetched {} prices from CoinGecko", prices_collected);

    // Step 3: Store prices in database
    let prices_stored = store_prices(db, &price_data).await?;

    tracing::info!(
        "Price collection completed: {} assets tracked, {} prices collected, {} prices stored",
        assets_tracked,
        prices_collected,
        prices_stored
    );

    Ok(CollectionResult {
        success: true,
        assets_tracked,
        prices_collected,
        prices_stored,
        error: None,
    })
}

/// Get list of tracked assets (Top N + portfolio assets)
async fn get_tracked_assets(
    db: &DatabaseConnection,
    top_n_limit: usize,
) -> Result<Vec<assets::Model>, Box<dyn Error + Send + Sync>> {
    let mut tracked_asset_ids: HashSet<Uuid> = HashSet::new();

    // Get top N assets by market cap rank
    // We'll get assets that have rankings (which means they're in the top coins)
    let top_assets = assets::Entity::find()
        .filter(assets::Column::IsActive.eq(true))
        .filter(assets::Column::CoingeckoId.is_not_null())
        .limit(top_n_limit as u64)
        .all(db)
        .await?;

    for asset in &top_assets {
        tracked_asset_ids.insert(asset.id);
    }

    tracing::debug!("Found {} top assets by market cap", tracked_asset_ids.len());

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
        .filter(assets::Column::CoingeckoId.is_not_null()) // Only assets with CoinGecko ID
        .all(db)
        .await?;

    Ok(tracked_assets)
}

/// Fetch prices from CoinGecko for given assets
async fn fetch_prices_for_assets(
    connector: &CoinGeckoConnector,
    tracked_assets: &[assets::Model],
    top_n_limit: usize,
) -> Result<Vec<PriceData>, Box<dyn Error + Send + Sync>> {
    // Use the fetch_top_coins method which gives us prices for top coins
    // This is efficient as it gives us up to 250 coins in a single API call
    let coins = connector.fetch_top_coins(top_n_limit).await?;

    // Map CoinGecko coin data to our tracked assets
    let mut price_data = Vec::new();
    
    for asset in tracked_assets {
        if let Some(coingecko_id) = &asset.coingecko_id {
            // Find matching coin data
            if let Some(coin) = coins.iter().find(|c| &c.id == coingecko_id) {
                price_data.push(PriceData {
                    asset_id: asset.id,
                    price_usd: coin.current_price,
                    volume_24h_usd: coin.total_volume,
                    market_cap_usd: Some(coin.market_cap),
                    change_percent_24h: coin.price_change_percentage_24h,
                });
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
}

/// Store prices in the database
async fn store_prices(
    db: &DatabaseConnection,
    price_data: &[PriceData],
) -> Result<usize, Box<dyn Error + Send + Sync>> {
    let timestamp = Utc::now();
    let source = "coingecko";
    let mut stored_count = 0;

    for data in price_data {
        // Check if price already exists for this asset/timestamp/source combination
        // Round timestamp to the nearest minute to avoid duplicates from multiple runs
        let rounded_timestamp = timestamp
            .date_naive()
            .and_hms_opt(timestamp.hour(), timestamp.minute(), 0)
            .unwrap()
            .and_utc();

        let existing = asset_prices::Entity::find()
            .filter(asset_prices::Column::AssetId.eq(data.asset_id))
            .filter(asset_prices::Column::Timestamp.eq(rounded_timestamp))
            .filter(asset_prices::Column::Source.eq(source))
            .one(db)
            .await?;

        if existing.is_some() {
            tracing::debug!(
                "Price already exists for asset {} at timestamp {}, skipping",
                data.asset_id,
                rounded_timestamp
            );
            continue;
        }

        // Create new price entry
        let price_usd = Decimal::from_str(&data.price_usd.to_string())
            .unwrap_or_else(|_| Decimal::ZERO);
        let volume_24h_usd = data.volume_24h_usd
            .and_then(|v| Decimal::from_str(&v.to_string()).ok());
        let market_cap_usd = data.market_cap_usd
            .and_then(|v| Decimal::from_str(&v.to_string()).ok());
        let change_percent_24h = data.change_percent_24h
            .and_then(|v| Decimal::from_str(&v.to_string()).ok());

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
        };

        match new_price.insert(db).await {
            Ok(_) => {
                stored_count += 1;
                tracing::debug!("Stored price for asset {} at {}", data.asset_id, rounded_timestamp);
            }
            Err(e) => {
                tracing::warn!("Failed to store price for asset {}: {}", data.asset_id, e);
            }
        }
    }

    Ok(stored_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collection_result_creation() {
        let result = CollectionResult {
            success: true,
            assets_tracked: 150,
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
