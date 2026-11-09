//! Domain tests: canonical identity, FIFO, fees, transfers, P&L coverage, and
//! corrections.

use std::collections::BTreeMap;

use chrono::{DateTime, TimeZone, Utc};
use ledger_domain::{
    canonical_key, compute_pnl, consume_fifo, effective_transaction, effective_value, find_match,
    parse_field_value, replay, AssetPnl, Correction, CorrectionField, CostBasisLot, CostBasisLotId,
    CoverageLabel, DomainError, EffectiveTransaction, FieldValue, FxRates, HeldAsset, Kind,
    LedgerSubject, LedgerSubjectId, NewTransaction, PnlSnapshot, PnlSnapshotId, PriceFreshness,
    PriceMark, PriceSource, Source, Transaction, TransactionId, TransferPair, TransferPairId,
    TransferStatus,
};
use rust_decimal::Decimal;
use uuid::Uuid;

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 6, hour, 0, 0).unwrap()
}

fn dec(raw: &str) -> Decimal {
    raw.parse().unwrap()
}

fn fx() -> FxRates {
    FxRates::base_only("USD")
}

fn marks(pairs: &[(&str, &str)]) -> BTreeMap<String, PriceMark> {
    pairs
        .iter()
        .map(|(asset, price)| {
            (
                (*asset).to_string(),
                PriceMark {
                    price: Some(dec(price)),
                    currency: Some("USD".to_string()),
                    as_of: Some(at(12)),
                    freshness: PriceFreshness::Live,
                },
            )
        })
        .collect()
}

fn holdings(pairs: &[(&str, &str)]) -> Vec<HeldAsset> {
    pairs
        .iter()
        .map(|(asset, quantity)| HeldAsset {
            asset_id: (*asset).to_string(),
            quantity: dec(quantity),
        })
        .collect()
}

fn subject() -> LedgerSubjectId {
    LedgerSubjectId::new(Uuid::from_u128(9))
}

fn id(value: u128) -> TransactionId {
    TransactionId::new(Uuid::from_u128(value))
}

fn record(
    value: u128,
    asset: &str,
    kind: Kind,
    quantity: &str,
    price: Option<&str>,
    hour: u32,
) -> Transaction {
    record_in(value, asset, kind, quantity, price, "USD", hour)
}

fn record_in(
    value: u128,
    asset: &str,
    kind: Kind,
    quantity: &str,
    price: Option<&str>,
    currency: &str,
    hour: u32,
) -> Transaction {
    Transaction::record(
        id(value),
        NewTransaction {
            subject_id: subject(),
            user_id: "user-1".to_string(),
            account_id: None,
            asset_id: asset.to_string(),
            kind,
            quantity: dec(quantity),
            price: price.map(dec),
            price_source: price.map(|_| PriceSource::Venue),
            currency: currency.to_string(),
            source: Source::Venue,
            external_ref: None,
            executed_at: at(hour),
            recorded_at: at(23),
        },
    )
    .unwrap()
}

fn effective(transaction: &Transaction) -> EffectiveTransaction {
    effective_transaction(transaction, &[] as &[Correction]).unwrap()
}

fn correction(field: CorrectionField, new_value: &str, hour: u32) -> Correction {
    Correction::record(
        ledger_domain::CorrectionId::new(Uuid::from_u128(100 + u128::from(hour))),
        id(1),
        field,
        "old",
        new_value,
        "fix a typo",
        "user-1",
        at(hour),
    )
    .unwrap()
}

// --- Canonical identity (BR1.2) -------------------------------------------

#[test]
fn canonical_key_is_the_same_across_sources_and_ignores_seconds() {
    let from_venue = record(1, "BTC", Kind::Buy, "1.5", Some("100.00"), 10);
    let mut from_csv = record(2, "BTC", Kind::Buy, "1.50", Some("100"), 10);
    from_csv = Transaction::from_stored(
        from_csv.id(),
        subject(),
        None,
        "BTC".to_string(),
        Kind::Buy,
        dec("1.5"),
        Some(dec("100")),
        Some(PriceSource::Csv),
        "USD".to_string(),
        Source::Csv,
        Some("row-1".to_string()),
        from_csv.canonical_key().to_string(),
        Some("user-1|csv|row-1".to_string()),
        at(10),
        at(23),
    );
    assert_eq!(from_venue.canonical_key(), from_csv.canonical_key());
}

