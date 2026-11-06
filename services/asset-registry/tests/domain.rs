//! Domain invariant tests for the Asset Registry (BR1.1, BR1.2, BR1.3, BR2.1, BR2.2).
//!
//! These tests are written before the implementation they describe.

use asset_registry_domain::{
    ensure_decimals_match, ensure_unique_enabled_priorities, normalize_symbol, Asset, AssetId,
    Chain, ChainFamily, ChainId, ContractAddress, DomainError, ObservationStatus, Price,
    PriceSource, UnmappedObservation,
};
use chrono::{TimeZone, Utc};
use rust_decimal::Decimal;

fn asset_with_decimals(decimals: u8) -> Asset {
    Asset::new("asset-eth", "ETH", "Ether", decimals, true).expect("valid asset")
}

fn chain_id() -> ChainId {
    ChainId::new("ethereum").expect("valid chain id")
}

fn at() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 6, 12, 0, 0).unwrap()
}

#[test]
fn asset_normalizes_symbol_to_uppercase_and_trims_whitespace() {
    let asset = Asset::new("asset-eth", "  eth  ", "Ether", 18, true).expect("valid asset");
    assert_eq!(asset.symbol(), "ETH");
    assert_eq!(asset.decimals(), 18);
    assert!(asset.canonical());
}

#[test]
fn asset_rejects_empty_symbol_and_out_of_range_decimals() {
    assert!(matches!(
        Asset::new("asset-1", "   ", "Blank", 18, true),
        Err(DomainError::EmptySymbol)
    ));
    assert!(matches!(
        Asset::new("asset-2", "ETH", "Ether", 37, true),
        Err(DomainError::DecimalsOutOfRange(37))
    ));
    assert!(matches!(
        Asset::new("asset-3", "ETH", "", 18, true),
        Err(DomainError::EmptyName)
    ));
}

#[test]
fn symbol_normalization_is_explicit_and_rejects_blank_input() {
    assert_eq!(normalize_symbol(" sol ").unwrap(), "SOL");
    assert!(matches!(
        normalize_symbol(""),
        Err(DomainError::EmptySymbol)
    ));
    assert!(matches!(
        normalize_symbol(&"X".repeat(33)),
        Err(DomainError::InvalidSymbol(_))
    ));
}

#[test]
fn contract_decimals_must_match_the_canonical_asset() {
    let asset = asset_with_decimals(18);
    assert_eq!(ensure_decimals_match(&asset, 18), Ok(()));
    assert_eq!(
        ensure_decimals_match(&asset, 6),
        Err(DomainError::DecimalsMismatch {
            asset: 18,
            contract: 6,
        })
    );
}

#[test]
fn contract_address_preserves_case_and_lowercases_only_for_evm_families() {
    let address = ContractAddress::parse("0xAbC123").expect("valid address");
    assert_eq!(address.as_str(), "0xAbC123");
    assert_eq!(address.key_for(ChainFamily::Evm), "0xabc123");
    assert_eq!(address.key_for(ChainFamily::Bnb), "0xabc123");
    assert_eq!(address.key_for(ChainFamily::Solana), "0xAbC123");
}

#[test]
fn contract_address_rejects_blank_and_control_characters() {
    assert!(matches!(
        ContractAddress::parse("   "),
        Err(DomainError::InvalidContractAddress(_))
    ));
    assert!(matches!(
        ContractAddress::parse("0xabc\n123"),
        Err(DomainError::InvalidContractAddress(_))
    ));
}

#[test]
fn chain_family_round_trips_and_rejects_unknown_values() {
    assert_eq!(ChainFamily::parse("evm").unwrap(), ChainFamily::Evm);
    assert_eq!(ChainFamily::parse("bnb").unwrap(), ChainFamily::Bnb);
    assert_eq!(ChainFamily::parse("solana").unwrap(), ChainFamily::Solana);
    assert!(matches!(
        ChainFamily::parse("cosmos"),
        Err(DomainError::InvalidChainFamily(_))
    ));
    assert_eq!(ChainFamily::Bnb.as_str(), "bnb");

    let chain = Chain::new("ethereum", "Ethereum", ChainFamily::Evm, true).expect("valid chain");
    assert!(chain.launch_coverage());
}

