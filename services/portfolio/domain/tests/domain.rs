//! Domain tests: portfolio and membership validation, targets and guardrails,
//! the valuation math with provenance and degraded states, snapshots, intents,
//! recorded trades, and the service allow-list.

use std::str::FromStr;

use chrono::{DateTime, TimeZone, Utc};
use portfolio_domain::{
    apply_overlays, canonical_trade_key, compute_valuation, crossing_key, hour_bucket,
    normalise_currency, parse_decimal, preview_targets, project_intent,
    require_target_for_guardrail, service_allowed, validate_name, validate_target, AccountId,
    AuditOutcome, AuditRecord, DataStatus, DomainError, Freshness, FxRate, Guardrail,
    GuardrailDraft, GuardrailId, GuardrailTransition, Holding, IntentStatus, Operation,
    OrderIntent, OrderIntentId, OrderSide, Portfolio, PortfolioAccount, PortfolioAccountId,
    PortfolioId, PriceSource, Quote, RecordedTrade, RecordedTradeId, TargetAllocation,
    TargetAllocationId, TradeOverlay, TradeOverlayId, UserId, ValuationSnapshot,
    ValuationSnapshotId, ValuationStatus,
};
use rust_decimal::Decimal;

fn d(raw: &str) -> Decimal {
    Decimal::from_str(raw).unwrap()
}

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, hour, 0, 0).unwrap()
}

fn user() -> UserId {
    UserId::parse("11111111-1111-4111-8111-111111111111").unwrap()
}

fn portfolio() -> Portfolio {
    Portfolio::new(
        PortfolioId::generate(),
        user(),
        "My portfolio",
        "USD",
        at(0),
    )
    .unwrap()
}

fn target(asset: &str, percent: &str) -> TargetAllocation {
    TargetAllocation::new(
        TargetAllocationId::generate(),
        portfolio().id(),
        asset,
        d(percent),
        at(0),
    )
    .unwrap()
}

fn holding(asset: &str, quantity: &str) -> Holding {
    Holding::new(Some(AccountId::generate()), asset, d(quantity), at(0)).unwrap()
}

fn quote(asset: &str, price: &str, currency: &str) -> Quote {
    Quote::new(
        asset,
        Some(d(price)),
        Some(currency.to_string()),
        Some("test".to_string()),
        Some(at(0)),
        Freshness::Live,
    )
    .unwrap()
}

// ---------------------------------------------------------------------------
// Portfolio
// ---------------------------------------------------------------------------

#[test]
fn portfolio_valid_name_and_currency() {
    let value = portfolio();
    assert_eq!(value.name(), "My portfolio");
    assert_eq!(value.base_currency(), "USD");
    assert_eq!(value.user_id(), user());
    assert!(value.anonymised_at().is_none());
}

#[test]
fn portfolio_name_is_trimmed() {
    let value = Portfolio::new(
        PortfolioId::generate(),
        user(),
        "  Retirement  ",
        "USD",
        at(0),
    )
    .unwrap();
    assert_eq!(value.name(), "Retirement");
}

#[test]
fn portfolio_blank_name_is_refused() {
    let error = Portfolio::new(PortfolioId::generate(), user(), "   ", "USD", at(0)).unwrap_err();
    assert!(matches!(error, DomainError::Required { field: "name" }));
}

#[test]
fn portfolio_long_name_is_refused() {
    let long = "x".repeat(61);
    let error = Portfolio::new(PortfolioId::generate(), user(), &long, "USD", at(0)).unwrap_err();
    assert!(matches!(error, DomainError::TooLarge { field: "name", .. }));
}

#[test]
fn portfolio_currency_is_normalised() {
    assert_eq!(normalise_currency(" usd ").unwrap(), "USD");
}

#[test]
fn portfolio_blank_currency_is_refused() {
    assert!(matches!(
        normalise_currency("  ").unwrap_err(),
        DomainError::Required {
            field: "baseCurrency"
        }
    ));
}

#[test]
fn portfolio_bad_currency_is_refused() {
    assert!(matches!(
        normalise_currency("US").unwrap_err(),
        DomainError::Invalid {
            field: "baseCurrency",
            ..
        }
    ));
    assert!(matches!(
        normalise_currency("US1").unwrap_err(),
        DomainError::Invalid {
            field: "baseCurrency",
            ..
        }
    ));
}

#[test]
fn validate_name_rejects_only_blank_and_long() {
    assert_eq!(validate_name("A").unwrap(), "A");
    assert!(validate_name("").is_err());
}

