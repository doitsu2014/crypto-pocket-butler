use crypto_pocket_butler_backend::entities::{assets, asset_prices};
use crypto_pocket_butler_backend::jobs::fetch_all_coins;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue, Database, DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait,
    sea_query::OnConflict, Insert, PaginatorTrait,
};
use std::str::FromStr;
use uuid::Uuid;

/// Helper to create a test database connection
/// This requires DATABASE_URL environment variable or will use a default test database
/// 
/// NOTE: The default connection string uses test credentials (postgres:postgres) which are
/// ONLY suitable for local testing. Never use these credentials in production environments.
async fn setup_test_db() -> Result<DatabaseConnection, Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/crypto_pocket_butler_test".to_string());
    
    let db = Database::connect(&database_url).await?;
    Ok(db)
}

/// Test batch insert with duplicate entries
/// This integration test validates that the deduplication logic prevents the
/// "ON CONFLICT DO UPDATE command cannot affect row a second time" error
#[tokio::test]
#[ignore] // Run with: cargo test --test fetch_all_coins_integration_test -- --ignored
async fn test_batch_insert_with_duplicates() {
    // Setup
    let db = match setup_test_db().await {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Skipping integration test: Failed to connect to test database: {}", e);
            eprintln!("Set DATABASE_URL environment variable to run integration tests");
            return;
        }
    };
    
    // Create test asset
    let asset_id = Uuid::new_v4();
    let timestamp = Utc::now();
    
    let new_asset = assets::ActiveModel {
        id: ActiveValue::Set(asset_id),
        symbol: ActiveValue::Set("TEST".to_string()),
        name: ActiveValue::Set("Test Coin".to_string()),
        asset_type: ActiveValue::Set("cryptocurrency".to_string()),
        coingecko_id: ActiveValue::Set(Some("test-coin".to_string())),
        coinmarketcap_id: ActiveValue::NotSet,
        logo_url: ActiveValue::NotSet,
        description: ActiveValue::NotSet,
        decimals: ActiveValue::NotSet,
        is_active: ActiveValue::Set(true),
        created_at: ActiveValue::Set(timestamp.into()),
        updated_at: ActiveValue::Set(timestamp.into()),
    };
    
    // Insert test asset
    let _ = new_asset.insert(&db).await;
    
    // Create batch with duplicate entries (same asset_id, timestamp, source)
    let mut prices_batch = Vec::new();
    
    for i in 0..3 {
        prices_batch.push(asset_prices::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            asset_id: ActiveValue::Set(asset_id),
            timestamp: ActiveValue::Set(timestamp.into()),
            price_usd: ActiveValue::Set(Decimal::from_str(&format!("{}.0", 1000 + i * 100)).unwrap()),
            source: ActiveValue::Set("coinpaprika".to_string()),
            volume_24h_usd: ActiveValue::Set(Some(Decimal::from_str("1000000.0").unwrap())),
            market_cap_usd: ActiveValue::Set(Some(Decimal::from_str("1000000000.0").unwrap())),
            change_percent_24h: ActiveValue::Set(Some(Decimal::from_str("2.5").unwrap())),
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
    
    // Deduplicate before insert (this is the fix)
    let deduplicated = fetch_all_coins::deduplicate_prices(prices_batch);
    
    // Should have only 1 price after deduplication
    assert_eq!(deduplicated.len(), 1, "Should deduplicate to 1 price");
    
    // Try to insert the deduplicated batch - should succeed
    let result = Insert::many(deduplicated)
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
            ])
            .to_owned(),
        )
        .exec(&db)
        .await;
    
    // Assert insert succeeded
    assert!(result.is_ok(), "Batch insert should succeed with deduplicated prices: {:?}", result.err());
    
    // Verify only one price record exists
    let count = asset_prices::Entity::find()
        .filter(asset_prices::Column::AssetId.eq(asset_id))
        .filter(asset_prices::Column::Source.eq("coinpaprika"))
        .count(&db)
        .await
        .unwrap();
    
    assert_eq!(count, 1, "Should have exactly 1 price record in database");
    
    // Cleanup
    let _ = asset_prices::Entity::delete_many()
        .filter(asset_prices::Column::AssetId.eq(asset_id))
        .exec(&db)
        .await;
    let _ = assets::Entity::delete_by_id(asset_id).exec(&db).await;
}

