# 03 — Portfolio Model (Schema)

# Portfolio Model (Schema)
## Core entities
- Account: {type: wallet|exchange, name, chain/exchange, identifiers}
- Asset: {symbol, network, contract, decimals, tags}
- Holding: {account_id, asset_id, qty, price, value_base}
## Optional entities
- Position (derivatives): {instrument, side, size, entry, PnL, margin}
- Transactions: deposits/withdrawals, trades, transfers (for cost basis).
## Normalization rules
1. Choose a base currency (USD/EUR/USDT/USDC).
1. Map symbols + networks to a canonical asset id.
1. Use consistent pricing source + timestamp.
## Outputs
- Allocation tables: by asset / by account / by tag.
- Risk metrics: concentration, stablecoin %, drawdown proxy.