#[test]
fn portfolio_from_stored_keeps_anonymised_at() {
    let id = PortfolioId::generate();
    let value = Portfolio::from_stored(
        id,
        user(),
        "Anonymised portfolio abc".to_string(),
        "EUR".to_string(),
        at(0),
        at(1),
        Some(at(2)),
    );
    assert_eq!(value.id(), id);
    assert_eq!(value.base_currency(), "EUR");
    assert_eq!(value.anonymised_at(), Some(at(2)));
}

// ---------------------------------------------------------------------------
// Membership
// ---------------------------------------------------------------------------

#[test]
fn membership_accessors() {
    let id = PortfolioAccountId::generate();
    let owner = portfolio();
    let row = PortfolioAccount::new(id, owner.id(), AccountId::generate(), at(0));
    assert_eq!(row.id(), id);
    assert_eq!(row.portfolio_id(), owner.id());
    assert_eq!(row.added_at(), at(0));
    let stored = PortfolioAccount::from_stored(id, owner.id(), row.account_id(), at(1));
    assert_eq!(stored.added_at(), at(1));
}

// ---------------------------------------------------------------------------
// Targets
// ---------------------------------------------------------------------------

#[test]
fn target_valid() {
    let value = target("bitcoin", "50");
    assert_eq!(value.asset_id(), "bitcoin");
    assert_eq!(value.target_percent(), d("50"));
}

#[test]
fn target_negative_is_refused() {
    assert!(matches!(
        TargetAllocation::new(
            TargetAllocationId::generate(),
            PortfolioId::generate(),
            "bitcoin",
            d("-1"),
            at(0)
        )
        .unwrap_err(),
        DomainError::Invalid {
            field: "targetPercent",
            ..
        }
    ));
}

#[test]
fn target_above_hundred_is_refused() {
    assert!(TargetAllocation::new(
        TargetAllocationId::generate(),
        PortfolioId::generate(),
        "bitcoin",
        d("100.01"),
        at(0)
    )
    .is_err());
}

#[test]
fn target_with_three_decimals_is_refused() {
    assert!(TargetAllocation::new(
        TargetAllocationId::generate(),
        PortfolioId::generate(),
        "bitcoin",
        d("12.345"),
        at(0)
    )
    .is_err());
}

#[test]
fn target_blank_asset_is_refused() {
    assert!(matches!(
        TargetAllocation::new(
            TargetAllocationId::generate(),
            PortfolioId::generate(),
            "  ",
            d("10"),
            at(0)
        )
        .unwrap_err(),
        DomainError::Required { field: "assetId" }
    ));
}

#[test]
fn preview_sums_and_reports_the_under_allocation() {
    let preview = preview_targets(&[target("a", "40"), target("b", "50")]);
    assert_eq!(preview.sum, d("90"));
    assert_eq!(preview.difference, d("10"));
}

#[test]
fn preview_reports_a_negative_difference_over_hundred() {
    let preview = preview_targets(&[target("a", "70"), target("b", "50")]);
    assert_eq!(preview.sum, d("120"));
    assert_eq!(preview.difference, d("-20"));
}

#[test]
fn preview_of_no_targets_is_zero() {
    let preview = preview_targets(&[]);
    assert_eq!(preview.sum, Decimal::ZERO);
    assert_eq!(preview.difference, d("100"));
}

#[test]
fn guardrail_requires_a_matching_target() {
    let targets = vec!["bitcoin".to_string(), "ethereum".to_string()];
    assert!(require_target_for_guardrail("bitcoin", &targets).is_ok());
    assert!(matches!(
        require_target_for_guardrail("solana", &targets).unwrap_err(),
        DomainError::Invalid {
            field: "guardrailPoints",
            ..
        }
    ));
}

#[test]
fn validate_target_accepts_the_bounds() {
    assert!(validate_target("a", Decimal::ZERO).is_ok());
    assert!(validate_target("a", d("100")).is_ok());
    assert!(validate_target("  ", d("10")).is_err());
}

// ---------------------------------------------------------------------------
// Guardrails
// ---------------------------------------------------------------------------

#[test]
fn guardrail_new_is_inside() {
    let value = Guardrail::new(
        GuardrailId::generate(),
        PortfolioId::generate(),
        "bitcoin",
        d("5"),
        at(0),
    )
    .unwrap();
    assert!(!value.is_outside());
    assert!(value.last_crossing_key().is_none());
}

#[test]
fn guardrail_zero_threshold_is_refused() {
    assert!(Guardrail::new(
        GuardrailId::generate(),
        PortfolioId::generate(),
        "bitcoin",
        Decimal::ZERO,
        at(0)
    )
    .is_err());
}

#[test]
fn guardrail_negative_threshold_is_refused() {
    assert!(GuardrailDraft {
        asset_id: "bitcoin".to_string(),
        threshold_points: d("-1"),
    }
    .validate()
    .is_err());
}

