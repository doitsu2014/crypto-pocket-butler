//! Domain tests: freshness tiers, observations, served quotes, FX rates, refresh
//! runs and leases, rate buckets, audit records, and the service-token
//! allow-list. Every test is deterministic: time is supplied, never read.

use chrono::{Duration, TimeZone, Utc};
use market_data_domain::audit::{AuditOutcome, AuditRecord};
use market_data_domain::constants::{
    CLOCK_TOLERANCE_SECONDS, FX_BOUND_SECONDS, HARD_BOUND_SECONDS, MAX_ASSET_IDS,
    MAX_ERROR_MESSAGE_CHARS, RETENTION_DAYS, RUN_LEASE_SECONDS, TAIL_BOUND_SECONDS,
    TOP100_BOUND_SECONDS,
};
use market_data_domain::freshness::{classify_freshness, Freshness};
use market_data_domain::fx::{normalise_currency, FxRate, ServedFxRate};
use market_data_domain::ids::{FxRateId, ObservationId, RefreshRunId};
use market_data_domain::price::{parse_price, ObservationStatus, PriceObservation, PriceQuote};
use market_data_domain::rate::{bucket_try_take, refill_tokens};
use market_data_domain::run::{ErrorCategory, RefreshJob, RefreshRun, RunStatus, Trigger};
use market_data_domain::secrets::{looks_like_secret, sanitise_message};
use market_data_domain::service::Operation;
use market_data_domain::tier::{bound_for_rank, validate_asset_id, AssetPriceTier, PriceTier};
use market_data_domain::DomainError;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::str::FromStr;

fn at(minute: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, 10, minute, 0).unwrap()
}

fn decimal(raw: &str) -> Decimal {
    Decimal::from_str(raw).unwrap()
}

fn observation(price: &str, observed_at: chrono::DateTime<Utc>) -> PriceObservation {
    PriceObservation::accept(
        ObservationId::generate(),
        "asset-btc",
        "binance",
        decimal(price),
        "usd",
        observed_at,
        at(10),
    )
    .expect("a valid observation")
}

// ---------------------------------------------------------------------------
// Tiers and constants
// ---------------------------------------------------------------------------

#[test]
fn the_rules_constants_match_the_approved_design() {
    assert_eq!(TOP100_BOUND_SECONDS, 300);
    assert_eq!(TAIL_BOUND_SECONDS, 900);
    assert_eq!(HARD_BOUND_SECONDS, 86_400);
    assert_eq!(CLOCK_TOLERANCE_SECONDS, 300);
    assert_eq!(FX_BOUND_SECONDS, 3_600);
    assert_eq!(RUN_LEASE_SECONDS, 300);
    assert_eq!(RETENTION_DAYS, 90);
    assert_eq!(MAX_ASSET_IDS, 1_000);
}

#[test]
fn rank_one_to_one_hundred_is_top100_and_the_rest_is_tail() {
    assert_eq!(bound_for_rank(1), TOP100_BOUND_SECONDS);
    assert_eq!(bound_for_rank(100), TOP100_BOUND_SECONDS);
    assert_eq!(bound_for_rank(101), TAIL_BOUND_SECONDS);
    assert_eq!(bound_for_rank(2_000), TAIL_BOUND_SECONDS);
    assert_eq!(PriceTier::Top100.bound_seconds(), 300);
    assert_eq!(PriceTier::Tail.bound_seconds(), 900);
}

#[test]
fn a_tier_row_carries_the_rank_and_derived_tier() {
    let tier = AssetPriceTier::for_rank("asset-btc", 42, at(10)).unwrap();
    assert_eq!(tier.asset_id(), "asset-btc");
    assert_eq!(tier.rank(), 42);
    assert_eq!(tier.tier(), PriceTier::Top100);
    assert_eq!(tier.bound_seconds(), 300);
    assert_eq!(tier.updated_at(), at(10));
}

#[test]
fn a_tail_tier_row_uses_the_tail_bound() {
    let tier = AssetPriceTier::for_rank("asset-longtail", 101, at(10)).unwrap();
    assert_eq!(tier.tier(), PriceTier::Tail);
    assert_eq!(tier.bound_seconds(), 900);
}

