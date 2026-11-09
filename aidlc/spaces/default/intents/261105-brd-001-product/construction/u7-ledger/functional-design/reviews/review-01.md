## Review

**Verdict:** READY

**Reviewer:** aidlc-architecture-reviewer-agent

**Iteration:** 1

**Verdict Dictated:** no

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u7-ledger/functional-design/entities.md > entity Transaction constraints, and functional-spec.md > WF1 step 2 | The model still refers to a removed `dedupeKey`: the Transaction constraint says "dedupeKey is unique per user" and WF1 step 2 says "Each row builds its dedupeKey", although the entity now defines `canonicalKey` and `externalKey`. | Replace both references with the canonicalKey and externalKey uniqueness rule so the model and workflow match the rule. | New |
| R-02 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u7-ledger/functional-design/functional-spec.md > WF3 step 1 and Rules summary | WF3 step 1 still says "A buy or a priced transfer_in creates a FIFO lot", which contradicts BR2.4's "a matched transfer_in never creates an independent priced lot". The rules summary also still describes cost basis as BR2.1 to BR2.3 and omits the new BR2.4 and BR2.5. | Scope WF3 step 1 to an unmatched or external transfer_in, and update the rules summary to include BR2.4 (matched transfer moves basis) and BR2.5 (fee attachment). | New |
| R-03 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u7-ledger/functional-design/functional-spec.md > State machines, TransferPair.status | BR6.1 defines an explicit user rejection that sets `rejected`, and the entity allows that status, but the state machine has no pending-to-rejected transition. | Add the pending-to-rejected transition (and the rejected terminal state) to the state machine. | New |
| R-04 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u7-ledger/functional-design/traceability.json > reverse | The reverse lists only BR6.1, but BR2.4 (matched transfer basis) and BR2.5 (fee attachment) also have no acceptance criterion, and BR2.4's source cites AC4.1.1, which is the one-entry duplicate AC rather than the transfer-basis rule. | Add BR2.4 and BR2.5 to the reverse as no-AC rules and correct BR2.4's source to FR5.1 (or FR5.2) only. | New |
| R-05 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u7-ledger/functional-design/entities.md > entity Transaction, canonicalKey | The canonicalKey compares quantity and price exactly, so the same trade from a venue and a CSV that rounds the price or quantity slightly will not dedupe, even though AC4.1.1 expects the exchange-plus-CSV duplicate to be caught. | State the decimal scale or tolerance used in the canonical key and add a test with a rounded CSV price. | New |
| R-06 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u7-ledger/functional-design/functional-spec.md > WF5 step 3 | WF5 step 3 says the correction stores "the old value copied", while BR4.1 requires oldValue to be the value in force after earlier corrections. | Align WF5 with BR4.1's effective-value definition. | New |

### Summary

Both prior Majors are fixed: dedupe now uses a canonical cross-source identity with an optional external key and a duplicate check on both, covering AC4.1.1, and a matched transfer now moves the remaining lot cost to the incoming lot with no realized P&L and no independent priced lot (BR2.4). The Minors are also in: per-asset P&L is transient in C7 with only the total stored, the recorded-trade and list/correction contract changes are recorded, fees have BR2.5, pairing uses quantity within 1% and nearest time with a rejected transition, corrections use the current effective oldValue with per-field validation and assetId reflow, net invested is the open-lot cost with total return separate, coverage is by value with uncovered assets named and excluded, and the ImportBatch supersession is stated. Only six small alignment items remain (stale dedupeKey wording, WF3 versus BR2.4, the missing rejected transition, the reverse traceability for BR2.4 and BR2.5, the canonical key's decimal tolerance, and the WF5 wording), so the stage is READY.