#[test]
fn canonical_key_rounds_executed_at_to_the_minute() {
    let mut first = record(1, "BTC", Kind::Buy, "1", Some("100"), 10);
    let second = record(2, "BTC", Kind::Buy, "1", Some("100"), 10);
    first = Transaction::from_stored(
        first.id(),
        subject(),
        None,
        "BTC".to_string(),
        Kind::Buy,
        dec("1"),
        Some(dec("100")),
        Some(PriceSource::Venue),
        "USD".to_string(),
        Source::Venue,
        None,
        first.canonical_key().to_string(),
        None,
        at(10) + chrono::Duration::seconds(45),
        at(23),
    );
    assert_eq!(
        canonical_key(
            "user-1",
            "BTC",
            Kind::Buy,
            dec("1"),
            Some(dec("100")),
            at(10)
        ),
        canonical_key(
            "user-1",
            "BTC",
            Kind::Buy,
            dec("1"),
            Some(dec("100")),
            at(10) + chrono::Duration::seconds(45)
        )
    );
    assert_eq!(first.canonical_key(), second.canonical_key());
}

#[test]
fn external_key_includes_the_source_and_the_reference() {
    let transaction = Transaction::record(
        id(1),
        NewTransaction {
            subject_id: subject(),
            user_id: "user-1".to_string(),
            account_id: None,
            asset_id: "BTC".to_string(),
            kind: Kind::Buy,
            quantity: dec("1"),
            price: Some(dec("100")),
            price_source: Some(PriceSource::Venue),
            currency: "USD".to_string(),
            source: Source::Csv,
            external_ref: Some(" row-1 ".to_string()),
            executed_at: at(10),
            recorded_at: at(23),
        },
    )
    .unwrap();
    assert_eq!(transaction.external_key(), Some("user-1|csv|row-1"));
    assert_eq!(transaction.external_ref(), Some("row-1"));
}

// --- Transaction validation (BR1.3, NFR1.7) -------------------------------

#[test]
fn a_transaction_rejects_a_zero_quantity() {
    let error = Transaction::record(
        id(1),
        NewTransaction {
            subject_id: subject(),
            user_id: "user-1".to_string(),
            account_id: None,
            asset_id: "BTC".to_string(),
            kind: Kind::Buy,
            quantity: Decimal::ZERO,
            price: Some(dec("100")),
            price_source: Some(PriceSource::Venue),
            currency: "USD".to_string(),
            source: Source::Venue,
            external_ref: None,
            executed_at: at(10),
            recorded_at: at(23),
        },
    )
    .unwrap_err();
    assert_eq!(error.field(), "quantity");
}

#[test]
fn a_transaction_rejects_a_negative_price() {
    let error = Transaction::record(
        id(1),
        NewTransaction {
            subject_id: subject(),
            user_id: "user-1".to_string(),
            account_id: None,
            asset_id: "BTC".to_string(),
            kind: Kind::Buy,
            quantity: dec("1"),
            price: Some(dec("-1")),
            price_source: Some(PriceSource::Venue),
            currency: "USD".to_string(),
            source: Source::Venue,
            external_ref: None,
            executed_at: at(10),
            recorded_at: at(23),
        },
    )
    .unwrap_err();
    assert_eq!(error.field(), "price");
}

#[test]
fn a_missing_price_is_allowed_and_reported_as_missing() {
    let transaction = record(1, "BTC", Kind::Buy, "1", None, 10);
    assert!(transaction.price_missing());
    assert_eq!(transaction.price(), None);
}

#[test]
fn a_future_timestamp_beyond_the_skew_is_clamped() {
    let transaction = Transaction::record(
        id(1),
        NewTransaction {
            subject_id: subject(),
            user_id: "user-1".to_string(),
            account_id: None,
            asset_id: "BTC".to_string(),
            kind: Kind::Buy,
            quantity: dec("1"),
            price: Some(dec("100")),
            price_source: Some(PriceSource::Venue),
            currency: "USD".to_string(),
            source: Source::Venue,
            external_ref: None,
            executed_at: at(23) + chrono::Duration::hours(1),
            recorded_at: at(10),
        },
    )
    .unwrap();
    assert_eq!(transaction.executed_at(), at(10));
}

// --- Corrections (BR2.3, BR4.1) -------------------------------------------

#[test]
fn quantity_correction_requires_a_positive_decimal() {
    assert!(matches!(
        parse_field_value(CorrectionField::Quantity, "0", at(10)),
        Err(DomainError::Invalid {
            field: "newValue",
            ..
        })
    ));
    assert!(matches!(
        parse_field_value(CorrectionField::Quantity, "2.5", at(10)),
        Ok(FieldValue::Quantity(value)) if value == dec("2.5")
    ));
}

