/// Allocation domain services
///
/// Provides weight calculation and price enrichment logic for the allocation
/// bounded context.

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use super::entities::AllocationItem;

/// Recalculate portfolio weights for a collection of allocation items.
///
/// Sets `weight` on each priced item as a percentage of `total_value_usd`.
/// Unpriced items retain a weight of `0.0`.
///
/// # Arguments
/// * `items` - Mutable slice of allocation items to update in-place.
/// * `total_value_usd` - Total portfolio value used as the denominator.
pub fn recalculate_weights(items: &mut [AllocationItem], total_value_usd: Decimal) {
    let total_f64 = total_value_usd.to_f64().unwrap_or(0.0);
    if total_f64 <= 0.0 {
        return;
    }
    for item in items.iter_mut() {
        if !item.unpriced {
            item.weight = (item.value_usd / total_f64) * 100.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::allocation::entities::AllocationItem;

    fn make_item(asset: &str, value_usd: f64, unpriced: bool) -> AllocationItem {
        AllocationItem {
            asset: asset.to_string(),
            chain: None,
            quantity: "1.0".to_string(),
            price_usd: if unpriced { None } else { Some(value_usd) },
            value_usd,
            weight: 0.0,
            unpriced,
        }
    }

    #[test]
    fn test_recalculate_weights_two_assets() {
        let mut items = vec![
            make_item("BTC", 75000.0, false),
            make_item("ETH", 25000.0, false),
        ];
        let total = Decimal::from(100000u64);
        recalculate_weights(&mut items, total);

        assert!((items[0].weight - 75.0).abs() < 0.001);
        assert!((items[1].weight - 25.0).abs() < 0.001);
    }

    #[test]
    fn test_recalculate_weights_unpriced_stays_zero() {
        let mut items = vec![
            make_item("BTC", 100000.0, false),
            make_item("UNKNOWN", 0.0, true),
        ];
        let total = Decimal::from(100000u64);
        recalculate_weights(&mut items, total);

        assert!((items[0].weight - 100.0).abs() < 0.001);
        assert_eq!(items[1].weight, 0.0);
    }

    #[test]
    fn test_recalculate_weights_zero_total_no_panic() {
        let mut items = vec![make_item("BTC", 0.0, false)];
        recalculate_weights(&mut items, Decimal::ZERO);
        assert_eq!(items[0].weight, 0.0);
    }
}