#[test]
fn price_source_requires_a_licence_reference_before_it_can_be_enabled() {
    let mut unlicensed =
        PriceSource::new("src-1", "Unlicensed Feed", 1, None).expect("valid source");
    assert!(!unlicensed.enabled());
    assert_eq!(
        unlicensed.enable(),
        Err(DomainError::UnlicensedSource("Unlicensed Feed".to_string()))
    );

    let mut licensed =
        PriceSource::new("src-2", "Licensed Feed", 2, Some("licence-1")).expect("valid source");
    assert!(licensed.enabled());
    licensed.disable();
    assert!(!licensed.enabled());
    licensed
        .enable()
        .expect("licensed source can be enabled again");
}

#[test]
fn price_source_priority_must_be_at_least_one() {
    assert!(matches!(
        PriceSource::new("src-1", "Feed", 0, Some("licence-1")),
        Err(DomainError::InvalidPriority(0))
    ));
}

#[test]
fn enabled_price_sources_must_hold_unique_priorities() {
    let first = PriceSource::new("src-1", "Feed One", 1, Some("licence-1")).unwrap();
    let second = PriceSource::new("src-2", "Feed Two", 2, Some("licence-2")).unwrap();
    assert_eq!(
        ensure_unique_enabled_priorities(&[first.clone(), second.clone()]),
        Ok(())
    );

    let mut duplicate = PriceSource::new("src-3", "Feed Three", 1, Some("licence-3")).unwrap();
    assert_eq!(
        ensure_unique_enabled_priorities(&[first, duplicate.clone()]),
        Err(DomainError::DuplicatePriority(1))
    );

    duplicate.disable();
    let first_enabled = PriceSource::new("src-1", "Feed One", 1, Some("licence-1")).unwrap();
    assert_eq!(
        ensure_unique_enabled_priorities(&[first_enabled, duplicate]),
        Ok(()),
        "a disabled source does not hold a priority slot"
    );
}

#[test]
fn observation_quarantine_resolve_and_ignore_transitions_are_enforced() {
    let mut observation = UnmappedObservation::quarantine("obs-1", "SCAM", &chain_id(), None, at())
        .expect("valid observation");
    assert_eq!(observation.status(), &ObservationStatus::Quarantined);

    let target = AssetId::new("asset-1").expect("valid asset id");
    observation
        .resolve(target.clone())
        .expect("quarantined observation resolves");
    assert_eq!(
        observation.status(),
        &ObservationStatus::Resolved {
            asset_id: target.clone()
        }
    );
    // Reusing the mapping is idempotent.
    observation
        .resolve(target.clone())
        .expect("same mapping is idempotent");
    // A different target is a refused transition.
    assert!(matches!(
        observation.resolve(AssetId::new("asset-2").unwrap()),
        Err(DomainError::InvalidTransition { .. })
    ));
    // A resolved observation cannot be ignored.
    assert!(matches!(
        observation.ignore("operator changed their mind"),
        Err(DomainError::InvalidTransition { .. })
    ));

    let mut ignored =
        UnmappedObservation::quarantine("obs-2", "JUNK", &chain_id(), None, at()).unwrap();
    ignored
        .ignore("spam token")
        .expect("quarantined observation can be ignored");
    assert_eq!(
        ignored.status(),
        &ObservationStatus::Ignored {
            reason: "spam token".to_string()
        }
    );
    assert!(matches!(
        ignored.resolve(target),
        Err(DomainError::InvalidTransition { .. })
    ));
}

#[test]
fn observation_ignore_requires_a_reason() {
    let mut observation =
        UnmappedObservation::quarantine("obs-1", "SCAM", &chain_id(), None, at()).unwrap();
    assert!(matches!(
        observation.ignore("   "),
        Err(DomainError::EmptyIgnoreReason)
    ));
}

#[test]
fn price_rejects_zero_and_negative_values() {
    assert!(matches!(
        Price::new(Decimal::ZERO),
        Err(DomainError::InvalidPrice(_))
    ));
    assert!(matches!(
        Price::new(Decimal::from(-1)),
        Err(DomainError::InvalidPrice(_))
    ));
    let price = Price::new(Decimal::new(12345, 2)).expect("positive price");
    assert_eq!(price.value(), Decimal::new(12345, 2));
}

#[test]
fn identifiers_reject_blank_values() {
    assert!(AssetId::new(" ").is_err());
    assert!(ChainId::new("").is_err());
}
