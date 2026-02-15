# API Reference (draft)

This is a planning-level reference. Adjust paths and schemas to match the Rust implementation.

## Conventions

- Base URL: `${CPB_API_BASE_URL}`
- Auth header: `Authorization: Bearer <access_token>`
- All IDs are UUIDs unless stated.
- Base currency: USD.

## Health

- `GET /health` → `{ status: "ok" }`

## Portfolios

- `GET /v1/portfolios`
  - Returns portfolios owned by current user.

- `POST /v1/portfolios`
  - Body: `{ name, base_currency }`

- `GET /v1/portfolios/{portfolio_id}`

- `PATCH /v1/portfolios/{portfolio_id}`

## Accounts (wallets / exchanges)

- `GET /v1/accounts`
- `POST /v1/accounts`
  - wallet: `{ type:"wallet", chain:"EVM"|"BTC"|"SOL", address, name }`
  - exchange: `{ type:"exchange", exchange_name:"OKX", account_ref, name }`

## Portfolio composition

- `GET /v1/portfolios/{portfolio_id}/accounts`
- `PUT /v1/portfolios/{portfolio_id}/accounts`
  - Body: `{ account_ids: [..] }`

## Holdings (latest)

- `GET /v1/portfolios/{portfolio_id}/holdings/latest`
  - Returns aggregated holdings for that portfolio.

## Snapshots

- `POST /v1/portfolios/{portfolio_id}/snapshots/eod`
  - Creates end-of-day snapshot (server-side).

- `GET /v1/portfolios/{portfolio_id}/snapshots?limit=30`
- `GET /v1/portfolios/{portfolio_id}/snapshots/latest`

## Recommendations (suggest-only)

- `POST /v1/portfolios/{portfolio_id}/recommendations`
  - Body includes:
    - targets
    - guardrails
    - proposed_orders (venue/symbol/side/size)
    - rationale

- `GET /v1/portfolios/{portfolio_id}/recommendations?limit=20`

## Authorization model (draft)

- A user can only access resources where `resource.user_id == token.sub`.
- Optional roles for future expansion:
  - `portfolio:write`
  - `trade:approve`
  - `admin`
