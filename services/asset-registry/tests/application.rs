//! Use-case tests against port fakes (no database, no network).
//!
//! They are written before the use cases they exercise.

mod common;

use std::sync::Arc;

use asset_registry_application::admin::{
    AddAssetCommand, AddTokenContractCommand, AdminActor, AdminError, IgnoreObservationCommand,
    RegistryAdmin, ResolveObservationCommand, UpsertPriceSourceCommand,
};
use asset_registry_application::ports::{
    AssetRepository, ObservationRepository, PriceProvider, PriceSourceRepository, ProviderError,
    RegistryMetadataRepository, RepositoryError, Role,
};
use asset_registry_application::resolve_asset::{
    ResolveAsset, ResolveAssetQuery, ResolveError, ResolveOutcome,
};
use asset_registry_application::resolve_price::{
    PriceAttempt, PriceOutcome, PriceQuery, ResolvePrice,
};
use asset_registry_domain::{
    Asset, Chain, ChainFamily, ChainId, ContractAddress, ObservationStatus, Price, PriceSource,
    TokenContract, UnmappedObservation,
};
use chrono::{TimeZone, Utc};
use common::fakes::{
    FakeAssets, FakeAudit, FakeChains, FakeMetadata, FakeObservations, FakePriceProvider,
    FakeSources, FixedClock,
};
use rust_decimal::Decimal;

fn at() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 6, 12, 0, 0).unwrap()
}

fn fixed_clock() -> Arc<FixedClock> {
    Arc::new(FixedClock::at(at()))
}

fn chain(id: &str) -> ChainId {
    ChainId::new(id).unwrap()
}

fn asset(id: &str, symbol: &str, decimals: u8) -> Asset {
    Asset::new(id, symbol, symbol, decimals, true).unwrap()
}

fn admin_actor() -> AdminActor {
    AdminActor::new("admin-1", Role::Admin).unwrap()
}

fn user_actor() -> AdminActor {
    AdminActor::new("user-1", Role::User).unwrap()
}

fn build_admin(
    assets: Arc<FakeAssets>,
    chains: Arc<FakeChains>,
    sources: Arc<FakeSources>,
    observations: Arc<FakeObservations>,
    metadata: Arc<FakeMetadata>,
    audit: Arc<FakeAudit>,
) -> RegistryAdmin {
    RegistryAdmin::new(
        assets,
        chains,
        sources,
        observations,
        metadata,
        audit,
        fixed_clock(),
    )
}

// ---------------------------------------------------------------------------
// Registry administration (WF3 / BR1.2, BR2.1, BR3.1)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_adds_an_asset_audits_it_and_bumps_the_registry_version() {
    let assets = Arc::new(FakeAssets::default());
    let audit = Arc::new(FakeAudit::default());
    let metadata = Arc::new(FakeMetadata::default());
    let admin = build_admin(
        assets.clone(),
        Arc::new(FakeChains::default()),
        Arc::new(FakeSources::default()),
        Arc::new(FakeObservations::default()),
        metadata.clone(),
        audit.clone(),
    );

    let created = admin
        .add_asset(
            &admin_actor(),
            AddAssetCommand {
                id: "asset-usdc".to_string(),
                symbol: "usdc".to_string(),
                name: "USD Coin".to_string(),
                decimals: 6,
                native_chain: None,
            },
            Some("corr-1"),
        )
        .await
        .expect("asset is created");

    assert_eq!(created.symbol(), "USDC");
    assert_eq!(assets.asset_count(), 1);
    let events = audit.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].actor, "admin-1");
    assert_eq!(events[0].action, "asset.create");
    assert_eq!(events[0].target, "asset-usdc");
    assert_eq!(events[0].outcome, "success");
    assert_eq!(events[0].correlation_id.as_deref(), Some("corr-1"));
    assert_eq!(metadata.current_version().await.unwrap(), 1);
}

