use crate::connectors::coingecko::CoinGeckoConnector;
use crate::entities::{assets, asset_rankings};
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

/// Result of collecting top coins
#[derive(Debug)]
pub struct CollectionResult {
    pub success: bool,
    pub coins_collected: usize,
    pub assets_created: usize,
    pub assets_updated: usize,
    pub rankings_created: usize,
    pub error: Option<String>,
}

/// Collect top N coins from CoinGecko and store in database
/// 
/// This function:
/// 1. Fetches top N coins by market cap from CoinGecko
/// 2. Upserts asset metadata (creates new or updates existing)
/// 3. Creates ranking snapshots for the current date (idempotent via DB constraints)
/// 
/// # Arguments
/// * `db` - Database connection
/// * `limit` - Number of top coins to collect (default: 100)
/// 
/// # Returns
/// CollectionResult with statistics about the operation
pub async fn collect_top_coins(
    db: &DatabaseConnection,
    limit: usize,
) -> Result<CollectionResult, Box<dyn Error + Send + Sync>> {
    let runner = JobRunner::new(format!("top_coins_collection(limit={})", limit));

    let result = runner.execute(|| async {
        // Create CoinGecko connector
        let connector = CoinGeckoConnector::new();

        // Fetch top coins
        let coins = connector.fetch_top_coins(limit).await
            .map_err(|e| format!("Failed to fetch coins: {}", e))?;

        let coins_collected = coins.len();
        let mut assets_created = 0;
        let mut assets_updated = 0;
        let mut rankings_created = 0;

        let snapshot_date = Utc::now().date_naive();
        let source = "coingecko";

        for coin in coins {
            // Skip if no market cap rank
            let rank = match coin.market_cap_rank {
                Some(r) => r as i32,
                None => {
                    tracing::warn!("Skipping coin {} - no market cap rank", coin.id);
                    continue;
                }
            };

            // Check if asset already exists by symbol or coingecko_id
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
                    asset_update.logo_url = ActiveValue::Set(Some(coin.image.clone()));
                    asset_update.is_active = ActiveValue::Set(true);
                    asset_update.updated_at = ActiveValue::Set(Utc::now().into());
                    
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
                        logo_url: ActiveValue::Set(Some(coin.image.clone())),
                        description: ActiveValue::NotSet,
                        decimals: ActiveValue::NotSet,
                        is_active: ActiveValue::Set(true),
                        created_at: ActiveValue::Set(Utc::now().into()),
                        updated_at: ActiveValue::Set(Utc::now().into()),
                    };
                    
                    let inserted = new_asset.insert(db).await
                        .map_err(|e| format!("Failed to insert asset: {}", e))?;
                    assets_created += 1;
                    tracing::debug!("Created asset: {} ({})", inserted.symbol, inserted.id);
                    inserted.id
                }
            };

            // Upsert ranking using ON CONFLICT (idempotent)
            // The unique constraint on (asset_id, snapshot_date, source) ensures idempotency
            let market_cap_usd = Decimal::from_str(&coin.market_cap.to_string())
                .unwrap_or_else(|_| Decimal::ZERO);
            let price_usd = Decimal::from_str(&coin.current_price.to_string())
                .unwrap_or_else(|_| Decimal::ZERO);
            let volume_24h_usd = coin.total_volume
                .and_then(|v| Decimal::from_str(&v.to_string()).ok());
            let change_percent_24h = coin.price_change_percentage_24h
                .and_then(|v| Decimal::from_str(&v.to_string()).ok());
            let dominance = None;

            let new_ranking = asset_rankings::ActiveModel {
                id: ActiveValue::Set(Uuid::new_v4()),
                asset_id: ActiveValue::Set(asset_id),
                snapshot_date: ActiveValue::Set(snapshot_date),
                rank: ActiveValue::Set(rank),
                market_cap_usd: ActiveValue::Set(market_cap_usd),
                price_usd: ActiveValue::Set(price_usd),
                volume_24h_usd: ActiveValue::Set(volume_24h_usd),
                change_percent_24h: ActiveValue::Set(change_percent_24h),
                dominance: ActiveValue::Set(dominance),
                source: ActiveValue::Set(source.to_string()),
                created_at: ActiveValue::Set(Utc::now().into()),
            };

            // Use ON CONFLICT to handle duplicates (idempotent upsert)
            let _insert_result = Insert::one(new_ranking)
                .on_conflict(
                    OnConflict::columns([
                        asset_rankings::Column::AssetId,
                        asset_rankings::Column::SnapshotDate,
                        asset_rankings::Column::Source,
                    ])
                    .update_columns([
                        asset_rankings::Column::Rank,
                        asset_rankings::Column::MarketCapUsd,
                        asset_rankings::Column::PriceUsd,
                        asset_rankings::Column::Volume24hUsd,
                        asset_rankings::Column::ChangePercent24h,
                        asset_rankings::Column::Dominance,
                    ])
                    .to_owned(),
                )
                .exec(db)
                .await
                .map_err(|e| format!("Failed to upsert ranking: {}", e))?;

            rankings_created += 1;
            tracing::debug!(
                "Upserted ranking: {} (rank {}) on {}",
                coin.symbol,
                rank,
                snapshot_date
            );
        }

        Ok(JobMetrics {
            items_processed: coins_collected,
            items_created: assets_created + rankings_created,
            items_updated: assets_updated,
            items_skipped: 0,
            custom: serde_json::json!({
                "coins_collected": coins_collected,
                "assets_created": assets_created,
                "assets_updated": assets_updated,
                "rankings_created": rankings_created,
            }),
        })
    }).await;

    // Convert JobResult to CollectionResult for backwards compatibility
    Ok(CollectionResult {
        success: result.success,
        coins_collected: result.metrics.custom["coins_collected"].as_u64().unwrap_or(0) as usize,
        assets_created: result.metrics.items_created,
        assets_updated: result.metrics.custom["assets_updated"].as_u64().unwrap_or(0) as usize,
        rankings_created: result.metrics.custom["rankings_created"].as_u64().unwrap_or(0) as usize,
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
            coins_collected: 100,
            assets_created: 50,
            assets_updated: 50,
            rankings_created: 100,
            error: None,
        };
        assert!(result.success);
        assert_eq!(result.coins_collected, 100);
    }
}
