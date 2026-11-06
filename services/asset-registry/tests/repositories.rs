//! Adapter integration tests against a real Postgres schema.
//!
//! Run with `TEST_DATABASE_URL` pointing at a disposable database:
//! `TEST_DATABASE_URL=postgres://... cargo test --manifest-path services/asset-registry/Cargo.toml --test repositories -- --ignored`

mod common;

use asset_registry_application::ports::{
    AssetRepository, ChainRepository, ObservationRepository, PriceSourceRepository,
    RegistryMetadataRepository, RepositoryError,
};
use asset_registry_domain::{
    Asset, ChainFamily, ChainId, ContractAddress, ObservationStatus, PriceSource, TokenContract,
    UnmappedObservation,
};
use asset_registry_infrastructure::repositories::{
    PgAssetRepository, PgChainRepository, PgMetadataRepository, PgObservationRepository,
    PgPriceSourceRepository,
};
use chrono::Utc;

fn chain(id: &str) -> ChainId {
    ChainId::new(id).expect("valid chain id")
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn resolve_by_contract_is_case_insensitive_for_evm() {
    let (pool, schema) = common::test_pool().await;
    let assets = PgAssetRepository::new(pool.clone());
    let ethereum = chain("ethereum");

    let asset = Asset::new("asset-usdc", "USDC", "USD Coin", 6, true).unwrap();
    assets.insert_asset(&asset, None).await.unwrap();
    let contract = TokenContract::new(
        "tc-usdc",
        &ethereum,
        ContractAddress::parse("0xAbCdEf").unwrap(),
        asset.id(),
        6,
    )
    .unwrap();
    assets.insert_token_contract(&contract).await.unwrap();

    let found = assets
        .find_asset_by_contract(
            &ethereum,
            ChainFamily::Evm,
            &ContractAddress::parse("0xabcdef").unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(found, Some(asset));

    let other_chain = assets
        .find_asset_by_contract(
            &chain("solana"),
            ChainFamily::Solana,
            &ContractAddress::parse("0xabcdef").unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(other_chain, None);

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn native_resolution_is_chain_scoped_and_never_symbol_only() {
    let (pool, schema) = common::test_pool().await;
    let assets = PgAssetRepository::new(pool.clone());

    let ethereum_eth = assets
        .find_native_asset(&chain("ethereum"), "ETH")
        .await
        .unwrap()
        .expect("seeded Ethereum native asset");
    assert_eq!(ethereum_eth.id().as_str(), "native-eth");

    assert_eq!(
        assets
            .find_native_asset(&chain("solana"), "ETH")
            .await
            .unwrap(),
        None,
        "a symbol alone never identifies an asset"
    );

    let solana_eth = Asset::new("asset-sol-eth", "ETH", "Wrapped Ether", 9, true).unwrap();
    assets
        .insert_asset(&solana_eth, Some(&chain("solana")))
        .await
        .unwrap();
    assert_eq!(
        assets
            .find_native_asset(&chain("solana"), "ETH")
            .await
            .unwrap(),
        Some(solana_eth)
    );

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn duplicate_contract_is_rejected() {
    let (pool, schema) = common::test_pool().await;
    let assets = PgAssetRepository::new(pool.clone());
    let ethereum = chain("ethereum");
    let asset = Asset::new("asset-usdc", "USDC", "USD Coin", 6, true).unwrap();
    assets.insert_asset(&asset, None).await.unwrap();

    assets
        .insert_token_contract(
            &TokenContract::new(
                "tc-1",
                &ethereum,
                ContractAddress::parse("0xAbC").unwrap(),
                asset.id(),
                6,
            )
            .unwrap(),
        )
        .await
        .unwrap();

    let duplicate = assets
        .insert_token_contract(
            &TokenContract::new(
                "tc-2",
                &ethereum,
                ContractAddress::parse("0xabc").unwrap(),
                asset.id(),
                6,
            )
            .unwrap(),
        )
        .await;
    assert!(matches!(duplicate, Err(RepositoryError::Conflict(_))));

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn quarantine_is_deduplicated_and_status_round_trips() {
    let (pool, schema) = common::test_pool().await;
    let observations = PgObservationRepository::new(pool.clone());
    let ethereum = chain("ethereum");

    let observation =
        UnmappedObservation::quarantine("obs-1", "SCAM", &ethereum, None, Utc::now()).unwrap();
    observations.insert(&observation, None).await.unwrap();

    let found = observations
        .find_by_token(&ethereum, "SCAM", None)
        .await
        .unwrap()
        .expect("quarantined observation");
    assert_eq!(found.status(), &ObservationStatus::Quarantined);

    let duplicate =
        UnmappedObservation::quarantine("obs-2", "SCAM", &ethereum, None, Utc::now()).unwrap();
    assert!(matches!(
        observations.insert(&duplicate, None).await,
        Err(RepositoryError::Conflict(_))
    ));

    let mut resolved = found;
    resolved
        .resolve(asset_registry_domain::AssetId::new("native-eth").unwrap())
        .unwrap();
    observations.save_status(&resolved).await.unwrap();
    let reloaded = observations
        .find_by_id(resolved.id())
        .await
        .unwrap()
        .expect("observation");
    assert!(matches!(
        reloaded.status(),
        ObservationStatus::Resolved { asset_id } if asset_id.as_str() == "native-eth"
    ));

    let quarantined = observations
        .list_by_status(&ObservationStatus::Quarantined)
        .await
        .unwrap();
    assert!(quarantined.is_empty());

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn registry_version_bumps_monotonically() {
    let (pool, schema) = common::test_pool().await;
    let metadata = PgMetadataRepository::new(pool.clone());

    assert_eq!(metadata.current_version().await.unwrap(), 0);
    assert_eq!(metadata.bump_version().await.unwrap(), 1);
    assert_eq!(metadata.bump_version().await.unwrap(), 2);
    assert_eq!(metadata.current_version().await.unwrap(), 2);

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn enabled_price_sources_hold_unique_priorities() {
    let (pool, schema) = common::test_pool().await;
    let sources = PgPriceSourceRepository::new(pool.clone());

    let first = PriceSource::new("src-a", "Feed A", 1, Some("licence-a")).unwrap();
    sources.upsert(&first).await.unwrap();
    assert!(sources.enabled_priority_exists(1, None).await.unwrap());
    assert!(!sources
        .enabled_priority_exists(1, Some("Feed A"))
        .await
        .unwrap());

    let duplicate = PriceSource::new("src-b", "Feed B", 1, Some("licence-b")).unwrap();
    assert!(matches!(
        sources.upsert(&duplicate).await,
        Err(RepositoryError::Conflict(_))
    ));

    let disabled = PriceSource::new("src-c", "Feed C", 1, None).unwrap();
    sources.upsert(&disabled).await.unwrap();
    assert!(!sources
        .enabled_priority_exists(1, Some("Feed A"))
        .await
        .unwrap());

    assert_eq!(
        sources.list_enabled_by_priority().await.unwrap(),
        vec![first]
    );

    common::drop_schema(&schema).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]
async fn chains_are_seeded_and_readable() {
    let (pool, schema) = common::test_pool().await;
    let chains = PgChainRepository::new(pool.clone());

    let all = chains.list().await.unwrap();
    assert_eq!(all.len(), 3);
    let ethereum = chains
        .find_by_id(&chain("ethereum"))
        .await
        .unwrap()
        .expect("seeded chain");
    assert_eq!(ethereum.family(), ChainFamily::Evm);
    assert!(ethereum.launch_coverage());

    common::drop_schema(&schema).await;
}
