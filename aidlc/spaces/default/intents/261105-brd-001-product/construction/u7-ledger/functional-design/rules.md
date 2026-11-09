# Business Rules — u7-ledger

```yaml
rules:
  - id: BR1.1
    statement: The ledger is append-only.
    category: constraint
    applies_to: Transaction
    trigger: Any change to a recorded transaction.
    logic: IF a change is needed THEN record a Correction row; never update or delete the Transaction.
    violation: The original row and its history remain recoverable.
    source: FR5.1, AC4.1.3

  - id: BR1.2
    statement: A movement is recorded once across sources, by canonical identity.
    category: policy
    applies_to: Transaction
    trigger: A transaction arrives from a venue, a CSV, or a recorded trade.
    logic: IF a Transaction with the same canonicalKey = (userId, assetId, kind, quantity, price, executedAt rounded to the minute) exists THEN skip it and report the duplicate, regardless of source or externalRef; IF only the same (userId, source, externalRef) exists THEN skip it too; ELSE store it with both keys.
    violation: The same trade from an exchange and a CSV never appears twice.
    source: FR5.1, AC4.1.1, AC1.4.2

  - id: BR1.3
    statement: Every transaction carries source, asset, quantity, price, and time.
    category: constraint
    applies_to: Transaction
    trigger: A transaction is read.
    logic: IF price is absent THEN the entry is shown as price-missing, never as zero; source, asset, quantity, and executedAt are always present.
    violation: A missing price is never rendered as 0.
    source: FR5.1, AC4.1.2, design-system

  - id: BR1.4
    statement: Imported rows are accepted only with their line-level errors from Account.
    category: policy
    applies_to: Transaction
    trigger: Account's C5 returns imported rows.
    logic: IF a row carries a line error THEN it is reported and not stored as a transaction; ELSE it enters dedupe and storage.
    violation: A malformed row never becomes a transaction.
    source: FR2.3, AC1.4.3

  - id: BR2.1
    statement: Cost basis is FIFO per asset.
    category: calculation
    applies_to: CostBasisLot
    trigger: A buy or a priced transfer_in is recorded, or a sell settles.
    logic: IF an acquisition with a price arrives THEN create a lot; IF a sell arrives THEN consume the oldest lots first and compute realized P&L = proceeds − (consumed quantity × lot unitCost).
    violation: Realized and unrealized reconcile to the remaining lots.
    source: FR5.2, AC4.2.1

  - id: BR2.2
    statement: Net invested is the cost basis of the lots still held.
    category: calculation
    applies_to: PnlSnapshot
    trigger: P&L is requested.
    logic: IF history is complete THEN netInvested = sum(remaining quantity × unitCost) of open lots; total return = realized + unrealized; realized = proceeds − consumed cost; unrealized = current value − remaining cost. Total acquisition cost (consumed + remaining cost) is reported separately and never called net invested.
    violation: The figures reconcile to the ledger, and net invested is capital still at cost.
    source: FR5.2, AC4.2.2

  - id: BR2.3
    statement: A correction reflows cost basis and P&L.
    category: calculation
    applies_to: Correction
    trigger: A correction is recorded.
    logic: IF the corrected field is quantity, price, kind, executedAt, or assetId THEN rebuild the affected lots (including moving a lot between assets) and recompute P&L and coverage.
    violation: Corrected figures reconcile to the corrected ledger.
    source: FR5.4, AC4.2.3

  - id: BR2.4
    statement: A matched transfer moves cost basis, not ownership at a gain.
    category: calculation
    applies_to: TransferPair
    trigger: A transfer_out and its matching transfer_in are paired.
    logic: IF a pair is matched THEN consume the outgoing lots' remaining quantity and cost without realized P&L and create the incoming lot(s) with that same total cost; a matched transfer_in never creates an independent priced lot.
    violation: The asset's basis is never counted in both places and no transfer creates a taxable gain.
    source: FR5.1, AC4.1.1

  - id: BR2.5
    statement: A fee attaches to its trade; a standalone fee is its own cost.
    category: calculation
    applies_to: Transaction
    trigger: A fee is recorded.
    logic: IF the fee has the same asset and an executedAt within the same minute as a buy or sell THEN add it to the buy's cost or subtract it from the sell's proceeds; ELSE record it as a standalone cost and include it in total cost, not as a disposal.
    violation: Fees reconcile and never create a fake disposal.
    source: FR5.2, AC4.2.1

  - id: BR3.1
    statement: P&L always carries a coverage label.
    category: policy
    applies_to: PnlSnapshot
    trigger: P&L is read.
    logic: IF every held asset by current value has acquisition history THEN coverage = 1 and the label is complete; IF some do not THEN coverage = covered value / total held value, the response states the covered portion and names the uncovered assets in the gap, and the returned figures exclude the uncovered holdings; IF none do THEN P&L is unavailable with coverage 0.
    violation: A partial history is never shown as a complete P&L.
    source: FR5.3, AC4.3.1–4.3.2

  - id: BR3.2
    statement: New history clears the coverage gap.
    category: policy
    applies_to: PnlSnapshot
    trigger: An import or sync completes missing history and P&L recalculates.
    logic: IF the previously missing assets now have lots THEN coverage rises and the label clears when it reaches 1.
    violation: The label is never stale after recalculation.
    source: AC4.3.3

  - id: BR4.1
    statement: A correction requires a reason, is validated per field, and is audited.
    category: policy
    applies_to: Correction
    trigger: A correction is submitted.
    logic: IF field and new value are valid for that field (assetId exists; quantity > 0; price >= 0; kind in the enum; executedAt not future) AND a reason is present THEN store the Correction with oldValue = the value in force after any earlier corrections, and write a C10 event; ELSE refuse with a field-level reason.
    violation: An invalid correction is never stored; the correction commits before its audit event, which is retried and a persistent failure is surfaced (the committed correction stays), and oldValue is the value actually replaced.
    source: FR5.4, AC4.5.1–4.5.3

  - id: BR5.1
    statement: A recorded trade creates a ledger entry and an audit record.
    category: policy
    applies_to: Transaction
    trigger: Portfolio submits an executed trade.
    logic: IF the trade has asset, kind, quantity, price, and time THEN store a Transaction with source recorded_trade and write a C10 event; ELSE refuse.
    violation: A recorded trade is visible in the ledger and audited.
    source: FR6.4, AC5.4.2

  - id: BR5.2
    statement: A missing price must be supplied by the user and labelled.
    category: validation
    applies_to: Transaction
    trigger: A recorded trade omits a price.
    logic: IF price is missing THEN require the user to supply it and record priceSource = user; ELSE use it.
    violation: A user-entered price is labelled and never confused with a venue price.
    source: AC5.4.3

  - id: BR6.1
    statement: A transfer pair is matched by asset, opposite kinds, quantity, and time.
    category: policy
    applies_to: TransferPair
    trigger: A transfer_out or transfer_in is ingested.
    logic: IF an unmatched counter-entry of the same asset with quantity within 1% and executedAt within 24 hours exists THEN match the nearest in time; IF several qualify THEN match the nearest and leave the rest pending; IF an explicit user rejection is recorded THEN status = rejected and the entry stays unpaired.
    violation: A pending or rejected transfer never creates a taxable disposal.
    source: FR5.1
```

## Rules summary

| Rule | Category | Statement |
|---|---|---|
| BR1.1 | constraint | Append-only, corrections as rows |
| BR1.2 | policy | Canonical cross-source dedupe |
| BR1.3 | constraint | Missing price never zero |
| BR1.4 | policy | Imported line errors reported, not stored |
| BR2.1 | calculation | FIFO lots |
| BR2.2 | calculation | Net invested = open-lot cost; total return separate |
| BR2.3 | calculation | Corrections reflow basis, including assetId |
| BR2.4 | calculation | Matched transfer moves basis, no gain |
| BR2.5 | calculation | Fee attachment |
| BR3.1 | policy | Coverage by value, gap names assets |
| BR3.2 | policy | New history clears the gap |
| BR4.1 | policy | Per-field validation, current oldValue, audit |
| BR5.1 | policy | Recorded trade + audit |
| BR5.2 | validation | User price labelled |
| BR6.1 | policy | Pairing by asset, opposite kinds, quantity, time |
