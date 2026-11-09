# Business Rules — u6-portfolio

```yaml
rules:
  - id: BR1.1
    statement: Every portfolio belongs to one user and every read is owner-scoped.
    category: authorization
    applies_to: Portfolio
    trigger: Any read or write of a portfolio or its children.
    logic: IF the caller is the owner THEN allow; ELSE refuse not-found.
    violation: No other user's portfolio data is ever returned.
    source: FR1.6, FR4.1

  - id: BR1.2
    statement: A user always has one portfolio, created on demand.
    category: policy
    applies_to: Portfolio
    trigger: The user opens the app with no portfolio.
    logic: IF the user has no portfolio THEN create a default one and open it; ELSE open the existing one.
    violation: The app never shows an empty portfolio list to a signed-in user.
    source: FR2.4, AC3.2.3

  - id: BR1.3
    statement: An account belongs to at most one portfolio.
    category: constraint
    applies_to: PortfolioAccount
    trigger: An account is assigned or moved.
    logic: IF the account is already in another portfolio THEN move it; ELSE assign it.
    violation: Both portfolios never value the same account.
    source: FR2.4, AC3.2.1–3.2.2

  - id: BR2.1
    statement: Valuation is quantity × price, converted to the base currency, in the backend.
    category: calculation
    applies_to: ValuationSnapshot
    trigger: A portfolio is valued.
    logic: IF holdings are available from C5 AND quotes from C6 THEN value each holding at its quote and convert with C6 FX; the SPA never computes a value.
    violation: No business math runs in the browser.
    source: FR4.1, FR3.1, AC3.1.1

  - id: BR2.2
    statement: A foreign-currency holding shows the FX rate and its timestamp; every asset shows its price timestamp.
    category: policy
    applies_to: ValuationSnapshot
    trigger: A valuation is rendered.
    logic: IF a holding is not in the base currency THEN include the FX rate, its source (and `derivedFrom` for a cross rate), and observedAt; always include the quote source and observedAt. The base currency comes from the portfolio, defaulted from the user's Identity preference.
    violation: No value is shown without its provenance.
    source: FR3.4, AC3.1.2–3.1.3

  - id: BR2.3
    statement: An unpriced asset is excluded from the priced total and labelled.
    category: policy
    applies_to: ValuationSnapshot
    trigger: A holding has no price.
    logic: IF a quote is unpriced THEN exclude the holding from the total, keep it in the breakdown as a labelled line with its quantity and `unpriced` status, and return the count of excluded assets (no value is fabricated for them).
    violation: The total is never silently wrong.
    source: FR3.5, AC3.4.1–3.4.2

  - id: BR3.4
    statement: A guardrail requires a target for the same asset.
    category: validation
    applies_to: Guardrail
    trigger: Targets and guardrails are saved.
    logic: IF a guardrail has no matching TargetAllocation in the saved set THEN refuse with a field-level reason.
    violation: A guardrail never exists without a target.
    source: FR4.2

  - id: BR3.1
    statement: Drift is actual minus target in percentage points.
    category: calculation
    applies_to: ValuationSnapshot
    trigger: A portfolio with targets is viewed.
    logic: IF targets exist THEN actual allocation = priced line value / priced total (unpriced holdings are outside the denominator), and drift = actual − target in percentage points with one decimal.
    violation: Drift is never shown as a percentage change.
    source: FR4.3, AC5.2.1–5.2.2

  - id: BR3.2
    statement: Targets need not sum to 100; the difference is shown before saving.
    category: policy
    applies_to: TargetAllocation
    trigger: Targets are saved.
    logic: IF the target set sums to less or more than 100 THEN return the difference to the preview; save only when the user confirms.
    violation: The saved targets are what the user saw.
    source: FR4.2, AC5.1.1–5.1.2

  - id: BR3.3
    statement: With no targets, drift is absent with a prompt.
    category: policy
    applies_to: Portfolio
    trigger: A portfolio without targets is viewed.
    logic: IF no target exists THEN omit drift and return a set-targets prompt.
    violation: Drift is never fabricated.
    source: AC5.2.3

  - id: BR4.1
    statement: A snapshot is immutable and reproducible from its stored inputs.
    category: constraint
    applies_to: ValuationSnapshot
    trigger: A snapshot is taken or read.
    logic: IF a snapshot exists THEN never update or delete it; a recompute stores a new snapshot.
    violation: A later sync, price, or target change never alters an existing snapshot.
    source: FR4.4, NFR8, AC4.4.1–4.4.2

  - id: BR4.2
    statement: A snapshot freezes holdings, quotes, FX, and targets with a digest.
    category: policy
    applies_to: ValuationSnapshot
    trigger: A snapshot is created.
    logic: IF a snapshot is created THEN store the input set and a SHA-256 digest; reproduction verifies the digest and recomputes the same lines.
    violation: A snapshot with a mismatched digest is flagged, never silently served.
    source: NFR8, AC4.4.2

  - id: BR5.1
    statement: An order intent is advisory only.
    category: constraint
    applies_to: OrderIntent
    trigger: An intent is created or cancelled.
    logic: IF an intent is created or cancelled THEN no external order call is made.
    violation: The product never places a trade.
    source: FR6.1, AC5.5.2

  - id: BR5.2
    statement: An intent preview projects allocation and drift, including a shortfall.
    category: calculation
    applies_to: OrderIntent
    trigger: An intent is previewed or created.
    logic: IF the intent quantity is valid THEN project post-trade allocation and drift; IF a sell exceeds holdings THEN state the shortfall before recording.
    violation: An intent is never recorded without its projection and any shortfall.
    source: FR6.2, AC5.3.1–5.3.3

  - id: BR5.3
    statement: A cancelled intent stays visible.
    category: policy
    applies_to: OrderIntent
    trigger: An intent is cancelled.
    logic: IF cancelled THEN set status cancelled and cancelledAt; keep it in the list.
    violation: History is never erased.
    source: AC5.5.1, AC5.5.3

  - id: BR6.1
    statement: A recorded trade writes a ledger entry and an audit record and updates allocation immediately.
    category: policy
    applies_to: RecordedTrade
    trigger: A trade is recorded.
    logic: IF the trade validates THEN call Ledger C7 `POST /v1/users/{userId}/trades` with a client idempotency key, write C10, and add a TradeOverlay delta; the valuation adds overlays to C5 holdings so allocation and drift reflect the trade immediately. When a C5 sync reports a trade whose canonical key matches, the overlay is removed so nothing is double counted; the post-trade recompute also runs guardrail evaluation.
    violation: The portfolio is never stale after a recorded trade, and no trade is counted twice.
    source: FR6.4, AC5.4.1–5.4.2

  - id: BR6.2
    statement: A recorded trade with no price requires a user-entered price.
    category: validation
    applies_to: RecordedTrade
    trigger: A trade is recorded without a price.
    logic: IF price is missing THEN require it and label priceSource user; ELSE use the venue price.
    violation: A user price is labelled, never confused with a venue price.
    source: AC5.4.3

  - id: BR7.1
    statement: Portfolio owns guardrail crossing state and notifies once per crossing.
    category: policy
    applies_to: Guardrail
    trigger: A valuation completes.
    logic: IF |drift| crosses the guardrail from inside THEN store isOutside = true, crossedAt, and lastCrossingKey = (user, portfolio, asset, guardrail, crossingAt), and submit a C9 `drift` alert with the portfolio, asset, drift, and guardrail; IF it stays outside THEN reuse the key; IF it returns inside THEN clear isOutside and the key.
    violation: One crossing, one notification; a new crossing after recovery notifies again.
    source: FR7.4, AC5.6.1–5.6.3
```

## Rules summary

| Rule | Category | Statement |
|---|---|---|
| BR1.1 | authorization | Owner-scoped reads |
| BR1.2 | policy | One portfolio, created on demand |
| BR1.3 | constraint | Account in one portfolio |
| BR2.1 | calculation | Backend valuation |
| BR2.2 | policy | Provenance on every value |
| BR2.3 | policy | Unpriced excluded and labelled |
| BR3.1 | calculation | Drift in percentage points |
| BR3.2 | policy | Non-100 targets shown before save |
| BR3.3 | policy | No targets, no drift |
| BR4.1 | constraint | Immutable snapshots |
| BR4.2 | policy | Frozen inputs + digest |
| BR5.1 | constraint | Intent is advisory |
| BR5.2 | calculation | Projection and shortfall |
| BR5.3 | policy | Cancelled stays visible |
| BR6.1 | policy | Recorded trade → Ledger + audit + immediate allocation |
| BR6.2 | validation | User price labelled |
| BR7.1 | policy | Guardrail crossing state and one alert per crossing |