#[test]
fn price_correction_rejects_a_negative_value() {
    assert!(matches!(
        parse_field_value(CorrectionField::Price, "-0.5", at(10)),
        Err(DomainError::Invalid {
            field: "newValue",
            ..
        })
    ));
}

#[test]
fn kind_correction_rejects_an_unknown_kind() {
    assert!(matches!(
        parse_field_value(CorrectionField::Kind, "gift", at(10)),
        Err(DomainError::Invalid { field: "kind", .. })
    ));
}

#[test]
fn executed_at_correction_rejects_a_future_value() {
    assert!(matches!(
        parse_field_value(CorrectionField::ExecutedAt, &at(22).to_rfc3339(), at(10)),
        Err(DomainError::Invalid {
            field: "newValue",
            ..
        })
    ));
}

#[test]
fn old_value_is_the_value_in_force_after_earlier_corrections() {
    let transaction = record(1, "BTC", Kind::Buy, "1", Some("100"), 10);
    let first = correction(CorrectionField::Quantity, "2", 11);
    let second = correction(CorrectionField::Quantity, "3", 12);
    assert_eq!(
        effective_value(&transaction, &[first, second], CorrectionField::Quantity),
        "3"
    );
}

#[test]
fn effective_transaction_reflows_asset_and_quantity() {
    let transaction = record(1, "BTC", Kind::Buy, "1", Some("100"), 10);
    let corrections = vec![
        correction(CorrectionField::Quantity, "2", 11),
        correction(CorrectionField::AssetId, "ETH", 12),
    ];
    let effective = effective_transaction(&transaction, &corrections).unwrap();
    assert_eq!(effective.quantity, dec("2"));
    assert_eq!(effective.asset_id, "ETH");
}

// --- FIFO (BR2.1) ---------------------------------------------------------

#[test]
fn fifo_consumes_the_oldest_lot_first() {
    let mut lots = vec![
        CostBasisLot::acquire(
            CostBasisLotId::new(Uuid::from_u128(1)),
            subject(),
            "BTC",
            at(9),
            dec("1"),
            dec("100"),
            "USD".to_string(),
            id(1),
        )
        .unwrap(),
        CostBasisLot::acquire(
            CostBasisLotId::new(Uuid::from_u128(2)),
            subject(),
            "BTC",
            at(10),
            dec("1"),
            dec("300"),
            "USD".to_string(),
            id(2),
        )
        .unwrap(),
    ];
    let (quantity, cost) = consume_fifo(&mut lots, dec("1.5"));
    assert_eq!(quantity, dec("1.5"));
    assert_eq!(cost, dec("250"));
    assert_eq!(lots[0].remaining(), Decimal::ZERO);
    assert_eq!(lots[1].remaining(), dec("0.5"));
}

#[test]
fn fifo_never_goes_negative() {
    let mut lot = CostBasisLot::acquire(
        CostBasisLotId::new(Uuid::from_u128(1)),
        subject(),
        "BTC",
        at(9),
        dec("1"),
        dec("100"),
        "USD".to_string(),
        id(1),
    )
    .unwrap();
    let (quantity, _) = consume_fifo(std::slice::from_mut(&mut lot), dec("2"));
    assert_eq!(quantity, dec("1"));
    assert_eq!(lot.remaining(), Decimal::ZERO);
}

// --- Transfer pairing (BR6.1) ---------------------------------------------

#[test]
fn transfer_pairs_the_nearest_matching_counter_entry() {
    let out = record(1, "BTC", Kind::TransferOut, "1", None, 10);
    let near = record(2, "BTC", Kind::TransferIn, "1.005", None, 11);
    let far = record(3, "BTC", Kind::TransferIn, "1", None, 20);
    assert_eq!(find_match(&out, &[far, near]), Some(id(2)));
}

#[test]
fn transfer_does_not_pair_outside_the_window_or_tolerance() {
    let out = record(1, "BTC", Kind::TransferOut, "1", None, 10);
    let too_late = Transaction::record(
        id(2),
        NewTransaction {
            subject_id: subject(),
            user_id: "user-1".to_string(),
            account_id: None,
            asset_id: "BTC".to_string(),
            kind: Kind::TransferIn,
            quantity: dec("1"),
            price: None,
            price_source: None,
            currency: "USD".to_string(),
            source: Source::Venue,
            external_ref: None,
            executed_at: at(10) + chrono::Duration::hours(25),
            recorded_at: at(10) + chrono::Duration::days(3),
        },
    )
    .unwrap();
    let too_small = record(3, "BTC", Kind::TransferIn, "0.5", None, 10);
    assert_eq!(find_match(&out, &[too_late, too_small]), None);
}

