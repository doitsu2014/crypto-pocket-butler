use crypto_pocket_butler_backend::entities::{assets, asset_prices};
use crypto_pocket_butler_backend::helpers::asset_identity::{AssetIdentityNormalizer, NormalizationResult};
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue, Database, DatabaseConnection, EntityTrait,
};
use uuid::Uuid;

/// Helper to create a test database connection
async fn setup_test_db() -> Result<DatabaseConnection, Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/crypto_pocket_butler_test".to_string());
    
    let db = Database::connect(&database_url).await?;
    Ok(db)
}

/// Test that normalize_from_symbol prefers assets with better (lower) rank
/// This tests the fix for the ETH duplicate symbol issue where ETH (rank 2) should
/// be preferred over Ethereum Carbon (rank 9xx)
#[tokio::test]
#[ignore] // Run with: cargo test --test asset_rank_selection_test -- --ignored
async fn test_normalize_from_symbol_prefers_better_rank() {
    let db = match setup_test_db().await {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Skipping test: Failed to connect to test database: {}", e);
            return;
        }
    };
    
    let timestamp = Utc::now();
    
    // Create first asset: ETH (Ethereum Carbon) with high rank (bad)
    let asset1_id = Uuid::new_v4();
    let asset1 = assets::ActiveModel {
        id: ActiveValue::Set(asset1_id),
        symbol: ActiveValue::Set("ETH".to_string()),
        name: ActiveValue::Set("Ethereum Carbon".to_string()),
        asset_type: ActiveValue::Set("cryptocurrency".to_string()),
        coingecko_id: ActiveValue::Set(Some("ethereum-carbon".to_string())),
        coinmarketcap_id: ActiveValue::NotSet,
        logo_url: ActiveValue::NotSet,
        description: ActiveValue::NotSet,
        decimals: ActiveValue::NotSet,
        is_active: ActiveValue::Set(true),
        created_at: ActiveValue::Set(timestamp.into()),
        updated_at: ActiveValue::Set(timestamp.into()),
    };
    
    let _inserted_asset1 = asset1.insert(&db).await.expect("Should insert first asset");
    
    // Create price record for Ethereum Carbon with rank 950
    let price1_id = Uuid::new_v4();
    let price1 = asset_prices::ActiveModel {
        id: ActiveValue::Set(price1_id),
        asset_id: ActiveValue::Set(asset1_id),
        timestamp: ActiveValue::Set(timestamp.into()),
        price_usd: ActiveValue::Set(Decimal::from(1)),
        volume_24h_usd: ActiveValue::NotSet,
        market_cap_usd: ActiveValue::NotSet,
        change_percent_24h: ActiveValue::NotSet,
        source: ActiveValue::Set("test".to_string()),
        created_at: ActiveValue::Set(timestamp.into()),
        rank: ActiveValue::Set(Some(950)), // High rank (bad)
        circulating_supply: ActiveValue::NotSet,
        total_supply: ActiveValue::NotSet,
        max_supply: ActiveValue::NotSet,
        beta_value: ActiveValue::NotSet,
        percent_change_1h: ActiveValue::NotSet,
        percent_change_7d: ActiveValue::NotSet,
        percent_change_30d: ActiveValue::NotSet,
        ath_price: ActiveValue::NotSet,
        ath_date: ActiveValue::NotSet,
        percent_from_price_ath: ActiveValue::NotSet,
    };
    
    let _ = price1.insert(&db).await.expect("Should insert first price");
    
    // Create second asset: ETH (Ethereum) with low rank (good)
    let asset2_id = Uuid::new_v4();
    let asset2 = assets::ActiveModel {
        id: ActiveValue::Set(asset2_id),
        symbol: ActiveValue::Set("ETH".to_string()),
        name: ActiveValue::Set("Ethereum".to_string()),
        asset_type: ActiveValue::Set("cryptocurrency".to_string()),
        coingecko_id: ActiveValue::Set(Some("ethereum".to_string())),
        coinmarketcap_id: ActiveValue::NotSet,
        logo_url: ActiveValue::NotSet,
        description: ActiveValue::NotSet,
        decimals: ActiveValue::NotSet,
        is_active: ActiveValue::Set(true),
        created_at: ActiveValue::Set(timestamp.into()),
        updated_at: ActiveValue::Set(timestamp.into()),
    };
    
    let _inserted_asset2 = asset2.insert(&db).await.expect("Should insert second asset");
    
    // Create price record for Ethereum with rank 2
    let price2_id = Uuid::new_v4();
    let price2 = asset_prices::ActiveModel {
        id: ActiveValue::Set(price2_id),
        asset_id: ActiveValue::Set(asset2_id),
        timestamp: ActiveValue::Set(timestamp.into()),
        price_usd: ActiveValue::Set(Decimal::from(3000)),
        volume_24h_usd: ActiveValue::NotSet,
        market_cap_usd: ActiveValue::NotSet,
        change_percent_24h: ActiveValue::NotSet,
        source: ActiveValue::Set("test".to_string()),
        created_at: ActiveValue::Set(timestamp.into()),
        rank: ActiveValue::Set(Some(2)), // Low rank (good)
        circulating_supply: ActiveValue::NotSet,
        total_supply: ActiveValue::NotSet,
        max_supply: ActiveValue::NotSet,
        beta_value: ActiveValue::NotSet,
        percent_change_1h: ActiveValue::NotSet,
        percent_change_7d: ActiveValue::NotSet,
        percent_change_30d: ActiveValue::NotSet,
        ath_price: ActiveValue::NotSet,
        ath_date: ActiveValue::NotSet,
        percent_from_price_ath: ActiveValue::NotSet,
    };
    
    let _ = price2.insert(&db).await.expect("Should insert second price");
    
    // Test the normalizer - should return Ethereum (rank 2), not Ethereum Carbon (rank 950)
    let normalizer = AssetIdentityNormalizer::new(db.clone());
    let result = normalizer.normalize_from_symbol("ETH").await;
    
    // Verify we got a mapped result
    assert!(matches!(result, NormalizationResult::Mapped(_)), 
        "Should find ETH asset");
    
    if let NormalizationResult::Mapped(identity) = result {
        assert_eq!(identity.symbol, "ETH", "Symbol should be ETH");
        assert_eq!(identity.name, "Ethereum", 
            "Should map to 'Ethereum' (rank 2), not 'Ethereum Carbon' (rank 950)");
        assert_eq!(identity.asset_id, asset2_id, 
            "Should return the asset with better rank");
    }
    
    // Cleanup
    let _ = asset_prices::Entity::delete_by_id(price1_id).exec(&db).await;
    let _ = asset_prices::Entity::delete_by_id(price2_id).exec(&db).await;
    let _ = assets::Entity::delete_by_id(asset1_id).exec(&db).await;
    let _ = assets::Entity::delete_by_id(asset2_id).exec(&db).await;
}