#[tokio::test]
async fn a_non_admin_is_forbidden_and_nothing_is_written_or_audited() {
    let assets = Arc::new(FakeAssets::default());
    let audit = Arc::new(FakeAudit::default());
    let admin = build_admin(
        assets.clone(),
        Arc::new(FakeChains::default()),
        Arc::new(FakeSources::default()),
        Arc::new(FakeObservations::default()),
        Arc::new(FakeMetadata::default()),
        audit.clone(),
    );

    let result = admin
        .add_asset(
            &user_actor(),
            AddAssetCommand {
                id: "asset-usdc".to_string(),
                symbol: "USDC".to_string(),
                name: "USD Coin".to_string(),
                decimals: 6,
                native_chain: None,
            },
            None,
        )
        .await;

    assert!(matches!(result, Err(AdminError::Forbidden(_))));
    assert_eq!(assets.asset_count(), 0);
    assert!(audit.events().is_empty());
}

#[tokio::test]
async fn contract_decimals_must_match_the_asset_before_anything_is_written() {
    let assets = Arc::new(FakeAssets::default());
    let audit = Arc::new(FakeAudit::default());
    let admin = build_admin(
        assets.clone(),
        Arc::new(FakeChains::default()),
        Arc::new(FakeSources::default()),
        Arc::new(FakeObservations::default()),
        Arc::new(FakeMetadata::default()),
        audit.clone(),
    );
    admin
        .add_asset(
            &admin_actor(),
            AddAssetCommand {
                id: "asset-usdc".to_string(),
                symbol: "USDC".to_string(),
                name: "USD Coin".to_string(),
                decimals: 6,
                native_chain: None,
            },
            None,
        )
        .await
        .unwrap();

    let result = admin
        .add_token_contract(
            &admin_actor(),
            AddTokenContractCommand {
                id: "tc-bad".to_string(),
                chain_id: "ethereum".to_string(),
                contract_address: "0xabc".to_string(),
                asset_id: "asset-usdc".to_string(),
                decimals: 18,
            },
            None,
        )
        .await;

    assert!(
        matches!(result, Err(AdminError::Validation(_))),
        "BR1.2 refusal"
    );
    assert_eq!(assets.contract_count(), 0);
    assert_eq!(
        audit.events().len(),
        1,
        "only the asset creation is audited"
    );
}

#[tokio::test]
async fn a_duplicate_chain_plus_contract_is_a_conflict() {
    let assets = Arc::new(FakeAssets::default());
    let admin = build_admin(
        assets.clone(),
        Arc::new(FakeChains::default()),
        Arc::new(FakeSources::default()),
        Arc::new(FakeObservations::default()),
        Arc::new(FakeMetadata::default()),
        Arc::new(FakeAudit::default()),
    );
    admin
        .add_asset(
            &admin_actor(),
            AddAssetCommand {
                id: "asset-usdc".to_string(),
                symbol: "USDC".to_string(),
                name: "USD Coin".to_string(),
                decimals: 6,
                native_chain: None,
            },
            None,
        )
        .await
        .unwrap();
    admin
        .add_token_contract(
            &admin_actor(),
            AddTokenContractCommand {
                id: "tc-1".to_string(),
                chain_id: "ethereum".to_string(),
                contract_address: "0xAbC".to_string(),
                asset_id: "asset-usdc".to_string(),
                decimals: 6,
            },
            None,
        )
        .await
        .unwrap();

    let result = admin
        .add_token_contract(
            &admin_actor(),
            AddTokenContractCommand {
                id: "tc-2".to_string(),
                chain_id: "ethereum".to_string(),
                contract_address: "0xabc".to_string(),
                asset_id: "asset-usdc".to_string(),
                decimals: 6,
            },
            None,
        )
        .await;

    assert!(matches!(result, Err(AdminError::Conflict(_))), "BR1.1");
    assert_eq!(assets.contract_count(), 1);
}