#[test]
fn an_unpaired_transfer_is_never_a_disposal() {
    let transfer = record(1, "BTC", Kind::TransferOut, "1", None, 10);
    let outcome = replay(subject(), &[effective(&transfer)], &[], &fx()).unwrap();
    assert!(outcome.lots.is_empty());
    assert_eq!(outcome.total_realized(), Decimal::ZERO);
    assert!(outcome.shortfall_by_asset.is_empty());
}

// --- Replay (BR2.1, BR2.4, BR2.5) -----------------------------------------

#[test]
fn a_partial_sell_realises_against_the_oldest_lot() {
    let buy = record(1, "BTC", Kind::Buy, "1", Some("100"), 9);
    let sell = record(2, "BTC", Kind::Sell, "0.5", Some("300"), 10);
    let outcome = replay(subject(), &[effective(&buy), effective(&sell)], &[], &fx()).unwrap();
    assert_eq!(outcome.total_realized(), dec("100"));
    assert_eq!(outcome.held_quantity("BTC"), dec("0.5"));
}

#[test]
fn a_fee_in_the_same_minute_is_capitalised_into_the_buy() {
    let buy = record(1, "BTC", Kind::Buy, "1", Some("100"), 9);
    let fee = record(2, "BTC", Kind::Fee, "0.1", Some("100"), 9);
    let outcome = replay(subject(), &[effective(&buy), effective(&fee)], &[], &fx()).unwrap();
    assert_eq!(outcome.lots.len(), 1);
    assert_eq!(outcome.lots[0].unit_cost(), dec("110"));
    assert_eq!(outcome.standalone_fee_cost, Decimal::ZERO);
}

#[test]
fn a_fee_outside_the_trade_minute_is_a_standalone_cost() {
    let buy = record(1, "BTC", Kind::Buy, "1", Some("100"), 9);
    let fee = record(2, "BTC", Kind::Fee, "0.1", Some("100"), 11);
    let outcome = replay(subject(), &[effective(&buy), effective(&fee)], &[], &fx()).unwrap();
    assert_eq!(outcome.lots[0].unit_cost(), dec("100"));
    assert_eq!(outcome.standalone_fee_cost, dec("10"));
}

#[test]
fn a_matched_transfer_moves_no_gain_and_creates_no_second_lot() {
    let buy = record(1, "BTC", Kind::Buy, "1", Some("100"), 9);
    let out = record(2, "BTC", Kind::TransferOut, "0.4", None, 10);
    let incoming = record(3, "BTC", Kind::TransferIn, "0.4", Some("900"), 10);
    let pair = TransferPair::from_stored(
        TransferPairId::new(Uuid::from_u128(50)),
        subject(),
        Some(id(2)),
        Some(id(3)),
        "BTC".to_string(),
        TransferStatus::Matched,
        Some(at(10)),
    );
    let outcome = replay(
        subject(),
        &[effective(&buy), effective(&out), effective(&incoming)],
        &[pair],
        &fx(),
    )
    .unwrap();
    assert_eq!(outcome.total_realized(), Decimal::ZERO);
    assert_eq!(outcome.held_quantity("BTC"), dec("1"));
    assert_eq!(outcome.lots.len(), 1);
}

#[test]
fn an_unmatched_priced_transfer_in_creates_a_lot() {
    let incoming = record(1, "BTC", Kind::TransferIn, "1", Some("500"), 10);
    let outcome = replay(subject(), &[effective(&incoming)], &[], &fx()).unwrap();
    assert_eq!(outcome.held_quantity("BTC"), dec("1"));
    assert_eq!(outcome.lots[0].unit_cost(), dec("500"));
}

#[test]
fn an_ingested_sell_beyond_holdings_records_a_shortfall() {
    let buy = record(1, "BTC", Kind::Buy, "1", Some("100"), 9);
    let sell = record(2, "BTC", Kind::Sell, "1.5", Some("200"), 10);
    let outcome = replay(subject(), &[effective(&buy), effective(&sell)], &[], &fx()).unwrap();
    assert_eq!(outcome.held_quantity("BTC"), Decimal::ZERO);
    assert_eq!(outcome.shortfall_by_asset.get("BTC"), Some(&dec("0.5")));
    assert_eq!(outcome.total_realized(), dec("100"));
}

// --- P&L and coverage (BR2.2, BR3.1) --------------------------------------

