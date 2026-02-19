use crate::connectors::coinpaprika::CoinPaprikaConnector;
use crate::entities::{assets, asset_prices};
use crate::jobs::runner::{JobRunner, JobMetrics};
use chrono::Utc;
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

/// Helper function to parse decimal from f64 option
fn parse_decimal_from_f64(value: Option<f64>) -> Option<Decimal> {
    value.and_then(|v| Decimal::from_str(&v.to_string()).ok())
}

/// Deduplicate prices by keeping only the last occurrence of each unique (asset_id, timestamp, source) combination
/// This prevents the "ON CONFLICT DO UPDATE command cannot affect row a second time" error
/// 
/// When duplicates exist, the last occurrence in the input vector is kept, ensuring the most recent
/// data for each unique key is retained.
pub fn deduplicate_prices(prices: Vec<asset_prices::ActiveModel>) -> Vec<asset_prices::ActiveModel> {
    use std::collections::HashMap;
    
    let mut price_map: HashMap<(Uuid, i64, String), asset_prices::ActiveModel> = HashMap::new();
    
    for price in prices {
        // Extract the key fields
        let asset_id = match &price.asset_id {
            ActiveValue::Set(id) => *id,
            _ => continue,
        };
        let timestamp_millis = match &price.timestamp {
            ActiveValue::Set(ts) => ts.timestamp_millis(),
            _ => continue,
        };
        let source = match &price.source {
            ActiveValue::Set(s) => s.clone(),
            _ => continue,
        };
        
        let key = (asset_id, timestamp_millis, source);
        // This will overwrite any previous entry with the same key, keeping only the last one
        price_map.insert(key, price);
    }
    
    price_map.into_values().collect()
}

