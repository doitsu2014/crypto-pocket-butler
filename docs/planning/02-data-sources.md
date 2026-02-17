# 02 — Data Sources & Integrations

# Data Sources & Integrations
## Exchanges (priority order)
- OKX: balances, positions, orders, deposits/withdrawals history (read-only).
- Future: Binance / Coinbase / Bybit.
## Wallets
- EVM: address holdings + DeFi positions (optional)
- BTC: UTXO balance.
- Solana: SPL token balances.
## Price & metadata
- Prices: exchange tickers first; fallback to CoinPaprika.
- Token metadata: symbol mapping, decimals, chain, sector tags.
## Integration requirements
- [ ] API key storage + rotation
- [ ] Rate limiting + retry strategy
- [ ] Symbol normalization (e.g., BTC vs XBT, USDT on multiple chains)
