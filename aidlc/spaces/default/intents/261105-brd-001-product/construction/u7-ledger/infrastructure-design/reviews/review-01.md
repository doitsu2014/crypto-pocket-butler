## Review

**Verdict:** READY

**Reviewer:** aidlc-architecture-reviewer-agent

**Iteration:** 1

**Verdict Dictated:** no

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u7-ledger/infrastructure-design/infrastructure-specification.md > Supabase Postgres row, and traceability.json > rows NFR2.6, NFR1.7 and NFR1.4 | The traceability targets point at content the Supabase row does not contain: NFR2.6 says "(lot indexes)" and NFR1.7 says "integrity" but the row lists only tables and partitioning, with no indexes or constraints; NFR1.4 points at the monitoring audit-failure alert only, not at the commit-first ordering described in the reliability design. | List the FIFO/list indexes and the integrity constraints in the Supabase row (or retarget NFR2.6 and NFR1.7 to the nfr-design), and add the reliability design as a second NFR1.4 target. | New |
| R-02 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u7-ledger/infrastructure-design/cicd-pipeline.md > Deployment strategy | The line says the `ledger_anonymiser` role is used "only by the erasure job (C7 anonymise), never by the running service", but the infrastructure spec says the API opens a short-lived anonymiser connection inside the C7 handler; the running service does use that credential for one operation. | Reword the line to "the API uses the anonymiser DSN only inside the C7 anonymise handler; the runtime pool uses ledger_svc" so the two artifacts agree. | New |
| R-03 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u7-ledger/infrastructure-design/infrastructure-specification.md > Deployment, Storage row | The Storage row lists only `ledger_svc` and `ledger_anonymiser` as the roles, although the design now provisions a third `ledger_migrator` role for the migrate job. | Add `ledger_migrator` (migration job only) to the Storage row so the role inventory matches the Infrastructure Services table. | New |

### Summary

All three Majors are fixed: `ledger_svc` no longer has UPDATE on `ledger_subjects`, so the `ledger_anonymiser` role is the only updater, and the C7 anonymise handler opens a short-lived connection with that DSN for the one operation while the runtime pool stays on `ledger_svc`; a `ledger_migrator` role plus `ledger_migrate_database_url` and `ledger_anonymiser_database_url` are provisioned and the migration check asserts the three roles and their grants; and the traceability now carries all 35 detailed IDs with the previously missing infrastructure requirements. The Minors are in too: the latency alert and scaling runbook, the token registry seeding including `smoke-reader`, and the first-miss reconcile page. Only three consistency leftovers remain (the Supabase row versus the NFR2.6/NFR1.7/NFR1.4 targets, the CI wording on the anonymiser role, and the missing migrator in the Storage row), so the stage is READY.
