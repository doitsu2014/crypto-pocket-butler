## Review

**Verdict:** READY

**Reviewer:** aidlc-architecture-reviewer-agent

**Iteration:** 1

**Verdict Dictated:** no

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u5-market-data/infrastructure-design/infrastructure-specification.md > Infrastructure Services, Service-token registry and Smoke task rows, and nfr-design/security-design.md > Authentication and authorization | The new read-only `smoke-reader` caller is seeded in the service-token registry and used by the smoke task, but the READY security design's C6 read allow-list still names only Portfolio, Reporting and Job Orchestration. If that allow-list is enforced as written, the smoke token resolves but the smoke calls return 403 and the release gate fails. | Record `smoke-reader` in the security design's C6 read allow-list (read operations only), or state in the infrastructure spec that the registry row carries its read-only allow-list, so the gate is authorised by both artifacts. | New |
| R-02 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u5-market-data/infrastructure-design/monitoring-design.md > Alerts, Tail freshness row | The tail freshness ticket fires at "live < 99% over 20 min (below the 15-minute SLO)", but a sub-99% live fraction over a 20-minute window is the SLO breach itself, not a threshold below it; the note is inaccurate. The 12-minute tail-overdue alert is the genuinely early signal. | Either tighten the tail freshness condition so it fires before the breach (for example live < 99.5% over 15 minutes) or correct the note to say the overdue alert is the early signal and this ticket is the breach confirmation. | New |

### Summary

The smoke gate is now executable and authorised end to end: a one-shot `market-data-smoke` task on the `internal` overlay uses a `smoke-reader` registry row and the `market_data_smoke_token` Docker secret held by the release job, with no published port. The topology no longer routes the SPA through the gateway to Market Data, the migrate and smoke jobs are created by the release job rather than the stack, the C11 run token is read from Vault at startup, the tail alerts and traceability rows are corrected, and Supavisor pooling, the single `market_data_svc` role and the Vault-based token paths all hold. The two remaining items are a registry/allow-list alignment for the smoke caller and one alert-threshold note, so the stage is READY.
