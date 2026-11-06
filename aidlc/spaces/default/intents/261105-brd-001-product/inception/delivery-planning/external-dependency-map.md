# External Dependency Map — Crypto Pocket Butler

A **Bolt** is one build pass over one Unit of Work. The build is AI-contained: there is no
external team hand-off. The only external dependencies are the services in the design, and the
approvals the requirements already list.

## Service dependencies by Bolt

| Bolt | Unit | External dependency | Kind | Blocking? |
|---|---|---|---|---|
| B1 | U1 walking skeleton | Supabase Auth (JWKS), Supabase Postgres | Service | Yes — credentials and schema |
| B2 | U12 audit | Supabase Postgres (insert-only role) | Service | Yes |
| B3 | U3 identity | Supabase Auth (sessions, Admin API), Supabase Vault | Service | Yes |
| B4 | U13 support console | Help Desk & Status Page | Service | No — can stub behind an adapter |
| B5 | U5 market data | Market Data & FX Providers | Service | Yes — needs at least one licensed source |
| B6 | U4 account | Binance API, OKX API, EVM/BNB/Solana RPC providers, WalletConnect Relay, Supabase Vault | Service | Yes |
| B7 | U10 analytics | None beyond Supabase Postgres | Service | No |
| B8 | U9 notification | Transactional Email Provider | Service | Yes — needed for email delivery |
| B9 | U7 ledger | None beyond Supabase Postgres | Service | No |
| B10 | U6 portfolio | None beyond the internal services | Service | No |
| B11 | U8 reporting | Transactional Email Provider (through Notification) | Service | No — inherited from B8 |
| B12 | U11 job orchestration | Supabase Cron, Supabase Vault | Service | Yes |
| B13 | U2 web app | Supabase Auth, WalletConnect Relay | Service | Yes |

## Approvals and non-service blockers

| Item | Owner | Blocks | If it slips |
|---|---|---|---|
| Exchange API terms for Binance and OKX in the launch markets | Legal | B6 | Use the public-address path and CSV import for first value; add exchange accounts later behind the same contract |
| Market-data and FX licence | Legal/Data | B5 | Use one permitted source and label freshness; the multi-source path stays behind the provider port |
| Launch jurisdictions and legal review (Vietnam, EU, UK) | Compliance | B3, B13 | Keep the jurisdiction gate configurable; deny by default in unreviewed markets |
| Transactional email provider account | Operations | B8 | In-app notifications still work; email delivery stays queued and logged until the provider is live |

The human answered that nothing beyond the services is expected to hold the build up, so these
are recorded as watch items rather than gated hand-offs. None is a cross-team hand-off.
