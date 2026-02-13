# 08 — Roadmap

This roadmap is now organized around the **core split**:

- **A) Infrastructure Data**: cron-collected market reference + user account holdings (qty only)
- **B) Portfolios**: construct allocation on demand + snapshots (EOD or manual)

See also: `docs/planning/10-core-split-infra-vs-portfolios.md`.

## Phase A — Infrastructure Data

### A1) Market Reference Data (cron)

- [ ] Price collector job (store price snapshots per asset)
- [ ] Top-100 coins collector job (rank + metadata)
- [ ] Contracts/address registry per chain (for top assets)
- [ ] Symbol normalization mapping (BTC vs XBT, USDT variants, etc.)

### A2) User Accounts (holdings qty only)

- [ ] Account model supports detailed config
  - EVM: address + enabled chains (from supported list)
  - OKX: read-only credentials profile
- [ ] Account ingestion stores holdings **without pricing**
- [ ] Support list of chains exposed via API (frontend can render)

## Phase B — Portfolios

### B1) Construct allocation (manual)

- [ ] Portfolio composition: select which accounts belong to portfolio
- [ ] "Construct portfolio" endpoint/action
  - aggregates account holdings
  - values using market reference prices
  - stores/returns Portfolio Allocation
  - **Persist** allocation and expose last_constructed_at

### B2) Snapshots

- [ ] EOD snapshot job per portfolio (automatic)
- [ ] Manual snapshot trigger (button: "Construct snapshot")
- [ ] Snapshot list + latest snapshot endpoints

## Phase C — Agent & Insights (optional but valuable)

- [ ] OpenClaw agent reads snapshots + allocations
- [ ] Daily brief + rebalancing suggestions (suggest-only)
- [ ] Alerts on concentration/drawdown

## Phase D — Execution (optional)

- [ ] One-click approval workflow
- [ ] Optional auto-trade with strict guardrails + full audit log
