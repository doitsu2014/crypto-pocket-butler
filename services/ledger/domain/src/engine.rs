//! The deterministic ledger replay: FIFO lots, realised P&L, transfer basis,
//! fee attachment, and shortfalls (BR2.1, BR2.4, BR2.5, NFR1.7).
//!
//! Every monetary result is converted into the user's base currency at compute
//! time through the C6 FX table; a missing rate fails the replay rather than
//! mixing currencies.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, SecondsFormat, Timelike, Utc};
use rust_decimal::Decimal;

use crate::correction::EffectiveTransaction;
use crate::error::DomainError;
use crate::fx::FxRates;
use crate::ids::{CostBasisLotId, LedgerSubjectId, TransactionId};
use crate::lot::CostBasisLot;
use crate::transaction::Kind;
use crate::transfer::TransferPair;

/// The result of replaying one subject's ledger, in the base currency.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplayOutcome {
    /// The open lots, oldest first per asset, with their acquisition currency.
    pub lots: Vec<CostBasisLot>,
    /// Realised P&L per asset, in the base currency.
    pub realized_by_asset: BTreeMap<String, Decimal>,
    /// Total acquisition cost per asset, in the base currency.
    pub acquisition_cost_by_asset: BTreeMap<String, Decimal>,
    /// Quantity sold beyond the lots held, per asset (a coverage gap).
    pub shortfall_by_asset: BTreeMap<String, Decimal>,
    /// Standalone fees, in the base currency; their own cost, never a disposal.
    pub standalone_fee_cost: Decimal,
}

impl ReplayOutcome {
    /// The quantity still held for an asset.
    #[must_use]
    pub fn held_quantity(&self, asset_id: &str) -> Decimal {
        self.lots
            .iter()
            .filter(|lot| lot.asset_id() == asset_id)
            .map(CostBasisLot::remaining)
            .sum()
    }

    /// The open lots for an asset.
    #[must_use]
    pub fn open_lots_for(&self, asset_id: &str) -> Vec<&CostBasisLot> {
        self.lots
            .iter()
            .filter(|lot| lot.asset_id() == asset_id)
            .collect()
    }

    /// Total realised P&L, in the base currency.
    #[must_use]
    pub fn total_realized(&self) -> Decimal {
        self.realized_by_asset.values().copied().sum()
    }

    /// Total acquisition cost (consumed and remaining) plus standalone fees,
    /// in the base currency.
    #[must_use]
    pub fn total_acquisition_cost(&self) -> Decimal {
        self.acquisition_cost_by_asset
            .values()
            .copied()
            .sum::<Decimal>()
            + self.standalone_fee_cost
    }
}

