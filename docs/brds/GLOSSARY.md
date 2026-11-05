# Business Glossary

One plain definition per term, grouped by business domain. The defining BRD is in parentheses.

## Identity & Access

- **Portfolio Owner** — the user who owns accounts, portfolios, transactions, snapshots, and orders. (BRD-002)
- **Admin** — an Owner with administrative capabilities for registries, users, entitlements, jobs, and flags. (BRD-002)
- **Support** — a read-only, time-bound, audited role for account metadata and tickets; never credentials. (BRD-002/019)
- **Analyst** — post-MVP read-only role for analytics and shared portfolios. (BRD-002)
- **Viewer** — post-MVP read-only role for specific shared portfolios. (BRD-002)
- **Disabled user** — an internal user denied access even with a valid provider token. (BRD-003)
- **Jurisdiction gate** — the check that denies or restricts sign-up and use in a market. (BRD-003/012/017)

## Account Management

Two account types, each owning its own holdings:

- **Account** — a read-only source of holdings: exchange, on-chain, WalletConnect, watch-only, or imported. Each account owns its own holdings. (BRD-004)
- **Exchange account** — connected to a centralized exchange via **verified read-only** API credentials. (BRD-004)
- **On-chain account** — identified by public address only, on one or more enabled chains; no private keys are ever accepted. (BRD-004)
- **WalletConnect account** — a read-only wallet session or public address connected through WalletConnect or equivalent. (BRD-004)
- **Watch-only account** — an xpub or public address observed without keys. (BRD-004)
- **Imported account** — a CSV-derived source reconciled against synced holdings. (BRD-004/013)
- **Sub-account** — an exchange sub-account rolled up to one parent without double-counting. (BRD-004)
- **Holding** — a quantity of an asset held in an account; holdings belong to their account. (BRD-004)
- **Frozen** — quantity on an exchange not currently available for trading or withdrawal. (BRD-004)
- **Spam token** — an unsolicited or worthless token hidden by default and excluded from valuation. (BRD-004)
- **Dust** — a balance too small to affect valuation; filtered from display, not destroyed. (BRD-004)
- **DeFi position** — a staking, lending, or liquidity position that is not a plain token balance; labelled supported, unsupported, or unpriced. (BRD-004)
- **Coverage matrix** — the per-region list of exchanges, chains, and import methods the product supports. (BRD-004)

## Asset Catalog & Market Data

An **asset** (BTC is one asset) is the root: chain representations and metric series all belong to it.

- **Asset** — one canonical, tradable unit, identified by a unique symbol (BTC, ETH, USDC). (BRD-005)
- **Asset contract** — the asset's representation on a chain: contract, mint, or address format, with decimals. (BRD-005)
- **Asset metric series** — point-in-time observations of one fact about an asset, each with timestamp, source, and confidence. (BRD-005)
- **Price observation** — a metric series: the asset's price at a timestamp from a named source. (BRD-005)
- **FX rate** — a metric series: the conversion rate between two currencies at a timestamp. (BRD-005)
- **Price source priority** — the deterministic order in which sources are tried for an asset. (BRD-005)
- **Unmapped asset** — a token that cannot be resolved to a canonical asset; excluded from valuation. (BRD-005)
- **Unpriced asset** — an asset with quantity but no usable price; excluded from valued totals and reported separately. (BRD-005/007)
- **De-peg** — a stablecoin deviating from its target value beyond a threshold, triggering a factual alert. (BRD-005)
- **Data coverage** — the share of portfolio value that is priced, fresh, and supported. (BRD-005/007)

## Portfolio Management

