# Tech Stack Decisions — u6-jurisdiction-gate

**Upstream:** `performance-requirements.md`, `security-requirements.md`, `scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`, `functional-spec.md`, `technology-stack.md`.

| Decision | Choice | Rationale | Alternatives rejected |
|---|---|---|---|
| Language and web framework | Rust 2021 with Axum and tower | Project mandate; the service is a four-crate workspace in the Identity service | Other languages/frameworks — forbidden by project memory |
| Async runtime | tokio | Project mandate; consistent with the existing services | — |
| Persistence | sqlx against the Identity Postgres schema (`jurisdiction_list_versions`, `jurisdiction_rules`) | Project mandate (sqlx only); one schema per service | An ORM — forbidden; a separate store — adds a service boundary with no benefit |
| List serving | Load the active version into memory per replica; refresh on publish | Meets the p95 target without a per-request database read (NFR-PERF-U6.4) | Per-request query — slower and noisier |
| Geolocation | An external provider over HTTPS with a bounded HTTP client (2 s timeout, one retry) | Meets NFR-PERF-U6.3 and NFR-REL-U6.2; isolated behind a port | Bundled geo-IP database — stale data and a licensing obligation |
| Messaging | None | Project mandate: no message queues | Event bus — forbidden |
| Audit | HTTP event write to the Audit service | Project mandate; one append-only store | Local audit table — splits the audit trail |
| Migrations | Versioned sqlx up/down pairs | Project mandate; every schema change ships both directions | — |

These choices are constraints from the project memory and the contract summary, not new decisions; the only local choice is the memory-resident list, which follows directly from the latency target.
