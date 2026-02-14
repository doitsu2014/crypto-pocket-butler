use crate::connectors::coingecko::CoinGeckoConnector;
use crate::entities::{assets, asset_contracts};
use crate::jobs::runner::{JobRunner, JobMetrics};
use chrono::Utc;
use sea_orm::{
    ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QuerySelect, sea_query::OnConflict, Insert,
};
use std::error::Error;
use tracing;
use uuid::Uuid;

/// Result of collecting contract addresses
#[derive(Debug)]
pub struct CollectionResult {
    pub success: bool,
    pub assets_processed: usize,
    pub contracts_created: usize,
    pub contracts_updated: usize,
    pub assets_skipped: usize,
    pub error: Option<String>,
}

/// Collect contract addresses for assets from CoinGecko and store in database
/// 
/// This function:
/// 1. Fetches all active assets that have a coingecko_id
/// 2. For each asset, fetches detailed coin info from CoinGecko
/// 3. Extracts contract addresses from platforms field
/// 4. Upserts contract addresses into asset_contracts table (idempotent via DB constraints)
/// 
/// # Arguments
/// * `db` - Database connection
/// * `limit` - Optional limit on number of assets to process (for testing/rate limiting)
/// 
/// # Returns
/// CollectionResult with statistics about the operation
pub async fn collect_contract_addresses(
    db: &DatabaseConnection,
    limit: Option<usize>,
) -> Result<CollectionResult, Box<dyn Error + Send + Sync>> {
    let runner = JobRunner::new(format!("contract_addresses_collection(limit={:?})", limit));

    let result = runner.execute(|| async {
        // Create CoinGecko connector
        let connector = CoinGeckoConnector::new();

        // Fetch all active assets with coingecko_id
        let mut query = assets::Entity::find()
            .filter(assets::Column::IsActive.eq(true))
            .filter(assets::Column::CoingeckoId.is_not_null());
        
        if let Some(limit_val) = limit {
            query = query.limit(limit_val as u64);
        }
        
        let assets_list = query.all(db).await
            .map_err(|e| format!("Failed to query assets: {}", e))?;
        
        if assets_list.is_empty() {
            return Ok(JobMetrics {
                items_processed: 0,
                items_created: 0,
                items_updated: 0,
                items_skipped: 0,
                custom: serde_json::json!({
                    "assets_processed": 0,
                    "contracts_created": 0,
                    "contracts_updated": 0,
                    "assets_skipped": 0,
                }),
            });
        }

        tracing::info!("Found {} assets to process for contract addresses", assets_list.len());

        let mut assets_processed = 0;
        let mut contracts_created = 0;
        let mut assets_skipped = 0;

        for asset in assets_list {
            let coingecko_id = match &asset.coingecko_id {
                Some(id) => id,
                None => {
                    assets_skipped += 1;
                    continue;
                }
            };

            // Fetch coin detail from CoinGecko
            let coin_detail = match connector.fetch_coin_detail(coingecko_id).await {
                Ok(detail) => detail,
                Err(e) => {
                    tracing::warn!(
                        "Failed to fetch coin detail for asset {} ({}): {}",
                        asset.symbol,
                        coingecko_id,
                        e
                    );
                    assets_skipped += 1;
                    continue;
                }
            };

            // Process each platform/contract address
            for (platform, contract_address) in coin_detail.platforms {
                // Skip empty contract addresses
                if contract_address.is_empty() {
                    continue;
                }

                // Normalize platform name to chain identifier
                let chain = normalize_platform_name(&platform);

                // Upsert contract using ON CONFLICT (idempotent)
                // The unique constraint on (chain, contract_address) ensures idempotency
                let new_contract = asset_contracts::ActiveModel {
                    id: ActiveValue::Set(Uuid::new_v4()),
                    asset_id: ActiveValue::Set(asset.id),
                    chain: ActiveValue::Set(chain.clone()),
                    contract_address: ActiveValue::Set(contract_address.clone()),
                    token_standard: ActiveValue::Set(infer_token_standard(&chain)),
                    decimals: ActiveValue::Set(asset.decimals),
                    is_verified: ActiveValue::Set(true),
                    created_at: ActiveValue::Set(Utc::now().into()),
                    updated_at: ActiveValue::Set(Utc::now().into()),
                };

                match Insert::one(new_contract)
                    .on_conflict(
                        OnConflict::columns([
                            asset_contracts::Column::Chain,
                            asset_contracts::Column::ContractAddress,
                        ])
                        .update_columns([
                            asset_contracts::Column::AssetId,
                            asset_contracts::Column::TokenStandard,
                            asset_contracts::Column::Decimals,
                            asset_contracts::Column::IsVerified,
                            asset_contracts::Column::UpdatedAt,
                        ])
                        .to_owned(),
                    )
                    .exec(db)
                    .await
                {
                    Ok(_) => {
                        contracts_created += 1;
                        tracing::debug!(
                            "Upserted contract for {} on {} at {}",
                            asset.symbol,
                            chain,
                            contract_address
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to upsert contract for {} on {}: {}",
                            asset.symbol,
                            chain,
                            e
                        );
                    }
                }
            }

            assets_processed += 1;

            // Add a small delay to respect rate limits (CoinGecko free tier has rate limits)
            // Sleep for 1.5 seconds to stay under 40 calls/minute (60,000ms / 1,500ms = 40)
            tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
        }

        Ok(JobMetrics {
            items_processed: assets_processed,
            items_created: contracts_created,
            items_updated: 0, // Upserts not tracked separately for simplicity
            items_skipped: assets_skipped,
            custom: serde_json::json!({
                "assets_processed": assets_processed,
                "contracts_created": contracts_created,
                "assets_skipped": assets_skipped,
            }),
        })
    }).await;

    // Convert JobResult to CollectionResult for backwards compatibility
    Ok(CollectionResult {
        success: result.success,
        assets_processed: result.metrics.items_processed,
        contracts_created: result.metrics.items_created,
        contracts_updated: 0, // Upserts not tracked separately for simplicity
        assets_skipped: result.metrics.items_skipped,
        error: result.error,
    })
}