#[test]
fn guardrail_three_decimal_threshold_is_refused() {
    assert!(GuardrailDraft {
        asset_id: "bitcoin".to_string(),
        threshold_points: d("1.234"),
    }
    .validate()
    .is_err());
}

#[test]
fn crossing_from_inside_creates_a_keyed_crossing() {
    let value = Guardrail::new(
        GuardrailId::generate(),
        PortfolioId::generate(),
        "bitcoin",
        d("5"),
        at(0),
    )
    .unwrap();
    let (updated, transition) = value.evaluate(d("6"), user(), at(1));
    assert!(updated.is_outside());
    assert_eq!(updated.crossed_at(), Some(at(1)));
    match transition {
        GuardrailTransition::Crossed { key } => {
            assert_eq!(updated.last_crossing_key(), Some(key.as_str()));
            assert!(key.starts_with("drift:"));
        }
        other => panic!("expected a crossing, got {other:?}"),
    }
}

#[test]
fn staying_outside_retains_the_key_and_does_not_notify() {
    let (crossed, transition) = Guardrail::new(
        GuardrailId::generate(),
        PortfolioId::generate(),
        "bitcoin",
        d("5"),
        at(0),
    )
    .unwrap()
    .evaluate(d("6"), user(), at(1));
    let key = match transition {
        GuardrailTransition::Crossed { key } => key,
        other => panic!("expected a crossing, got {other:?}"),
    };
    let (updated, transition) = crossed.evaluate(d("7"), user(), at(2));
    assert!(updated.is_outside());
    assert_eq!(updated.last_crossing_key(), Some(key.as_str()));
    assert!(matches!(transition, GuardrailTransition::Retained { .. }));
}

#[test]
fn returning_inside_clears_the_state() {
    let (crossed, _) = Guardrail::new(
        GuardrailId::generate(),
        PortfolioId::generate(),
        "bitcoin",
        d("5"),
        at(0),
    )
    .unwrap()
    .evaluate(d("6"), user(), at(1));
    let (cleared, transition) = crossed.evaluate(d("2"), user(), at(2));
    assert!(!cleared.is_outside());
    assert!(cleared.last_crossing_key().is_none());
    assert!(cleared.crossed_at().is_none());
    assert!(matches!(transition, GuardrailTransition::Cleared));
}

#[test]
fn staying_inside_is_a_no_op() {
    let (_, transition) = Guardrail::new(
        GuardrailId::generate(),
        PortfolioId::generate(),
        "bitcoin",
        d("5"),
        at(0),
    )
    .unwrap()
    .evaluate(d("2"), user(), at(1));
    assert!(matches!(transition, GuardrailTransition::None));
}

#[test]
fn drift_exactly_at_the_threshold_stays_inside() {
    let (updated, transition) = Guardrail::new(
        GuardrailId::generate(),
        PortfolioId::generate(),
        "bitcoin",
        d("5"),
        at(0),
    )
    .unwrap()
    .evaluate(d("5"), user(), at(1));
    assert!(!updated.is_outside());
    assert!(matches!(transition, GuardrailTransition::None));
}

#[test]
fn a_negative_drift_uses_its_magnitude() {
    let (updated, transition) = Guardrail::new(
        GuardrailId::generate(),
        PortfolioId::generate(),
        "bitcoin",
        d("5"),
        at(0),
    )
    .unwrap()
    .evaluate(d("-6"), user(), at(1));
    assert!(updated.is_outside());
    assert!(matches!(transition, GuardrailTransition::Crossed { .. }));
}

#[test]
fn crossing_key_is_stable_and_identifies_the_crossing() {
    let portfolio_id = portfolio().id();
    let guardrail_id = GuardrailId::generate();
    let key = crossing_key(user(), portfolio_id, "bitcoin", guardrail_id, at(1));
    assert!(key.contains(&user().to_string()));
    assert!(key.contains(&portfolio_id.to_string()));
    assert!(key.contains(&guardrail_id.to_string()));
    assert_eq!(
        key,
        crossing_key(user(), portfolio_id, "bitcoin", guardrail_id, at(1))
    );
}

#[test]
fn guardrail_from_stored_accessors() {
    let id = GuardrailId::generate();
    let value = Guardrail::from_stored(
        id,
        portfolio().id(),
        "bitcoin".to_string(),
        d("5"),
        true,
        Some("drift:key".to_string()),
        Some(at(1)),
        at(2),
    );
    assert_eq!(value.id(), id);
    assert!(value.is_outside());
    assert_eq!(value.last_crossing_key(), Some("drift:key"));
    assert_eq!(value.updated_at(), at(2));
}

