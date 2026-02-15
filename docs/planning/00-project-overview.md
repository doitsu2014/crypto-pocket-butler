# [Project] Crypto Portfolio Management



## Project Summary
Build a crypto portfolio management system that aggregates positions across wallets + exchanges, computes portfolio views per account and in total, and uses an AI agent (OpenClaw) to propose and/or execute rebalancing and risk controls.

## Goals (MVP)
- Core split: (A) Infrastructure Data (market reference + account holdings) and (B) Portfolios (construct allocation + snapshots).
- User authentication and authorization via Keycloak (web + API).
- User-defined portfolios that choose which wallets/exchanges are included.
- Daily end-of-day (EOD) portfolio snapshots.
- Connect data sources: on-chain wallets + exchange accounts (read-only first).
- Normalize holdings into a single schema (asset, quantity, USD value, cost basis if available).
- Portfolio views: per-wallet/per-exchange + consolidated total; allocation by asset, sector, chain, and risk bucket.
- AI agent outputs daily/weekly: summary, risks, and rebalancing suggestions (no auto-trade in MVP).
## Key Features (Phase 2+)
- Rebalancing engine: target allocations, drift bands, tax/fee awareness, minimum order sizes.
- Execution modes: (1) Suggest-only, (2) One-click approve, (3) Auto-trade with strict guardrails.
- Risk controls: max drawdown alerts, leverage limits, stablecoin buffer, exposure caps per token/chain.
- Performance: PnL, time-weighted return, benchmark vs BTC/ETH, contribution analysis.
## Data Sources
- Wallets (on-chain): EVM (ETH/Arb/OP/Base), BTC, Solana — via public RPC / indexer.
- Exchanges: OKX (start), then Binance/Coinbase/Bybit — via API keys.
- Pricing: CoinGecko/CoinMarketCap/Exchange tickers; FX rates for EUR/USD if needed.
## Agent (OpenClaw) Responsibilities
1. Ingest: pull latest positions + prices on schedule.
1. Analyze: compute allocation, drift vs targets, volatility proxy, liquidity notes, and concentration risk.
1. Recommend: propose trades (asset, side, size, venue) and explain rationale.
1. Report: write a Daily Brief into Notion + send a short Telegram summary.
## Security / Guardrails (must-have)
- Start with read-only API keys. Never enable withdrawals.
- If trading is enabled later: IP whitelist, max order size, allowlist symbols, require human approval for first N trades.
- Audit log: every agent action stored (time, data snapshot hash, suggested orders, decision reason).
## Open Questions
- [ ] What is your target base currency (USD/EUR/USDT/USDC)?
- [ ] Spot only, or also futures/perps?
- [ ] Rebalancing style: fixed targets vs risk-parity vs trend-following?
- [ ] How often should the agent run (daily, hourly, on price move)?

- 01 — MVP Scope & User Stories
- 02 — Data Sources & Integrations
- 03 — Portfolio Model (Schema)
- 04 — Rebalancing & Risk Engine
- 05 — OpenClaw Agent Design
- 06 — Security, Permissions & Audit
- 07 — Notion Reporting (Dashboards)
- 08 — Roadmap
- 09 — Technical Design (Rust + React)
## Open Questions — Answers (2026-02-07)
- Base currency: USD.
- Trading style: Spot most of the time; occasional small futures exposure.
- Rebalancing style chosen: Fixed targets + guardrails (simple).
