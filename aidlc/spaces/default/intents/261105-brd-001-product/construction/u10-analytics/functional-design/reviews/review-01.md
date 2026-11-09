## Review

**Verdict:** READY

**Reviewer:** aidlc-architecture-reviewer-agent

**Iteration:** 1

**Verdict Dictated:** no

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u10-analytics/functional-design/rules.md > BR3.1 | The engagement metric is "distinct daily active / weekly active", but "active" is never defined by the events that count (any analytics event, valuation_read only, or another set), so the metric cannot be implemented or tested consistently. | Define "active" by the event set used for the daily and weekly numerator and denominator. | New |
| R-02 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u10-analytics/functional-design/rules.md > BR2.3 and BR4.2 | The rules say the pseudonym is stable "in normal operation", but a suspected-compromise salt rotation changes the current-version userRef, so flag buckets and cohort and retention joins shift for the transition, and BR4.2's "the same user never flips variants" does not say what happens at that rotation. | State the compromise-rotation behaviour: whether flags accept a one-time re-bucketing or are pinned through the transition, and how cohort and retention joins handle the boundary between salt versions. | New |
| R-03 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u10-analytics/functional-design/functional-spec.md > Entity-relationship view | The ER shows `CONSENT_CHECK \|\|--o{ ANALYTICS_EVENT`, but BR1.2 stores one consent check per request and each request is one event, so the cardinality should be one check per gated event (or the diagram should explain the grouping). | Change the relationship to one-to-one (or explain why one check gates several events) so the model matches the per-request rule. | New |

### Summary

Both prior Majors are fixed: erasure takes the raw authUserId and Analytics hashes it under every retained salt version, and the salt now rotates only on suspected compromise with current and previous versions kept 12 months, so normal-operation pseudonyms, flag buckets and cohort joins are stable. The C12 X-Auth-Subject header and trust boundary, the erase 403, the analytics-class signup event, the single analytics consent purpose, the concrete secret pattern, the FR12.3 delivery and marketing-consent boundaries, and the components.md userRef and FeatureFlag.status are all in place, and the event catalogue, PII patterns, metric formulas, suppression, retention and erasure rules are now testable. Only three Minor clarifications remain (the engagement "active" definition, the compromise-rotation transition, and the consent-check ER cardinality), so the stage is READY.