#[test]
fn complete_coverage_returns_the_total_and_per_asset_figures() {
    let buy = record(1, "BTC", Kind::Buy, "1", Some("100"), 9);
    let sell = record(2, "BTC", Kind::Sell, "0.5", Some("300"), 10);
    let outcome = replay(subject(), &[effective(&buy), effective(&sell)], &[], &fx()).unwrap();
    let report = compute_pnl(at(12), &outcome, &[], &marks(&[("BTC", "300")]), &fx()).unwrap();
    assert_eq!(report.label, CoverageLabel::Complete);
    assert_eq!(report.coverage, Decimal::ONE);
    assert_eq!(report.net_invested, Some(dec("50")));
    assert_eq!(report.realized, Some(dec("100")));
    assert_eq!(report.unrealized, Some(dec("100")));
    assert_eq!(report.total_return, Some(dec("200")));
    assert!(report.coverage_gap.is_empty());
}

#[test]
fn partial_coverage_names_the_uncovered_asset_and_excludes_it() {
    let buy = record(1, "BTC", Kind::Buy, "1", Some("100"), 9);
    let sell = record(2, "ETH", Kind::Sell, "2", Some("50"), 10);
    let outcome = replay(subject(), &[effective(&buy), effective(&sell)], &[], &fx()).unwrap();
    let report = compute_pnl(
        at(12),
        &outcome,
        &holdings(&[("ETH", "2")]),
        &marks(&[("BTC", "100"), ("ETH", "50")]),
        &fx(),
    )
    .unwrap();
    assert_eq!(report.label, CoverageLabel::Partial);
    assert_eq!(report.coverage_gap, vec!["ETH".to_string()]);
    assert!(report.coverage < Decimal::ONE);
    let eth: &AssetPnl = report
        .assets
        .iter()
        .find(|asset| asset.asset_id == "ETH")
        .unwrap();
    assert!(!eth.covered);
    assert_eq!(report.net_invested, Some(dec("100")));
}

#[test]
fn no_history_at_all_returns_unavailable_with_no_figures() {
    let sell = record(1, "ETH", Kind::Sell, "2", Some("50"), 10);
    let outcome = replay(subject(), &[effective(&sell)], &[], &fx()).unwrap();
    let report = compute_pnl(at(12), &outcome, &[], &BTreeMap::new(), &fx()).unwrap();
    assert_eq!(report.label, CoverageLabel::Unavailable);
    assert_eq!(report.coverage, Decimal::ZERO);
    assert_eq!(report.net_invested, None);
    assert_eq!(report.realized, None);
    assert_eq!(report.unrealized, None);
    assert_eq!(report.coverage_gap, vec!["ETH".to_string()]);
}

#[test]
fn net_invested_is_the_open_lot_cost_not_the_total_acquisition_cost() {
    let buy = record(1, "BTC", Kind::Buy, "2", Some("100"), 9);
    let sell = record(2, "BTC", Kind::Sell, "1", Some("100"), 10);
    let outcome = replay(subject(), &[effective(&buy), effective(&sell)], &[], &fx()).unwrap();
    let report = compute_pnl(at(12), &outcome, &[], &marks(&[("BTC", "100")]), &fx()).unwrap();
    assert_eq!(report.net_invested, Some(dec("100")));
    assert_eq!(report.total_acquisition_cost, Some(dec("200")));
}

// --- Snapshot and subject --------------------------------------------------

#[test]
fn a_snapshot_round_trips_the_total_figures() {
    let buy = record(1, "BTC", Kind::Buy, "1", Some("100"), 9);
    let outcome = replay(subject(), &[effective(&buy)], &[], &fx()).unwrap();
    let report = compute_pnl(at(12), &outcome, &[], &marks(&[("BTC", "100")]), &fx()).unwrap();
    let snapshot = PnlSnapshot::record(
        PnlSnapshotId::new(Uuid::from_u128(70)),
        subject(),
        None,
        &report,
        at(12),
    );
    assert_eq!(snapshot.coverage(), Decimal::ONE);
    assert_eq!(snapshot.net_invested(), Some(dec("100")));
    assert_eq!(snapshot.computed_at(), at(12));
}

#[test]
fn a_subject_starts_identifiable_and_becomes_anonymised() {
    let subject = LedgerSubject::new(subject_id(), "user-1", at(9)).unwrap();
    assert_eq!(subject.user_id(), Some("user-1"));
    assert!(!subject.is_anonymised());
    let anonymised =
        LedgerSubject::from_stored(subject_id(), None, Some("anon-1".to_string()), at(9));
    assert!(anonymised.is_anonymised());
}