/// Replays a subject's corrected transactions in execution order.
///
/// A matched transfer is a no-op at the user level because both legs carry the
/// same asset: the outgoing lots' quantity and cost stay exactly where they
/// are, so no realised gain appears and no independent priced lot is created
/// (BR2.4). An unmatched transfer_out is an external send to an untracked
/// wallet: it consumes the outgoing lots but produces no realised P&L. An
/// unmatched priced transfer_in behaves like an acquisition (BR6.1).
///
/// # Errors
/// Returns [`DomainError`] when an acquisition cannot form a valid lot or an FX
/// rate is missing.
pub fn replay(
    subject_id: LedgerSubjectId,
    transactions: &[EffectiveTransaction],
    pairs: &[TransferPair],
    fx: &FxRates,
) -> Result<ReplayOutcome, DomainError> {
    let matched_out: BTreeSet<TransactionId> = pairs
        .iter()
        .filter(|pair| pair.is_matched())
        .filter_map(TransferPair::out_transaction_id)
        .collect();
    let matched_in: BTreeSet<TransactionId> = pairs
        .iter()
        .filter(|pair| pair.is_matched())
        .filter_map(TransferPair::in_transaction_id)
        .collect();

    let (attached_fee, attached_fee_indices) = attach_fees(transactions, fx)?;
    let standalone_fee_cost = standalone_fees(transactions, &attached_fee_indices, fx)?;

    let mut ordered: Vec<(usize, &EffectiveTransaction)> =
        transactions.iter().enumerate().collect();
    ordered.sort_by_key(|(_, transaction)| (transaction.executed_at, transaction.id.as_uuid()));

    let mut lots_by_asset: BTreeMap<String, Vec<CostBasisLot>> = BTreeMap::new();
    let mut realized_by_asset: BTreeMap<String, Decimal> = BTreeMap::new();
    let mut acquisition_cost_by_asset: BTreeMap<String, Decimal> = BTreeMap::new();
    let mut shortfall_by_asset: BTreeMap<String, Decimal> = BTreeMap::new();

    for (index, transaction) in ordered {
        match transaction.kind {
            Kind::Fee => {}
            Kind::Buy => {
                acquire(
                    subject_id,
                    transaction,
                    &attached_fee,
                    index,
                    fx,
                    &mut lots_by_asset,
                    &mut acquisition_cost_by_asset,
                )?;
            }
            Kind::TransferIn => {
                if !matched_in.contains(&transaction.id) && transaction.price.is_some() {
                    acquire(
                        subject_id,
                        transaction,
                        &attached_fee,
                        index,
                        fx,
                        &mut lots_by_asset,
                        &mut acquisition_cost_by_asset,
                    )?;
                }
            }
            Kind::TransferOut => {
                if !matched_out.contains(&transaction.id) {
                    // An external send consumes the outgoing lots without a
                    // realised gain; the basis leaves the tracked set.
                    let lots = lots_by_asset
                        .entry(transaction.asset_id.clone())
                        .or_default();
                    consume_lots(lots, transaction.quantity, fx)?;
                }
            }
            Kind::Sell => {
                let lots = lots_by_asset
                    .entry(transaction.asset_id.clone())
                    .or_default();
                let (consumed_quantity, consumed_cost) =
                    consume_lots(lots, transaction.quantity, fx)?;
                let uncovered = transaction.quantity - consumed_quantity;
                if uncovered > Decimal::ZERO {
                    *shortfall_by_asset
                        .entry(transaction.asset_id.clone())
                        .or_insert(Decimal::ZERO) += uncovered;
                }
                let unit_price = transaction.price.unwrap_or(Decimal::ZERO);
                let proceeds = convert(consumed_quantity * unit_price, &transaction.currency, fx)?;
                let fee = attached_fee.get(&index).copied().unwrap_or(Decimal::ZERO);
                let fee_share = if transaction.quantity > Decimal::ZERO {
                    convert(fee, &transaction.currency, fx)? * consumed_quantity
                        / transaction.quantity
                } else {
                    Decimal::ZERO
                };
                *realized_by_asset
                    .entry(transaction.asset_id.clone())
                    .or_insert(Decimal::ZERO) += proceeds - fee_share - consumed_cost;
            }
        }
    }

    let mut lots: Vec<CostBasisLot> = lots_by_asset
        .into_values()
        .flatten()
        .filter(CostBasisLot::is_open)
        .collect();
    lots.sort_by_key(|lot| {
        (
            lot.asset_id().to_string(),
            lot.acquired_at(),
            lot.id().as_uuid(),
        )
    });

    Ok(ReplayOutcome {
        lots,
        realized_by_asset,
        acquisition_cost_by_asset,
        shortfall_by_asset,
        standalone_fee_cost,
    })
}

fn acquire(
    subject_id: LedgerSubjectId,
    transaction: &EffectiveTransaction,
    attached_fee: &BTreeMap<usize, Decimal>,
    index: usize,
    fx: &FxRates,
    lots_by_asset: &mut BTreeMap<String, Vec<CostBasisLot>>,
    acquisition_cost_by_asset: &mut BTreeMap<String, Decimal>,
) -> Result<(), DomainError> {
    let unit_cost = transaction.price.unwrap_or(Decimal::ZERO);
    let fee = fee_in_trade_currency(
        attached_fee.get(&index).copied().unwrap_or(Decimal::ZERO),
        &transaction.currency,
        fx,
    )?;
    let total_cost = transaction.quantity * unit_cost + fee;
    let lot = CostBasisLot::acquire(
        CostBasisLotId::new(transaction.id.as_uuid()),
        subject_id,
        transaction.asset_id.clone(),
        transaction.executed_at,
        transaction.quantity,
        total_cost,
        transaction.currency.clone(),
        transaction.id,
    )?;
    *acquisition_cost_by_asset
        .entry(transaction.asset_id.clone())
        .or_insert(Decimal::ZERO) += convert(total_cost, &transaction.currency, fx)?;
    lots_by_asset
        .entry(transaction.asset_id.clone())
        .or_default()
        .push(lot);
    Ok(())
}