// ---------------------------------------------------------------------------
// Decimal, quote, FX, and holding validation
// ---------------------------------------------------------------------------

#[test]
fn parse_decimal_accepts_plain_values() {
    assert_eq!(parse_decimal("quantity", "12.5").unwrap(), d("12.5"));
}

#[test]
fn parse_decimal_requires_a_value() {
    assert!(matches!(
        parse_decimal("quantity", "  ").unwrap_err(),
        DomainError::Required { field: "quantity" }
    ));
}

#[test]
fn parse_decimal_refuses_scientific_notation() {
    assert!(parse_decimal("quantity", "1e3").is_err());
}

#[test]
fn parse_decimal_refuses_junk() {
    assert!(parse_decimal("quantity", "abc").is_err());
}

#[test]
fn parse_decimal_refuses_more_than_eighteen_places() {
    let raw = format!("0.{}", "1".repeat(19));
    assert!(parse_decimal("quantity", &raw).is_err());
}

#[test]
fn quote_preserves_provenance() {
    let value = quote("bitcoin", "100", "USD");
    assert_eq!(value.price(), Some(d("100")));
    assert_eq!(value.currency(), Some("USD"));
    assert_eq!(value.source(), Some("test"));
    assert_eq!(value.observed_at(), Some(at(0)));
}

#[test]
fn quote_refuses_a_negative_price() {
    assert!(Quote::new(
        "bitcoin",
        Some(d("-1")),
        Some("USD".to_string()),
        None,
        None,
        Freshness::Live
    )
    .is_err());
}

#[test]
fn quote_refuses_a_blank_asset() {
    assert!(Quote::new(" ", None, None, None, None, Freshness::Unpriced).is_err());
}

#[test]
fn freshness_parses_its_labels() {
    assert_eq!(Freshness::parse("live").unwrap(), Freshness::Live);
    assert_eq!(Freshness::parse("UNPRICED").unwrap(), Freshness::Unpriced);
    assert!(Freshness::parse("ancient").is_err());
}

#[test]
fn fx_rate_requires_a_positive_rate() {
    assert!(FxRate::new("EUR", "USD", Decimal::ZERO, at(0), "test", Vec::new()).is_err());
    assert!(FxRate::new("", "USD", d("1.1"), at(0), "test", Vec::new()).is_err());
    let rate = FxRate::new("EUR", "USD", d("1.1"), at(0), "ecb", vec!["a".to_string()]).unwrap();
    assert_eq!(rate.base(), "EUR");
    assert_eq!(rate.derived_from().len(), 1);
}

#[test]
fn holding_refuses_a_negative_quantity() {
    assert!(Holding::new(None, "bitcoin", d("-1"), at(0)).is_err());
    assert!(Holding::new(None, " ", d("1"), at(0)).is_err());
}

// ---------------------------------------------------------------------------
// Valuation
// ---------------------------------------------------------------------------

#[test]
fn one_priced_holding_is_the_total() {
    let valuation = compute_valuation(
        "USD",
        &[holding("bitcoin", "2")],
        &[quote("bitcoin", "100", "USD")],
        &[],
        &[],
        true,
    );
    assert_eq!(valuation.status(), ValuationStatus::Complete);
    assert_eq!(valuation.total_value(), Some(d("200")));
    assert_eq!(valuation.priced_coverage(), d("1"));
    assert_eq!(valuation.lines()[0].allocation(), Some(d("100")));
    assert_eq!(valuation.lines()[0].data_status(), DataStatus::Priced);
}

#[test]
fn two_holdings_get_their_allocations() {
    let valuation = compute_valuation(
        "USD",
        &[holding("a", "3"), holding("b", "1")],
        &[quote("a", "100", "USD"), quote("b", "100", "USD")],
        &[],
        &[],
        true,
    );
    assert_eq!(valuation.total_value(), Some(d("400")));
    assert_eq!(valuation.line("a").unwrap().allocation(), Some(d("75")));
    assert_eq!(valuation.line("b").unwrap().allocation(), Some(d("25")));
}

#[test]
fn an_unpriced_holding_is_excluded_and_counted() {
    let valuation = compute_valuation(
        "USD",
        &[holding("a", "1"), holding("b", "5")],
        &[
            quote("a", "100", "USD"),
            Quote::new("b", None, None, None, None, Freshness::Unpriced).unwrap(),
        ],
        &[],
        &[],
        true,
    );
    assert_eq!(valuation.status(), ValuationStatus::Partial);
    assert_eq!(valuation.total_value(), Some(d("100")));
    assert_eq!(valuation.unpriced_count(), 1);
    assert_eq!(valuation.excluded_count(), 1);
    assert_eq!(valuation.priced_coverage(), d("0.5"));
    let line = valuation.line("b").unwrap();
    assert_eq!(line.data_status(), DataStatus::Unpriced);
    assert_eq!(line.quantity(), d("5"));
    assert!(line.value().is_none());
}

