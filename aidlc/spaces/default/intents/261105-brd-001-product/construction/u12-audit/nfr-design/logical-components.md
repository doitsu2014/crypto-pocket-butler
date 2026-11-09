# Audit — Logical Components

Where the NFR patterns actually live, and what fails when.

## Component inventory

| Component | Kind | Responsibility | Failure domain |
|---|---|---|---|
| `audit-api` | Swarm service, 3 replicas | Append, query, export, support-session start and check, privacy deletion | Replica failure affects only that replica; the routing mesh removes it |
| `audit-retention` | Swarm service, 1 replica, scheduled | Past-window drops and batched deletes, privacy deletions, and extending the partition horizon if a release has not, under the retention credential | Its own credential and process. A partition shortfall only reaches appends if the deploy migration's three-month horizon runs out, which the horizon alert fires on first |
| `audit schema` | Supabase Postgres schema `audit` | The trail, workflow records, three roles | Shared with every other schema on the same instance; the connection budget is the isolation |

There is **no export file store**: exports stream from the cursor into the HTTP response, so no
replica holds state another replica would need, and the "stateless and interchangeable" claim is
literally true.

`audit-api` and `audit-retention` are separate images and separate credentials. That separation is
the whole of the "stolen application credential cannot erase history" claim.

## Blast radius

| If this fails | What the user sees | What is lost |
|---|---|---|
| `audit-api` fully down | Security-event appends fail their callers; sign-in and credential actions refuse rather than proceed unlogged | Nothing already written |
| `audit-api` slow | Callers' latency rises by the append's share; the circuit breaker opens and callers fail fast | Nothing |
| `audit-retention` down | No effect on traffic while the partition horizon holds (three months from the last release); logs grow past six months | Nothing; retention is late, retention and privacy-deletion requests queue behind it, and the heartbeat alert says so |
| Partition horizon under two months | None yet — but appends fail once it empties | Nothing yet; the horizon alert fires well before that |
| Postgres unreachable | Every replica fails readiness; no append succeeds | In-flight appends only |
| One partition corrupt | Queries spanning it fail; other partitions are fine | That month's rows, restored from backup |

## Shared resources and their budgets

| Shared resource | Budget | Isolation mechanism |
|---|---|---|
| Postgres connections | 24 for `audit-api`, 2 for `audit-retention` | Per-replica pools with separate append, query, and export budgets |
| Postgres instance | Shared with every other schema | Retention keeps the row count bounded; partition drops are metadata operations |
| Observability stack (Loki/Prometheus/Tempo) | Shared | Low-cardinality labels only: action, class, result — never an id as a label |
| Vault | Shared | Each role has its own path and its own credential |

## Boundaries this design depends on

- No component reads another service's schema. Account metadata and tickets are served by their
  owner; Audit only says whether a session allows the read.
- The retention service never holds the append path's credentials, and `audit-api` never holds a
  delete grant. Retention does hold `INSERT` on `audit_events` for its own run and deletion events,
  which is a narrower grant than the append role's: it can write its own evidence and not another
  service's.
- Release images are digest-pinned in the Swarm stack file and vulnerability-scanned by the
  pipeline before promotion (NFR12.1).
- The gateway is the only inbound edge; there is no direct public route to the audit service.

## Sources

- `construction/u12-audit/nfr-requirements/security-requirements.md` — NFR19.1 to NFR19.4
- `construction/u12-audit/nfr-requirements/scalability-requirements.md` — NFR20.1, NFR20.7
- `construction/u12-audit/nfr-requirements/reliability-requirements.md` — NFR1.3, NFR14.3
- `construction/u12-audit/functional-design/functional-spec.md` — section 3 integration points
- `inception/units-generation/unit-of-work.md` — U12's responsibility statement