fn subject_id() -> LedgerSubjectId {
    LedgerSubjectId::new(Uuid::from_u128(9))
}

// --- Wire enums and further edge cases ------------------------------------

#[test]
fn kind_source_and_price_source_round_trip() {
    for kind in [
        Kind::Buy,
        Kind::Sell,
        Kind::TransferIn,
        Kind::TransferOut,
        Kind::Fee,
    ] {
        assert_eq!(Kind::parse(kind.as_str()).unwrap(), kind);
    }
    for source in [Source::Venue, Source::Csv, Source::RecordedTrade] {
        assert_eq!(Source::parse(source.as_str()).unwrap(), source);
    }
    for source in [PriceSource::Venue, PriceSource::Csv, PriceSource::User] {
        assert_eq!(PriceSource::parse(source.as_str()).unwrap(), source);
    }
}

#[test]
fn unknown_wire_values_are_refused() {
    assert_eq!(Kind::parse("gift").unwrap_err().field(), "kind");
    assert_eq!(Source::parse("manual").unwrap_err().field(), "source");
    assert_eq!(
        PriceSource::parse("guess").unwrap_err().field(),
        "priceSource"
    );
    assert_eq!(TransferStatus::parse("open").unwrap_err().field(), "status");
}

#[test]
fn canonical_key_distinguishes_kind_and_price() {
    let buy = canonical_key("u", "BTC", Kind::Buy, dec("1"), Some(dec("100")), at(10));
    let sell = canonical_key("u", "BTC", Kind::Sell, dec("1"), Some(dec("100")), at(10));
    let priced = canonical_key("u", "BTC", Kind::Buy, dec("1"), Some(dec("101")), at(10));
    let unpriced = canonical_key("u", "BTC", Kind::Buy, dec("1"), None, at(10));
    assert_ne!(buy, sell);
    assert_ne!(buy, priced);
    assert_ne!(buy, unpriced);
}

#[test]
fn a_transaction_without_a_reference_has_no_external_key() {
    let transaction = record(1, "BTC", Kind::Buy, "1", Some("100"), 10);
    assert_eq!(transaction.external_key(), None);
}

#[test]
fn transfer_matching_requires_the_same_asset_and_opposite_kinds() {
    let out = record(1, "BTC", Kind::TransferOut, "1", None, 10);
    let wrong_asset = record(2, "ETH", Kind::TransferIn, "1", None, 10);
    let same_kind = record(3, "BTC", Kind::TransferOut, "1", None, 10);
    assert_eq!(find_match(&out, &[wrong_asset, same_kind]), None);
}

#[test]
fn fifo_with_no_lots_consumes_nothing() {
    let (quantity, cost) = consume_fifo(&mut [], dec("1"));
    assert_eq!(quantity, Decimal::ZERO);
    assert_eq!(cost, Decimal::ZERO);
}

#[test]
fn consuming_a_lot_caps_at_its_remaining() {
    let mut lot = CostBasisLot::acquire(
        CostBasisLotId::new(Uuid::from_u128(1)),
        subject(),
        "BTC",
        at(9),
        dec("1"),
        dec("100"),
        "USD".to_string(),
        id(1),
    )
    .unwrap();
    let cost = lot.consume(dec("2"));
    assert_eq!(cost, dec("100"));
    assert_eq!(lot.remaining(), Decimal::ZERO);
    assert!(!lot.is_open());
}

#[test]
fn a_buy_without_a_price_creates_a_zero_cost_lot() {
    let buy = record(1, "BTC", Kind::Buy, "1", None, 9);
    let outcome = replay(subject(), &[effective(&buy)], &[], &fx()).unwrap();
    assert_eq!(outcome.held_quantity("BTC"), dec("1"));
    assert_eq!(outcome.lots[0].unit_cost(), Decimal::ZERO);
}

#[test]
fn a_fee_reduces_a_sells_realized_proceeds() {
    let buy = record(1, "BTC", Kind::Buy, "1", Some("100"), 9);
    let sell = record(2, "BTC", Kind::Sell, "0.5", Some("300"), 10);
    let fee = record(3, "BTC", Kind::Fee, "0.1", Some("300"), 10);
    let outcome = replay(
        subject(),
        &[effective(&buy), effective(&sell), effective(&fee)],
        &[],
        &fx(),
    )
    .unwrap();
    assert_eq!(outcome.total_realized(), dec("70"));
}