#[test]
fn an_unpriced_holding_is_outside_the_denominator() {
    let valuation = compute_valuation(
        "USD",
        &[holding("a", "1"), holding("b", "1")],
        &[
            quote("a", "100", "USD"),
            Quote::new("b", None, None, None, None, Freshness::Unpriced).unwrap(),
        ],
        &[],
        &[target("a", "100")],
        true,
    );
    assert_eq!(valuation.line("a").unwrap().allocation(), Some(d("100")));
    assert_eq!(
        valuation.line("a").unwrap().drift_points(),
        Some(Decimal::ZERO)
    );
}

#[test]
fn a_foreign_holding_is_converted_with_its_fx_provenance() {
    let fx = FxRate::new("EUR", "USD", d("1.1"), at(0), "ecb", Vec::new()).unwrap();
    let valuation = compute_valuation(
        "USD",
        &[holding("a", "10")],
        &[quote("a", "100", "EUR")],
        &[fx],
        &[],
        true,
    );
    assert_eq!(valuation.total_value(), Some(d("1100")));
    let line = valuation.line("a").unwrap();
    assert_eq!(line.fx_rate(), Some(d("1.1")));
    assert_eq!(line.fx_source(), Some("ecb"));
    assert_eq!(line.fx_observed_at(), Some(at(0)));
}

#[test]
fn an_inverse_fx_rate_is_used() {
    let fx = FxRate::new("USD", "EUR", d("2"), at(0), "ecb", Vec::new()).unwrap();
    let valuation = compute_valuation(
        "USD",
        &[holding("a", "10")],
        &[quote("a", "100", "EUR")],
        &[fx],
        &[],
        true,
    );
    assert_eq!(valuation.total_value(), Some(d("500")));
    assert_eq!(valuation.line("a").unwrap().fx_rate(), Some(d("0.5")));
}

#[test]
fn a_foreign_holding_without_fx_is_unpriced() {
    let valuation = compute_valuation(
        "USD",
        &[holding("a", "10")],
        &[quote("a", "100", "EUR")],
        &[],
        &[],
        true,
    );
    assert_eq!(valuation.status(), ValuationStatus::Partial);
    assert_eq!(valuation.unpriced_count(), 1);
    assert!(valuation.total_value().unwrap().is_zero());
}

#[test]
fn a_quote_without_a_currency_is_unpriced() {
    // A missing quote currency is never assumed to be the base.
    let valuation = compute_valuation(
        "USD",
        &[holding("a", "1")],
        &[Quote::new("a", Some(d("100")), None, None, None, Freshness::Live).unwrap()],
        &[],
        &[],
        true,
    );
    assert_eq!(valuation.status(), ValuationStatus::Partial);
    assert_eq!(valuation.unpriced_count(), 1);
    assert!(valuation.lines()[0].value().is_none());
}

#[test]
fn a_non_usd_base_converts_with_provenance() {
    let fx = FxRate::new("USD", "EUR", d("0.9"), at(0), "ecb", Vec::new()).unwrap();
    let valuation = compute_valuation(
        "EUR",
        &[holding("a", "1")],
        &[quote("a", "100", "USD")],
        &[fx],
        &[],
        true,
    );
    assert_eq!(valuation.currency(), "EUR");
    assert_eq!(valuation.total_value(), Some(d("90")));
    let line = valuation.line("a").unwrap();
    assert_eq!(line.fx_rate(), Some(d("0.9")));
    assert_eq!(line.fx_source(), Some("ecb"));
}

#[test]
fn drift_is_actual_minus_target_in_points() {
    let valuation = compute_valuation(
        "USD",
        &[holding("a", "6"), holding("b", "4")],
        &[quote("a", "100", "USD"), quote("b", "100", "USD")],
        &[],
        &[target("a", "50"), target("b", "50")],
        true,
    );
    assert_eq!(valuation.line("a").unwrap().allocation(), Some(d("60")));
    assert_eq!(valuation.line("a").unwrap().target_percent(), Some(d("50")));
    assert_eq!(valuation.line("a").unwrap().drift_points(), Some(d("10")));
    assert_eq!(valuation.line("b").unwrap().drift_points(), Some(d("-10")));
}

