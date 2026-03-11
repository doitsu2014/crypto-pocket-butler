/// Asset domain services
///
/// Pure domain logic for asset-related computations.

use rust_decimal::Decimal;
use std::collections::HashMap;

use super::entities::AssetPrice;

/// Resolve prices for a list of asset symbols from a price map.
///
/// Returns a map of symbol → `AssetPrice` for symbols that have prices.
pub fn resolve_prices<'a>(
    symbols: &[String],
    price_map: &'a HashMap<String, AssetPrice>,
) -> HashMap<&'a str, &'a AssetPrice> {
    symbols
        .iter()
        .filter_map(|sym| price_map.get(sym.as_str()).map(|p| (p.asset.as_str(), p)))
        .collect()
}

/// Categorise assets into priced and unpriced groups.
pub fn categorise_by_price(
    holdings: &[(String, Decimal)],
    prices: &HashMap<String, Decimal>,
) -> (Vec<(String, Decimal)>, Vec<(String, Decimal)>) {
    let mut priced = Vec::new();
    let mut unpriced = Vec::new();
    for (asset, qty) in holdings {
        if prices.contains_key(asset.as_str()) {
            priced.push((asset.clone(), *qty));
        } else {
            unpriced.push((asset.clone(), *qty));
        }
    }
    (priced, unpriced)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorise_by_price() {
        let holdings = vec![
            ("BTC".into(), Decimal::new(1, 0)),
            ("UNKNOWN".into(), Decimal::new(100, 0)),
        ];
        let mut prices = HashMap::new();
        prices.insert("BTC".into(), Decimal::new(50000, 0));

        let (priced, unpriced) = categorise_by_price(&holdings, &prices);
        assert_eq!(priced.len(), 1);
        assert_eq!(priced[0].0, "BTC");
        assert_eq!(unpriced.len(), 1);
        assert_eq!(unpriced[0].0, "UNKNOWN");
    }
}