#[test]
fn a_tier_row_requires_a_valid_asset_id_and_rank() {
    assert!(matches!(
        AssetPriceTier::for_rank(" ", 1, at(10)),
        Err(DomainError::Required { field: "assetId" })
    ));
    assert!(matches!(
        AssetPriceTier::for_rank("asset-btc", 0, at(10)),
        Err(DomainError::Invalid { field: "rank", .. })
    ));
    assert!(matches!(
        PriceTier::parse("mid"),
        Err(DomainError::Invalid { field: "tier", .. })
    ));
    assert_eq!(PriceTier::parse("top100").unwrap(), PriceTier::Top100);
    assert_eq!(PriceTier::parse("tail").unwrap(), PriceTier::Tail);
}

#[test]
fn asset_ids_are_trimmed_and_bounded() {
    assert_eq!(validate_asset_id(" asset-btc ").unwrap(), "asset-btc");
    assert!(matches!(
        validate_asset_id(""),
        Err(DomainError::Required { field: "assetId" })
    ));
    let long = "x".repeat(201);
    assert!(matches!(
        validate_asset_id(&long),
        Err(DomainError::TooLarge {
            field: "assetId",
            ..
        })
    ));
    assert!(validate_asset_id(&"x".repeat(200)).is_ok());
}

// ---------------------------------------------------------------------------
// Freshness
// ---------------------------------------------------------------------------

#[test]
fn an_observation_inside_its_bound_is_live() {
    assert_eq!(classify_freshness(0, 300, false, true), Freshness::Live);
    assert_eq!(classify_freshness(300, 300, false, true), Freshness::Live);
    assert_eq!(classify_freshness(299, 300, false, true), Freshness::Live);
}

#[test]
fn a_delayed_feed_inside_its_bound_is_delayed_before_live() {
    assert_eq!(classify_freshness(0, 300, true, true), Freshness::Delayed);
    assert_eq!(classify_freshness(300, 300, true, true), Freshness::Delayed);
}

#[test]
fn an_observation_beyond_its_bound_is_stale_within_the_hard_bound() {
    assert_eq!(classify_freshness(301, 300, false, true), Freshness::Stale);
    assert_eq!(classify_freshness(900, 900, true, true), Freshness::Delayed);
    assert_eq!(
        classify_freshness(HARD_BOUND_SECONDS, 300, false, true),
        Freshness::Stale
    );
}

#[test]
fn an_observation_beyond_the_hard_bound_is_unpriced() {
    assert_eq!(
        classify_freshness(HARD_BOUND_SECONDS + 1, 300, false, true),
        Freshness::Unpriced
    );
    assert_eq!(
        classify_freshness(0, 300, false, false),
        Freshness::Unpriced
    );
}

#[test]
fn freshness_labels_round_trip() {
    for freshness in [
        Freshness::Live,
        Freshness::Delayed,
        Freshness::Stale,
        Freshness::Unpriced,
    ] {
        assert_eq!(Freshness::parse(freshness.as_str()).unwrap(), freshness);
    }
    assert!(Freshness::parse("fresh").is_err());
}

// ---------------------------------------------------------------------------
// Observations and quotes
// ---------------------------------------------------------------------------

#[test]
fn price_strings_are_parsed_as_plain_positive_decimals() {
    assert_eq!(
        parse_price(" 123.45678901 ").unwrap(),
        decimal("123.45678901")
    );
    assert!(matches!(
        parse_price(""),
        Err(DomainError::Required { field: "price" })
    ));
    assert!(matches!(
        parse_price("1e5"),
        Err(DomainError::Invalid { .. })
    ));
    assert!(matches!(
        parse_price("not-a-number"),
        Err(DomainError::Invalid { .. })
    ));
    assert!(matches!(
        parse_price("0.0000000000000000001"),
        Err(DomainError::Invalid { .. })
    ));
}

#[test]
fn an_accepted_observation_normalises_the_currency() {
    let observation = observation("123.45", at(9));
    assert_eq!(observation.currency(), "USD");
    assert_eq!(observation.status(), ObservationStatus::Accepted);
    assert!(observation.is_accepted());
    assert_eq!(observation.rejection_reason(), None);
}

#[test]
fn a_positive_price_within_clock_tolerance_is_accepted() {
    let observation = PriceObservation::evaluate(
        ObservationId::generate(),
        "asset-eth",
        "coingecko",
        "2000.00",
        "USD",
        at(10) + Duration::seconds(CLOCK_TOLERANCE_SECONDS),
        at(10),
    )
    .unwrap();
    assert!(observation.is_accepted());
}