- **Portfolio** — a user's grouping of accounts, with an optional target and guardrails; the unit of valuation, drift, snapshots, orders, and reporting. (BRD-006)
- **Shared account** — an account that is a member of more than one portfolio; holdings are counted once per portfolio and trades affect every portfolio containing it. (BRD-006)
- **Default portfolio** — the user's primary portfolio; exactly one per user. (BRD-006)
- **Target allocation** — intended percentage weight per asset; must total 100% when defined, including `unallocated`. (BRD-006)
- **Target range** — an intended minimum/maximum weight per asset as an alternative to a single percentage. (BRD-006)
- **Unallocated** — an explicit target bucket for value not assigned to a named asset. (BRD-006)
- **Guardrails** — constraints orders and drift alerts must respect: maximum deviation, minimum and maximum order size, allowed assets. (BRD-006)
- **Model template** — a preset target allocation that seeds a portfolio; it never generates orders. (BRD-006)
- **Rebalancing proposal** — an informational, user-initiated calculation of trades that would move allocation toward target; it never executes. (BRD-006)

## Valuation & Snapshots

- **Valuation** — an asset's or portfolio's value in the user's base currency, from quantity × price × FX. (BRD-007)
- **Allocation** — a derived side object of a portfolio: the point-in-time breakdown of its priced value by asset, each weight in percent, always totalling 100%. (BRD-007)
- **Weight** — an asset's share of the portfolio's priced total, as a percentage. (BRD-007)
- **Drift** — an asset's actual weight minus its target weight, in percentage points. (BRD-007)
- **Base currency** — the currency value is reported in; user-selected, USD as accounting fallback. (BRD-007/018)
- **Snapshot** — an immutable point-in-time record of holdings, valuation, weights, base currency, FX, and data coverage. **EOD** snapshots are scheduled; **manual** snapshots are on request. (BRD-007)
- **EOD** — end-of-day, taken at the user's local midnight by default. (BRD-007/011)
- **Change attribution** — the split of a period change into market movement, flows, fees, and income. (BRD-007/020)

## Transactions, Cost Basis & P&L

- **Transaction** — one economic event in an account: buy, sell, deposit, withdrawal, internal transfer, fee, reward, staking income, airdrop, or adjustment. (BRD-013)
- **Internal transfer** — a movement between the user's own accounts; not a P&L event. (BRD-013)
- **Cost basis** — the base-currency cost of the quantity still held, under FIFO or average cost at launch. (BRD-013)
- **Lot** — a parcel of an asset acquired at a specific cost and time. (BRD-013)
- **Net invested** — external deposits minus external withdrawals in base currency. (BRD-013)
- **Realized P&L** — proceeds minus the cost basis of the quantity sold, minus sell fees. (BRD-013)
- **Unrealized P&L** — current market value minus the cost basis of the quantity still held. (BRD-013)
- **Income** — rewards, staking income, and airdrops at receipt value. (BRD-013)
- **Total P&L** — realized + unrealized + income − fees. (BRD-013)
- **Simple return** — total P&L divided by net invested, when net invested is positive. (BRD-013)
- **Reconciliation** — the comparison of ledger-derived quantity against synced holdings, with `matched`, `mismatched`, or `unavailable` states. (BRD-013)
- **Tax-ready export** — a data export with jurisdiction-neutral columns; not tax advice or a filing. (BRD-013)

## Orders & Trades

- **Order intent** — a user-created buy/sell instruction for an asset in a portfolio. An instruction only: pending, filled, or cancelled. (BRD-008)
- **Trade** — the executed transaction that fills an order: asset, side, quantity, venue, execution time. A trade changes the account immediately. (BRD-008)
- **Reconciliation state** — a trade's match against the next holdings sync: pending, matched, mismatched, or stale. (BRD-008)
- **Deep link** — a convenience link to the user's venue for execution; not execution by the product. (BRD-008)

## Reporting

- **Brief** — a scheduled daily or weekly portfolio report, composed in-app and deliverable by email and PDF. (BRD-009)

## Performance & Benchmarks

- **TWR** — time-weighted return, removing the effect of external cash flows. (BRD-020)
- **MWR / IRR** — money-weighted return, reflecting the timing and size of flows. (BRD-020)
- **Benchmark** — a reference return (BTC, ETH, or a broad index) with source, currency, and timestamp. (BRD-020)
- **Contribution** — an asset-level effect on portfolio return. (BRD-020)
- **Attribution** — the split of return into market movement, flows, fees, and income. (BRD-020)
- **Volatility** — the dispersion of returns over a period. (BRD-020)
- **Maximum drawdown** — the largest peak-to-trough decline over a period. (BRD-020)