#[test]
fn an_empty_outcome_is_a_complete_zero_report() {
    let outcome = replay(subject(), &[], &[], &fx()).unwrap();
    let report = compute_pnl(at(12), &outcome, &[], &BTreeMap::new(), &fx()).unwrap();
    assert_eq!(report.label, CoverageLabel::Complete);
    assert_eq!(report.coverage, Decimal::ONE);
    assert_eq!(report.net_invested, Some(Decimal::ZERO));
    assert!(report.assets.is_empty());
}

#[test]
fn a_partially_covered_asset_is_named_and_excluded() {
    let btc_buy = record(1, "BTC", Kind::Buy, "1", Some("100"), 9);
    let btc_sell = record(2, "BTC", Kind::Sell, "2", Some("200"), 10);
    let eth_buy = record(3, "ETH", Kind::Buy, "1", Some("50"), 9);
    let outcome = replay(
        subject(),
        &[
            effective(&btc_buy),
            effective(&btc_sell),
            effective(&eth_buy),
        ],
        &[],
        &fx(),
    )
    .unwrap();
    let report = compute_pnl(
        at(12),
        &outcome,
        &[],
        &marks(&[("BTC", "200"), ("ETH", "50")]),
        &fx(),
    )
    .unwrap();
    assert_eq!(report.label, CoverageLabel::Partial);
    assert_eq!(report.coverage_gap, vec!["BTC".to_string()]);
    assert_eq!(report.net_invested, Some(dec("50")));
}

#[test]
fn a_subject_requires_a_user_id() {
    let error = LedgerSubject::new(subject(), "   ", at(9)).unwrap_err();
    assert_eq!(error.field(), "userId");
}

#[test]
fn a_snapshot_from_stored_round_trips() {
    let snapshot = PnlSnapshot::from_stored(
        PnlSnapshotId::new(Uuid::from_u128(90)),
        subject(),
        None,
        at(12),
        "USD".to_string(),
        None,
        false,
        Some(dec("100")),
        Some(dec("10")),
        Some(dec("5")),
        Some(dec("15")),
        Some(dec("200")),
        Decimal::ONE,
        vec!["ETH".to_string()],
        at(12),
    );
    assert_eq!(snapshot.total_return(), Some(dec("15")));
    assert_eq!(snapshot.coverage_gap(), &["ETH".to_string()]);
}

#[test]
fn fifo_sorts_lots_by_acquisition_time_before_consuming() {
    let mut lots = vec![
        CostBasisLot::acquire(
            CostBasisLotId::new(Uuid::from_u128(2)),
            subject(),
            "BTC",
            at(11),
            dec("1"),
            dec("300"),
            "USD".to_string(),
            id(2),
        )
        .unwrap(),
        CostBasisLot::acquire(
            CostBasisLotId::new(Uuid::from_u128(1)),
            subject(),
            "BTC",
            at(9),
            dec("1"),
            dec("100"),
            "USD".to_string(),
            id(1),
        )
        .unwrap(),
    ];
    let (_, cost) = consume_fifo(&mut lots, dec("1"));
    assert_eq!(cost, dec("100"));
    assert_eq!(lots[0].acquired_at(), at(9));
    assert_eq!(lots[0].remaining(), Decimal::ZERO);
    assert_eq!(lots[1].remaining(), dec("1"));
}

#[test]
fn canonical_decimal_strips_trailing_zeros() {
    assert_eq!(ledger_domain::canonical_decimal(dec("1.500")), "1.5");
    assert_eq!(ledger_domain::canonical_decimal(dec("100.00")), "100");
}

#[test]
fn an_asset_correction_parses_to_its_value() {
    assert_eq!(
        parse_field_value(CorrectionField::AssetId, " ETH ", at(10)),
        Ok(FieldValue::AssetId("ETH".to_string()))
    );
}

// --- Review fixes: market marks, C5 holdings, external transfers, currency ---

#[test]
fn an_external_transfer_out_consumes_lots_without_realised_pnl() {
    let buy = record(1, "BTC", Kind::Buy, "1", Some("100"), 9);
    let out = record(2, "BTC", Kind::TransferOut, "0.4", None, 10);
    let outcome = replay(subject(), &[effective(&buy), effective(&out)], &[], &fx()).unwrap();
    assert_eq!(outcome.held_quantity("BTC"), dec("0.6"));
    assert_eq!(outcome.total_realized(), Decimal::ZERO);
    assert!(outcome.shortfall_by_asset.is_empty());
}