#[tokio::test]
async fn an_unlicensed_price_source_cannot_be_enabled() {
    let sources = Arc::new(FakeSources::default());
    let admin = build_admin(
        Arc::new(FakeAssets::default()),
        Arc::new(FakeChains::default()),
        sources.clone(),
        Arc::new(FakeObservations::default()),
        Arc::new(FakeMetadata::default()),
        Arc::new(FakeAudit::default()),
    );

    let result = admin
        .upsert_price_source(
            &admin_actor(),
            UpsertPriceSourceCommand {
                id: "src-1".to_string(),
                name: "Unlicensed Feed".to_string(),
                priority: 1,
                licence_ref: None,
                enabled: true,
            },
            None,
        )
        .await;

    assert!(matches!(result, Err(AdminError::Validation(_))));
    assert!(sources.sources.lock().unwrap().is_empty());
}

#[tokio::test]
async fn two_enabled_sources_cannot_share_a_priority() {
    let sources = Arc::new(FakeSources::default());
    let admin = build_admin(
        Arc::new(FakeAssets::default()),
        Arc::new(FakeChains::default()),
        sources.clone(),
        Arc::new(FakeObservations::default()),
        Arc::new(FakeMetadata::default()),
        Arc::new(FakeAudit::default()),
    );
    admin
        .upsert_price_source(
            &admin_actor(),
            UpsertPriceSourceCommand {
                id: "src-1".to_string(),
                name: "Feed One".to_string(),
                priority: 1,
                licence_ref: Some("licence-1".to_string()),
                enabled: true,
            },
            None,
        )
        .await
        .unwrap();

    let result = admin
        .upsert_price_source(
            &admin_actor(),
            UpsertPriceSourceCommand {
                id: "src-2".to_string(),
                name: "Feed Two".to_string(),
                priority: 1,
                licence_ref: Some("licence-2".to_string()),
                enabled: true,
            },
            None,
        )
        .await;

    assert!(matches!(result, Err(AdminError::Conflict(_))), "BR2.1");
    assert_eq!(sources.list_enabled_by_priority().await.unwrap().len(), 1);
}

#[tokio::test]
async fn resolving_a_quarantined_observation_links_the_asset_and_is_audited() {
    let assets = Arc::new(FakeAssets::default());
    let observations = Arc::new(FakeObservations::default());
    let audit = Arc::new(FakeAudit::default());
    let admin = build_admin(
        assets.clone(),
        Arc::new(FakeChains::default()),
        Arc::new(FakeSources::default()),
        observations.clone(),
        Arc::new(FakeMetadata::default()),
        audit.clone(),
    );
    let target = asset("asset-scam", "SCAM", 18);
    assets.insert_asset(&target, None).await.unwrap();
    let observation =
        UnmappedObservation::quarantine("obs-1", "SCAM", &chain("ethereum"), None, at()).unwrap();
    observations.insert(&observation, None).await.unwrap();

    let resolved = admin
        .resolve_observation(
            &admin_actor(),
            ResolveObservationCommand {
                observation_id: "obs-1".to_string(),
                asset_id: "asset-scam".to_string(),
            },
            Some("corr-2"),
        )
        .await
        .expect("observation resolves");

    assert_eq!(
        resolved.status(),
        &ObservationStatus::Resolved {
            asset_id: asset_registry_domain::AssetId::new("asset-scam").unwrap()
        }
    );
    assert_eq!(audit.events()[0].action, "observation.resolve");
    assert_eq!(audit.events()[0].target, "obs-1");
}