## Notifications

- **Notification** — a message delivered in-app, by email, or by web push. (BRD-015)
- **Alert** — a notification triggered by a defined condition such as drift, price, sync failure, or key revocation. (BRD-015)
- **Mandatory alert** — a security, credential-integrity, or billing alert that cannot be fully disabled. (BRD-015)
- **Quiet hours** — the user's local window in which non-urgent alerts wait. (BRD-015)
- **Digest** — a batched notification for non-urgent events. (BRD-015)
- **Dedupe window** — the period in which the same alert is not repeated. (BRD-015)

## Billing

- **Plan** — Free, Pro, Team, or Enterprise, with a defined entitlement set. (BRD-016)
- **Entitlement** — a capability or limit granted by a plan, enforced server-side. (BRD-016)
- **Quota** — a measured limit such as accounts, portfolios, exports, or API calls. (BRD-016)
- **Merchant of record** — the party that calculates, collects, and remits subscription tax globally. (BRD-016)
- **Grace period** — the window after a failed payment before downgrade. (BRD-016)
- **Past due** — a subscription state after a failed renewal. (BRD-016)

## Privacy & Data Rights

- **Personal data** — information linked to an identifiable user, including public addresses in context. (BRD-017)
- **Secret** — exchange credentials, API keys, tokens, and any value that grants access. (BRD-012/017)
- **Data map** — the inventory of data categories, purposes, locations, and retention. (BRD-017)
- **Retention** — the defined period after which data is deleted or anonymised. (BRD-017)
- **Subprocessor** — a third party processing user data on the product's behalf. (BRD-017)
- **Data residency** — the region where data is stored and processed. (BRD-017)
- **Legal hold** — a requirement to preserve records beyond normal retention. (BRD-017)
- **Consent** — an explicit, withdrawable permission for marketing or non-essential analytics. (BRD-017)
- **Crypto-shredding** — destroying the key that makes encrypted data readable. (BRD-012/017)

## Localization & Accessibility

- **Locale** — the combination of language, region, number/date format, and text direction. (BRD-018)
- **Base currency** — see Valuation; user-selected display and accounting currency. (BRD-018)
- **FX display** — conversion for display only; the original value is retained. (BRD-018)
- **RTL** — right-to-left text direction, required before launching an RTL locale. (BRD-018)
- **WCAG 2.2 AA** — the accessibility conformance target for core flows. (BRD-018)

## Support & Operations

- **Ticket** — a user support request with status, history, and notifications. (BRD-019)
- **SEV** — incident severity from SEV-1 (critical) to SEV-4 (cosmetic). (BRD-019)
- **Status page** — the public component-health and incident-history page. (BRD-019)
- **Postmortem** — the published review of a SEV-1 or SEV-2 incident. (BRD-019)
- **Support access** — read-only, time-bound, reason-recorded access to account metadata. (BRD-002/019)
- **System job** — a scheduled process that refreshes data or creates records without a user session. (BRD-011)
- **Dead-man check** — the alert that fires when the scheduler heartbeat is stale. (BRD-011)

## Analytics & Experimentation

- **Event taxonomy** — the versioned list of analytics events and non-PII properties. (BRD-021)
- **Activation** — the first value event: a first portfolio valuation within 24 hours of sign-up. (BRD-021)
- **North star** — the primary product metric: weekly active portfolios with a valid valuation. (BRD-021)
- **Guardrail metric** — a metric that must not degrade during an experiment. (BRD-021)
- **Feature flag** — a controlled switch with an owner, expiry, and rollout percentage. (BRD-021)
- **Experiment** — a pre-registered test with a hypothesis, sample size, and guardrails. (BRD-021)
