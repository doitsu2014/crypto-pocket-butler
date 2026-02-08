# 01 — MVP Scope & User Stories

## MVP definition (updated)

The MVP proves the full end-to-end loop:
1) user logs in (Keycloak)
2) user defines a portfolio and selects data sources (wallets/exchanges)
3) system ingests holdings and computes allocation
4) system creates **end-of-day snapshots**
5) agent generates a daily brief + simple rebalance suggestion (suggest-only)

## Primary user

- Single user (you) first; built to support multiple users later via Keycloak.

## MVP user stories

1. **Auth**
   - As a user, I can sign in via **Keycloak** and access only my own data.
2. **Accounts**
   - As a user, I can register accounts (wallet addresses, OKX account) as data sources.
3. **Portfolios**
   - As a user, I can create portfolios and choose which accounts belong to each portfolio.
4. **Holdings**
   - As a user, I can view latest holdings + allocation for a portfolio (base currency: USD).
5. **Snapshots (EOD)**
   - As a user, I can see an end-of-day snapshot for each day (value + composition).
6. **Recommendations (suggest-only)**
   - As a user, I receive a simple, explainable rebalance suggestion (fixed targets + guardrails).
7. **Reporting**
   - As a user, I get a daily/weekly brief written to Notion and a short Telegram summary.

## Non-goals (MVP)

- Auto trading
- Perp/futures execution (tracking later)
- Tax reporting / perfect cost basis
- Multi-exchange support (start with OKX)

## MVP deliverables checklist

- [ ] Keycloak login works in frontend; backend validates JWTs
- [ ] Portfolio CRUD + portfolio-to-account mapping
- [ ] OKX read-only ingestion + EVM wallet ingestion
- [ ] Latest holdings + allocation endpoint
- [ ] EOD snapshot job (daily)
- [ ] Notion daily brief output (initial template)
- [ ] OpenClaw skill can authenticate (client_credentials) and call backend API