/// Test batch insert without duplicates
#[tokio::test]
#[ignore] // Run with: cargo test --test fetch_all_coins_integration_test -- --ignored
async fn test_batch_insert_without_duplicates() {
    // Setup
    let db = match setup_test_db().await {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Skipping integration test: Failed to connect to test database: {}", e);
            eprintln!("Set DATABASE_URL environment variable to run integration tests");
            return;
        }
    };
    
    // Create test assets
    let asset_id_1 = Uuid::new_v4();
    let asset_id_2 = Uuid::new_v4();
    let timestamp = Utc::now();
    
    for (id, symbol) in [(asset_id_1, "TEST1"), (asset_id_2, "TEST2")] {
        let new_asset = assets::ActiveModel {
            id: ActiveValue::Set(id),
            symbol: ActiveValue::Set(symbol.to_string()),
            name: ActiveValue::Set(format!("{} Coin", symbol)),
            asset_type: ActiveValue::Set("cryptocurrency".to_string()),
            coingecko_id: ActiveValue::Set(Some(format!("{}-coin", symbol.to_lowercase()))),
            coinmarketcap_id: ActiveValue::NotSet,
            logo_url: ActiveValue::NotSet,
            description: ActiveValue::NotSet,
            decimals: ActiveValue::NotSet,
            is_active: ActiveValue::Set(true),
            created_at: ActiveValue::Set(timestamp.into()),
            updated_at: ActiveValue::Set(timestamp.into()),
        };
        let _ = new_asset.insert(&db).await;
    }
    
    // Create batch with NO duplicates
    let prices_batch = vec![
        asset_prices::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            asset_id: ActiveValue::Set(asset_id_1),
            timestamp: ActiveValue::Set(timestamp.into()),
            price_usd: ActiveValue::Set(Decimal::from_str("1000.0").unwrap()),
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
            asset_id: ActiveValue::Set(asset_id_2),
            timestamp: ActiveValue::Set(timestamp.into()),
            price_usd: ActiveValue::Set(Decimal::from_str("2000.0").unwrap()),
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
    
    // Deduplicate (should remain unchanged since no duplicates)
    let deduplicated = fetch_all_coins::deduplicate_prices(prices_batch);
    assert_eq!(deduplicated.len(), 2, "Should keep all prices when no duplicates");
    
    // Insert the batch
    let result = Insert::many(deduplicated)
        .on_conflict(
            OnConflict::columns([
                asset_prices::Column::AssetId,
                asset_prices::Column::Timestamp,
                asset_prices::Column::Source,
            ])
            .update_columns([
                asset_prices::Column::PriceUsd,
                asset_prices::Column::Rank,
            ])
            .to_owned(),
        )
        .exec(&db)
        .await;
    
    // Assert insert succeeded
    assert!(result.is_ok(), "Batch insert should succeed: {:?}", result.err());
    
    // Verify both price records exist
    let count = asset_prices::Entity::find()
        .filter(
            asset_prices::Column::AssetId.is_in([asset_id_1, asset_id_2])
        )
        .filter(asset_prices::Column::Source.eq("coinpaprika"))
        .count(&db)
        .await
        .unwrap();
    
    assert_eq!(count, 2, "Should have exactly 2 price records in database");
    
    // Cleanup
    let _ = asset_prices::Entity::delete_many()
        .filter(asset_prices::Column::AssetId.is_in([asset_id_1, asset_id_2]))
        .exec(&db)
        .await;
    let _ = assets::Entity::delete_many()
        .filter(assets::Column::Id.is_in([asset_id_1, asset_id_2]))
        .exec(&db)
        .await;
}

/// Test that the job completes successfully when called
/// This is a smoke test that validates the overall job flow
#[tokio::test]
#[ignore] // Run with: cargo test --test fetch_all_coins_integration_test -- --ignored
async fn test_fetch_all_coins_job_execution() {
    // Setup
    let db = match setup_test_db().await {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Skipping integration test: Failed to connect to test database: {}", e);
            eprintln!("Set DATABASE_URL environment variable to run integration tests");
            return;
        }
    };
    
    // Note: This test requires actual API connectivity to CoinPaprika
    // If the API is unavailable, the test will fail gracefully
    
    let result = fetch_all_coins::fetch_all_coins(&db).await;
    
    // The job should return a result (even if it fails due to API issues)
    assert!(result.is_ok(), "Job should return a result struct");
    
    let collection_result = result.unwrap();
    
    // Log the results for debugging
    eprintln!("Integration test results:");
    eprintln!("  Success: {}", collection_result.success);
    eprintln!("  Coins fetched: {}", collection_result.coins_fetched);
    eprintln!("  Assets created: {}", collection_result.assets_created);
    eprintln!("  Assets updated: {}", collection_result.assets_updated);
    eprintln!("  Prices stored: {}", collection_result.prices_stored);
    if let Some(error) = &collection_result.error {
        eprintln!("  Error: {}", error);
    }
    
    // If the job succeeded, verify some data was processed
    if collection_result.success {
        assert!(collection_result.coins_fetched > 0, "Should fetch some coins");
        // Note: prices_stored might be 0 if deduplication removed everything,
        // but that's unlikely in a real API response
    }
}