/// Consumes the oldest lots first, converting each lot's cost from its own
/// currency into the base. Returns the consumed quantity and base cost.
fn consume_lots(
    lots: &mut [CostBasisLot],
    quantity: Decimal,
    fx: &FxRates,
) -> Result<(Decimal, Decimal), DomainError> {
    let mut needed = quantity.max(Decimal::ZERO);
    if needed <= Decimal::ZERO {
        return Ok((Decimal::ZERO, Decimal::ZERO));
    }
    lots.sort_by_key(|lot| (lot.acquired_at(), lot.id().as_uuid()));
    let mut consumed_quantity = Decimal::ZERO;
    let mut consumed_cost = Decimal::ZERO;
    for lot in lots.iter_mut() {
        if needed <= Decimal::ZERO {
            break;
        }
        let before = lot.remaining();
        let cost = lot.consume(needed);
        let taken = before - lot.remaining();
        consumed_quantity += taken;
        consumed_cost += convert(cost, lot.currency(), fx)?;
        needed -= taken;
    }
    Ok((consumed_quantity, consumed_cost))
}

fn convert(amount: Decimal, currency: &str, fx: &FxRates) -> Result<Decimal, DomainError> {
    fx.convert(amount, currency)
        .ok_or_else(|| DomainError::Invalid {
            field: "currency",
            reason: format!("no FX rate from {currency} to {}", fx.base()),
        })
}

fn fee_in_trade_currency(
    fee_base: Decimal,
    trade_currency: &str,
    fx: &FxRates,
) -> Result<Decimal, DomainError> {
    if fee_base.is_zero() {
        return Ok(Decimal::ZERO);
    }
    let trade_rate = fx
        .rate(trade_currency)
        .ok_or_else(|| DomainError::Invalid {
            field: "currency",
            reason: format!("no FX rate from {trade_currency} to {}", fx.base()),
        })?;
    if trade_rate.is_zero() {
        return Err(DomainError::Invalid {
            field: "currency",
            reason: format!("the FX rate for {trade_currency} is zero"),
        });
    }
    Ok(fee_base / trade_rate)
}

/// Attaches each fee to the nearest buy or sell of the same asset inside the
/// same minute (BR2.5). The value is the fee's cost in the base currency; the
/// caller converts it back to the trade's currency.
fn attach_fees(
    transactions: &[EffectiveTransaction],
    fx: &FxRates,
) -> Result<(BTreeMap<usize, Decimal>, BTreeSet<usize>), DomainError> {
    let trades: Vec<usize> = transactions
        .iter()
        .enumerate()
        .filter(|(_, transaction)| matches!(transaction.kind, Kind::Buy | Kind::Sell))
        .map(|(index, _)| index)
        .collect();
    let mut attached = BTreeMap::new();
    let mut attached_indices = BTreeSet::new();
    for (fee_index, fee) in transactions.iter().enumerate() {
        if fee.kind != Kind::Fee {
            continue;
        }
        let fee_minute = minute_of(fee.executed_at);
        let best = trades
            .iter()
            .filter(|&&index| transactions[index].asset_id == fee.asset_id)
            .filter(|&&index| minute_of(transactions[index].executed_at) == fee_minute)
            .min_by_key(|&&index| {
                (
                    (transactions[index].executed_at - fee.executed_at)
                        .num_seconds()
                        .abs(),
                    transactions[index].id.as_uuid(),
                )
            });
        if let Some(&index) = best {
            let unit = fee.price.unwrap_or(Decimal::ZERO);
            *attached.entry(index).or_insert(Decimal::ZERO) +=
                convert(fee.quantity * unit, &fee.currency, fx)?;
            attached_indices.insert(fee_index);
        }
    }
    Ok((attached, attached_indices))
}

fn standalone_fees(
    transactions: &[EffectiveTransaction],
    attached: &BTreeSet<usize>,
    fx: &FxRates,
) -> Result<Decimal, DomainError> {
    let mut total = Decimal::ZERO;
    for (index, transaction) in transactions
        .iter()
        .enumerate()
        .filter(|(_, transaction)| transaction.kind == Kind::Fee)
        .filter(|(index, _)| !attached.contains(index))
    {
        let _ = index;
        let cost = transaction.quantity * transaction.price.unwrap_or(Decimal::ZERO);
        total += convert(cost, &transaction.currency, fx)?;
    }
    Ok(total)
}

fn minute_of(value: DateTime<Utc>) -> String {
    value
        .with_second(0)
        .and_then(|at| at.with_nanosecond(0))
        .unwrap_or(value)
        .to_rfc3339_opts(SecondsFormat::Secs, true)
}