#[test]
fn a_non_positive_reading_is_rejected_and_kept_for_diagnosis() {
    let zero = PriceObservation::evaluate(
        ObservationId::generate(),
        "asset-btc",
        "binance",
        "0",
        "USD",
        at(9),
        at(10),
    )
    .unwrap();
    assert_eq!(zero.status(), ObservationStatus::Rejected);
    assert_eq!(zero.price(), Decimal::ZERO);
    assert_eq!(zero.rejection_reason(), Some("non-positive price"));

    let negative = PriceObservation::evaluate(
        ObservationId::generate(),
        "asset-btc",
        "binance",
        "-1.5",
        "USD",
        at(9),
        at(10),
    )
    .unwrap();
    assert!(!negative.is_accepted());
    assert!(negative.rejection_reason().is_some());
}

#[test]
fn a_malformed_reading_is_rejected_not_accepted() {
    let malformed = PriceObservation::evaluate(
        ObservationId::generate(),
        "asset-btc",
        "binance",
        "NaN",
        "USD",
        at(9),
        at(10),
    )
    .unwrap();
    assert!(!malformed.is_accepted());
    assert_eq!(malformed.rejection_reason(), Some("malformed price"));
    assert_eq!(malformed.price(), Decimal::ZERO);
}

#[test]
fn a_reading_more_than_five_minutes_ahead_is_rejected() {
    let skewed = PriceObservation::evaluate(
        ObservationId::generate(),
        "asset-btc",
        "binance",
        "100",
        "USD",
        at(10) + Duration::seconds(CLOCK_TOLERANCE_SECONDS + 1),
        at(10),
    )
    .unwrap();
    assert!(!skewed.is_accepted());
    assert_eq!(
        skewed.rejection_reason(),
        Some("clock skew beyond tolerance")
    );
}

#[test]
fn an_observation_rejects_blank_identity_fields() {
    assert!(matches!(
        PriceObservation::accept(
            ObservationId::generate(),
            " ",
            "binance",
            decimal("1"),
            "USD",
            at(9),
            at(10)
        ),
        Err(DomainError::Required { field: "assetId" })
    ));
    assert!(matches!(
        PriceObservation::accept(
            ObservationId::generate(),
            "asset-btc",
            " ",
            decimal("1"),
            "USD",
            at(9),
            at(10)
        ),
        Err(DomainError::Required {
            field: "sourceName"
        })
    ));
    assert!(matches!(
        PriceObservation::accept(
            ObservationId::generate(),
            "asset-btc",
            "binance",
            decimal("1"),
            "US",
            at(9),
            at(10)
        ),
        Err(DomainError::Invalid {
            field: "currency",
            ..
        })
    ));
}

#[test]
fn a_rejection_reason_never_carries_secret_shaped_text() {
    let secret = format!("0x{}", "a".repeat(64));
    let rejected = PriceObservation::reject(
        ObservationId::generate(),
        "asset-btc",
        "binance",
        decimal("1"),
        "USD",
        at(9),
        at(10),
        &secret,
    )
    .unwrap();
    assert_eq!(rejected.rejection_reason(), Some("reading rejected"));
}

#[test]
fn a_priced_quote_carries_price_source_and_time() {
    let observation = observation("123.45", at(9));
    let quote = PriceQuote::priced("asset-btc", &observation, Freshness::Live).unwrap();
    assert_eq!(quote.price(), Some(decimal("123.45")));
    assert_eq!(quote.source(), Some("binance"));
    assert_eq!(quote.observed_at(), Some(at(9)));
    assert_eq!(quote.freshness(), Freshness::Live);
    assert_eq!(quote.currency(), Some("USD"));
}

#[test]
fn an_unpriced_quote_carries_no_price_source_or_time() {
    let quote = PriceQuote::unpriced("asset-btc").unwrap();
    assert_eq!(quote.freshness(), Freshness::Unpriced);
    assert_eq!(quote.price(), None);
    assert_eq!(quote.source(), None);
    assert_eq!(quote.observed_at(), None);
    assert_eq!(quote.currency(), None);
}

#[test]
fn a_priced_quote_refuses_a_rejected_observation_or_unpriced_label() {
    let rejected = PriceObservation::reject(
        ObservationId::generate(),
        "asset-btc",
        "binance",
        decimal("1"),
        "USD",
        at(9),
        at(10),
        "malformed",
    )
    .unwrap();
    assert!(PriceQuote::priced("asset-btc", &rejected, Freshness::Live).is_err());
    let accepted = observation("1", at(9));
    assert!(PriceQuote::priced("asset-btc", &accepted, Freshness::Unpriced).is_err());
}

