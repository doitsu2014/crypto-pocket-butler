# crypto-pocket-butler

A small (but serious) pet project: **crypto portfolio management** across wallets + exchanges, with an **OpenClaw agent** that produces rebalancing suggestions and writes daily briefs to Notion.

## Planned stack
- **Backend:** Rust (Axum) + Postgres
- **Frontend:** React (Next.js) + TypeScript
- **Agent:** OpenClaw (suggestions first, execution later with guardrails)

## MVP (first milestone)
- Connect OKX (read-only) + 1 wallet type
- Normalize holdings into one schema
- Show consolidated portfolio + allocation
- Generate a daily/weekly rebalancing suggestion

## Guardrails (draft)
- Base currency: USD
- Rebalancing: fixed targets + guardrails
- Stablecoin minimum: TBD
- Futures cap: TBD

## Security
- Start **read-only** for exchanges.
- Never enable withdrawals.
- If trading is enabled later: strict allowlists + max order sizes + full audit log.