#[test]
fn no_targets_means_no_drift() {
    let valuation = compute_valuation(
        "USD",
        &[holding("a", "1")],
        &[quote("a", "100", "USD")],
        &[],
        &[],
        true,
    );
    assert!(valuation.line("a").unwrap().target_percent().is_none());
    assert!(valuation.line("a").unwrap().drift_points().is_none());
}

#[test]
fn unavailable_holdings_return_partial_with_no_fabricated_lines() {
    let valuation = compute_valuation("USD", &[], &[], &[], &[], false);
    assert_eq!(valuation.status(), ValuationStatus::Partial);
    assert!(valuation.holdings_unavailable());
    assert!(valuation.total_value().is_none());
    assert!(valuation.lines().is_empty());
    assert_eq!(valuation.priced_coverage(), Decimal::ZERO);
}

#[test]
fn no_holdings_is_a_complete_zero_total() {
    let valuation = compute_valuation("USD", &[], &[], &[], &[], true);
    assert_eq!(valuation.status(), ValuationStatus::Complete);
    assert_eq!(valuation.total_value(), Some(Decimal::ZERO));
    assert!(valuation.lines().is_empty());
}

#[test]
fn overlays_add_a_buy_and_create_a_missing_asset() {
    let overlay = TradeOverlay::new(
        TradeOverlayId::generate(),
        RecordedTradeId::generate(),
        portfolio().id(),
        None,
        "bitcoin",
        d("2"),
        at(1),
    )
    .unwrap();
    let holdings = apply_overlays(
        &[holding("bitcoin", "1")],
        std::slice::from_ref(&overlay),
        at(1),
    );
    assert_eq!(holdings[0].quantity(), d("3"));
    let holdings = apply_overlays(&[], std::slice::from_ref(&overlay), at(1));
    assert_eq!(holdings[0].asset_id(), "bitcoin");
    assert_eq!(holdings[0].quantity(), d("2"));
}

#[test]
fn an_overlay_sell_never_drives_a_holding_negative() {
    let overlay = TradeOverlay::new(
        TradeOverlayId::generate(),
        RecordedTradeId::generate(),
        portfolio().id(),
        None,
        "bitcoin",
        d("-5"),
        at(1),
    )
    .unwrap();
    let holdings = apply_overlays(&[holding("bitcoin", "2")], &[overlay], at(1));
    assert_eq!(holdings[0].quantity(), Decimal::ZERO);
}

// ---------------------------------------------------------------------------
// Order intents
// ---------------------------------------------------------------------------

fn valued_portfolio() -> portfolio_domain::Valuation {
    compute_valuation(
        "USD",
        &[holding("a", "6"), holding("b", "4")],
        &[quote("a", "100", "USD"), quote("b", "100", "USD")],
        &[],
        &[target("a", "50")],
        true,
    )
}

#[test]
fn a_buy_projects_a_higher_allocation() {
    let projection =
        project_intent(&valued_portfolio(), "a", OrderSide::Buy, d("2"), None).unwrap();
    // a becomes 800 of 1200 -> 66.7%.
    assert_eq!(projection.projected_allocation, d("66.7"));
    assert!(projection.shortfall.is_none());
}

#[test]
fn a_sell_states_the_shortfall() {
    let projection =
        project_intent(&valued_portfolio(), "a", OrderSide::Sell, d("10"), None).unwrap();
    assert_eq!(projection.shortfall, Some(d("4")));
    assert_eq!(projection.projected_allocation, Decimal::ZERO);
}

#[test]
fn a_sell_within_holdings_has_no_shortfall() {
    let projection =
        project_intent(&valued_portfolio(), "a", OrderSide::Sell, d("2"), None).unwrap();
    assert!(projection.shortfall.is_none());
}

#[test]
fn a_non_positive_intent_quantity_is_refused() {
    assert!(project_intent(
        &valued_portfolio(),
        "a",
        OrderSide::Buy,
        Decimal::ZERO,
        None
    )
    .is_err());
}

#[test]
fn a_portfolio_with_no_priced_value_cannot_project() {
    let empty = compute_valuation("USD", &[], &[], &[], &[], true);
    assert!(project_intent(&empty, "a", OrderSide::Buy, d("1"), Some(d("10"))).is_err());
}

#[test]
fn a_new_asset_without_a_price_cannot_project() {
    assert!(project_intent(&valued_portfolio(), "z", OrderSide::Buy, d("1"), None).is_err());
}

#[test]
fn a_new_asset_with_a_price_projects() {
    let projection = project_intent(
        &valued_portfolio(),
        "z",
        OrderSide::Buy,
        d("1"),
        Some(d("200")),
    )
    .unwrap();
    assert!(projection.projected_allocation > Decimal::ZERO);
}