// ---------------------------------------------------------------------------
// FX
// ---------------------------------------------------------------------------

fn fx(base: &str, quote: &str, rate: &str, observed_at: chrono::DateTime<Utc>) -> FxRate {
    FxRate::record(
        FxRateId::generate(),
        base,
        quote,
        decimal(rate),
        "binance",
        observed_at,
        at(10),
    )
    .expect("a valid rate")
}

#[test]
fn currency_codes_are_three_uppercase_letters_or_token_symbols() {
    assert_eq!(normalise_currency(" eur ").unwrap(), "EUR");
    assert_eq!(normalise_currency("usdt").unwrap(), "USDT");
    assert!(matches!(
        normalise_currency("EU"),
        Err(DomainError::Invalid {
            field: "currency",
            ..
        })
    ));
    assert!(matches!(
        normalise_currency("EU1"),
        Err(DomainError::Invalid { .. })
    ));
    assert!(matches!(
        normalise_currency(""),
        Err(DomainError::Required { field: "currency" })
    ));
}

#[test]
fn an_fx_rate_requires_a_positive_decimal_and_distinct_currencies() {
    let rate = fx("EUR", "USD", "1.0875", at(9));
    assert_eq!(rate.rate(), decimal("1.0875"));
    assert_eq!(rate.base(), "EUR");
    assert_eq!(rate.quote(), "USD");
    assert!(matches!(
        FxRate::record(
            FxRateId::generate(),
            "EUR",
            "EUR",
            decimal("1"),
            "binance",
            at(9),
            at(10)
        ),
        Err(DomainError::Invalid { field: "base", .. })
    ));
    assert!(matches!(
        FxRate::record(
            FxRateId::generate(),
            "EUR",
            "USD",
            Decimal::ZERO,
            "binance",
            at(9),
            at(10)
        ),
        Err(DomainError::Invalid { field: "rate", .. })
    ));
    assert!(matches!(
        FxRate::record(
            FxRateId::generate(),
            "EUR",
            "USD",
            decimal("-1"),
            "binance",
            at(9),
            at(10)
        ),
        Err(DomainError::Invalid { field: "rate", .. })
    ));
}

#[test]
fn a_direct_fx_reading_serves_its_source_and_time() {
    let rate = fx("EUR", "USD", "1.0875", at(9));
    let served = ServedFxRate::from_direct(&rate);
    assert_eq!(served.rate(), decimal("1.0875"));
    assert_eq!(served.source(), "binance");
    assert_eq!(served.observed_at(), at(9));
    assert!(served.derived_from().is_empty());
}

#[test]
fn a_cross_rate_multiplies_two_legs_and_names_both_sources() {
    let first = fx("EUR", "USD", "1.10", at(8));
    let second = fx("USD", "GBP", "0.80", at(9));
    let served = ServedFxRate::derived("EUR", "GBP", &first, &second).unwrap();
    assert_eq!(served.rate(), decimal("0.88"));
    assert_eq!(served.source(), "derived");
    assert_eq!(served.observed_at(), at(8));
    assert_eq!(served.derived_from().len(), 2);
}

#[test]
fn a_cross_rate_refuses_legs_that_do_not_join_through_usd() {
    let first = fx("EUR", "USD", "1.10", at(8));
    let second = fx("GBP", "JPY", "190", at(9));
    assert!(ServedFxRate::derived("EUR", "JPY", &first, &second).is_err());
}

// ---------------------------------------------------------------------------
// Refresh jobs and leases
// ---------------------------------------------------------------------------

#[test]
fn jobs_round_trip_and_expose_their_contract_names() {
    assert_eq!(RefreshJob::Top100.job_name(), "price-refresh-top100");
    assert_eq!(RefreshJob::Tail.job_name(), "price-refresh-tail");
    assert_eq!(
        RefreshJob::parse_job_name("price-refresh-top100"),
        Some(RefreshJob::Top100)
    );
    assert_eq!(
        RefreshJob::parse_job_name("price-refresh-tail"),
        Some(RefreshJob::Tail)
    );
    assert_eq!(RefreshJob::parse_job_name("price-refresh-all"), None);
    assert_eq!(RefreshJob::Top100.deadline_seconds(), 180);
    assert_eq!(RefreshJob::Tail.deadline_seconds(), 600);
    assert_eq!(RefreshJob::parse("top100").unwrap(), RefreshJob::Top100);
    assert_eq!(RefreshJob::parse("tail").unwrap(), RefreshJob::Tail);
}

