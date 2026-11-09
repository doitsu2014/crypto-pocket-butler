## Review

**Verdict:** READY

**Reviewer:** aidlc-architecture-reviewer-agent

**Iteration:** 2

**Verdict Dictated:** no

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u6-portfolio/nfr-design/observability-design.md > metric table | `portfolio_guardrail_alerts_failed_total` is described in the alerts text but is absent from the metrics table, and no alert or ticket is set on repeated failures. | Add the counter to the table, and consider a page when failed guardrail alerts recur. | New |
| R-02 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u6-portfolio/nfr-design/performance-design.md > Preview | The preview projects over the latest snapshot's lines, but the design does not say what a preview returns when the portfolio has no snapshot yet (a new portfolio that has not been valued). | State the no-snapshot preview behaviour (for example an empty projection with a "valuation not available yet" label). | New |
| R-03 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u6-portfolio/nfr-design/performance-design.md > Indexes | The performance design's index list still shows only `(account_id)` membership, while the scalability design adds `(portfolio_id)` membership and `(portfolio_id, asset_id)` overlays; the two files disagree. | Sync the index list with the scalability design. | New |
| R-04 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u6-portfolio/nfr-design/performance-design.md > Valuation run | The performance design says "4 in flight", while the scalability design says four batches in flight with four portfolios in flight per batch. The run budget's concurrency assumption is stated twice with different meanings. | Use one wording for the run concurrency in both files. | New |
| R-05 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u6-portfolio/nfr-design/scalability-design.md > Snapshots | The per-snapshot size (≈1–2 KB, bounded at 64 KB) and the row count are stated but the resulting total storage (roughly 0.35–0.7 TB) and the platform limit are not. | State the resulting total storage and confirm it fits the Supabase limits, or record that the infrastructure stage owns it. | New |
| R-06 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u6-portfolio/nfr-design/security-design.md > Audit, with functional-design/rules.md | The design now names the four C10 events and their actions, but the READY functional design's rules still define an audit only for the recorded trade; the target, guardrail and intent-cancellation audits exist only at the NFR level. | Add the audit rules to the functional design (or record the NFR-level addition and its origin) so the rule set and the design agree. | New |
| R-07 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u6-portfolio/nfr-design/traceability.json > NFR1.1 | NFR1.1's availability requirement is mapped to "Alerts", while the design now defines the read availability SLI and the degraded-as-good rule in the SLIs section. | Point the traceability at the SLI definition. | New |

### Summary

The two Majors are fixed and verified: the run lease now equals the 15-minute run budget, is renewed every 60 s, stops the run on a failed renewal and sweeps an expired lease to failed, and Identity is added to the C8 privacy caller set alongside Reporting. The Minor fixes also hold: the preview projects over the latest snapshot's labelled lines without a fresh compute, the TradeOverlay is applied on the read and snapshot paths, `(portfolio_id, hour_bucket)` uniqueness makes a retried run idempotent, the digest is verified on write, reproduction and nightly, the availability SLI counts degraded labelled reads as good, a failed C9 alert keeps the crossing state inside with a failure counter, the membership and overlay indexes are present, the run concurrency is stated, the snapshot estimate is 1–2 KB bounded at 64 KB, and the C10 action names are defined. The remaining items are Minor consistency and completeness fixes — the guardrail-alert failure metric in the table, the preview's no-snapshot case, the two files' index and concurrency wording, the total snapshot storage, the NFR-level audit rules not yet in the functional design, and the NFR1.1 traceability target.
