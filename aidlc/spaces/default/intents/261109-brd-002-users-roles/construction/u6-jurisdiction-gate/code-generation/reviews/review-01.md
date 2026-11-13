## Review

**Verdict:** NOT-READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-11-10T06:14:21Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Critical | services/gateway/http/src/middleware.rs > client_ip | The trusted client address is taken from the observed peer before the edge header. The gateway is published with `mode: ingress` in `infra/swarm-stack.yml` and the documented edge is a CDN/WAF, so the observed peer is the ingress or CDN address, never the end user's; `services/gateway/http/src/server.rs > serve` always attaches `ConnectInfo`, so the `x-real-ip` branch is never reached in production. Every jurisdiction check then receives an edge or private address (fail-closed 503, or an unrelated country) and the rate limiter keys every user to one bucket, so the BR1.3 IP signal and the NFR23.3 control do not work as built. | Derive the address under an explicit proxy-trust policy: accept the edge header only when the observed peer is in a configured trusted-proxy range, or expose the gateway on a path that preserves the client address (host mode, or an external load balancer that sets a signed header). Add a test for the CDN/ingress case in which a peer is present and the edge header must win. | New |
| R-02 | Major | services/identity/application/src/jurisdiction.rs > CheckJurisdiction::check | Every check calls `ListRepository::active()` and so runs two Postgres queries (`jurisdiction_list_versions` and `jurisdiction_list_rules`) per decision. The performance design and NFR2.5 require the active version to be memory-resident with a refresh interval and at most one database read per list version. The per-request read is not among the declared deviations. | Hold the active list version in memory, refresh it on the designed interval and after a publish, or record the per-request read as an explicit, justified deviation from NFR2.5 with an owner. | New |
| R-03 | Major | services/identity/application/src/jurisdiction.rs > PublishList::publish | The publish audit event uses the list checksum as its idempotency key. The Audit service dedupes on (emitting_service, idempotency_key) and rejects a reused key carrying a different payload with 409 (`services/audit/application/src/append.rs`). Two publishes whose entries are identical, such as a routine republish of an unchanged list, reuse the key with a different version and timestamp: the second publish commits, its audit write is refused, the call returns 503 after the fact, and that version has no evidence record; the operator's retry then fails on the version primary key. | Key the publish evidence on the version label or a generated id instead of the checksum, and add a test with two publishes that share an identical entry set. | New |
| R-04 | Minor | services/identity/infrastructure/src/clients/geo_locator.rs > GeoLocator::country_for | The transport error is embedded verbatim in the provider-unavailable message, and reqwest's `Display` appends the request URL, which contains the raw IP (`{base}/{ip}/json/`). `AppError::from_external` places that text in the caller-visible 503 detail, so a provider outage can return the user's raw IP in an error body although the unit records that the IP never leaves memory (NFR19.1). | Return a fixed reason for provider transport failures and log only the failure class, never the full request URL. | New |
| R-05 | Minor | services/identity/application/src/jurisdiction.rs > PublishList::publish | Publish commits the version and then writes the audit event. A failed audit returns 503 although the publish already succeeded, and a retry of the same version is refused as a conflict, so no API path can complete the evidence for a committed publish. The deviation is declared but not mitigated. | Make the evidence write recoverable, for example by recording the publish as pending and activating it only after the audit is accepted or by adding a durable retry, or document the operational recovery procedure and accept the gap explicitly. | New |
| R-06 | Minor | aidlc/spaces/default/intents/261109-brd-002-users-roles/construction/u6-jurisdiction-gate/code-generation/code-summary.md > Gate results (coverage) | The approved plan's Definition of Done requires the 80% per-crate coverage target; the delivered state is identity-http 76.85% and identity-infrastructure 29.50%, and the new `HttpDecisionAudit` file has no unit coverage. The shortfall is declared, but the plan's own completion criterion is unmet. | Accept the deviations against the plan Definition of Done with an owner and date, or add adapter-level coverage for the new infrastructure code. | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| cargo test -p identity --test domain jurisdiction | 9 passed, 27 filtered out | Domain rules pass, including the fail-closed absent-country and checksum-mismatch cases. |
| cargo test -p identity --test application jurisdiction | 8 passed, 22 filtered out | Use-case tests pass, including the provider circuit and the audit-retry idempotency cases. |
| cargo test -p identity --test http jurisdiction | 7 passed, 25 filtered out | The filter also selects the unauthenticated-caller and publish-validation tests; all pass. |
| cargo test -p gateway --test http client_ip | 3 passed, 12 filtered out | Confirms header stripping and peer derivation; no test covers the CDN/ingress topology in the Critical finding. |
| cargo test -p identity --test repositories -- --ignored jurisdiction | Not run | Needs a disposable Postgres via `TEST_DATABASE_URL`; the developer reports 4 passing. Unverified in this pass. |
| bash scripts/check-boundaries.sh | Not run | Reported pre-existing failure in `reporting-domain`; context only, not attributed to this unit. |

### Summary

The domain and application logic are well tested and correctly fail closed, but the client-address control the whole IP signal depends on cannot produce the user's address in the deployed Swarm/CDN topology, which makes the gate fail or resolve the wrong country in production. Two further gaps compound it: the active list is read from Postgres on every check despite NFR2.5, and the publish audit key collides across republishes so a committed version can end up with no evidence record.