#[test]
fn triggers_round_trip() {
    assert_eq!(Trigger::Scheduled.as_str(), "scheduled");
    assert_eq!(Trigger::Manual.as_str(), "manual");
    assert_eq!(Trigger::parse("manual").unwrap(), Trigger::Manual);
    assert_eq!(Trigger::parse("scheduled").unwrap(), Trigger::Scheduled);
    assert!(Trigger::parse("cron").is_err());
}

#[test]
fn a_claimed_run_is_running_with_a_live_lease() {
    let run = RefreshRun::claim(RefreshRunId::generate(), RefreshJob::Top100, at(10));
    assert_eq!(run.status(), RunStatus::Running);
    assert_eq!(
        run.lease_until(),
        at(10) + Duration::seconds(RUN_LEASE_SECONDS)
    );
    assert!(run.is_live(at(10)));
    assert!(!run.is_live(at(10) + Duration::seconds(RUN_LEASE_SECONDS)));
    assert_eq!(run.requested_count(), 0);
}

#[test]
fn renewal_extends_a_live_lease_and_refuses_an_expired_one() {
    let mut run = RefreshRun::claim(RefreshRunId::generate(), RefreshJob::Tail, at(10));
    assert!(run.renew(at(10) + Duration::seconds(60)));
    assert_eq!(run.lease_until(), at(10) + Duration::seconds(360));
    assert!(!run.renew(at(10) + Duration::seconds(400)));
}

#[test]
fn an_expired_running_run_is_failed_by_the_sweep() {
    let mut run = RefreshRun::claim(RefreshRunId::generate(), RefreshJob::Top100, at(10));
    run.record_counts(4, 3, 1).unwrap();
    assert!(run.expire_if_due(at(10) + Duration::seconds(301)));
    assert_eq!(run.status(), RunStatus::Failed);
    assert_eq!(run.error_category(), Some(ErrorCategory::Data));
    assert_eq!(run.error_message(), Some("lease expired"));
    assert!(!run.expire_if_due(at(11)));
}

#[test]
fn a_successful_run_reports_reconciling_counts() {
    let mut run = RefreshRun::claim(RefreshRunId::generate(), RefreshJob::Top100, at(10));
    run.record_counts(5, 4, 1).unwrap();
    run.succeed(at(11)).unwrap();
    assert_eq!(run.status(), RunStatus::Succeeded);
    assert_eq!(run.requested_count(), 5);
    assert_eq!(run.priced_count(), 4);
    assert_eq!(run.unpriced_count(), 1);
    assert_eq!(run.finished_at(), Some(at(11)));
}

#[test]
fn counts_that_do_not_add_up_are_refused() {
    let mut run = RefreshRun::claim(RefreshRunId::generate(), RefreshJob::Tail, at(10));
    assert!(matches!(
        run.record_counts(5, 4, 0),
        Err(DomainError::Invalid {
            field: "requestedCount",
            ..
        })
    ));
}

#[test]
fn a_failed_run_stores_a_bounded_non_secret_reason() {
    let mut run = RefreshRun::claim(RefreshRunId::generate(), RefreshJob::Tail, at(10));
    let long = "x".repeat(MAX_ERROR_MESSAGE_CHARS + 10);
    run.fail(at(11), ErrorCategory::Provider, &long).unwrap();
    assert_eq!(run.status(), RunStatus::Failed);
    assert_eq!(
        run.error_message().unwrap().chars().count(),
        MAX_ERROR_MESSAGE_CHARS
    );
    let secret = format!("0x{}", "a".repeat(64));
    let mut other = RefreshRun::claim(RefreshRunId::generate(), RefreshJob::Top100, at(10));
    other.fail(at(11), ErrorCategory::Data, &secret).unwrap();
    assert_eq!(other.error_message(), Some("refresh failed"));
}

#[test]
fn a_finished_run_cannot_finish_again() {
    let mut run = RefreshRun::claim(RefreshRunId::generate(), RefreshJob::Top100, at(10));
    run.succeed(at(11)).unwrap();
    assert!(matches!(
        run.fail(at(12), ErrorCategory::Data, "again"),
        Err(DomainError::InvalidTransition { .. })
    ));
}

