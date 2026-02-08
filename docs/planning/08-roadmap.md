# 08 — Roadmap

## Phase 0 — Foundations

- [ ] Decide target allocations + guardrails numbers (drift band, stablecoin min, futures cap, max alt cap)
- [ ] Define EOD snapshot cutover rule (UTC vs local time)

## Phase 1 — MVP (Keycloak + portfolios + snapshots)

- [ ] Keycloak integration (frontend PKCE + backend JWT validation)
- [ ] Data model + Postgres schema (users/accounts/portfolios/snapshots)
- [ ] Accounts management (wallets + OKX connector read-only)
- [ ] Portfolio composition (choose which accounts feed which portfolio)
- [ ] Latest holdings + allocation
- [ ] EOD snapshots (daily)
- [ ] Agent reporting to Notion/Telegram (suggest-only)

## Phase 2 — Suggestions (smarter but still safe)

- [ ] Fixed-target rebalancing suggestions + drift bands
- [ ] Risk alerts (concentration/drawdown) + stablecoin buffer checks
- [ ] Snapshot-based performance metrics (basic)

## Phase 3 — Execution (optional)

- [ ] One-click approval workflow
- [ ] Optional auto-trade with strict guardrails + full audit log
