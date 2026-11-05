# Build vs Buy

## Decision Summary

Buy licensed market data and FX; integrate exchange read-only APIs through thin in-house adapters; reach chains through public chain RPC and open-source indexers, adding paid RPC capacity only where rate limits or reliability require it; build the aggregation, valuation, cost-basis and UX layer ourselves.

The split follows lock-in profile: chain RPC is low because it speaks standard JSON-RPC and has open-source indexers; market data is medium-high because of licensing, attribution and vendor asset IDs; tax engines are high because lot logic and report formats are proprietary. Concretely: CoinGecko Analyst primary with CoinMarketCap as cross-check, exchangerate.host Business for FX, a public RPC endpoint as the default with Alchemy or Infura as failover, self-hosted Blockstream Esplora for Bitcoin, one adapter per exchange, and a domain-owned lot engine. At list prices the recommended stack is roughly USD 208/month for market data (CoinGecko Analyst USD 129 + CoinMarketCap Startup USD 79), USD 99.99/month for FX, and from USD 49/month for Helius, plus up to ~USD 200/month of RPC overage — about USD 357–557/month at launch.

Q2 chose "public RPC and indexers". This plan keeps public endpoints as the default and treats paid RPC as a reliability refinement, not a change to the buy/build boundary. If strictly unpaid endpoints are intended, remove the RPC line and accept rate-limit and uptime risk; the build and data-licensing decisions are unchanged.

## Market Data and FX Layer

Retail tiers meet the freshness targets; the deciding factors are licence terms and dual-sourcing.

