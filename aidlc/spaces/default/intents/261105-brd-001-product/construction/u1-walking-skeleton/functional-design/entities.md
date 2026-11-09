# Entities — u1-walking-skeleton (Asset Registry)

Scoped by `units-generation/unit-of-work.md` and the ApiGateway and AssetRegistry components in
`domain-design/components.md`; the obligations these entities satisfy come from
`requirements-analysis/requirements.md` (FR3.1–FR3.5, FR11.3, FR13.1–FR13.2) and the stories
assigned to this Unit in `unit-of-work-story-map.md`.

```yaml
entities:
  - name: Asset
    description: One canonical, tradeable or holdable thing a portfolio can hold, independent of how many chains or contracts represent it.
    attributes:
      - name: id
        type: string
        required: true
        unique: true
        constraints: stable, opaque, never reused
      - name: symbol
        type: string
        required: true
        unique: false
        constraints: uppercase ticker as the venue reports it, not unique across assets
      - name: name
        type: string
        required: true
        constraints: display name
      - name: decimals
        type: integer
        required: true
        min: 0
        max: 36
        constraints: the canonical precision used for all quantities of this asset
      - name: canonical
        type: boolean
        required: true
        default: true
        constraints: false marks a placeholder created from an unresolved observation
    constraints:
      - symbol alone never identifies an asset
      - decimals is authoritative for every chain representation of this asset
    relationships:
      - target: TokenContract
        cardinality: one-to-many
        direction: Asset has many TokenContract
      - target: PriceSource
        cardinality: one-to-many
        direction: Asset is priced by many PriceSource

  - name: Chain
    description: A network whose addresses and contracts are observed at launch.
    attributes:
      - name: id
        type: string
        required: true
        unique: true
      - name: name
        type: string
        required: true
        unique: true
      - name: family
        type: string
        required: true
        allowed_values: [evm, bnb, solana]
      - name: launchCoverage
        type: boolean
        required: true
        default: true
        constraints: false excludes observations from portfolios
    constraints:
      - only the launch networks are accepted; everything else is out of coverage
    relationships:
      - target: TokenContract
        cardinality: one-to-many
        direction: Chain holds many TokenContract
      - target: UnmappedObservation
        cardinality: one-to-many
        direction: Chain produces many UnmappedObservation

  - name: TokenContract
    description: A contract address on one chain that represents a canonical asset.
    attributes:
      - name: id
        type: string
        required: true
        unique: true
      - name: chainId
        type: string
        required: true
        references: Chain.id
      - name: contractAddress
        type: string
        required: true
        unique: false
        constraints: opaque; case and checksum preserved; unique together with chainId
      - name: assetId
        type: string
        required: true
        references: Asset.id
      - name: decimals
        type: integer
        required: true
        min: 0
        max: 36
        constraints: must match the Asset decimals; a mismatch is a registry error
    constraints:
      - the pair (chainId, contractAddress) is unique
      - two contracts on different chains may point at one asset
      - one chain contract never points at two assets
    relationships:
      - target: Asset
        cardinality: many-to-one
        direction: TokenContract belongs to Asset
      - target: Chain
        cardinality: many-to-one
        direction: TokenContract lives on Chain

  - name: PriceSource
    description: A provider that can price assets, with a configured priority.
    attributes:
      - name: id
        type: string
        required: true
        unique: true
      - name: name
        type: string
        required: true
        unique: true
      - name: priority
        type: integer
        required: true
        min: 1
        constraints: lower value is tried first; ties are refused at save time
      - name: licenceRef
        type: string
        required: false
        constraints: names the licence that permits the use
      - name: enabled
        type: boolean
        required: true
        default: true
    constraints:
      - at most one enabled source holds a given priority
      - a source without a licence reference may be disabled but not enabled for production
    relationships:
      - target: Asset
        cardinality: many-to-many
        direction: PriceSource prices many Asset

  - name: UnmappedObservation
    description: A token observed in an account or chain that no canonical asset resolves.
    attributes:
      - name: id
        type: string
        required: true
        unique: true
      - name: observedSymbol
        type: string
        required: true
      - name: chainId
        type: string
        required: true
        references: Chain.id
      - name: contractAddress
        type: string
        required: false
      - name: firstSeenAt
        type: timestamp
        required: true
      - name: status
        type: string
        required: true
        default: quarantined
        allowed_values: [quarantined, resolved, ignored]
    constraints:
      - an observation is quarantined until an operator maps it or ignores it
      - a resolved observation links to the Asset it was mapped to
    relationships:
      - target: Chain
        cardinality: many-to-one
        direction: UnmappedObservation was seen on Chain
```

## Summary

The registry has five entities. **Asset** is the canonical identity a portfolio holds;
**TokenContract** is how one chain expresses it; **Chain** bounds what is observed; **PriceSource**
is who may price it; **UnmappedObservation** is the quarantine for anything the registry cannot
resolve. The key invariant is that an asset is identified by chain plus contract address (or by
symbol only for a native asset), so one asset observed on two chains is never counted twice.
