# Business Rules — u5-market-data

The YAML block below is the source of truth for the business rules. Rules are grouped: BR1 price
selection and request validation, BR2 freshness and provenance, BR3 FX, BR4 refresh runs, BR5
access, audit, and retention.

Constants used by the rules: `TOP100_BOUND_SECONDS = 300`, `TAIL_BOUND_SECONDS = 900`,
`HARD_BOUND_SECONDS = 86400`, `CLOCK_TOLERANCE_SECONDS = 300`, `FX_BOUND_SECONDS = 3600`,
`RUN_LEASE_SECONDS = 300`.

```yaml
rules:
  - id: BR1.1
    statement: Prices are selected from enabled C4 sources in deterministic priority order.
    category: policy
    applies_to: PriceQuote
    trigger: A price is requested for an asset.
    logic: IF the enabled C4 sources are ordered by priority THEN serve the newest accepted observation whose age is within the asset's bound from the lowest priority number; ELSE continue to the next source.
    violation: The same request never returns two different sources for the same asset.
    source: FR3.2

  - id: BR1.2
    statement: A failing primary source falls back to the next source and records the fallback.
    category: policy
    applies_to: PriceQuote
    trigger: The preferred source errors, times out, or returns a rejected reading.
    logic: IF the preferred source fails THEN try the next enabled source in priority order and serve its observation with that source as provenance.
    violation: A failed source never causes an unpriced result while a usable lower-priority observation exists.
    source: FR3.2, AC3.3.3

  - id: BR1.3
    statement: No price is ever guessed or treated as zero.
    category: constraint
    applies_to: PriceQuote
    trigger: No source yields an observation within its bound.
    logic: IF any accepted observation exists within HARD_BOUND_SECONDS THEN return the newest one across the enabled sources as stale, naming its source; ELSE return unpriced with no price value.
    violation: A zero or invented price is never returned.
    source: FR3.5, AC3.4.1

  - id: BR1.4
    statement: The source registry is Asset Registry data; Market Data never owns a second copy.
    category: validation
    applies_to: PriceQuote
    trigger: A refresh or a price request starts.
    logic: IF C4 returns the enabled sources with a total priority order THEN use it as-is; IF C4 is unavailable THEN serve from stored observations and mark them stale, never from a cached mutable source list.
    violation: Priority can never diverge between the two services.
    source: FR3.2, NFR5

  - id: BR1.5
    statement: Price and FX requests are validated at the boundary.
    category: validation
    applies_to: PriceQuote, FxRate
    trigger: A C6 request arrives.
    logic: IF assetIds is empty or malformed THEN return 422; IF base equals quote or a currency is not a three-letter code THEN return 422; ELSE process.
    violation: A malformed request returns a problem document and no data.
    source: FR3.2, FR3.3

  - id: BR2.1
    statement: A price's freshness follows its tier bound.
    category: calculation
    applies_to: PriceQuote
    trigger: A quote is served.
    logic: IF the C4 source has delayedFeed true and the age is within the asset's boundSeconds THEN freshness is delayed; ELSE IF the age is within the boundSeconds THEN freshness is live; ELSE IF the age is beyond the bound but within HARD_BOUND_SECONDS THEN freshness is stale; ELSE IF no observation exists THEN freshness is unpriced.
    violation: A quote older than the bound is never labelled live.
    source: FR3.4, NFR5, AC3.3.2

  - id: BR2.2
    statement: Every priced quote carries its source and observedAt.
    category: constraint
    applies_to: PriceQuote
    trigger: A quote is served.
    logic: IF freshness is live, delayed, or stale THEN source and observedAt are present; IF freshness is unpriced THEN both are absent and no price value is returned.
    violation: A priced quote without provenance is refused.
    source: FR3.4, AC3.3.1

  - id: BR2.3
    statement: An asset with no sourceable price is excluded from priced totals and labelled.
    category: policy
    applies_to: PriceQuote
    trigger: A consumer values a portfolio.
    logic: IF a quote is unpriced THEN the consumer excludes the asset from the priced total and states the excluded value; Market Data never substitutes another asset's price.
    violation: The total is never silently reduced or inflated.
    source: FR3.5, AC3.4.1, AC3.4.2

  - id: BR2.4
    statement: A later available source prices an unpriced asset on the next refresh.
    category: policy
    applies_to: PriceQuote
    trigger: A refresh runs after a source starts covering the asset.
    logic: IF an acceptable observation exists at refresh time THEN the next quote is priced and the label clears.
    violation: A previously unpriced asset is not pinned to unpriced.
    source: AC3.4.3

  - id: BR3.1
    statement: Values are converted with the requested FX pair when it is fresh.
    category: calculation
    applies_to: FxRate
    trigger: A conversion is requested for a base and quote currency.
    logic: IF a stored reading for the pair is within FX_BOUND_SECONDS THEN serve it with source and observedAt; ELSE treat the conversion as unavailable and return a problem document.
    violation: An unavailable conversion is surfaced, never defaulted to a ratio of one.
    source: FR3.3, FR4.5

  - id: BR3.2
    statement: A missing direct pair may be derived through USD from two fresh legs.
    category: calculation
    applies_to: FxRate
    trigger: The direct pair is absent.
    logic: IF fresh base-to-USD and USD-to-quote readings exist THEN serve the product with both leg sources in derivedFrom; ELSE unavailable.
    violation: A cross rate is never served from a stale leg.
    source: FR3.3

  - id: BR3.3
    statement: FX rates are decimal and positive.
    category: validation
    applies_to: FxRate
    trigger: A rate is recorded.
    logic: IF rate is greater than zero THEN accept; ELSE reject and record the rejection.
    violation: A zero or negative rate is never stored or served.
    source: FR3.3, NFR7

  - id: BR4.1
    statement: At most one refresh run per job is active at a time, guarded by a per-job lease.
    category: constraint
    applies_to: RefreshRun
    trigger: A scheduled or manual refresh starts.
    logic: IF a run of the same job is running with leaseUntil in the future THEN the scheduled path returns 409, the manual path returns 202 with the active run id, and no new run of that job starts; IF the lease has expired THEN the sweep marks it failed and the new run claims; ELSE start a run for that job with a RUN_LEASE_SECONDS lease, renewed every 60 seconds. The top100 and tail jobs hold separate leases and may run concurrently.
    violation: A second concurrent run is never created; an abandoned run never blocks forever.
    source: FR10.1, NFR1

  - id: BR4.2
    statement: Transient source failures are retried with a bounded backoff.
    category: policy
    applies_to: RefreshRun
    trigger: A source call fails.
    logic: IF the failure is a timeout, a 5xx, or a 429 THEN retry up to three attempts with jitter; IF it is any other 4xx THEN treat it as permanent for this run and continue with the next asset.
    violation: A permanent failure is never retried forever; one failing source does not fail the whole run while another can serve.
    source: NFR5

  - id: BR4.3
    statement: Per-source rate limits are respected.
    category: constraint
    applies_to: RefreshRun
    trigger: A run calls a source.
    logic: IF the source's rate limit would be exceeded THEN wait or leave the asset to the next run, keeping any previous observation for selection; ELSE call the source.
    violation: A rate-limited source is never hammered; a 429 is recorded as rate_limit.
    source: FR3.2, NFR5

  - id: BR4.4
    statement: A malformed or rejected reading never replaces a served price.
    category: policy
    applies_to: PriceObservation
    trigger: A source returns an unusable reading.
    logic: IF the reading is malformed, non-positive, or outside CLOCK_TOLERANCE_SECONDS THEN store it as a rejected observation and keep the previous accepted observation for selection.
    violation: The last good price stays served until it becomes stale.
    source: FR3.2

  - id: BR4.5
    statement: Run counts reconcile and the status is derivable.
    category: policy
    applies_to: RefreshRun
    trigger: A run finishes.
    logic: IF the run finishes THEN requestedCount equals pricedCount plus unpricedCount; status is failed when no source produced any accepted observation during the run (or the run aborted), otherwise succeeded with the unpriced count recorded.
    violation: A run never reports counts that do not add up or an ambiguous status.
    source: FR10.1

  - id: BR5.1
    statement: Only registered internal callers reach Market Data.
    category: authorization
    applies_to: PriceQuote, FxRate
    trigger: An internal API call arrives.
    logic: IF the caller presents a registered service token whose allow-list includes the operation THEN allow; ELSE refuse with a problem document.
    violation: No data is returned to an unknown caller.
    source: NFR9, BRD-012 NFR-SEC-07

  - id: BR5.2
    statement: Source changes and failed refreshes are audited.
    category: policy
    applies_to: RefreshRun
    trigger: A source changes in C4, or a run fails.
    logic: IF the event occurs THEN write a C10 event with actor (or system), action, target, occurredAt, context (source name and run id only), and outcome.
    violation: A failed audit write is logged and surfaced, never silently dropped.
    source: NFR19

  - id: BR5.3
    statement: Observations and runs are retained under the retention policy.
    category: policy
    applies_to: PriceObservation, FxRate, RefreshRun
    trigger: The retention sweep runs.
    logic: IF a row is older than 90 days THEN delete it, except the newest observation per asset and source and the newest FX reading per pair, which are kept to serve stale prices and rates.
    violation: History is bounded; the latest observation always survives.
    source: NFR14
```