#[test]
fn a_fee_without_a_unit_price_is_refused() {
    let error = Transaction::record(
        id(1),
        NewTransaction {
            subject_id: subject(),
            user_id: "user-1".to_string(),
            account_id: None,
            asset_id: "BTC".to_string(),
            kind: Kind::Fee,
            quantity: dec("0.1"),
            price: None,
            price_source: None,
            currency: "USD".to_string(),
            source: Source::Venue,
            external_ref: None,
            executed_at: at(10),
            recorded_at: at(23),
        },
    )
    .unwrap_err();
    assert_eq!(error.field(), "price");
}

#[test]
fn a_fee_with_a_zero_price_is_still_a_cost_of_zero() {
    let fee = record(1, "BTC", Kind::Fee, "0.1", Some("0"), 10);
    assert_eq!(fee.price(), Some(Decimal::ZERO));
}

#[test]
fn a_non_base_cost_is_converted_through_fx_at_compute_time() {
    let buy = record_in(1, "BTC", Kind::Buy, "1", Some("100"), "EUR", 9);
    let sell = record_in(2, "BTC", Kind::Sell, "0.5", Some("200"), "EUR", 10);
    let mut rates = BTreeMap::new();
    rates.insert("EUR".to_string(), dec("1.1"));
    let fx = FxRates::new("USD", rates, Some(at(12)));
    let outcome = replay(subject(), &[effective(&buy), effective(&sell)], &[], &fx).unwrap();
    assert_eq!(outcome.total_realized(), dec("55"));
    let report = compute_pnl(at(12), &outcome, &[], &marks(&[("BTC", "300")]), &fx).unwrap();
    assert_eq!(report.currency, "USD");
    assert_eq!(report.fx_as_of, Some(at(12)));
    assert!(report.fx_estimated);
    assert_eq!(report.net_invested, Some(dec("55")));
}

#[test]
fn a_missing_fx_rate_fails_instead_of_mixing_currencies() {
    let buy = record_in(1, "BTC", Kind::Buy, "1", Some("100"), "EUR", 9);
    let error = replay(subject(), &[effective(&buy)], &[], &fx()).unwrap_err();
    assert_eq!(error.field(), "currency");
}

#[test]
fn an_unpriced_holding_is_named_and_never_valued_at_zero() {
    let buy = record(1, "BTC", Kind::Buy, "1", Some("100"), 9);
    let outcome = replay(subject(), &[effective(&buy)], &[], &fx()).unwrap();
    let mut prices = BTreeMap::new();
    prices.insert("BTC".to_string(), PriceMark::unpriced());
    let report = compute_pnl(at(12), &outcome, &[], &prices, &fx()).unwrap();
    assert_eq!(report.label, CoverageLabel::Unavailable);
    assert_eq!(report.coverage_gap, vec!["BTC".to_string()]);
    assert_eq!(report.net_invested, None);
    assert_eq!(report.assets[0].mark_price, None);
    assert_eq!(report.assets[0].price_freshness, "unpriced");
}

#[test]
fn c5_holdings_without_history_are_named_and_coverage_is_by_value() {
    let buy = record(1, "BTC", Kind::Buy, "1", Some("100"), 9);
    let outcome = replay(subject(), &[effective(&buy)], &[], &fx()).unwrap();
    let report = compute_pnl(
        at(12),
        &outcome,
        &holdings(&[("BTC", "1"), ("ETH", "2")]),
        &marks(&[("BTC", "100"), ("ETH", "50")]),
        &fx(),
    )
    .unwrap();
    assert_eq!(report.label, CoverageLabel::Partial);
    assert_eq!(report.coverage_gap, vec!["ETH".to_string()]);
    assert_eq!(report.coverage, dec("0.5"));
    assert_eq!(report.net_invested, Some(dec("100")));
    let eth: &AssetPnl = report
        .assets
        .iter()
        .find(|asset| asset.asset_id == "ETH")
        .unwrap();
    assert!(!eth.covered);
    assert_eq!(eth.mark_price, Some(dec("50")));
}

#[test]
fn a_holding_larger_than_its_lots_is_uncovered() {
    let buy = record(1, "BTC", Kind::Buy, "0.5", Some("100"), 9);
    let outcome = replay(subject(), &[effective(&buy)], &[], &fx()).unwrap();
    let report = compute_pnl(
        at(12),
        &outcome,
        &holdings(&[("BTC", "1")]),
        &marks(&[("BTC", "100")]),
        &fx(),
    )
    .unwrap();
    assert_eq!(report.coverage_gap, vec!["BTC".to_string()]);
    assert_eq!(report.coverage, Decimal::ZERO);
    assert_eq!(report.label, CoverageLabel::Unavailable);
}
