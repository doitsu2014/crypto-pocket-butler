# Entity Model — u7-ledger

Inputs: U7 in `inception/units-generation/unit-of-work.md`, requirements FR5.1–FR5.4 and FR6.4,
the Ledger component in `inception/domain-design/components.md`, and C5/C7/C10.

```yaml
entities:
  - name: Transaction
    description: One append-only ledger entry for a movement of an asset.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: userId
        type: uuid
        required: true
      - name: accountId
        type: uuid
        required: false
        description: Absent for a user-entered recorded trade not tied to a connected account.
      - name: assetId
        type: string
        required: true
      - name: kind
        type: enum
        required: true
        allowed_values: [buy, sell, transfer_in, transfer_out, fee]
      - name: quantity
        type: decimal
        required: true
      - name: price
        type: decimal
        required: false
        description: Unit price; absent for a transfer until paired.
      - name: priceSource
        type: enum
        required: false
        allowed_values: [venue, csv, user]
      - name: source
        type: enum
        required: true
        allowed_values: [venue, csv, recorded_trade]
      - name: externalRef
        type: string
        required: false
      - name: canonicalKey
        type: string
        required: true
        unique: true
        description: (userId, assetId, kind, quantity, price, executedAt rounded to the minute); the cross-source identity.
      - name: externalKey
        type: string
        required: false
        unique: true
        description: (userId, source, externalRef) when present; a second uniqueness check.
      - name: executedAt
        type: timestamp
        required: true
      - name: recordedAt
        type: timestamp
        required: true
    constraints:
      - A Transaction row is never updated or deleted; corrections are separate rows.
      - dedupeKey is unique per user.
      - quantity and price are decimals; no binary floating point.

  - name: Correction
    description: An append-only correction applied to an existing transaction.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: transactionId
        type: uuid
        required: true
        references:
          entity: Transaction
          relationship: many corrections may apply over time
      - name: field
        type: enum
        required: true
        allowed_values: [quantity, price, kind, executedAt, assetId]
      - name: oldValue
        type: string
        required: true
      - name: newValue
        type: string
        required: true
      - name: reason
        type: string
        required: true
        description: Free text from the user; never a credential.
      - name: actorId
        type: uuid
        required: true
      - name: correctedAt
        type: timestamp
        required: true
    constraints:
      - oldValue is the value in force after earlier corrections; the transaction row is never mutated.
      - A correction is never edited or deleted.

  - name: TransferPair
    description: Links a transfer_out to its matching transfer_in.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: outTransactionId
        type: uuid
        required: true
      - name: inTransactionId
        type: uuid
        required: false
        description: Absent until matched.
      - name: status
        type: enum
        required: true
        allowed_values: [pending, matched, rejected]
      - name: matchedAt
        type: timestamp
        required: false
    constraints:
      - A matched pair links two transactions of the same asset with opposite transfer kinds.
      - A transfer never becomes a taxable disposal when matched.

  - name: CostBasisLot
    description: One acquisition lot used by FIFO cost basis.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: userId
        type: uuid
        required: true
      - name: assetId
        type: string
        required: true
      - name: acquiredAt
        type: timestamp
        required: true
      - name: quantity
        type: decimal
        required: true
      - name: remaining
        type: decimal
        required: true
      - name: unitCost
        type: decimal
        required: true
        description: Cost per unit in the user's base currency at acquisition.
      - name: sourceTransactionId
        type: uuid
        required: true

  - name: PnlSnapshot
    description: A computed P&L figure with its coverage.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: userId
        type: uuid
        required: true
      - name: portfolioId
        type: uuid
        required: false
      - name: asOf
        type: timestamp
        required: true
      - name: netInvested
        type: decimal
        required: false
      - name: realized
        type: decimal
        required: false
      - name: unrealized
        type: decimal
        required: false
      - name: coverage
        type: decimal
        required: true
        description: 0–1 share of holdings with acquisition history.
      - name: coverageGap
        type: text
        required: false
        description: Which assets (by value) lack history and are excluded from the figures.
    constraints:
      - When coverage is below 1, netInvested/realized/unrealized are labelled partial, never presented as complete.
      - When coverage is 0, the figures are absent and P&L is unavailable.
```

## Summary

Ledger owns the append-only transactions, corrections, transfer pairs, FIFO cost-basis lots, and
computed P&L snapshots. Transactions arrive from Account's C5 (venue and imported rows) or a
recorded trade, dedupe on a deterministic key, and are never mutated: a correction is a new row.
P&L always carries a coverage label so a partial history is never shown as a complete answer.
