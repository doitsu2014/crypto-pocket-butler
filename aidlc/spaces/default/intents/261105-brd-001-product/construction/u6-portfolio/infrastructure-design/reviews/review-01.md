## Review

**Verdict:** READY

**Reviewer:** aidlc-architecture-reviewer-agent

**Iteration:** 2

**Verdict Dictated:** no

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u6-portfolio/infrastructure-design/infrastructure-specification.md > Service-token registry, with cicd-pipeline.md | The registry row says it is seeded and rotated by `infra/sql/portfolio-service-tokens.sql`, but that file does not exist in the repository, and the pipeline table has no seed step; as written the seeding cannot run and the registry stays empty. | Create the seed/rotation script (following the existing service-token scripts) and add the deploy-job seed step to the pipeline, or record the script as a code-generation deliverable. | New |
| R-02 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u6-portfolio/infrastructure-design/cicd-pipeline.md > Smoke | The smoke gate runs `/readyz` on the API and worker and `GET /v1/internal/smoke`, but does not say which service serves the smoke route; the worker's post-deploy verification is therefore still undefined. | Name the service(s) serving `/v1/internal/smoke`, or state that the worker is verified by `/readyz` alone. | New |
| R-03 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u6-portfolio/infrastructure-design/infrastructure-specification.md > Compute | The compute row says the worker "runs the hourly snapshot prune at two years", which conflates the hourly snapshot cadence with the two-year retention prune. | State the hourly snapshot run and the two-year prune as two separate behaviours. | New |
| R-04 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u6-portfolio/infrastructure-design/traceability.json > NFR20.6 | The requirement is mapped to "Supabase retention" (the database row, which states only backup and PITR), while the prune is now stated in the compute/worker row. | Retarget the traceability at the element that carries the prune. | New |

### Summary

The Major is fixed: the service-token registry now names Reporting, Identity, Job Orchestration (C11 valuation run) and the smoke reader, with the outbound token allow-listed at C5–C10, and the Minor corrections hold: the run and per-valuation duration metrics are split, the run-success SLI is hourly, the worker row states the 900 s lease with 60 s renewal and the two-year prune, the networks say C5–C10 are internal with only Postgres public, the scaling triggers separate the 300 ms snapshot and 800 ms fresh reads, and the gateway token is removed from the registry. The remaining items are Minor: the referenced token-seeding SQL file does not yet exist (and the pipeline has no seed step), the smoke route's service is unstated, the compute row conflates the hourly snapshot with the two-year prune, and NFR20.6's traceability points at the database row rather than the worker row. The deployment model, role separation, migrate/rollback/smoke path, CI/CD gates and monitoring/SLIs are implementable and coherent with the READY NFR design.
