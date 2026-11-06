# Business Rules — u1-walking-skeleton (Asset Registry and gateway)

```yaml
rules:
  - id: BR1.1
    statement: An observed token resolves to exactly one canonical asset, identified by chain plus contract address, or by symbol only when the asset is native to the chain.
    category: validation
    applies_to: Asset, TokenContract, Chain
    trigger: An account holding or a chain observation needs its asset identity
    logic: >
      IF the observation has a chain and a contract address THEN look up TokenContract by
      (chainId, contractAddress). IF one is found THEN return its Asset. IF the observation is a
      native asset THEN resolve by chain plus symbol. IF nothing matches THEN quarantine it
      (BR1.3). A contract never resolves to two assets, and an asset observed on two chains is
      returned as one asset.
    violation_behaviour: The observation is quarantined; the holding is excluded from valuation and labelled unpriced.
    source: FR3.1, US9.2

  - id: BR1.2
    statement: A contract's decimals must match its canonical asset's decimals.
    category: validation
    applies_to: TokenContract, Asset
    trigger: A registry entry is created or changed
    logic: >
      IF the contract decimals differ from the asset decimals THEN reject the registry change and
      report both values. IF they match THEN accept the change.
    violation_behaviour: The change is refused; the previous registry entry stays in force.
    source: FR3.1, FR3.5

  - id: BR1.3
    statement: An unresolved observation is quarantined, excluded from portfolios, and surfaced to the operator.
    category: constraint
    applies_to: UnmappedObservation
    trigger: No canonical asset resolves for an observation
    logic: >
      IF resolution fails THEN create an UnmappedObservation with status quarantined, exclude the
      holding from every portfolio total, and show it to the operator for mapping or ignoring.
    violation_behaviour: The holding is never guessed or approximated.
    source: FR3.5, US1.2, US9.2

  - id: BR2.1
    statement: A price comes from the highest-priority enabled source that answers, falling back to the next source on failure.
    category: policy
    applies_to: PriceSource, Asset
    trigger: A price is requested for an asset
    logic: >
      IF the highest-priority enabled source returns a usable price THEN use it and record its
      provenance and time. IF it fails or returns nothing THEN try the next enabled source in
      priority order. IF every source fails THEN return the unpriced result (BR2.2).
    violation_behaviour: The asset is marked unpriced with the attempted sources recorded.
    source: FR3.2, FR3.4

  - id: BR2.2
    statement: An asset with no usable price is reported as unpriced, never guessed and never zero.
    category: policy
    applies_to: Asset, PriceSource
    trigger: Price resolution yields no usable price
    logic: >
      IF no source returns a usable price THEN return status unpriced with the reason. The
      portfolio excludes the asset from the priced total and discloses the exclusion.
    violation_behaviour: The asset is shown as an em dash, never as 0 or a fabricated price.
    source: FR3.5

  - id: BR3.1
    statement: Every registry administration change is audited with actor, target, time, and outcome.
    category: policy
    applies_to: Asset, Chain, TokenContract, PriceSource, UnmappedObservation
    trigger: An operator creates, changes, disables, or resolves a registry entry
    logic: >
      IF an operator submits a registry change THEN validate it (BR1.2), apply it, and emit an
      audit event naming the actor, the target, the time, and the outcome.
    violation_behaviour: A change that cannot be audited is refused.
    source: FR11.3, US9.2

  - id: BR4.1
    statement: The gateway validates the Supabase JWT against JWKS before any routing.
    category: authorization
    applies_to: Gateway
    trigger: Any inbound application request
    logic: >
      IF the token is missing, malformed, expired, or fails signature, issuer, or audience checks
      THEN reject with 401 and audit the denial. IF it is valid THEN resolve the principal and
      continue.
    violation_behaviour: The request is rejected; no service is called.
    source: FR13.1, NFR10

  - id: BR4.2
    statement: Routing is deny by default; only a declared route reaches a service.
    category: authorization
    applies_to: Gateway
    trigger: A validated request is routed
    logic: >
      IF the requested route is declared in the route table THEN forward it to the owning service
      with the JWT unchanged. IF no route matches THEN reject with 404 and audit the denial.
    violation_behaviour: The request is rejected; no service is called.
    source: FR13.2, NFR11

  - id: BR4.3
    statement: Every request carries a correlation ID from the gateway to the owning service and back.
    category: constraint
    applies_to: Gateway
    trigger: A request enters the gateway
    logic: >
      IF the caller supplies a correlation ID THEN honour it. IF not THEN generate one. Propagate
      it on the forwarded request and include it in any error problem response.
    violation_behaviour: A request that cannot carry a correlation ID is refused at the gateway.
    source: FR13.2, NFR17
```

## Rules summary

| Rule | Category | Summary | Source |
|---|---|---|---|
| BR1.1 | validation | One asset by chain plus contract, symbol only for native | FR3.1 |
| BR1.2 | validation | Contract decimals must match the asset | FR3.1 |
| BR1.3 | constraint | Unresolved observations are quarantined and excluded | FR3.5 |
| BR2.1 | policy | Highest-priority source wins, with fallback | FR3.2 |
| BR2.2 | policy | No price means unpriced, never zero or guessed | FR3.5 |
| BR3.1 | policy | Registry changes are audited | FR11.3 |
| BR4.1 | authorization | JWT validated against JWKS before routing | FR13.1 |
| BR4.2 | authorization | Deny-by-default routing | FR13.2 |
| BR4.3 | constraint | Correlation ID end to end | FR13.2 |
