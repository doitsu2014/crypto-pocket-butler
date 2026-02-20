use crypto_pocket_butler_backend::entities::assets;
use crypto_pocket_butler_backend::helpers::asset_identity::AssetIdentityNormalizer;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue, Database, DatabaseConnection, EntityTrait, QueryFilter,
    ColumnTrait, PaginatorTrait,
};
use uuid::Uuid;

/// Helper to create a test database connection
async fn setup_test_db() -> Result<DatabaseConnection, Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/crypto_pocket_butler_test".to_string());
    
    let db = Database::connect(&database_url).await?;
    Ok(db)
}

/// Test that assets with same symbol but different names can coexist
#[tokio::test]
#[ignore] // Run with: cargo test --test asset_uniqueness_test -- --ignored
async fn test_same_symbol_different_names() {
    let db = match setup_test_db().await {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Skipping test: Failed to connect to test database: {}", e);
            return;
        }
    };
    
    let timestamp = Utc::now();
    
    // Create first asset with symbol "TEST" and name "Test Coin A"
    let asset1_id = Uuid::new_v4();
    let asset1 = assets::ActiveModel {
        id: ActiveValue::Set(asset1_id),
        symbol: ActiveValue::Set("TEST".to_string()),
        name: ActiveValue::Set("Test Coin A".to_string()),
        asset_type: ActiveValue::Set("cryptocurrency".to_string()),
        coinpaprika_id: ActiveValue::Set(Some("test-coin-a".to_string())),
        coinmarketcap_id: ActiveValue::NotSet,
        logo_url: ActiveValue::NotSet,
        description: ActiveValue::NotSet,
        decimals: ActiveValue::NotSet,
        is_active: ActiveValue::Set(true),
        created_at: ActiveValue::Set(timestamp.into()),
        updated_at: ActiveValue::Set(timestamp.into()),
    };
    
    let insert1 = asset1.insert(&db).await;
    assert!(insert1.is_ok(), "First asset should be inserted successfully");
    
    // Create second asset with same symbol "TEST" but different name "Test Coin B"
    let asset2_id = Uuid::new_v4();
    let asset2 = assets::ActiveModel {
        id: ActiveValue::Set(asset2_id),
        symbol: ActiveValue::Set("TEST".to_string()),
        name: ActiveValue::Set("Test Coin B".to_string()),
        asset_type: ActiveValue::Set("cryptocurrency".to_string()),
        coinpaprika_id: ActiveValue::Set(Some("test-coin-b".to_string())),
        coinmarketcap_id: ActiveValue::NotSet,
        logo_url: ActiveValue::NotSet,
        description: ActiveValue::NotSet,
        decimals: ActiveValue::NotSet,
        is_active: ActiveValue::Set(true),
        created_at: ActiveValue::Set(timestamp.into()),
        updated_at: ActiveValue::Set(timestamp.into()),
    };
    
    let insert2 = asset2.insert(&db).await;
    assert!(insert2.is_ok(), "Second asset with same symbol but different name should be inserted successfully");
    
    // Verify both assets exist
    let count = assets::Entity::find()
        .filter(assets::Column::Symbol.eq("TEST"))
        .count(&db)
        .await
        .unwrap();
    
    assert_eq!(count, 2, "Should have 2 assets with symbol 'TEST' but different names");
    
    // Cleanup
    let _ = assets::Entity::delete_by_id(asset1_id).exec(&db).await;
    let _ = assets::Entity::delete_by_id(asset2_id).exec(&db).await;
}

/// Test that assets with same (symbol, name) combination cannot coexist
#[tokio::test]
#[ignore] // Run with: cargo test --test asset_uniqueness_test -- --ignored
async fn test_same_symbol_and_name_fails() {
    let db = match setup_test_db().await {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Skipping test: Failed to connect to test database: {}", e);
            return;
        }
    };
    
    let timestamp = Utc::now();
    
    // Create first asset
    let asset1_id = Uuid::new_v4();
    let asset1 = assets::ActiveModel {
        id: ActiveValue::Set(asset1_id),
        symbol: ActiveValue::Set("UNIQUE".to_string()),
        name: ActiveValue::Set("Unique Coin".to_string()),
        asset_type: ActiveValue::Set("cryptocurrency".to_string()),
        coinpaprika_id: ActiveValue::Set(Some("unique-coin-1".to_string())),
        coinmarketcap_id: ActiveValue::NotSet,
        logo_url: ActiveValue::NotSet,
        description: ActiveValue::NotSet,
        decimals: ActiveValue::NotSet,
        is_active: ActiveValue::Set(true),
        created_at: ActiveValue::Set(timestamp.into()),
        updated_at: ActiveValue::Set(timestamp.into()),
    };
    
    let insert1 = asset1.insert(&db).await;
    assert!(insert1.is_ok(), "First asset should be inserted successfully");
    
    // Try to create second asset with same symbol AND name (should fail)
    let asset2_id = Uuid::new_v4();
    let asset2 = assets::ActiveModel {
        id: ActiveValue::Set(asset2_id),
        symbol: ActiveValue::Set("UNIQUE".to_string()),
        name: ActiveValue::Set("Unique Coin".to_string()),
        asset_type: ActiveValue::Set("cryptocurrency".to_string()),
        coinpaprika_id: ActiveValue::Set(Some("unique-coin-2".to_string())),
        coinmarketcap_id: ActiveValue::NotSet,
        logo_url: ActiveValue::NotSet,
        description: ActiveValue::NotSet,
        decimals: ActiveValue::NotSet,
        is_active: ActiveValue::Set(true),
        created_at: ActiveValue::Set(timestamp.into()),
        updated_at: ActiveValue::Set(timestamp.into()),
    };
    
    let insert2 = asset2.insert(&db).await;
    assert!(insert2.is_err(), "Second asset with same symbol AND name should fail due to unique constraint");
    
    // Verify only one asset exists
    let count = assets::Entity::find()
        .filter(assets::Column::Symbol.eq("UNIQUE"))
        .filter(assets::Column::Name.eq("Unique Coin"))
        .count(&db)
        .await
        .unwrap();
    
    assert_eq!(count, 1, "Should have only 1 asset with this symbol+name combination");
    
    // Cleanup
    let _ = assets::Entity::delete_by_id(asset1_id).exec(&db).await;
}