/// Test that normalize_from_okx also prefers assets with better rank
#[tokio::test]
#[ignore] // Run with: cargo test --test asset_rank_selection_test -- --ignored
async fn test_normalize_from_okx_prefers_better_rank() {
    let db = match setup_test_db().await {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Skipping test: Failed to connect to test database: {}", e);
            return;
        }
    };
    
    let timestamp = Utc::now();
    
    // Create two BTC assets with different ranks
    let asset1_id = Uuid::new_v4();
    let asset1 = assets::ActiveModel {
        id: ActiveValue::Set(asset1_id),
        symbol: ActiveValue::Set("BTC".to_string()),
        name: ActiveValue::Set("Bitcoin Fake".to_string()),
        asset_type: ActiveValue::Set("cryptocurrency".to_string()),
        coingecko_id: ActiveValue::Set(Some("bitcoin-fake".to_string())),
        coinmarketcap_id: ActiveValue::NotSet,
        logo_url: ActiveValue::NotSet,
        description: ActiveValue::NotSet,
        decimals: ActiveValue::NotSet,
        is_active: ActiveValue::Set(true),
        created_at: ActiveValue::Set(timestamp.into()),
        updated_at: ActiveValue::Set(timestamp.into()),
    };
    let _ = asset1.insert(&db).await.expect("Should insert first test asset");
    
    let price1_id = Uuid::new_v4();
    let price1 = asset_prices::ActiveModel {
        id: ActiveValue::Set(price1_id),
        asset_id: ActiveValue::Set(asset1_id),
        timestamp: ActiveValue::Set(timestamp.into()),
        price_usd: ActiveValue::Set(Decimal::from(100)),
        volume_24h_usd: ActiveValue::NotSet,
        market_cap_usd: ActiveValue::NotSet,
        change_percent_24h: ActiveValue::NotSet,
        source: ActiveValue::Set("test".to_string()),
        created_at: ActiveValue::Set(timestamp.into()),
        rank: ActiveValue::Set(Some(500)),
        circulating_supply: ActiveValue::NotSet,
        total_supply: ActiveValue::NotSet,
        max_supply: ActiveValue::NotSet,
        beta_value: ActiveValue::NotSet,
        percent_change_1h: ActiveValue::NotSet,
        percent_change_7d: ActiveValue::NotSet,
        percent_change_30d: ActiveValue::NotSet,
        ath_price: ActiveValue::NotSet,
        ath_date: ActiveValue::NotSet,
        percent_from_price_ath: ActiveValue::NotSet,
    };
    let _ = price1.insert(&db).await.expect("Should insert first test price");
    
    let asset2_id = Uuid::new_v4();
    let asset2 = assets::ActiveModel {
        id: ActiveValue::Set(asset2_id),
        symbol: ActiveValue::Set("BTC".to_string()),
        name: ActiveValue::Set("Bitcoin".to_string()),
        asset_type: ActiveValue::Set("cryptocurrency".to_string()),
        coingecko_id: ActiveValue::Set(Some("bitcoin".to_string())),
        coinmarketcap_id: ActiveValue::NotSet,
        logo_url: ActiveValue::NotSet,
        description: ActiveValue::NotSet,
        decimals: ActiveValue::NotSet,
        is_active: ActiveValue::Set(true),
        created_at: ActiveValue::Set(timestamp.into()),
        updated_at: ActiveValue::Set(timestamp.into()),
    };
    let _ = asset2.insert(&db).await.expect("Should insert second test asset");
    
    let price2_id = Uuid::new_v4();
    let price2 = asset_prices::ActiveModel {
        id: ActiveValue::Set(price2_id),
        asset_id: ActiveValue::Set(asset2_id),
        timestamp: ActiveValue::Set(timestamp.into()),
        price_usd: ActiveValue::Set(Decimal::from(50000)),
        volume_24h_usd: ActiveValue::NotSet,
        market_cap_usd: ActiveValue::NotSet,
        change_percent_24h: ActiveValue::NotSet,
        source: ActiveValue::Set("test".to_string()),
        created_at: ActiveValue::Set(timestamp.into()),
        rank: ActiveValue::Set(Some(1)),
        circulating_supply: ActiveValue::NotSet,
        total_supply: ActiveValue::NotSet,
        max_supply: ActiveValue::NotSet,
        beta_value: ActiveValue::NotSet,
        percent_change_1h: ActiveValue::NotSet,
        percent_change_7d: ActiveValue::NotSet,
        percent_change_30d: ActiveValue::NotSet,
        ath_price: ActiveValue::NotSet,
        ath_date: ActiveValue::NotSet,
        percent_from_price_ath: ActiveValue::NotSet,
    };
    let _ = price2.insert(&db).await.expect("Should insert second test price");
    
    // Test normalize_from_okx
    let normalizer = AssetIdentityNormalizer::new(db.clone());
    let result = normalizer.normalize_from_okx("BTC").await;
    
    assert!(matches!(result, NormalizationResult::Mapped(_)), 
        "Should find BTC asset");
    
    if let NormalizationResult::Mapped(identity) = result {
        assert_eq!(identity.symbol, "BTC");
        assert_eq!(identity.name, "Bitcoin", 
            "OKX lookup should prefer Bitcoin (rank 1) over Bitcoin Fake (rank 500)");
        assert_eq!(identity.asset_id, asset2_id);
    }
    
    // Cleanup
    let _ = asset_prices::Entity::delete_by_id(price1_id).exec(&db).await;
    let _ = asset_prices::Entity::delete_by_id(price2_id).exec(&db).await;
    let _ = assets::Entity::delete_by_id(asset1_id).exec(&db).await;
    let _ = assets::Entity::delete_by_id(asset2_id).exec(&db).await;
}