#[test]
fn statuses_and_categories_round_trip() {
    for status in [RunStatus::Running, RunStatus::Succeeded, RunStatus::Failed] {
        assert_eq!(RunStatus::parse(status.as_str()).unwrap(), status);
    }
    for category in [
        ErrorCategory::RateLimit,
        ErrorCategory::Provider,
        ErrorCategory::Network,
        ErrorCategory::Data,
    ] {
        assert_eq!(ErrorCategory::parse(category.as_str()).unwrap(), category);
    }
    assert!(RunStatus::parse("paused").is_err());
    assert!(ErrorCategory::parse("auth").is_err());
}

// ---------------------------------------------------------------------------
// Rate buckets, secrets, audit, and service identity
// ---------------------------------------------------------------------------

#[test]
fn a_bucket_refills_at_the_configured_rate_and_never_exceeds_capacity() {
    assert_eq!(refill_tokens(0, 60, 0), 0);
    assert_eq!(refill_tokens(0, 60, 60), 60);
    assert_eq!(refill_tokens(0, 60, 3_600), 60);
    assert_eq!(refill_tokens(50, 60, 60), 60);
    assert_eq!(refill_tokens(10, 60, 30), 40);
    assert_eq!(refill_tokens(10, 60, -5), 10);
}

#[test]
fn taking_a_token_succeeds_only_when_one_is_available() {
    assert_eq!(bucket_try_take(5, 60, 0), (true, 4));
    assert_eq!(bucket_try_take(0, 60, 0), (false, 0));
    assert_eq!(bucket_try_take(0, 60, 60), (true, 59));
}

#[test]
fn secret_detection_and_sanitisation_never_leak_key_material() {
    assert!(looks_like_secret(&format!("0x{}", "a".repeat(64))));
    assert!(looks_like_secret(&"f".repeat(64)));
    assert!(!looks_like_secret("provider answered 500"));
    let secret = format!("0x{}", "b".repeat(64));
    assert_eq!(
        sanitise_message(&secret, "refresh failed"),
        "refresh failed"
    );
    assert_eq!(
        sanitise_message("  network down  ", "refresh failed"),
        "network down"
    );
    assert_eq!(sanitise_message("   ", "refresh failed"), "refresh failed");
}

#[test]
fn an_audit_record_carries_the_required_fields() {
    let mut context = BTreeMap::new();
    context.insert("job".to_string(), "top100".to_string());
    let record = AuditRecord::new(
        "system",
        None,
        "price_refresh_failed",
        "run-1",
        AuditOutcome::Failed,
        at(10),
        context,
    )
    .unwrap();
    assert_eq!(record.actor_kind(), "system");
    assert_eq!(record.action(), "price_refresh_failed");
    assert_eq!(record.target(), "run-1");
    assert_eq!(record.outcome(), AuditOutcome::Failed);
    assert_eq!(record.context().get("job").unwrap(), "top100");
}

#[test]
fn an_audit_record_requires_an_action_and_target() {
    assert!(matches!(
        AuditRecord::new(
            "system",
            None,
            " ",
            "run-1",
            AuditOutcome::Failed,
            at(10),
            BTreeMap::new()
        ),
        Err(DomainError::Required { field: "action" })
    ));
    assert!(matches!(
        AuditRecord::new(
            "system",
            None,
            "price_refresh_failed",
            " ",
            AuditOutcome::Failed,
            at(10),
            BTreeMap::new()
        ),
        Err(DomainError::Required { field: "target" })
    ));
}

#[test]
fn operation_labels_round_trip() {
    for operation in [
        Operation::PricesRead,
        Operation::FxRead,
        Operation::RunRefresh,
    ] {
        assert_eq!(Operation::parse(operation.as_str()).unwrap(), operation);
    }
    assert!(Operation::parse("delete_everything").is_err());
}

#[test]
fn ids_parse_and_reject_bad_text() {
    let id = ObservationId::generate();
    assert_eq!(ObservationId::parse(&id.to_string()).unwrap(), id);
    assert!(matches!(
        ObservationId::parse("not-a-uuid"),
        Err(DomainError::InvalidId {
            entity: "price observation"
        })
    ));
    assert!(matches!(
        FxRateId::parse("not-a-uuid"),
        Err(DomainError::InvalidId { entity: "fx rate" })
    ));
    assert!(RefreshRunId::parse("not-a-uuid").is_err());
}
