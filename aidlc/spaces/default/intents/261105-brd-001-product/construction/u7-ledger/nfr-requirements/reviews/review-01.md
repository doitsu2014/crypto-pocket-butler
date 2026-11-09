## Review

**Verdict:** READY

**Reviewer:** aidlc-architecture-reviewer-agent

**Iteration:** 1

**Verdict Dictated:** no

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u7-ledger/functional-design/rules.md > BR4.1, and nfr-requirements/reliability-requirements.md > NFR1.4 | The functional rule's violation still says "No correction without an audit record", while NFR1.4 now commits the correction first and surfaces a persistent audit failure, explicitly allowing a committed correction to survive without its audit (visibly and paged). The two artifacts state opposite guarantees for the same failure mode. | Align BR4.1 (and its violation) with the commit-first, bounded-retry, surfaced-failure policy, or record the requirement change in the functional design. | New |
| R-02 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u7-ledger/nfr-requirements/security-requirements.md > Notes | NFR9.4 says the audit carries a reason hash, never the free text, but the Notes still say "A correction reason is stored on the correction row and in the audit event". The note reintroduces the free text the row excludes. | Update the Notes to say the free-text reason stays on the correction row and only its hash enters the C10 event. | New |
| R-03 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u7-ledger/nfr-requirements/observability-requirements.md > NFR17.7 | The ticket condition "coverage below 90% in the 100% bucket for a cohort" is ambiguous: a histogram bucket cannot itself hold a coverage percentage below 90%. | State the intended signal, for example the share of a cohort whose coverage is at 100% falls below 90%, or name the bucket share explicitly. | New |

### Summary

Both prior Majors are fixed: NFR1.4 now commits the correction, writes C10 with bounded retry, and surfaces a persistent audit failure instead of pretending to cross-service atomicity; and NFR1.7 refuses only a user-recorded trade beyond known holdings while ingested venue/CSV sells are always stored with any shortfall expressed as a coverage gap. The Minors are in as well: readiness checks Postgres only with C5 as an ingest signal, the audit carries a reason hash, the 5xx page is 0.05%, the coverage metric uses fixed buckets, NFR10 maps to NFR9.2/NFR9.3, NFR13 maps to the new NFR13.1 export/anonymise requirement and NFR20.6, NFR15/NFR16 have correct N/A reasons, retention is two years then anonymised, and ingested timestamps tolerate a 300-second skew with clamping. Only three small wording alignments remain (BR4.1 versus NFR1.4, the security Notes' reason text, and the coverage ticket wording), so the stage is READY.