#[test]
fn a_new_intent_is_pending() {
    let projection =
        project_intent(&valued_portfolio(), "a", OrderSide::Buy, d("1"), None).unwrap();
    let intent = OrderIntent::new(
        OrderIntentId::generate(),
        portfolio().id(),
        "a",
        OrderSide::Buy,
        d("1"),
        &projection,
        at(0),
    )
    .unwrap();
    assert_eq!(intent.status(), IntentStatus::Pending);
    assert!(intent.cancelled_at().is_none());
}

#[test]
fn cancelling_a_pending_intent_keeps_it_visible() {
    let projection =
        project_intent(&valued_portfolio(), "a", OrderSide::Buy, d("1"), None).unwrap();
    let mut intent = OrderIntent::new(
        OrderIntentId::generate(),
        portfolio().id(),
        "a",
        OrderSide::Buy,
        d("1"),
        &projection,
        at(0),
    )
    .unwrap();
    intent.cancel(at(1)).unwrap();
    assert_eq!(intent.status(), IntentStatus::Cancelled);
    assert_eq!(intent.cancelled_at(), Some(at(1)));
}

#[test]
fn cancelling_twice_is_refused() {
    let projection =
        project_intent(&valued_portfolio(), "a", OrderSide::Buy, d("1"), None).unwrap();
    let mut intent = OrderIntent::new(
        OrderIntentId::generate(),
        portfolio().id(),
        "a",
        OrderSide::Buy,
        d("1"),
        &projection,
        at(0),
    )
    .unwrap();
    intent.cancel(at(1)).unwrap();
    assert!(matches!(
        intent.cancel(at(2)).unwrap_err(),
        DomainError::InvalidTransition { .. }
    ));
}

#[test]
fn side_parses_its_labels() {
    assert_eq!(OrderSide::parse("BUY").unwrap(), OrderSide::Buy);
    assert_eq!(OrderSide::parse("sell").unwrap(), OrderSide::Sell);
    assert!(OrderSide::parse("hold").is_err());
}

// ---------------------------------------------------------------------------
// Snapshots
// ---------------------------------------------------------------------------

#[test]
fn hour_bucket_truncates_to_the_hour() {
    let value = Utc.with_ymd_and_hms(2026, 11, 7, 13, 42, 11).unwrap();
    assert_eq!(hour_bucket(value), at(13));
}

#[test]
fn snapshot_accepts_a_valid_digest() {
    let snapshot = ValuationSnapshot::new(
        ValuationSnapshotId::generate(),
        portfolio().id(),
        at(1),
        d("100"),
        "USD",
        "a".repeat(64),
        "{\"holdings\":[]}".to_string(),
        "{\"lines\":[]}".to_string(),
    )
    .unwrap();
    assert_eq!(snapshot.hour_bucket(), at(1));
    assert_eq!(snapshot.currency(), "USD");
}

#[test]
fn snapshot_refuses_a_bad_digest() {
    assert!(ValuationSnapshot::new(
        ValuationSnapshotId::generate(),
        portfolio().id(),
        at(1),
        d("100"),
        "USD",
        "ABC".to_string(),
        "{}".to_string(),
        "{}".to_string(),
    )
    .is_err());
}

#[test]
fn snapshot_refuses_blank_inputs() {
    assert!(ValuationSnapshot::new(
        ValuationSnapshotId::generate(),
        portfolio().id(),
        at(1),
        d("100"),
        "USD",
        "a".repeat(64),
        " ".to_string(),
        "{}".to_string(),
    )
    .is_err());
}

#[test]
fn snapshot_from_stored_accessors() {
    let id = ValuationSnapshotId::generate();
    let snapshot = ValuationSnapshot::from_stored(
        id,
        portfolio().id(),
        at(1),
        at(1),
        d("250"),
        "EUR".to_string(),
        "b".repeat(64),
        "{}".to_string(),
        "{}".to_string(),
    );
    assert_eq!(snapshot.id(), id);
    assert_eq!(snapshot.total_value(), d("250"));
    assert_eq!(snapshot.inputs_digest(), "b".repeat(64));
    assert_eq!(snapshot.inputs_json(), "{}");
    assert_eq!(snapshot.lines_json(), "{}");
}

// ---------------------------------------------------------------------------
// Recorded trades
// ---------------------------------------------------------------------------

#[test]
fn price_source_parses_its_labels() {
    assert_eq!(PriceSource::parse("user").unwrap(), PriceSource::User);
    assert_eq!(PriceSource::parse("VENUE").unwrap(), PriceSource::Venue);
    assert!(PriceSource::parse("market").is_err());
}

