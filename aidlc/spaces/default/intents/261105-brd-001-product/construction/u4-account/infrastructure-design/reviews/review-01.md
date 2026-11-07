## Review

**Verdict:** READY

**Reviewer:** aidlc-architecture-reviewer-agent

**Iteration:** 1

**Verdict Dictated:** no

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | monitoring-design.md > Metrics & KPIs, account_health | The row says the metric comes from `/readyz` and is the "Swarm health check and rolling-update gate", but the specification now uses `/healthz` for the Swarm health check and `/readyz` only for the release-job smoke. | Reword the metric (or split liveness and readiness) and point the "Readiness failing" alert at the probe the release job actually uses. | New |
| R-02 | Minor | infrastructure-specification.md > Infrastructure Services, Supabase Vault | `account.service_token_registry` is named with its shape but its population and rotation are not stated: the four caller tokens live in Vault, yet the registry must hold their hashes and allow-lists, and the text does not say whether a startup upsert, a migration seed, or a Docker secret supplies them. | State how the registry is seeded and how a token rotation reaches it without a redeploy. | New |
| R-03 | Minor | infrastructure-specification.md > Deployment, Import handoff; functional-design/entities.md > ImportBatch | `ImportBatch.status` (`pending`/`processing`/`complete`/`failed`) is used by the handoff but is absent from the functional entity model, whose `importedCount`/`skippedCount`/`failedCount` are required; the privacy anonymise path also does not mention deleting a stuck staging row. | Add the status attribute and pending-row semantics to the owning entity model and state that anonymise deletes a stuck staging row. | New |
| R-04 | Minor | traceability.json > coverage rows NFR1.2, NFR2.1, NFR2.2, NFR2.3, NFR4.1, NFR6.1, NFR9.2 | Several `OK` rows point at content that does not carry the requirement: NFR1.2 (lease) to Rollback; NFR2.1 and NFR2.2 (p95 latency) to Resource sizing; NFR2.3 (re-sync 202 with the run id) to Import handoff; NFR4.1 (sync ≤30 s) to the Supabase Postgres row; NFR6.1 (first sync) to the import handoff; NFR9.2 (periodic key re-verification) to a compute-model row that does not mention it. | Retarget each row at the row or section that actually carries the decision, or mark it N/A with the reason. | New |
| R-05 | Minor | infrastructure-specification.md > Deployment, Scaling mechanism; monitoring-design.md > Metrics & KPIs; nfr-design/scalability-design.md > Load distribution | The same pool signal is stated three ways: 80% of 12 for 10 min, 80% of 12 for 5 min, and 18/20 for 10 min. | Reconcile to one threshold across the three artifacts and correct or drop the upstream 18/20 figure. | New |

### Summary

The earlier blockers are fixed: both Account services reach the internal shared services and the external providers, the import handoff has one modelled staging store with a 10 MiB cap and a lifecycle, rollback has one order, the Swarm probe is liveness-only with a readiness smoke gate, the ports, secret names, overlay encryption and TLS DSN are consistent, and the service-token model is settled. The five remaining items above are documentation and operational precision, so with no Critical and no Major finding the design meets the readiness rule.
