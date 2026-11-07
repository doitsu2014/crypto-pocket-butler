# Entity Model — u5-market-data

The YAML block below is the source of truth for the entity model. Technology-agnostic: no SQL, no
framework, no language types.

Inputs: the Market Data unit in `inception/units-generation/unit-of-work.md`, the requirements in
`inception/requirements-analysis/requirements.md` (FR3, FR4.5, NFR5), the Market Data component in
`inception/domain-design/components.md`, and C4/C6 in
`inception/contract-design/contract-summary.md`. The price-source registry and the canonical asset
list, with each asset's rank, are owned by the Asset Registry (U1) and read through C4; Market Data
stores only observations, tiers, FX readings, and refresh runs.

```yaml
entities:
  - name: AssetPriceTier
    description: The freshness tier of one canonical asset, which sets its freshness bound.
    attributes:
      - name: assetId
        type: string
        required: true
        unique: true
        description: The canonical asset id from C4.
      - name: rank
        type: integer
        required: true
        min: 1
        description: Copied from C4 when the tier is written.
      - name: tier
        type: enum
        required: true
        allowed_values: [top100, tail]
      - name: boundSeconds
        type: integer
        required: true
        description: 300 for top100, 900 for tail.
      - name: updatedAt
        type: timestamp
        required: true
    constraints:
      - An asset with no tier row is treated as tail with a 900-second bound.
      - The tier is refreshed from C4 at the start of each refresh run.

  - name: PriceObservation
    description: One reading of an asset price from one source.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: assetId
        type: string
        required: true
      - name: sourceName
        type: string
        required: true
        description: The C4 PriceSource.name, for example binance or coingecko.
      - name: price
        type: decimal
        required: true
        description: Decimal string; never binary floating point.
      - name: currency
        type: string
        required: true
        description: The source's quote currency, normalised to uppercase.
      - name: observedAt
        type: timestamp
        required: true
        description: The source's own timestamp; the freshness rule reads this.
      - name: receivedAt
        type: timestamp
        required: true
      - name: status
        type: enum
        required: true
        allowed_values: [accepted, rejected]
        default: accepted
      - name: rejectionReason
        type: string
        required: false
        description: Present only for a rejected reading; never contains a credential.
    constraints:
      - price is greater than zero; a zero or negative reading is rejected, never stored as free.
      - A rejected observation is kept for diagnosis and is never served.
      - A reading whose observedAt is more than 300 seconds ahead of receivedAt is rejected.

  - name: PriceQuote
    description: The served price for one asset, selected across sources.
    attributes:
      - name: assetId
        type: string
        required: true
      - name: price
        type: decimal
        required: false
        description: Absent when freshness is unpriced.
      - name: currency
        type: string
        required: false
      - name: source
        type: string
        required: false
        description: The winning C4 source name; absent when unpriced.
      - name: observedAt
        type: timestamp
        required: false
      - name: freshness
        type: enum
        required: true
        allowed_values: [live, delayed, stale, unpriced]
    constraints:
      - A live, delayed, or stale quote carries price, source, and observedAt.
      - An unpriced quote carries none of them and no price value.

  - name: FxRate
    description: One reading of an exchange rate between two currencies.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: base
        type: string
        required: true
      - name: quote
        type: string
        required: true
      - name: rate
        type: decimal
        required: true
        description: One base unit in quote units; decimal, greater than zero.
      - name: source
        type: string
        required: true
      - name: observedAt
        type: timestamp
        required: true
      - name: receivedAt
        type: timestamp
        required: true
    constraints:
      - base differs from quote.
      - A rate is greater than zero; a zero or negative rate is rejected.
      - A derived cross rate is computed per request and is not stored as a source reading.

  - name: RefreshRun
    description: One scheduled or manual price-and-FX refresh.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: job
        type: enum
        required: true
        allowed_values: [top100, tail]
        description: Each job has its own single-flight lease.
      - name: startedAt
        type: timestamp
        required: true
      - name: leaseUntil
        type: timestamp
        required: true
        description: The run holds the single-flight guard until this time and renews it while working.
      - name: finishedAt
        type: timestamp
        required: false
      - name: status
        type: enum
        required: true
        allowed_values: [running, succeeded, failed]
        default: running
      - name: requestedCount
        type: integer
        required: true
        min: 0
      - name: pricedCount
        type: integer
        required: true
        min: 0
      - name: unpricedCount
        type: integer
        required: true
        min: 0
      - name: errorCategory
        type: enum
        required: false
        allowed_values: [rate_limit, provider, network, data]
      - name: errorMessage
        type: text
        required: false
        description: A non-secret summary; never a key or token.
    constraints:
      - At most one run per job is running with a live lease; the top100 and tail jobs may run concurrently.
      - A run whose lease expires without finishing is marked failed and releases the guard.
      - requestedCount equals pricedCount plus unpricedCount when the run finishes.

relationships:
  - from: RefreshRun
    to: PriceObservation
    cardinality: one-to-many
    direction: RefreshRun records the observations it made
  - from: AssetPriceTier
    to: PriceObservation
    cardinality: one-to-many
    direction: AssetPriceTier bounds the freshness of an asset's observations
```

## Summary

Market Data stores the per-asset freshness tier, every price observation with its source name and
timestamps, the FX readings, and each refresh run with its lease. The price-source registry and the
canonical asset list are Asset Registry data, read through C4. A served quote is selected from the
observations by `rules.md`; it always names its source and observation time unless it is unpriced.
Quantities and rates are decimal strings. Market Data owns no portfolio, holding, or valuation.