#[test]
fn a_recorded_trade_labels_the_user_price() {
    let trade = RecordedTrade::new(
        RecordedTradeId::generate(),
        portfolio().id(),
        user(),
        "bitcoin",
        OrderSide::Buy,
        d("2"),
        d("100"),
        PriceSource::User,
        at(0),
        at(1),
        "idem-1",
    )
    .unwrap();
    assert_eq!(trade.price_source(), PriceSource::User);
    assert_eq!(trade.idempotency_key(), "idem-1");
    assert_eq!(trade.quantity_delta(), d("2"));
}

#[test]
fn a_sell_trade_has_a_negative_delta() {
    let trade = RecordedTrade::new(
        RecordedTradeId::generate(),
        portfolio().id(),
        user(),
        "bitcoin",
        OrderSide::Sell,
        d("2"),
        d("100"),
        PriceSource::Venue,
        at(0),
        at(1),
        "idem-2",
    )
    .unwrap();
    assert_eq!(trade.quantity_delta(), d("-2"));
}

#[test]
fn a_recorded_trade_refuses_a_zero_quantity() {
    assert!(RecordedTrade::new(
        RecordedTradeId::generate(),
        portfolio().id(),
        user(),
        "bitcoin",
        OrderSide::Buy,
        Decimal::ZERO,
        d("100"),
        PriceSource::Venue,
        at(0),
        at(1),
        "idem",
    )
    .is_err());
}

#[test]
fn a_recorded_trade_refuses_a_negative_price() {
    assert!(RecordedTrade::new(
        RecordedTradeId::generate(),
        portfolio().id(),
        user(),
        "bitcoin",
        OrderSide::Buy,
        d("1"),
        d("-1"),
        PriceSource::Venue,
        at(0),
        at(1),
        "idem",
    )
    .is_err());
}

#[test]
fn a_recorded_trade_requires_an_idempotency_key() {
    assert!(RecordedTrade::new(
        RecordedTradeId::generate(),
        portfolio().id(),
        user(),
        "bitcoin",
        OrderSide::Buy,
        d("1"),
        d("100"),
        PriceSource::Venue,
        at(0),
        at(1),
        "  ",
    )
    .is_err());
}

#[test]
fn canonical_trade_key_identifies_asset_side_quantity_and_instant() {
    let key = canonical_trade_key("bitcoin", OrderSide::Buy, d("2"), at(0));
    assert_eq!(key, "bitcoin:buy:2:1794009600000");
    assert_ne!(
        key,
        canonical_trade_key("bitcoin", OrderSide::Sell, d("2"), at(0))
    );
}

// ---------------------------------------------------------------------------
// Service allow-list
// ---------------------------------------------------------------------------

#[test]
fn reporting_may_read_snapshots_only() {
    assert!(service_allowed("reporting", Operation::SnapshotsRead));
    assert!(!service_allowed("reporting", Operation::PrivacyExport));
}

#[test]
fn identity_may_export_and_anonymise_only() {
    assert!(service_allowed("identity", Operation::PrivacyExport));
    assert!(service_allowed("identity", Operation::PrivacyAnonymise));
    assert!(!service_allowed("identity", Operation::SnapshotsRead));
}

#[test]
fn job_orchestration_may_run_only() {
    assert!(service_allowed(
        "job-orchestration",
        Operation::RunValuation
    ));
    assert!(!service_allowed(
        "job-orchestration",
        Operation::SnapshotsRead
    ));
}

#[test]
fn an_unknown_caller_is_refused_everything() {
    assert!(!service_allowed("stranger", Operation::SnapshotsRead));
}

#[test]
fn operation_parse_round_trips() {
    for operation in [
        Operation::SnapshotsRead,
        Operation::PrivacyExport,
        Operation::PrivacyAnonymise,
        Operation::RunValuation,
    ] {
        assert_eq!(Operation::parse(operation.as_str()).unwrap(), operation);
    }
    assert!(Operation::parse("unknown").is_err());
}

// ---------------------------------------------------------------------------
// Audit
// ---------------------------------------------------------------------------

#[test]
fn an_audit_record_keeps_its_shape() {
    let record = AuditRecord::new(
        Some(user()),
        "portfolio.trade.recorded",
        "trade-1",
        AuditOutcome::Success,
        at(0),
        std::collections::BTreeMap::new(),
    )
    .unwrap();
    assert_eq!(record.action(), "portfolio.trade.recorded");
    assert_eq!(record.target(), "trade-1");
    assert_eq!(record.outcome().as_str(), "success");
}

#[test]
fn an_audit_record_requires_an_action() {
    assert!(AuditRecord::new(
        None,
        " ",
        "target",
        AuditOutcome::Failed,
        at(0),
        std::collections::BTreeMap::new(),
    )
    .is_err());
}