| Vendor | Coverage | Freshness | Licensing / cost signal | Lock-in risk | Source |
| --- | --- | --- | --- | --- | --- |
| CoinGecko | Not quantified on the pricing page; credit-based tiers | 60s free, 10s Basic, real-time from Analyst | "Powered by CoinGecko" attribution required, no redistribution or syndication, cache refreshed at least every 24h; Demo $0, Basic $35/mo, Analyst $129/mo, Lite $499/mo, Pro $999/mo, Pro+ up to $2,997/mo | Medium-high: attribution duty, no-redistribution clause, vendor asset IDs | [coingecko.com/en/api/pricing](https://www.coingecko.com/en/api/pricing), [api_terms](https://www.coingecko.com/en/api_terms) |
| CoinMarketCap | Full crypto history from 2010 | 60s on low tiers, real-time from Startup | All tiers include commercial-use rights; Basic free, Builder $29/mo, Startup $79/mo, Growth $299/mo, Professional $699/mo | Medium: friendly licence, still vendor-specific IDs | [coinmarketcap.com/api/pricing](https://coinmarketcap.com/api/pricing/) |
| Kaiko | Not quantified publicly; enterprise index/market data positioning | Not published | Pricing not publicly listed; sales/trial only; SOC-2 Type II and IOSCO-aligned index administration | High: enterprise contract | [kaiko.com](https://www.kaiko.com/), [products](https://www.kaiko.com/products) |
| CoinDesk Data | 300+ exchanges, 10,000+ coins, 300,000+ currency pairs, history to 2010 | Real-time; claims 99.99% 90-day API uptime | Pricing not publicly listed; free API tier retiring 21 May 2026 | High: opaque commercial terms | [data.coindesk.com](https://data.coindesk.com/) |
| exchangerate.host (FX) | Rates from financial data providers and banks including the ECB | Daily free, hourly Basic, every 10 min Professional Plus, every 60s Business | Free tier non-commercial only; Basic $14.99/mo, Professional Plus $59.99/mo, Business $99.99/mo; historical data is end-of-day only | Medium: rate timestamp must be persisted per valuation | [exchangerate.host/pricing](https://exchangerate.host/pricing) |

Messari/Blockworks pricing and limits are unverified: both API and pricing URLs redirect to Blockworks and repeated fetches returned HTTP 429, so no price or quota claim is made here ([messari.io/api](https://messari.io/api)).

## Chain Data Layer

Reach chain RPC, build the aggregation logic. [Alchemy](https://www.alchemy.com/pricing) offers 30M compute units/month free, then $0.52 per 1M CU with no platform fee across 100+ chains — a fit for spiky portfolio refresh. [Infura](https://www.infura.io/pricing) lists Core free at 3M credits/day, Developer $50/mo and Team $225/mo, though its FAQ states 6M credits/day free, so the allowance must be confirmed. [QuickNode](https://www.quicknode.com/pricing) bundles more credit volume at $49–$2,999/month with all supported chains included; for Solana, [Helius](https://www.helius.dev/pricing) adds the DAS asset API from $49/mo.

Bitcoin and TRON should not carry lock-in. [Blockstream Esplora](https://github.com/Blockstream/esplora/blob/master/API.md) is open source, self-hostable and documents no rate limit; [mempool.space](https://mempool.space/docs/api/rest) enforces unpublished limits and sells higher quotas via enterprise sponsorship. [TronGrid](https://developers.tron.network/docs/trongrid) publishes no numeric limits or prices and routes quota increases to sales, so assume negotiation and cache against TRON's ~3-second block time.

## Exchange Integration Layer

Build thin per-exchange adapters: authentication and metering differ fundamentally. [Binance](https://developers.binance.com/docs/binance-spot-api-docs/rest-api/general-api-information) meters per IP rather than per key, escalates 429 to an HTTP 418 ban of 2 minutes to 3 days, and defaults keys to no TRADE permission; USER_DATA covers private reads. [Kraken](https://docs.kraken.com/api/docs/guides/spot-rest-ratelimits) meters per key with a decaying counter (15–20) and documents query-only permissions ([permission list](https://support.kraken.com/hc/en-us/articles/360000919966-API-key-permissions)). [Coinbase Advanced Trade](https://docs.cdp.coinbase.com/coinbase-app/advanced-trade-apis/rest-api.md) scopes CDP JWTs to view/trade/transfer, with public market data requiring no JWT.

Two constraints: partner programmes are contractual and gated — [Binance.US API partner terms](https://www.binance.us/api-partner-terms-of-use) start at Bronze with $1M average monthly volume, five active traders, US licences and a $10,000 liability cap; and APIs churn — [Binance retired listenKey streams and SBE schemas and mandates percent-encoded payloads](https://github.com/binance/binance-spot-api-docs/blob/master/CHANGELOG.md), while [Coinbase's Pro-to-Advanced-Trade migration invalidated existing keys](https://docs.cdp.coinbase.com/coinbase-business/advanced-trade-apis/faq).

## Cost Basis and Tax Engine

Build the lot engine in the domain layer; it is a stated differentiator and the highest lock-in component. [rotki](https://rotki.com/) is the reference implementation — open source, local-first, self-hostable via Docker, free to 1,000 events and €9/€25/€45 per month above that. Do not make a consumer tax tool a runtime dependency: [CoinLedger](https://coinledger.io/pricing) sells one-time reports at $49/$99/$199+ with no documented developer API, and [CoinTracking](https://cointracking.info/api/) caps its read-only Data API at 20–60 calls/hour. [Awaken Tax](https://awaken.tax/pricing) advertises an embeddable Tax API and [TaxBit](https://taxbit.com/) positions as API-first, but neither publishes API pricing and TaxBit publishes no public reference. Koinly and CoinTracker details are unverified: both returned HTTP 403.

## Recommended Composition

- **Buy**: CoinGecko Analyst primary, CoinMarketCap cross-check, exchangerate.host Business FX; public RPC endpoint as default with Alchemy/Infura failover; Helius for Solana where the DAS API is needed.
- **Launch coverage**: 5 exchange venues (Coinbase, Kraken and Binance analysed here; the remaining two are named in delivery planning) and 4 chain families (EVM, Solana, Bitcoin, Tron). The 200–700 integration table stakes is the roadmap bar, not launch scope; the launch claim is reconciliation quality, not breadth.
- **Self-host / open source**: Blockstream Esplora for Bitcoin; rotki as cost-basis reference oracle in testing.
- **Integrate**: exchange read-only APIs via one least-privilege adapter per venue.
- **Build**: canonical asset registry, transfer matching, missing-cost-basis detection, valuation, local-currency P&L, reconciliation workflow and all UX.
- **Ports**: MarketDataProvider, FxProvider, ChainRpcProvider, ExchangeReader, CostBasisEngine — each adapter replaceable without domain changes; vendor DTOs never cross inward.

## Build Cost and TCO

The build side is the dominant cost, not the vendor line. The ranges below are planning estimates for a greenfield team, not quotations; validate them in delivery planning. A team that already owns some components starts lower.

| Build item | Effort (engineer-weeks) | Basis of estimate |
| --- | --- | --- |
| Canonical asset registry, valuation and local-currency P&L | 4–8 | One model, two quote paths, FX timestamping |
| Exchange read-only adapters | 2–4 per venue; 5 launch venues = 10–20 | Auth, pagination, rate limits, reconciliation edge cases |
| Chain readers (EVM, Solana, Bitcoin, Tron) | 3–5 each = 12–20 | Address history, token discovery, confirmation handling |
| Cost-basis lot engine | 6–10 | Lot selection, fees, transfers, missing-basis detection |
| Transfer matching and reconciliation queue | 4–8 | Cross-venue matching, duplicate suppression, user resolution |
| React SPA, onboarding, reconciliation queue and design system | 12–20 | Screens, state, PWA shell, accessibility, error and empty states |
| Auth, subscription billing/entitlements and notifications | 5–9 | Supabase Auth, plans and quotas, email and web push |
| Localization, WCAG 2.2 AA and PWA shell | 4–7 | Locale plumbing, translated copy, accessibility audit, installability |
| Compliance surface (jurisdiction gating, disclosures, DPIA) | 3–6 | Jurisdiction checks, legal surfaces, privacy review |
| Ports, observability, tests and security review | 4–6 | Five ports, adapter contract tests, threat model |
| **Total** | **64–114 engineer-weeks (~15–27 engineer-months)** | Sum of the rows |

At a fully loaded USD 10,000–15,000 per engineer-month (planning assumption; varies by market), the one-time build is USD 150,000–405,000. Ongoing maintenance at 20–30% of build per year is USD 30,000–122,000/year. Vendor data at roughly USD 4,300–6,700/year (including up to ~USD 200/month of RPC overage), plus ~USD 50–150/month for self-hosted Esplora hosting (planning assumption), is an order of magnitude cheaper than building the data layer, while the build items are the product's differentiators. Break-even on building data in-house would require a scale and licensing position this launch does not have.

The TCO covers build, data and hosting only. Payment-processing fees, customer support, go-to-market and per-jurisdiction legal/licensing costs are delivery-planning lines and are excluded here.

## Cost and Operational Risk

Launch vendor run-rate is roughly USD 357–557/month including up to ~USD 200/month of RPC overage, plus self-hosted Esplora hosting (~USD 50–150/month, planning assumption) — small against the one-time build above and against the ARR targets; the real risks are contractual and operational. Licensing: CoinGecko forbids redistribution and derived valuations absent consent, so per-jurisdiction display rights need written confirmation. Accuracy: >99% requires dual-sourcing and divergence alerting, and single-vendor tail-asset quality is unverified. Availability: the 99.9% target needs a defined fallback — cached last-known price with a staleness badge. Exchange fragility: bans, key rotation and deprecations demand adapter tests and per-venue circuit breakers. Scope: drifting into tax filing adds audit and privacy obligations beyond the product boundary.

## Assumptions & Open Questions

- Enterprise quotes for Kaiko, CoinDesk Data and Messari/Blockworks are not publicly listed; assume annual, post-MVP.
- Redistribution and derived-valuation rights at retail price remain unresolved; CoinGecko forbids them and CoinMarketCap's display terms need legal review.
- Tail-asset accuracy outside the top 100 is unverified against a second source.
- Exchange commercial-use and retention terms for storing full trade history were not researched.
- DeFi position decoding (LP tokens, staking, lending) is explicitly post-MVP per the intent boundary, so it is excluded from the launch build and TCO; pulling it into launch would add roughly 6–10 engineer-weeks.
- Koinly and CoinTracker API/pricing details are unverified: Koinly's pricing renders dynamically and CoinTracker's pricing page returned HTTP 403.