/// Normalize CoinGecko platform names to our chain identifiers
fn normalize_platform_name(platform: &str) -> String {
    match platform {
        "ethereum" => "ethereum".to_string(),
        "binance-smart-chain" => "bsc".to_string(),
        "polygon-pos" => "polygon".to_string(),
        "arbitrum-one" => "arbitrum".to_string(),
        "optimistic-ethereum" => "optimism".to_string(),
        "avalanche" => "avalanche".to_string(),
        "fantom" => "fantom".to_string(),
        "base" => "base".to_string(),
        "solana" => "solana".to_string(),
        // Add more mappings as needed
        _ => platform.to_string(),
    }
}

/// Infer token standard from chain
fn infer_token_standard(chain: &str) -> Option<String> {
    match chain {
        "ethereum" | "arbitrum" | "optimism" | "base" => Some("ERC20".to_string()),
        "bsc" => Some("BEP20".to_string()),
        "polygon" => Some("ERC20".to_string()), // Polygon is ERC20-compatible
        "solana" => Some("SPL".to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_platform_name() {
        assert_eq!(normalize_platform_name("ethereum"), "ethereum");
        assert_eq!(normalize_platform_name("binance-smart-chain"), "bsc");
        assert_eq!(normalize_platform_name("polygon-pos"), "polygon");
        assert_eq!(normalize_platform_name("arbitrum-one"), "arbitrum");
        assert_eq!(normalize_platform_name("optimistic-ethereum"), "optimism");
        assert_eq!(normalize_platform_name("unknown-chain"), "unknown-chain");
    }

    #[test]
    fn test_infer_token_standard() {
        assert_eq!(infer_token_standard("ethereum"), Some("ERC20".to_string()));
        assert_eq!(infer_token_standard("bsc"), Some("BEP20".to_string()));
        assert_eq!(infer_token_standard("polygon"), Some("ERC20".to_string()));
        assert_eq!(infer_token_standard("arbitrum"), Some("ERC20".to_string()));
        assert_eq!(infer_token_standard("solana"), Some("SPL".to_string()));
        assert_eq!(infer_token_standard("unknown-chain"), None);
    }
}