/// Test fallback behavior when no asset has price data with rank
#[tokio::test]
#[ignore] // Run with: cargo test --test asset_rank_selection_test -- --ignored
async fn test_normalize_fallback_without_rank_data() {
    let db = match setup_test_db().await {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Skipping test: Failed to connect to test database: {}", e);
            return;
        }
    };
    
    let timestamp = Utc::now();
    
    // Create an asset without any price data
    let asset_id = Uuid::new_v4();
    let asset = assets::ActiveModel {
        id: ActiveValue::Set(asset_id),
        symbol: ActiveValue::Set("NORANK".to_string()),
        name: ActiveValue::Set("No Rank Coin".to_string()),
        asset_type: ActiveValue::Set("cryptocurrency".to_string()),
        coingecko_id: ActiveValue::Set(Some("no-rank-coin".to_string())),
        coinmarketcap_id: ActiveValue::NotSet,
        logo_url: ActiveValue::NotSet,
        description: ActiveValue::NotSet,
        decimals: ActiveValue::NotSet,
        is_active: ActiveValue::Set(true),
        created_at: ActiveValue::Set(timestamp.into()),
        updated_at: ActiveValue::Set(timestamp.into()),
    };
    let _ = asset.insert(&db).await.expect("Should insert test asset without rank");
    
    // Test the normalizer - should still find the asset even without price data
    let normalizer = AssetIdentityNormalizer::new(db.clone());
    let result = normalizer.normalize_from_symbol("NORANK").await;
    
    assert!(matches!(result, NormalizationResult::Mapped(_)), 
        "Should find asset even without price data");
    
    if let NormalizationResult::Mapped(identity) = result {
        assert_eq!(identity.symbol, "NORANK");
        assert_eq!(identity.name, "No Rank Coin");
        assert_eq!(identity.asset_id, asset_id);
    }
    
    // Cleanup
    let _ = assets::Entity::delete_by_id(asset_id).exec(&db).await;
}
