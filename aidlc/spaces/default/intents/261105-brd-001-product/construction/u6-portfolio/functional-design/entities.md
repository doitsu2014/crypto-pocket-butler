# Entity Model — u6-portfolio

Inputs: U6 in `inception/units-generation/unit-of-work.md`,
`inception/requirements-analysis/requirements.md` (FR4, FR6), the Portfolio component in
`inception/domain-design/components.md`, and C5/C6/C7/C8/C9/C10.

```yaml
entities:
  - name: Portfolio
    description: One named portfolio owned by a user, with its base currency.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: userId
        type: uuid
        required: true
      - name: name
        type: string
        required: true
        min_length: 1
      - name: baseCurrency
        type: string
        required: true
        description: ISO 4217 code, defaulting to the user's preference.
      - name: createdAt
        type: timestamp
        required: true
      - name: updatedAt
        type: timestamp
        required: true
    constraints:
      - name is unique per user (case-insensitive).
      - A user always has at least one portfolio; the first is created on demand.

  - name: PortfolioAccount
    description: An Account assigned to exactly one portfolio.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: portfolioId
        type: uuid
        required: true
      - name: accountId
        type: uuid
        required: true
        unique: true
        description: An account belongs to at most one portfolio.
      - name: addedAt
        type: timestamp
        required: true
    constraints:
      - An account can be in one portfolio only; moving it updates this row.

  - name: TargetAllocation
    description: The target percentage for one asset in one portfolio.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: portfolioId
        type: uuid
        required: true
      - name: assetId
        type: string
        required: true
      - name: targetPercent
        type: decimal
        required: true
        description: 0–100, two decimals.
      - name: updatedAt
        type: timestamp
        required: true
    constraints:
      - (portfolioId, assetId) is unique.
      - The saved set may not sum to 100; the difference is shown before saving.

  - name: Guardrail
    description: An optional per-asset drift threshold in percentage points.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: portfolioId
        type: uuid
        required: true
      - name: assetId
        type: string
        required: true
      - name: thresholdPoints
        type: decimal
        required: true
        description: Greater than 0; |drift| beyond it triggers the guardrail.
      - name: isOutside
        type: boolean
        required: true
        default: false
        description: The stored crossing state used for one-alert-per-crossing dedupe.
      - name: lastCrossingKey
        type: string
        required: false
        description: The C9 idempotency key of the active crossing.
      - name: crossedAt
        type: timestamp
        required: false
      - name: updatedAt
        type: timestamp
        required: true
    constraints:
      - (portfolioId, assetId) is unique.
      - A guardrail without a target is refused.

  - name: ValuationSnapshot
    description: An immutable record of a portfolio's valuation.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: portfolioId
        type: uuid
        required: true
      - name: asOf
        type: timestamp
        required: true
      - name: totalValue
        type: decimal
        required: true
      - name: currency
        type: string
        required: true
      - name: inputsDigest
        type: string
        required: true
        description: SHA-256 over the stored holdings, prices, FX, and targets used.
      - name: inputs
        type: map
        required: true
        description: The frozen input set (holding quantities, quotes with source/time, FX, targets).
      - name: lines
        type: map
        required: true
        description: Per-asset value, allocation, target, drift, and data status at asOf.
    constraints:
      - A snapshot row is never updated or deleted; a recompute creates a new snapshot.
      - Reproducing from stored inputs yields the same lines and total exactly.

  - name: OrderIntent
    description: A user's buy or sell intention with a projected allocation; never an external order.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: portfolioId
        type: uuid
        required: true
      - name: assetId
        type: string
        required: true
      - name: side
        type: enum
        required: true
        allowed_values: [buy, sell]
      - name: quantity
        type: decimal
        required: true
      - name: projectedAllocation
        type: decimal
        required: true
      - name: projectedDriftPoints
        type: decimal
        required: false
      - name: shortfall
        type: decimal
        required: false
        description: For a sell beyond holdings, the amount by which it exceeds them.
      - name: status
        type: enum
        required: true
        allowed_values: [pending, cancelled]
        default: pending
      - name: createdAt
        type: timestamp
        required: true
      - name: cancelledAt
        type: timestamp
        required: false
    constraints:
      - No external order call exists; every intent is advisory.
      - A cancelled intent stays visible with its cancelledAt.

  - name: TradeOverlay
    description: A recorded trade applied to the current holdings until a venue sync reflects it.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: recordedTradeId
        type: uuid
        required: true
        unique: true
      - name: portfolioId
        type: uuid
        required: true
      - name: accountId
        type: uuid
        required: false
        description: The account the overlay applies to; absent when the trade is portfolio-only.
      - name: assetId
        type: string
        required: true
      - name: quantityDelta
        type: decimal
        required: true
      - name: appliedAt
        type: timestamp
        required: true
    constraints:
      - The valuation adds each overlay delta to the C5 holding for the asset.
      - When a C5 sync reports a trade whose canonical key matches a recorded trade, the overlay row is removed so the trade is not double counted.

  - name: RecordedTrade
    description: A user-confirmed executed trade that becomes a ledger entry.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: portfolioId
        type: uuid
        required: true
      - name: assetId
        type: string
        required: true
      - name: side
        type: enum
        required: true
        allowed_values: [buy, sell]
      - name: quantity
        type: decimal
        required: true
      - name: price
        type: decimal
        required: true
        description: User-supplied when the venue price is missing; labelled user-entered.
      - name: priceSource
        type: enum
        required: true
        allowed_values: [venue, user]
      - name: executedAt
        type: timestamp
        required: true
      - name: createdAt
        type: timestamp
        required: true
    constraints:
      - Recording a trade writes the Ledger entry (C7) and a C10 audit event.
```

## Summary

Portfolio owns portfolios and their account membership, targets and guardrails, the computed
valuation with provenance, immutable snapshots with their frozen inputs, advisory order intents, and
recorded trades. It consumes Account (C5), Market Data (C6), Ledger (C7), Notification (C9), and
Audit (C10). All business math (value, allocation, drift) happens here, never in the SPA.
