use crate::connectors::coingecko::CoinGeckoConnector;
use crate::entities::{assets, asset_contracts};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QuerySelect,
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
/// 4. Upserts contract addresses into asset_contracts table
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
    tracing::info!("Starting contract addresses collection job");

    // Create CoinGecko connector
    let connector = CoinGeckoConnector::new();

    // Fetch all active assets with coingecko_id
    let mut query = assets::Entity::find()
        .filter(assets::Column::IsActive.eq(true))
        .filter(assets::Column::CoingeckoId.is_not_null());
    
    if let Some(limit_val) = limit {
        query = query.limit(limit_val as u64);
    }
    
    let assets_list = query.all(db).await?;
    
    if assets_list.is_empty() {
        tracing::warn!("No active assets with coingecko_id found");
        return Ok(CollectionResult {
            success: true,
            assets_processed: 0,
            contracts_created: 0,
            contracts_updated: 0,
            assets_skipped: 0,
            error: None,
        });
    }

    tracing::info!("Found {} assets to process for contract addresses", assets_list.len());

    let mut assets_processed = 0;
    let mut contracts_created = 0;
    let mut contracts_updated = 0;
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

            // Check if contract already exists for this asset, chain, and address
            let existing_contract = asset_contracts::Entity::find()
                .filter(asset_contracts::Column::AssetId.eq(asset.id))
                .filter(asset_contracts::Column::Chain.eq(&chain))
                .filter(asset_contracts::Column::ContractAddress.eq(&contract_address))
                .one(db)
                .await?;

            if let Some(existing) = existing_contract {
                // Update existing contract
                let mut contract_update: asset_contracts::ActiveModel = existing.into();
                contract_update.is_verified = ActiveValue::Set(true);
                contract_update.updated_at = ActiveValue::Set(Utc::now().into());
                
                contract_update.update(db).await?;
                contracts_updated += 1;
                
                tracing::debug!(
                    "Updated contract for {} on {} at {}",
                    asset.symbol,
                    chain,
                    contract_address
                );
            } else {
                // Create new contract
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

                new_contract.insert(db).await?;
                contracts_created += 1;
                
                tracing::debug!(
                    "Created contract for {} on {} at {}",
                    asset.symbol,
                    chain,
                    contract_address
                );
            }
        }

        assets_processed += 1;

        // Add a small delay to respect rate limits (CoinGecko free tier has rate limits)
        // Sleep for 1.5 seconds to stay under 50 calls/minute
        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
    }

    tracing::info!(
        "Contract addresses collection completed: {} assets processed, {} contracts created, {} updated, {} skipped",
        assets_processed,
        contracts_created,
        contracts_updated,
        assets_skipped
    );

    Ok(CollectionResult {
        success: true,
        assets_processed,
        contracts_created,
        contracts_updated,
        assets_skipped,
        error: None,
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
        "polygon" => Some("ERC20".to_string()), // Polygon uses ERC20 standard
        "solana" => Some("SPL".to_string()),
        _ => None,
    }
}