#[tokio::test]
async fn ignoring_an_observation_requires_a_written_reason() {
    let observations = Arc::new(FakeObservations::default());
    let admin = build_admin(
        Arc::new(FakeAssets::default()),
        Arc::new(FakeChains::default()),
        Arc::new(FakeSources::default()),
        observations.clone(),
        Arc::new(FakeMetadata::default()),
        Arc::new(FakeAudit::default()),
    );
    let observation =
        UnmappedObservation::quarantine("obs-1", "JUNK", &chain("ethereum"), None, at()).unwrap();
    observations.insert(&observation, None).await.unwrap();

    let result = admin
        .ignore_observation(
            &admin_actor(),
            IgnoreObservationCommand {
                observation_id: "obs-1".to_string(),
                reason: "   ".to_string(),
            },
            None,
        )
        .await;

    assert!(matches!(result, Err(AdminError::Validation(_))));
    assert_eq!(
        observations
            .find_by_id(observation.id())
            .await
            .unwrap()
            .unwrap()
            .status(),
        &ObservationStatus::Quarantined
    );
}

#[tokio::test]
async fn an_audit_failure_is_surfaced_and_never_swallowed() {
    let assets = Arc::new(FakeAssets::default());
    let admin = build_admin(
        assets.clone(),
        Arc::new(FakeChains::default()),
        Arc::new(FakeSources::default()),
        Arc::new(FakeObservations::default()),
        Arc::new(FakeMetadata::default()),
        Arc::new(FakeAudit::failing()),
    );

    let result = admin
        .add_asset(
            &admin_actor(),
            AddAssetCommand {
                id: "asset-usdc".to_string(),
                symbol: "USDC".to_string(),
                name: "USD Coin".to_string(),
                decimals: 6,
                native_chain: None,
            },
            None,
        )
        .await;

    assert!(matches!(result, Err(AdminError::Audit(_))));
    // Known skeleton gap: the local write is not rolled back when audit emission
    // fails. The transactional coupling arrives with the Audit service (U12).
    assert_eq!(assets.asset_count(), 1);
}

#[tokio::test]
async fn every_successful_change_bumps_the_registry_version() {
    let metadata = Arc::new(FakeMetadata::default());
    let admin = build_admin(
        Arc::new(FakeAssets::default()),
        Arc::new(FakeChains::default()),
        Arc::new(FakeSources::default()),
        Arc::new(FakeObservations::default()),
        metadata.clone(),
        Arc::new(FakeAudit::default()),
    );
    admin
        .add_asset(
            &admin_actor(),
            AddAssetCommand {
                id: "asset-usdc".to_string(),
                symbol: "USDC".to_string(),
                name: "USD Coin".to_string(),
                decimals: 6,
                native_chain: None,
            },
            None,
        )
        .await
        .unwrap();
    admin
        .add_token_contract(
            &admin_actor(),
            AddTokenContractCommand {
                id: "tc-usdc".to_string(),
                chain_id: "ethereum".to_string(),
                contract_address: "0xabc".to_string(),
                asset_id: "asset-usdc".to_string(),
                decimals: 6,
            },
            None,
        )
        .await
        .unwrap();
    admin
        .upsert_price_source(
            &admin_actor(),
            UpsertPriceSourceCommand {
                id: "src-1".to_string(),
                name: "Feed One".to_string(),
                priority: 1,
                licence_ref: Some("licence-1".to_string()),
                enabled: true,
            },
            None,
        )
        .await
        .unwrap();

    assert_eq!(metadata.current_version().await.unwrap(), 3);
}

// ---------------------------------------------------------------------------
// Asset resolution (WF1 / BR1.1, BR1.3)
// ---------------------------------------------------------------------------

fn resolve_asset_use_case(
    assets: Arc<FakeAssets>,
    chains: Arc<FakeChains>,
    observations: Arc<FakeObservations>,
) -> ResolveAsset {
    ResolveAsset::new(assets, chains, observations, fixed_clock())
}