/// Test the new normalize_from_symbol_and_name method
#[tokio::test]
#[ignore] // Run with: cargo test --test asset_uniqueness_test -- --ignored
async fn test_normalize_from_symbol_and_name() {
    let db = match setup_test_db().await {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Skipping test: Failed to connect to test database: {}", e);
            return;
        }
    };
    
    let timestamp = Utc::now();
    
    // Create test assets with same symbol but different names
    let asset1_id = Uuid::new_v4();
    let asset1 = assets::ActiveModel {
        id: ActiveValue::Set(asset1_id),
        symbol: ActiveValue::Set("BTC".to_string()),
        name: ActiveValue::Set("Bitcoin".to_string()),
        asset_type: ActiveValue::Set("cryptocurrency".to_string()),
        coinpaprika_id: ActiveValue::Set(Some("bitcoin".to_string())),
        coinmarketcap_id: ActiveValue::NotSet,
        logo_url: ActiveValue::NotSet,
        description: ActiveValue::NotSet,
        decimals: ActiveValue::NotSet,
        is_active: ActiveValue::Set(true),
        created_at: ActiveValue::Set(timestamp.into()),
        updated_at: ActiveValue::Set(timestamp.into()),
    };
    let _ = asset1.insert(&db).await;
    
    let asset2_id = Uuid::new_v4();
    let asset2 = assets::ActiveModel {
        id: ActiveValue::Set(asset2_id),
        symbol: ActiveValue::Set("BTC".to_string()),
        name: ActiveValue::Set("Bitcoin Wrapped".to_string()),
        asset_type: ActiveValue::Set("cryptocurrency".to_string()),
        coinpaprika_id: ActiveValue::Set(Some("bitcoin-wrapped".to_string())),
        coinmarketcap_id: ActiveValue::NotSet,
        logo_url: ActiveValue::NotSet,
        description: ActiveValue::NotSet,
        decimals: ActiveValue::NotSet,
        is_active: ActiveValue::Set(true),
        created_at: ActiveValue::Set(timestamp.into()),
        updated_at: ActiveValue::Set(timestamp.into()),
    };
    let _ = asset2.insert(&db).await;
    
    // Test the normalizer
    let normalizer = AssetIdentityNormalizer::new(db.clone());
    
    // Should find Bitcoin (exact match on symbol and name)
    let result1 = normalizer.normalize_from_symbol_and_name("BTC", "Bitcoin").await;
    assert!(result1.is_mapped(), "Should find Bitcoin");
    if let Some(identity) = result1.asset_identity() {
        assert_eq!(identity.symbol, "BTC");
        assert_eq!(identity.name, "Bitcoin");
        assert_eq!(identity.asset_id, asset1_id);
    }
    
    // Should find Bitcoin Wrapped (exact match on symbol and name)
    let result2 = normalizer.normalize_from_symbol_and_name("BTC", "Bitcoin Wrapped").await;
    assert!(result2.is_mapped(), "Should find Bitcoin Wrapped");
    if let Some(identity) = result2.asset_identity() {
        assert_eq!(identity.symbol, "BTC");
        assert_eq!(identity.name, "Bitcoin Wrapped");
        assert_eq!(identity.asset_id, asset2_id);
    }
    
    // Should NOT find anything with wrong name
    let result3 = normalizer.normalize_from_symbol_and_name("BTC", "Bitcoin Cash").await;
    assert!(!result3.is_mapped(), "Should not find Bitcoin Cash (doesn't exist)");
    
    // Cleanup
    let _ = assets::Entity::delete_by_id(asset1_id).exec(&db).await;
    let _ = assets::Entity::delete_by_id(asset2_id).exec(&db).await;
}