/// Fetch all active coins from CoinPaprika in one request and store in database
/// 
/// This function uses the CoinPaprika connector to:
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
        // Create CoinPaprika connector
        let connector = CoinPaprikaConnector::new();

        // Fetch all coins in one request
        tracing::info!("Fetching all coins from CoinPaprika");
        let coins = connector.fetch_all_coins()
            .await
            .map_err(|e| format!("Failed to fetch coins from CoinPaprika: {}", e))?;

        let coins_fetched = coins.len();
        tracing::info!("Successfully fetched {} coins from CoinPaprika", coins_fetched);

        let mut assets_created = 0;
        let mut assets_updated = 0;
        let mut prices_to_store = Vec::new();

        let source = "coinpaprika";
        let current_timestamp = Utc::now();

        for coin in coins {
            // Parse price from USD quote
            let price_usd = match Decimal::from_str(&coin.quotes.usd.price.to_string()) {
                Ok(price) => price,
                Err(e) => {
                    tracing::warn!(
                        "Failed to parse price for {} ({}): {}. Using ZERO.",
                        coin.symbol, coin.id, e
                    );
                    Decimal::ZERO
                }
            };
            let market_cap_usd = Decimal::from_str(&coin.quotes.usd.market_cap.to_string()).ok();
            let volume_24h_usd = parse_decimal_from_f64(coin.quotes.usd.volume_24h);
            let change_percent_24h = parse_decimal_from_f64(coin.quotes.usd.percent_change_24h);

            // Check if asset already exists by symbol or coinpaprika_id
            let existing_asset = assets::Entity::find()
                .filter(
                    assets::Column::Symbol.eq(&coin.symbol.to_uppercase())
                        .or(assets::Column::CoingeckoId.eq(&coin.id))
                )
                .one(db)
                .await
                .map_err(|e| format!("Failed to query assets: {}", e))?;

            let asset_id = match existing_asset {
                Some(existing) => {
                    // Update existing asset
                    let mut asset_update: assets::ActiveModel = existing.into();
                    asset_update.name = ActiveValue::Set(coin.name.clone());
                    asset_update.symbol = ActiveValue::Set(coin.symbol.to_uppercase());
                    asset_update.coingecko_id = ActiveValue::Set(Some(coin.id.clone()));
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
                        symbol: ActiveValue::Set(coin.symbol.to_uppercase()),
                        name: ActiveValue::Set(coin.name.clone()),
                        asset_type: ActiveValue::Set("cryptocurrency".to_string()),
                        coingecko_id: ActiveValue::Set(Some(coin.id.clone())),
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

            // Extended fields from CoinPaprika
            let rank = Some(coin.rank as i32);
            let circulating_supply = coin.circulating_supply.and_then(|v| Decimal::from_str(&v.to_string()).ok());
            let total_supply = coin.total_supply.and_then(|v| Decimal::from_str(&v.to_string()).ok());
            let max_supply = coin.max_supply.and_then(|v| Decimal::from_str(&v.to_string()).ok());
            let beta_value = parse_decimal_from_f64(coin.beta_value);
            
            let percent_change_1h = parse_decimal_from_f64(coin.quotes.usd.percent_change_1h);
            let percent_change_7d = parse_decimal_from_f64(coin.quotes.usd.percent_change_7d);
            let percent_change_30d = parse_decimal_from_f64(coin.quotes.usd.percent_change_30d);
            let ath_price = parse_decimal_from_f64(coin.quotes.usd.ath_price);
            
            let ath_date = coin.quotes.usd.ath_date
                .and_then(|d| chrono::DateTime::parse_from_rfc3339(&d).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc));
            
            let percent_from_price_ath = parse_decimal_from_f64(coin.quotes.usd.percent_from_price_ath);

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
                // Deduplicate to prevent "ON CONFLICT DO UPDATE command cannot affect row a second time" error
                let deduplicated = deduplicate_prices(prices_to_store);
                let count = deduplicated.len();
                match Insert::many(deduplicated)
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
            // Deduplicate to prevent "ON CONFLICT DO UPDATE command cannot affect row a second time" error
            let deduplicated = deduplicate_prices(prices_to_store);
            let count = deduplicated.len();
            match Insert::many(deduplicated)
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
    use chrono::Utc;

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

    #[test]
    fn test_deduplicate_prices_no_duplicates() {
        // Test with no duplicates - should return all prices unchanged
        let asset_id_1 = Uuid::new_v4();
        let asset_id_2 = Uuid::new_v4();
        let timestamp = Utc::now();
        
        let prices = vec![
            asset_prices::ActiveModel {
                id: ActiveValue::Set(Uuid::new_v4()),
                asset_id: ActiveValue::Set(asset_id_1),
                timestamp: ActiveValue::Set(timestamp.into()),
                price_usd: ActiveValue::Set(Decimal::from_str("100.0").unwrap()),
                source: ActiveValue::Set("coinpaprika".to_string()),
                volume_24h_usd: ActiveValue::Set(None),
                market_cap_usd: ActiveValue::Set(None),
                change_percent_24h: ActiveValue::Set(None),
                rank: ActiveValue::Set(None),
                circulating_supply: ActiveValue::Set(None),
                total_supply: ActiveValue::Set(None),
                max_supply: ActiveValue::Set(None),
                beta_value: ActiveValue::Set(None),
                percent_change_1h: ActiveValue::Set(None),
                percent_change_7d: ActiveValue::Set(None),
                percent_change_30d: ActiveValue::Set(None),
                ath_price: ActiveValue::Set(None),
                ath_date: ActiveValue::Set(None),
                percent_from_price_ath: ActiveValue::Set(None),
                created_at: ActiveValue::Set(timestamp.into()),
            },
            asset_prices::ActiveModel {
                id: ActiveValue::Set(Uuid::new_v4()),
                asset_id: ActiveValue::Set(asset_id_2),
                timestamp: ActiveValue::Set(timestamp.into()),
                price_usd: ActiveValue::Set(Decimal::from_str("200.0").unwrap()),
                source: ActiveValue::Set("coinpaprika".to_string()),
                volume_24h_usd: ActiveValue::Set(None),
                market_cap_usd: ActiveValue::Set(None),
                change_percent_24h: ActiveValue::Set(None),
                rank: ActiveValue::Set(None),
                circulating_supply: ActiveValue::Set(None),
                total_supply: ActiveValue::Set(None),
                max_supply: ActiveValue::Set(None),
                beta_value: ActiveValue::Set(None),
                percent_change_1h: ActiveValue::Set(None),
                percent_change_7d: ActiveValue::Set(None),
                percent_change_30d: ActiveValue::Set(None),
                ath_price: ActiveValue::Set(None),
                ath_date: ActiveValue::Set(None),
                percent_from_price_ath: ActiveValue::Set(None),
                created_at: ActiveValue::Set(timestamp.into()),
            },
        ];

        let deduplicated = deduplicate_prices(prices.clone());
        assert_eq!(deduplicated.len(), 2, "Should keep all prices when no duplicates");
    }

    #[test]
    fn test_deduplicate_prices_with_duplicates() {
        // Test with duplicates - should keep only the last occurrence
        let asset_id = Uuid::new_v4();
        let timestamp = Utc::now();
        
        let prices = vec![
            asset_prices::ActiveModel {
                id: ActiveValue::Set(Uuid::new_v4()),
                asset_id: ActiveValue::Set(asset_id),
                timestamp: ActiveValue::Set(timestamp.into()),
                price_usd: ActiveValue::Set(Decimal::from_str("100.0").unwrap()),
                source: ActiveValue::Set("coinpaprika".to_string()),
                volume_24h_usd: ActiveValue::Set(None),
                market_cap_usd: ActiveValue::Set(None),
                change_percent_24h: ActiveValue::Set(None),
                rank: ActiveValue::Set(Some(1)),
                circulating_supply: ActiveValue::Set(None),
                total_supply: ActiveValue::Set(None),
                max_supply: ActiveValue::Set(None),
                beta_value: ActiveValue::Set(None),
                percent_change_1h: ActiveValue::Set(None),
                percent_change_7d: ActiveValue::Set(None),
                percent_change_30d: ActiveValue::Set(None),
                ath_price: ActiveValue::Set(None),
                ath_date: ActiveValue::Set(None),
                percent_from_price_ath: ActiveValue::Set(None),
                created_at: ActiveValue::Set(timestamp.into()),
            },
            asset_prices::ActiveModel {
                id: ActiveValue::Set(Uuid::new_v4()),
                asset_id: ActiveValue::Set(asset_id),
                timestamp: ActiveValue::Set(timestamp.into()),
                price_usd: ActiveValue::Set(Decimal::from_str("200.0").unwrap()),
                source: ActiveValue::Set("coinpaprika".to_string()),
                volume_24h_usd: ActiveValue::Set(None),
                market_cap_usd: ActiveValue::Set(None),
                change_percent_24h: ActiveValue::Set(None),
                rank: ActiveValue::Set(Some(2)),
                circulating_supply: ActiveValue::Set(None),
                total_supply: ActiveValue::Set(None),
                max_supply: ActiveValue::Set(None),
                beta_value: ActiveValue::Set(None),
                percent_change_1h: ActiveValue::Set(None),
                percent_change_7d: ActiveValue::Set(None),
                percent_change_30d: ActiveValue::Set(None),
                ath_price: ActiveValue::Set(None),
                ath_date: ActiveValue::Set(None),
                percent_from_price_ath: ActiveValue::Set(None),
                created_at: ActiveValue::Set(timestamp.into()),
            },
        ];

        let deduplicated = deduplicate_prices(prices);
        
        // Should have only 1 price left (last one wins)
        assert_eq!(deduplicated.len(), 1, "Should deduplicate to 1 price");
        
        // Verify it's the last price (rank=2, price=200)
        let remaining_price = &deduplicated[0];
        match &remaining_price.price_usd {
            ActiveValue::Set(price) => {
                assert_eq!(price, &Decimal::from_str("200.0").unwrap(), "Should keep the last price");
            }
            _ => panic!("Price should be set"),
        }
        match &remaining_price.rank {
            ActiveValue::Set(rank) => {
                assert_eq!(rank, &Some(2), "Should keep the last rank value");
            }
            _ => panic!("Rank should be set"),
        }
    }

    #[test]
    fn test_deduplicate_prices_different_sources() {
        // Test that different sources are NOT deduplicated
        let asset_id = Uuid::new_v4();
        let timestamp = Utc::now();
        
        let prices = vec![
            asset_prices::ActiveModel {
                id: ActiveValue::Set(Uuid::new_v4()),
                asset_id: ActiveValue::Set(asset_id),
                timestamp: ActiveValue::Set(timestamp.into()),
                price_usd: ActiveValue::Set(Decimal::from_str("100.0").unwrap()),
                source: ActiveValue::Set("coinpaprika".to_string()),
                volume_24h_usd: ActiveValue::Set(None),
                market_cap_usd: ActiveValue::Set(None),
                change_percent_24h: ActiveValue::Set(None),
                rank: ActiveValue::Set(None),
                circulating_supply: ActiveValue::Set(None),
                total_supply: ActiveValue::Set(None),
                max_supply: ActiveValue::Set(None),
                beta_value: ActiveValue::Set(None),
                percent_change_1h: ActiveValue::Set(None),
                percent_change_7d: ActiveValue::Set(None),
                percent_change_30d: ActiveValue::Set(None),
                ath_price: ActiveValue::Set(None),
                ath_date: ActiveValue::Set(None),
                percent_from_price_ath: ActiveValue::Set(None),
                created_at: ActiveValue::Set(timestamp.into()),
            },
            asset_prices::ActiveModel {
                id: ActiveValue::Set(Uuid::new_v4()),
                asset_id: ActiveValue::Set(asset_id),
                timestamp: ActiveValue::Set(timestamp.into()),
                price_usd: ActiveValue::Set(Decimal::from_str("200.0").unwrap()),
                source: ActiveValue::Set("coingecko".to_string()),
                volume_24h_usd: ActiveValue::Set(None),
                market_cap_usd: ActiveValue::Set(None),
                change_percent_24h: ActiveValue::Set(None),
                rank: ActiveValue::Set(None),
                circulating_supply: ActiveValue::Set(None),
                total_supply: ActiveValue::Set(None),
                max_supply: ActiveValue::Set(None),
                beta_value: ActiveValue::Set(None),
                percent_change_1h: ActiveValue::Set(None),
                percent_change_7d: ActiveValue::Set(None),
                percent_change_30d: ActiveValue::Set(None),
                ath_price: ActiveValue::Set(None),
                ath_date: ActiveValue::Set(None),
                percent_from_price_ath: ActiveValue::Set(None),
                created_at: ActiveValue::Set(timestamp.into()),
            },
        ];

        let deduplicated = deduplicate_prices(prices);
        
        // Both should remain as they have different sources
        assert_eq!(deduplicated.len(), 2, "Should keep prices with different sources");
    }

    #[test]
    fn test_deduplicate_prices_different_timestamps() {
        // Test that different timestamps are NOT deduplicated
        let asset_id = Uuid::new_v4();
        let timestamp1 = Utc::now();
        let timestamp2 = timestamp1 + chrono::Duration::seconds(1);
        
        let prices = vec![
            asset_prices::ActiveModel {
                id: ActiveValue::Set(Uuid::new_v4()),
                asset_id: ActiveValue::Set(asset_id),
                timestamp: ActiveValue::Set(timestamp1.into()),
                price_usd: ActiveValue::Set(Decimal::from_str("100.0").unwrap()),
                source: ActiveValue::Set("coinpaprika".to_string()),
                volume_24h_usd: ActiveValue::Set(None),
                market_cap_usd: ActiveValue::Set(None),
                change_percent_24h: ActiveValue::Set(None),
                rank: ActiveValue::Set(None),
                circulating_supply: ActiveValue::Set(None),
                total_supply: ActiveValue::Set(None),
                max_supply: ActiveValue::Set(None),
                beta_value: ActiveValue::Set(None),
                percent_change_1h: ActiveValue::Set(None),
                percent_change_7d: ActiveValue::Set(None),
                percent_change_30d: ActiveValue::Set(None),
                ath_price: ActiveValue::Set(None),
                ath_date: ActiveValue::Set(None),
                percent_from_price_ath: ActiveValue::Set(None),
                created_at: ActiveValue::Set(timestamp1.into()),
            },
            asset_prices::ActiveModel {
                id: ActiveValue::Set(Uuid::new_v4()),
                asset_id: ActiveValue::Set(asset_id),
                timestamp: ActiveValue::Set(timestamp2.into()),
                price_usd: ActiveValue::Set(Decimal::from_str("200.0").unwrap()),
                source: ActiveValue::Set("coinpaprika".to_string()),
                volume_24h_usd: ActiveValue::Set(None),
                market_cap_usd: ActiveValue::Set(None),
                change_percent_24h: ActiveValue::Set(None),
                rank: ActiveValue::Set(None),
                circulating_supply: ActiveValue::Set(None),
                total_supply: ActiveValue::Set(None),
                max_supply: ActiveValue::Set(None),
                beta_value: ActiveValue::Set(None),
                percent_change_1h: ActiveValue::Set(None),
                percent_change_7d: ActiveValue::Set(None),
                percent_change_30d: ActiveValue::Set(None),
                ath_price: ActiveValue::Set(None),
                ath_date: ActiveValue::Set(None),
                percent_from_price_ath: ActiveValue::Set(None),
                created_at: ActiveValue::Set(timestamp2.into()),
            },
        ];

        let deduplicated = deduplicate_prices(prices);
        
        // Both should remain as they have different timestamps
        assert_eq!(deduplicated.len(), 2, "Should keep prices with different timestamps");
    }

    #[test]
    fn test_deduplicate_prices_multiple_duplicates() {
        // Test with multiple duplicate groups
        let asset_id_1 = Uuid::new_v4();
        let asset_id_2 = Uuid::new_v4();
        let timestamp = Utc::now();
        
        let prices = vec![
            // First duplicate group (asset_id_1)
            asset_prices::ActiveModel {
                id: ActiveValue::Set(Uuid::new_v4()),
                asset_id: ActiveValue::Set(asset_id_1),
                timestamp: ActiveValue::Set(timestamp.into()),
                price_usd: ActiveValue::Set(Decimal::from_str("100.0").unwrap()),
                source: ActiveValue::Set("coinpaprika".to_string()),
                volume_24h_usd: ActiveValue::Set(None),
                market_cap_usd: ActiveValue::Set(None),
                change_percent_24h: ActiveValue::Set(None),
                rank: ActiveValue::Set(Some(1)),
                circulating_supply: ActiveValue::Set(None),
                total_supply: ActiveValue::Set(None),
                max_supply: ActiveValue::Set(None),
                beta_value: ActiveValue::Set(None),
                percent_change_1h: ActiveValue::Set(None),
                percent_change_7d: ActiveValue::Set(None),
                percent_change_30d: ActiveValue::Set(None),
                ath_price: ActiveValue::Set(None),
                ath_date: ActiveValue::Set(None),
                percent_from_price_ath: ActiveValue::Set(None),
                created_at: ActiveValue::Set(timestamp.into()),
            },
            asset_prices::ActiveModel {
                id: ActiveValue::Set(Uuid::new_v4()),
                asset_id: ActiveValue::Set(asset_id_1),
                timestamp: ActiveValue::Set(timestamp.into()),
                price_usd: ActiveValue::Set(Decimal::from_str("150.0").unwrap()),
                source: ActiveValue::Set("coinpaprika".to_string()),
                volume_24h_usd: ActiveValue::Set(None),
                market_cap_usd: ActiveValue::Set(None),
                change_percent_24h: ActiveValue::Set(None),
                rank: ActiveValue::Set(Some(2)),
                circulating_supply: ActiveValue::Set(None),
                total_supply: ActiveValue::Set(None),
                max_supply: ActiveValue::Set(None),
                beta_value: ActiveValue::Set(None),
                percent_change_1h: ActiveValue::Set(None),
                percent_change_7d: ActiveValue::Set(None),
                percent_change_30d: ActiveValue::Set(None),
                ath_price: ActiveValue::Set(None),
                ath_date: ActiveValue::Set(None),
                percent_from_price_ath: ActiveValue::Set(None),
                created_at: ActiveValue::Set(timestamp.into()),
            },
            // Second duplicate group (asset_id_2)
            asset_prices::ActiveModel {
                id: ActiveValue::Set(Uuid::new_v4()),
                asset_id: ActiveValue::Set(asset_id_2),
                timestamp: ActiveValue::Set(timestamp.into()),
                price_usd: ActiveValue::Set(Decimal::from_str("300.0").unwrap()),
                source: ActiveValue::Set("coinpaprika".to_string()),
                volume_24h_usd: ActiveValue::Set(None),
                market_cap_usd: ActiveValue::Set(None),
                change_percent_24h: ActiveValue::Set(None),
                rank: ActiveValue::Set(Some(3)),
                circulating_supply: ActiveValue::Set(None),
                total_supply: ActiveValue::Set(None),
                max_supply: ActiveValue::Set(None),
                beta_value: ActiveValue::Set(None),
                percent_change_1h: ActiveValue::Set(None),
                percent_change_7d: ActiveValue::Set(None),
                percent_change_30d: ActiveValue::Set(None),
                ath_price: ActiveValue::Set(None),
                ath_date: ActiveValue::Set(None),
                percent_from_price_ath: ActiveValue::Set(None),
                created_at: ActiveValue::Set(timestamp.into()),
            },
            asset_prices::ActiveModel {
                id: ActiveValue::Set(Uuid::new_v4()),
                asset_id: ActiveValue::Set(asset_id_2),
                timestamp: ActiveValue::Set(timestamp.into()),
                price_usd: ActiveValue::Set(Decimal::from_str("350.0").unwrap()),
                source: ActiveValue::Set("coinpaprika".to_string()),
                volume_24h_usd: ActiveValue::Set(None),
                market_cap_usd: ActiveValue::Set(None),
                change_percent_24h: ActiveValue::Set(None),
                rank: ActiveValue::Set(Some(4)),
                circulating_supply: ActiveValue::Set(None),
                total_supply: ActiveValue::Set(None),
                max_supply: ActiveValue::Set(None),
                beta_value: ActiveValue::Set(None),
                percent_change_1h: ActiveValue::Set(None),
                percent_change_7d: ActiveValue::Set(None),
                percent_change_30d: ActiveValue::Set(None),
                ath_price: ActiveValue::Set(None),
                ath_date: ActiveValue::Set(None),
                percent_from_price_ath: ActiveValue::Set(None),
                created_at: ActiveValue::Set(timestamp.into()),
            },
        ];

        let deduplicated = deduplicate_prices(prices);
        
        // Should have 2 prices (one for each asset_id)
        assert_eq!(deduplicated.len(), 2, "Should deduplicate to 2 prices");
        
        // Check that we have one price for each asset
        let mut found_asset_1 = false;
        let mut found_asset_2 = false;
        
        for price in &deduplicated {
            match &price.asset_id {
                ActiveValue::Set(id) if *id == asset_id_1 => {
                    found_asset_1 = true;
                    // Should be the last one (price=150, rank=2)
                    match &price.price_usd {
                        ActiveValue::Set(p) => assert_eq!(p, &Decimal::from_str("150.0").unwrap()),
                        _ => panic!("Price should be set"),
                    }
                    match &price.rank {
                        ActiveValue::Set(r) => assert_eq!(r, &Some(2)),
                        _ => panic!("Rank should be set"),
                    }
                }
                ActiveValue::Set(id) if *id == asset_id_2 => {
                    found_asset_2 = true;
                    // Should be the last one (price=350, rank=4)
                    match &price.price_usd {
                        ActiveValue::Set(p) => assert_eq!(p, &Decimal::from_str("350.0").unwrap()),
                        _ => panic!("Price should be set"),
                    }
                    match &price.rank {
                        ActiveValue::Set(r) => assert_eq!(r, &Some(4)),
                        _ => panic!("Rank should be set"),
                    }
                }
                _ => {}
            }
        }
        
        assert!(found_asset_1, "Should have price for asset_id_1");
        assert!(found_asset_2, "Should have price for asset_id_2");
    }

    #[test]
    fn test_deduplicate_prices_empty_vec() {
        let prices: Vec<asset_prices::ActiveModel> = vec![];
        let deduplicated = deduplicate_prices(prices);
        assert_eq!(deduplicated.len(), 0, "Empty vector should remain empty");
    }

    #[test]
    fn test_parse_decimal_from_f64_valid() {
        let value = Some(123.45);
        let result = parse_decimal_from_f64(value);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), Decimal::from_str("123.45").unwrap());
    }

    #[test]
    fn test_parse_decimal_from_f64_none() {
        let value: Option<f64> = None;
        let result = parse_decimal_from_f64(value);
        assert!(result.is_none());
    }

    /// Integration test: Test batch insert with duplicates
    /// This test simulates the actual scenario that would cause the error
    #[test]
    fn test_batch_insert_deduplication_scenario() {
        // Simulate a scenario where the same coin appears multiple times in the API response
        // This would create duplicate (asset_id, timestamp, source) entries in the batch
        
        let asset_id = Uuid::new_v4();
        let timestamp = Utc::now();
        let source = "coinpaprika";
        
        // Create a batch with 3 prices, where 2 are duplicates
        let mut prices_batch = Vec::new();
        
        // First entry for BTC
        prices_batch.push(asset_prices::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            asset_id: ActiveValue::Set(asset_id),
            timestamp: ActiveValue::Set(timestamp.into()),
            price_usd: ActiveValue::Set(Decimal::from_str("50000.0").unwrap()),
            source: ActiveValue::Set(source.to_string()),
            volume_24h_usd: ActiveValue::Set(Some(Decimal::from_str("1000000.0").unwrap())),
            market_cap_usd: ActiveValue::Set(Some(Decimal::from_str("1000000000.0").unwrap())),
            change_percent_24h: ActiveValue::Set(Some(Decimal::from_str("2.5").unwrap())),
            rank: ActiveValue::Set(Some(1)),
            circulating_supply: ActiveValue::Set(Some(Decimal::from_str("19000000.0").unwrap())),
            total_supply: ActiveValue::Set(Some(Decimal::from_str("21000000.0").unwrap())),
            max_supply: ActiveValue::Set(Some(Decimal::from_str("21000000.0").unwrap())),
            beta_value: ActiveValue::Set(Some(Decimal::from_str("1.2").unwrap())),
            percent_change_1h: ActiveValue::Set(Some(Decimal::from_str("0.5").unwrap())),
            percent_change_7d: ActiveValue::Set(Some(Decimal::from_str("5.0").unwrap())),
            percent_change_30d: ActiveValue::Set(Some(Decimal::from_str("10.0").unwrap())),
            ath_price: ActiveValue::Set(Some(Decimal::from_str("69000.0").unwrap())),
            ath_date: ActiveValue::Set(None),
            percent_from_price_ath: ActiveValue::Set(Some(Decimal::from_str("-27.5").unwrap())),
            created_at: ActiveValue::Set(timestamp.into()),
        });
        
        // Duplicate entry for BTC (simulating API returning same coin twice)
        prices_batch.push(asset_prices::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            asset_id: ActiveValue::Set(asset_id),
            timestamp: ActiveValue::Set(timestamp.into()),
            price_usd: ActiveValue::Set(Decimal::from_str("50100.0").unwrap()), // Different price
            source: ActiveValue::Set(source.to_string()),
            volume_24h_usd: ActiveValue::Set(Some(Decimal::from_str("1100000.0").unwrap())),
            market_cap_usd: ActiveValue::Set(Some(Decimal::from_str("1010000000.0").unwrap())),
            change_percent_24h: ActiveValue::Set(Some(Decimal::from_str("2.7").unwrap())),
            rank: ActiveValue::Set(Some(1)),
            circulating_supply: ActiveValue::Set(Some(Decimal::from_str("19000000.0").unwrap())),
            total_supply: ActiveValue::Set(Some(Decimal::from_str("21000000.0").unwrap())),
            max_supply: ActiveValue::Set(Some(Decimal::from_str("21000000.0").unwrap())),
            beta_value: ActiveValue::Set(Some(Decimal::from_str("1.2").unwrap())),
            percent_change_1h: ActiveValue::Set(Some(Decimal::from_str("0.6").unwrap())),
            percent_change_7d: ActiveValue::Set(Some(Decimal::from_str("5.2").unwrap())),
            percent_change_30d: ActiveValue::Set(Some(Decimal::from_str("10.5").unwrap())),
            ath_price: ActiveValue::Set(Some(Decimal::from_str("69000.0").unwrap())),
            ath_date: ActiveValue::Set(None),
            percent_from_price_ath: ActiveValue::Set(Some(Decimal::from_str("-27.4").unwrap())),
            created_at: ActiveValue::Set(timestamp.into()),
        });
        
        // Different asset to ensure we don't over-deduplicate
        let asset_id_2 = Uuid::new_v4();
        prices_batch.push(asset_prices::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            asset_id: ActiveValue::Set(asset_id_2),
            timestamp: ActiveValue::Set(timestamp.into()),
            price_usd: ActiveValue::Set(Decimal::from_str("3000.0").unwrap()),
            source: ActiveValue::Set(source.to_string()),
            volume_24h_usd: ActiveValue::Set(Some(Decimal::from_str("500000.0").unwrap())),
            market_cap_usd: ActiveValue::Set(Some(Decimal::from_str("500000000.0").unwrap())),
            change_percent_24h: ActiveValue::Set(Some(Decimal::from_str("1.5").unwrap())),
            rank: ActiveValue::Set(Some(2)),
            circulating_supply: ActiveValue::Set(None),
            total_supply: ActiveValue::Set(None),
            max_supply: ActiveValue::Set(None),
            beta_value: ActiveValue::Set(None),
            percent_change_1h: ActiveValue::Set(Some(Decimal::from_str("0.3").unwrap())),
            percent_change_7d: ActiveValue::Set(Some(Decimal::from_str("3.0").unwrap())),
            percent_change_30d: ActiveValue::Set(Some(Decimal::from_str("8.0").unwrap())),
            ath_price: ActiveValue::Set(Some(Decimal::from_str("4800.0").unwrap())),
            ath_date: ActiveValue::Set(None),
            percent_from_price_ath: ActiveValue::Set(Some(Decimal::from_str("-37.5").unwrap())),
            created_at: ActiveValue::Set(timestamp.into()),
        });
        
        // Before deduplication: 3 prices
        assert_eq!(prices_batch.len(), 3, "Should start with 3 prices in batch");
        
        // After deduplication: should have 2 prices (duplicates merged, keeping last)
        let deduplicated = deduplicate_prices(prices_batch);
        assert_eq!(deduplicated.len(), 2, "Should have 2 prices after deduplication");
        
        // Verify the duplicate was removed and we kept the last one
        let btc_price = deduplicated.iter().find(|p| {
            matches!(&p.asset_id, ActiveValue::Set(id) if *id == asset_id)
        }).expect("Should find BTC price");
        
        // Should keep the last duplicate's values
        match &btc_price.price_usd {
            ActiveValue::Set(price) => {
                assert_eq!(price, &Decimal::from_str("50100.0").unwrap(), 
                    "Should keep the last duplicate's price");
            }
            _ => panic!("Price should be set"),
        }
        
        // Verify the other asset is still present
        let eth_price = deduplicated.iter().find(|p| {
            matches!(&p.asset_id, ActiveValue::Set(id) if *id == asset_id_2)
        }).expect("Should find ETH price");
        
        match &eth_price.price_usd {
            ActiveValue::Set(price) => {
                assert_eq!(price, &Decimal::from_str("3000.0").unwrap());
            }
            _ => panic!("Price should be set"),
        }
    }

    /// Test that demonstrates the fix prevents the database error
    #[test]
    fn test_large_batch_with_scattered_duplicates() {
        // Simulate a more realistic scenario with 10 prices where some are duplicates
        let timestamp = Utc::now();
        let source = "coinpaprika";
        
        let asset_ids: Vec<Uuid> = (0..5).map(|_| Uuid::new_v4()).collect();
        let mut prices_batch = Vec::new();
        
        // Add 10 prices with some duplicates scattered throughout
        for i in 0..10 {
            let asset_idx = i % 5; // This creates duplicates
            prices_batch.push(asset_prices::ActiveModel {
                id: ActiveValue::Set(Uuid::new_v4()),
                asset_id: ActiveValue::Set(asset_ids[asset_idx]),
                timestamp: ActiveValue::Set(timestamp.into()),
                price_usd: ActiveValue::Set(Decimal::from_str(&format!("{}.0", 100 + i)).unwrap()),
                source: ActiveValue::Set(source.to_string()),
                volume_24h_usd: ActiveValue::Set(None),
                market_cap_usd: ActiveValue::Set(None),
                change_percent_24h: ActiveValue::Set(None),
                rank: ActiveValue::Set(Some(i as i32)),
                circulating_supply: ActiveValue::Set(None),
                total_supply: ActiveValue::Set(None),
                max_supply: ActiveValue::Set(None),
                beta_value: ActiveValue::Set(None),
                percent_change_1h: ActiveValue::Set(None),
                percent_change_7d: ActiveValue::Set(None),
                percent_change_30d: ActiveValue::Set(None),
                ath_price: ActiveValue::Set(None),
                ath_date: ActiveValue::Set(None),
                percent_from_price_ath: ActiveValue::Set(None),
                created_at: ActiveValue::Set(timestamp.into()),
            });
        }
        
        // Should have 10 prices before deduplication
        assert_eq!(prices_batch.len(), 10, "Should start with 10 prices");
        
        // After deduplication: should have 5 unique prices (one per asset)
        let deduplicated = deduplicate_prices(prices_batch);
        assert_eq!(deduplicated.len(), 5, "Should have 5 unique prices after deduplication");
        
        // Verify each asset appears exactly once
        let mut asset_counts: std::collections::HashMap<Uuid, usize> = std::collections::HashMap::new();
        for price in &deduplicated {
            if let ActiveValue::Set(asset_id) = &price.asset_id {
                *asset_counts.entry(*asset_id).or_insert(0) += 1;
            }
        }
        
        for (asset_id, count) in asset_counts.iter() {
            assert_eq!(*count, 1, "Each asset should appear exactly once, but {:?} appears {} times", 
                asset_id, count);
        }
        
        // Verify we kept the last values (highest rank for each asset)
        // Asset 0 should have rank 5 (indices 0, 5), Asset 1 should have rank 6, etc.
        for (idx, asset_id) in asset_ids.iter().enumerate() {
            let price = deduplicated.iter().find(|p| {
                matches!(&p.asset_id, ActiveValue::Set(id) if id == asset_id)
            }).expect("Should find price for asset");
            
            let expected_rank = 5 + idx as i32;
            match &price.rank {
                ActiveValue::Set(Some(rank)) => {
                    assert_eq!(*rank, expected_rank, 
                        "Asset {} should have rank {} (last occurrence)", idx, expected_rank);
                }
                _ => panic!("Rank should be set"),
            }
        }
    }
}
