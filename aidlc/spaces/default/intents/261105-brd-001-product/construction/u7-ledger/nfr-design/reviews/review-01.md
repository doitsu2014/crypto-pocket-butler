## Review

**Verdict:** READY

**Reviewer:** aidlc-architecture-reviewer-agent

**Iteration:** 1

**Verdict Dictated:** no

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u7-ledger/nfr-design/performance-design.md > Budgets table, and scalability-design.md > Reconcile | The reconcile batching is inconsistent: the performance bullet now batches by transaction volume (at most 5,000 transactions), but the performance budget table still says "500-user batches" and the scalability design still says "500-user batches, one leased run". | Update the budget table element and the scalability design to the transaction-volume batching so all three statements match. | New |
| R-02 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u7-ledger/nfr-design/traceability.json > coverage row NFR17.8 | NFR17.8 (dashboards) points at "observability-design.md SLIs", but the dashboards now live in the design's Dashboards section and the SLIs do not cover them. | Retarget NFR17.8 to the observability design's Dashboards section. | New |
| R-03 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u7-ledger/functional-design/entities.md > entity Transaction | The NFR design introduces the mutable `LedgerSubject` indirection (transactions reference `subjectId` rather than the user id) to make anonymisation compatible with append-only rows, but the READY functional entity model still stores `userId` directly on the transaction. | Record the subject indirection in the functional entity model (or add a note that the NFR design refines it), so the implementation does not store the user id on the immutable rows. | New |

### Summary

The Major is fixed: transactions now reference a mutable `LedgerSubject`, anonymise clears its user id and sets a random anonymised reference while the append-only rows stay untouched, and a separate `ledger_anonymiser` role with UPDATE only on subjects runs the audited, idempotent one-shot job. The Minors are also in: the mutable-table grants are stated, BR4.1 now says the correction commits before its retried audit, the reconcile bullet batches by transaction volume, the dashboards exist, the logs drop amounts, the snapshot freshness rule requires `computed_at` after the last ledger change, and NFR1.1 targets the availability SLI. Only three consistency leftovers remain (the stale 500-user batching in the budget table and scalability design, the NFR17.8 traceability target, and the functional entity's direct user id), so the stage is READY.
