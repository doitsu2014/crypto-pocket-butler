---
name: crypto-pocket-butler-api
description: Integrate an OpenClaw agent with the Crypto Pocket Butler API (Rust/Axum) secured by Keycloak (OIDC). Use when the agent needs to authenticate via Keycloak, call API endpoints (portfolios, accounts, holdings, snapshots, recommendations), or write/read portfolio state.
---

# Crypto Pocket Butler API (Keycloak + API)

## Configuration (env)

Required:
- `CPB_API_BASE_URL` (e.g. `https://api.example.com`)
- `CPB_KEYCLOAK_TOKEN_URL` (e.g. `https://kc.example.com/realms/<realm>/protocol/openid-connect/token`)
- `CPB_KEYCLOAK_CLIENT_ID`

One of:
- **Service account (recommended for agents):**
  - `CPB_KEYCLOAK_CLIENT_SECRET`
  - Uses OAuth2 **client_credentials** to obtain an access token.
- **User token passthrough (interactive):**
  - `CPB_ACCESS_TOKEN`

Optional:
- `CPB_API_TIMEOUT_MS` (default 15000)

## Golden rules

- Prefer **read-only** operations unless the user explicitly requests changes.
- Never store or log secrets (client secret, tokens, exchange API keys).
- All API calls must include `Authorization: Bearer <token>`.

## Auth flow (agent/service)

1) Obtain token (client credentials):

```bash
curl -sS "$CPB_KEYCLOAK_TOKEN_URL" \
  -d "grant_type=client_credentials" \
  -d "client_id=$CPB_KEYCLOAK_CLIENT_ID" \
  -d "client_secret=$CPB_KEYCLOAK_CLIENT_SECRET"
```

2) Use `access_token` in API calls.

## API usage workflow (typical)

1) List portfolios for the authenticated user.
2) Pick portfolio (by `portfolio_id`) and pull latest snapshot.
3) Compute allocation/drift (API may provide) and produce:
   - a short Telegram summary
   - a Notion “Daily Brief” entry
4) If user asks to rebalance: create a **recommendation** object (suggest-only) and return proposed orders.

## References

- API endpoints + schemas: see `references/api.md`.
- Token fetching helper scripts: see `scripts/`.