## Rules summary

| Rule | Category | Statement |
|---|---|---|
| BR1.1 | policy | Deterministic priority selection from C4 sources |
| BR1.2 | policy | Fallback on source failure, provenance recorded |
| BR1.3 | constraint | No guessed or zero price; stale only within the hard bound |
| BR1.4 | validation | Source registry read from C4, no second copy |
| BR1.5 | validation | Boundary validation of price and FX requests |
| BR2.1 | calculation | Freshness bound by asset tier |
| BR2.2 | constraint | Priced quote carries source and observedAt |
| BR2.3 | policy | Unpriced excluded and labelled |
| BR2.4 | policy | Next refresh prices a newly covered asset |
| BR3.1 | calculation | Fresh FX pair or unavailable |
| BR3.2 | calculation | USD cross rate from two fresh legs |
| BR3.3 | validation | Positive decimal rates |
| BR4.1 | constraint | One leased refresh run; manual 202 on a held lease |
| BR4.2 | policy | Transient vs permanent failure classification |
| BR4.3 | constraint | Per-source rate limits, previous observation kept |
| BR4.4 | policy | Rejected reading never replaces a price |
| BR4.5 | policy | Run counts reconcile; status derivable |
| BR5.1 | authorization | Service-token callers only |
| BR5.2 | policy | Source changes and failed runs audited |
| BR5.3 | policy | 90-day retention with latest kept |