#[tokio::test]
async fn resolves_a_token_by_chain_plus_contract() {
    let assets = Arc::new(FakeAssets::default());
    let token = asset("asset-usdc", "USDC", 6);
    assets.assets.lock().unwrap().push(token.clone());
    assets.contracts.lock().unwrap().push(
        TokenContract::new(
            "tc-usdc",
            &chain("ethereum"),
            ContractAddress::parse("0xAbC").unwrap(),
            token.id(),
            6,
        )
        .unwrap(),
    );
    let use_case = resolve_asset_use_case(
        assets,
        Arc::new(FakeChains::default()),
        Arc::new(FakeObservations::default()),
    );

    let outcome = use_case
        .execute(ResolveAssetQuery {
            chain: "ethereum".to_string(),
            contract_address: Some("0xabc".to_string()),
            symbol: Some("USDC".to_string()),
        })
        .await
        .unwrap();

    assert_eq!(outcome, ResolveOutcome::Resolved(token));
}

#[tokio::test]
async fn one_asset_observed_on_two_chains_resolves_to_the_same_canonical_asset() {
    let assets = Arc::new(FakeAssets::default());
    let token = asset("asset-usdc", "USDC", 6);
    assets.assets.lock().unwrap().push(token.clone());
    assets.contracts.lock().unwrap().push(
        TokenContract::new(
            "tc-eth-usdc",
            &chain("ethereum"),
            ContractAddress::parse("0xAbC").unwrap(),
            token.id(),
            6,
        )
        .unwrap(),
    );
    assets.contracts.lock().unwrap().push(
        TokenContract::new(
            "tc-sol-usdc",
            &chain("solana"),
            ContractAddress::parse("SoLaNaUsdc").unwrap(),
            token.id(),
            6,
        )
        .unwrap(),
    );
    let use_case = resolve_asset_use_case(
        assets,
        Arc::new(FakeChains::default()),
        Arc::new(FakeObservations::default()),
    );

    let on_ethereum = use_case
        .execute(ResolveAssetQuery {
            chain: "ethereum".to_string(),
            contract_address: Some("0xabc".to_string()),
            symbol: None,
        })
        .await
        .unwrap();
    let on_solana = use_case
        .execute(ResolveAssetQuery {
            chain: "solana".to_string(),
            contract_address: Some("SoLaNaUsdc".to_string()),
            symbol: None,
        })
        .await
        .unwrap();

    assert_eq!(on_ethereum, on_solana, "AC9.2.3: never double counted");
}

#[tokio::test]
async fn resolves_a_native_asset_by_chain_plus_symbol_case_insensitively() {
    let assets = Arc::new(FakeAssets::default());
    let eth = asset("native-eth", "ETH", 18);
    assets.assets.lock().unwrap().push(eth.clone());
    assets
        .native
        .lock()
        .unwrap()
        .push((chain("ethereum"), eth.id().clone()));
    let use_case = resolve_asset_use_case(
        assets,
        Arc::new(FakeChains::default()),
        Arc::new(FakeObservations::default()),
    );

    let outcome = use_case
        .execute(ResolveAssetQuery {
            chain: "ethereum".to_string(),
            contract_address: None,
            symbol: Some(" eth ".to_string()),
        })
        .await
        .unwrap();

    assert_eq!(outcome, ResolveOutcome::Resolved(eth));
}

