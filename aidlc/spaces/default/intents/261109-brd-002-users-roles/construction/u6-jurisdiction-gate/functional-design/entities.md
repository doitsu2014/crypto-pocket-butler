# Entities — u6-jurisdiction-gate

The gate owns two entities: the restricted-jurisdiction list and its versions. Decisions themselves are evidence records written to the Audit service, not entities of this unit.

**Upstream:** `unit-of-work.md` (U6 `u6-jurisdiction-gate`), `requirements.md` (FR19.3, FR19.4), `components.md` (JurisdictionGate).

```yaml
entities:
  - name: JurisdictionRule
    description: One country's availability status in the maintained list.
    attributes:
      - name: countryCode
        type: string
        required: true
        unique: true
        constraints: "ISO 3166-1 alpha-2, uppercase; primary identifier"
      - name: status
        type: enum
        required: true
        allowed: [allowed, restricted]
      - name: source
        type: string
        required: true
        constraints: "who maintains this entry, e.g. compliance"
      - name: updatedAt
        type: timestamp
        required: true
    entity_constraints:
      - "The list is the single source of truth for restrictions; no rule may hardcode a country."
    relationships:
      - to: JurisdictionListVersion
        cardinality: many-to-one
        direction: "each rule belongs to the current list version"
  - name: JurisdictionListVersion
    description: An immutable snapshot version of the jurisdiction list, so every decision cites the list it used.
    attributes:
      - name: version
        type: string
        required: true
        unique: true
        constraints: "monotonic; e.g. 2026-11-10.1"
      - name: updatedAt
        type: timestamp
        required: true
      - name: entryCount
        type: integer
        required: true
        min: 0
      - name: checksum
        type: string
        required: true
        constraints: "digest of the ordered entries; detects an incomplete load"
    entity_constraints:
      - "A version is immutable; a change creates a new version and never edits a published one."
    relationships:
      - to: JurisdictionRule
        cardinality: one-to-many
        direction: "a version contains many rules"
```

## Summary

| Entity | Identifier | Purpose |
|---|---|---|
| JurisdictionRule | countryCode | One country's allowed/restricted status in the current list |
| JurisdictionListVersion | version | Immutable snapshot so each decision can cite the list version and checksum it used |

The check decision itself is returned to the caller and written to the Audit service as evidence; it holds no personal data.