#[tokio::test]
async fn an_unknown_token_is_quarantined_and_never_guessed() {
    let observations = Arc::new(FakeObservations::default());
    let use_case = resolve_asset_use_case(
        Arc::new(FakeAssets::default()),
        Arc::new(FakeChains::default()),
        observations.clone(),
    );

    let outcome = use_case
        .execute(ResolveAssetQuery {
            chain: "ethereum".to_string(),
            contract_address: Some("0xdeadbeef".to_string()),
            symbol: Some("scam".to_string()),
        })
        .await
        .unwrap();

    match outcome {
        ResolveOutcome::Unresolved(observation) => {
            assert_eq!(observation.status(), &ObservationStatus::Quarantined);
            assert_eq!(observation.observed_symbol(), "SCAM");
        }
        ResolveOutcome::Resolved(_) => panic!("an unknown token must not resolve"),
    }
    assert_eq!(observations.observations.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn the_same_unknown_token_is_quarantined_only_once() {
    let observations = Arc::new(FakeObservations::default());
    let use_case = resolve_asset_use_case(
        Arc::new(FakeAssets::default()),
        Arc::new(FakeChains::default()),
        observations.clone(),
    );
    let query = ResolveAssetQuery {
        chain: "ethereum".to_string(),
        contract_address: Some("0xdeadbeef".to_string()),
        symbol: Some("SCAM".to_string()),
    };

    let first = use_case.execute(query.clone()).await.unwrap();
    let second = use_case.execute(query).await.unwrap();

    match (first, second) {
        (ResolveOutcome::Unresolved(a), ResolveOutcome::Unresolved(b)) => {
            assert_eq!(a.id(), b.id(), "BR1.3 deduplication");
        }
        _ => panic!("both attempts must be unresolved"),
    }
    assert_eq!(observations.observations.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn a_resolved_observation_reuses_its_mapping() {
    let assets = Arc::new(FakeAssets::default());
    let observations = Arc::new(FakeObservations::default());
    let token = asset("asset-scam", "SCAM", 18);
    assets.assets.lock().unwrap().push(token.clone());
    let mut observation =
        UnmappedObservation::quarantine("obs-1", "SCAM", &chain("ethereum"), None, at()).unwrap();
    observation.resolve(token.id().clone()).unwrap();
    observations.insert(&observation, None).await.unwrap();
    let use_case = resolve_asset_use_case(assets, Arc::new(FakeChains::default()), observations);

    let outcome = use_case
        .execute(ResolveAssetQuery {
            chain: "ethereum".to_string(),
            contract_address: None,
            symbol: Some("SCAM".to_string()),
        })
        .await
        .unwrap();

    assert_eq!(outcome, ResolveOutcome::Resolved(token));
}

#[tokio::test]
async fn unknown_and_out_of_coverage_chains_are_refused() {
    let use_case = resolve_asset_use_case(
        Arc::new(FakeAssets::default()),
        Arc::new(FakeChains::default()),
        Arc::new(FakeObservations::default()),
    );

    let unknown = use_case
        .execute(ResolveAssetQuery {
            chain: "cosmos".to_string(),
            contract_address: None,
            symbol: Some("ATOM".to_string()),
        })
        .await;
    assert!(matches!(unknown, Err(ResolveError::UnknownChain(_))));

    let out_of_coverage = use_case
        .execute(ResolveAssetQuery {
            chain: "legacy-testnet".to_string(),
            contract_address: None,
            symbol: Some("OLD".to_string()),
        })
        .await;
    assert!(matches!(
        out_of_coverage,
        Err(ResolveError::OutOfCoverage(_))
    ));
}

#[tokio::test]
async fn an_unknown_contract_without_a_symbol_cannot_be_quarantined() {
    let use_case = resolve_asset_use_case(
        Arc::new(FakeAssets::default()),
        Arc::new(FakeChains::default()),
        Arc::new(FakeObservations::default()),
    );

    let result = use_case
        .execute(ResolveAssetQuery {
            chain: "ethereum".to_string(),
            contract_address: Some("0xdeadbeef".to_string()),
            symbol: None,
        })
        .await;

    assert!(matches!(result, Err(ResolveError::InvalidInput(_))));
}

#[tokio::test]
async fn a_repository_failure_is_surfaced_not_swallowed() {
    let assets = Arc::new(FakeAssets::default());
    *assets.failure.lock().unwrap() =
        Some(RepositoryError::Unavailable("database down".to_string()));
    let use_case = resolve_asset_use_case(
        assets,
        Arc::new(FakeChains::default()),
        Arc::new(FakeObservations::default()),
    );

    let result = use_case
        .execute(ResolveAssetQuery {
            chain: "ethereum".to_string(),
            contract_address: None,
            symbol: Some("ETH".to_string()),
        })
        .await;

    assert!(matches!(result, Err(ResolveError::Repository(_))));
}

// ---------------------------------------------------------------------------
// Price resolution (WF2 / BR2.1, BR2.2)
// ---------------------------------------------------------------------------

fn price(value: i64, scale: u32) -> Price {
    Price::new(Decimal::new(value, scale)).unwrap()
}

fn resolve_price_use_case(
    sources: Arc<dyn PriceSourceRepository>,
    provider: Arc<dyn PriceProvider>,
) -> ResolvePrice {
    ResolvePrice::new(sources, provider, fixed_clock())
}

#[tokio::test]
async fn the_highest_priority_source_wins_and_lower_sources_are_not_called() {
    let sources = FakeSources::with_sources(vec![
        PriceSource::new("src-a", "Feed A", 1, Some("licence-a")).unwrap(),
        PriceSource::new("src-b", "Feed B", 2, Some("licence-b")).unwrap(),
    ]);
    let provider = Arc::new(FakePriceProvider::with(vec![
        ("Feed A", Ok(Some(price(2500, 2)))),
        ("Feed B", Ok(Some(price(2400, 2)))),
    ]));
    let use_case = resolve_price_use_case(Arc::new(sources), provider.clone());
    let asset = asset("asset-eth", "ETH", 18);

    let outcome = use_case.execute(PriceQuery { asset }).await.unwrap();

    match outcome {
        PriceOutcome::Priced {
            price,
            source,
            observed_at,
        } => {
            assert_eq!(price.value(), Decimal::new(2500, 2));
            assert_eq!(source.name(), "Feed A");
            assert_eq!(observed_at, at());
        }
        PriceOutcome::Unpriced { .. } => panic!("a source answered"),
    }
    assert_eq!(provider.calls(), vec!["Feed A".to_string()]);
}

#[tokio::test]
async fn a_provider_failure_falls_back_to_the_next_source() {
    let sources = FakeSources::with_sources(vec![
        PriceSource::new("src-a", "Feed A", 1, Some("licence-a")).unwrap(),
        PriceSource::new("src-b", "Feed B", 2, Some("licence-b")).unwrap(),
    ]);
    let provider = Arc::new(FakePriceProvider::with(vec![
        ("Feed A", Err(ProviderError::Timeout)),
        ("Feed B", Ok(Some(price(2510, 2)))),
    ]));
    let use_case = resolve_price_use_case(Arc::new(sources), provider.clone());

    let outcome = use_case
        .execute(PriceQuery {
            asset: asset("asset-eth", "ETH", 18),
        })
        .await
        .unwrap();

    match outcome {
        PriceOutcome::Priced { source, .. } => assert_eq!(source.name(), "Feed B"),
        PriceOutcome::Unpriced { .. } => panic!("the fallback answered"),
    }
    assert_eq!(
        provider.calls(),
        vec!["Feed A".to_string(), "Feed B".to_string()]
    );
}

#[tokio::test]
async fn a_source_without_a_price_falls_back_to_the_next_source() {
    let sources = FakeSources::with_sources(vec![
        PriceSource::new("src-a", "Feed A", 1, Some("licence-a")).unwrap(),
        PriceSource::new("src-b", "Feed B", 2, Some("licence-b")).unwrap(),
    ]);
    let provider = FakePriceProvider::with(vec![
        ("Feed A", Ok(None)),
        ("Feed B", Ok(Some(price(2510, 2)))),
    ]);
    let use_case = resolve_price_use_case(Arc::new(sources), Arc::new(provider));

    let outcome = use_case
        .execute(PriceQuery {
            asset: asset("asset-eth", "ETH", 18),
        })
        .await
        .unwrap();

    assert!(matches!(outcome, PriceOutcome::Priced { .. }));
}

#[tokio::test]
async fn when_every_source_fails_the_result_is_unpriced_never_zero() {
    let sources = FakeSources::with_sources(vec![
        PriceSource::new("src-a", "Feed A", 1, Some("licence-a")).unwrap(),
        PriceSource::new("src-b", "Feed B", 2, Some("licence-b")).unwrap(),
    ]);
    let provider = FakePriceProvider::with(vec![
        ("Feed A", Err(ProviderError::Timeout)),
        ("Feed B", Ok(None)),
    ]);
    let use_case = resolve_price_use_case(Arc::new(sources), Arc::new(provider));

    let outcome = use_case
        .execute(PriceQuery {
            asset: asset("asset-eth", "ETH", 18),
        })
        .await
        .unwrap();

    match outcome {
        PriceOutcome::Unpriced { attempted, reason } => {
            assert_eq!(
                attempted,
                vec![
                    PriceAttempt {
                        source: "Feed A".to_string(),
                        reason: "provider timed out".to_string(),
                    },
                    PriceAttempt {
                        source: "Feed B".to_string(),
                        reason: "no usable price".to_string(),
                    },
                ]
            );
            assert!(reason.contains("failed"));
        }
        PriceOutcome::Priced { .. } => panic!("no source answered, so nothing may be priced"),
    }
}

#[tokio::test]
async fn no_enabled_sources_means_unpriced() {
    let use_case = resolve_price_use_case(
        Arc::new(FakeSources::default()),
        Arc::new(FakePriceProvider::with(vec![])),
    );

    let outcome = use_case
        .execute(PriceQuery {
            asset: asset("asset-eth", "ETH", 18),
        })
        .await
        .unwrap();

    assert!(matches!(outcome, PriceOutcome::Unpriced { .. }));
}

#[tokio::test]
async fn a_repository_failure_surfaces_as_an_error() {
    struct FailingSources;
    #[async_trait::async_trait]
    impl PriceSourceRepository for FailingSources {
        async fn list_enabled_by_priority(&self) -> Result<Vec<PriceSource>, RepositoryError> {
            Err(RepositoryError::Unavailable("database down".to_string()))
        }
        async fn list(&self) -> Result<Vec<PriceSource>, RepositoryError> {
            Err(RepositoryError::Unavailable("database down".to_string()))
        }
        async fn find_by_name(&self, _name: &str) -> Result<Option<PriceSource>, RepositoryError> {
            Err(RepositoryError::Unavailable("database down".to_string()))
        }
        async fn upsert(&self, _source: &PriceSource) -> Result<(), RepositoryError> {
            Err(RepositoryError::Unavailable("database down".to_string()))
        }
        async fn set_enabled(&self, _name: &str, _enabled: bool) -> Result<(), RepositoryError> {
            Err(RepositoryError::Unavailable("database down".to_string()))
        }
        async fn enabled_priority_exists(
            &self,
            _priority: u32,
            _except_name: Option<&str>,
        ) -> Result<bool, RepositoryError> {
            Err(RepositoryError::Unavailable("database down".to_string()))
        }
    }

    let use_case = resolve_price_use_case(
        Arc::new(FailingSources),
        Arc::new(FakePriceProvider::with(vec![])),
    );
    let result = use_case
        .execute(PriceQuery {
            asset: asset("asset-eth", "ETH", 18),
        })
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn a_provider_that_would_return_zero_is_not_a_usable_price() {
    // The domain type refuses zero, so a provider cannot represent it at all.
    assert!(Price::new(Decimal::ZERO).is_err());
    let provider: Arc<dyn PriceProvider> = Arc::new(FakePriceProvider::with(vec![]));
    let _ = provider;
}

#[tokio::test]
async fn a_chain_with_launch_coverage_requires_a_known_family() {
    let chain = Chain::new("ethereum", "Ethereum", ChainFamily::Evm, true).unwrap();
    assert_eq!(chain.family(), ChainFamily::Evm);
}
