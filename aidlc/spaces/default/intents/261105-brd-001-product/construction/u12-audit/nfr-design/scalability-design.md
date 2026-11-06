# Audit — Scalability Design

## Scaling model

The service is stateless behind the Swarm's routing mesh; replicas are interchangeable and hold no
session state, no cache, and no local queue. Scale-out is a replica count change, scale-up is a
task-resource change. At launch: 3 replicas × 0.5 vCPU / 512 MB.

Postgres is the constraint, not the replicas. The connection budget is what the design protects:
8 per replica (5 append, 1 query, 2 export) × 3 replicas = 24, plus 2 for the retention service.

## Partitioning

`audit_events` is declaratively partitioned by `recordedAt` month from the first migration:

- Inserts name the partition, so write cost does not grow with history.
- Queries carry a time range and prune to the partitions they need.
- **Retention decision rule, stated once.** A partition whose entire range is past the cutoff and
  that holds no legally held row is `DROP`ped — a metadata operation. A partition that is partly
  in-window, or holds even one legally held row, is swept by batched row `DELETE` (10 000 rows,
  200 ms pause). Both paths write the same `RetentionRun` record, so the record does not care which
  was used. A month held by one legal hold is kept whole and swept by rows, and it does not trip
  the "more than 9 live months" alert, which counts only partitions entirely past their window.

Partitions are created **by the deploy migration three months ahead**, so a routine release keeps
the horizon filled and the append path never depends on the retention service being alive. The
retention service only extends the horizon as a fallback if a release has not happened in time. The
default partition is deliberately absent, so a missing range fails loudly instead of silently
capturing data, and an alert fires when the horizon drops under two months (see the observability
design).

## Growth

One set of numbers, from one stated assumption: **average 2 events/second, peak 20 events/second**
(about 5.2 million events a month at average load, ~400 bytes a row with its indexes).

| Month | Rows | Size |
|---|---|---|
| 1 | 5.2 M | ~2.1 GB |
| 3 | 15.7 M | ~6.3 GB |
| 6 | 31.5 M | ~12.6 GB |

The previous draft carried a second figure of "roughly 320 M rows" — that is the impossible case of
sustaining the 20 events/second peak for six months. It is the capacity ceiling to size headroom
against, not the expected window; the instance class is chosen so that even that ceiling fits, and
the disk threshold is computed from the 31.5 M row expectation.

## Capacity thresholds

| Signal | Threshold | Action |
|---|---|---|
| Connection pool waiters | > 0 for a minute | Reduce the export budget first, then add replicas |
| Append p95 | > 100 ms for 10 min | Investigate before it becomes a caller-visible failure |
| Partition count | > 9 partitions entirely past their window | Retention is not running; treat as an incident |
| Partition horizon | < 2 months of future partitions | P1: the next release or the fallback job must fill it before appends start failing |
| Disk | 80% of the instance | Retention review and a capacity decision |

## Deliberately not designed

No read replicas, no sharding, no cache tier, no queue. Each would add a failure mode to a service
whose job is to be boringly correct; partitioning and a bounded window carry the load instead.

## Sources

- `construction/u12-audit/nfr-requirements/scalability-requirements.md` — NFR20.1 to NFR20.8
- `construction/u12-audit/nfr-requirements/performance-requirements.md` — NFR2.1, NFR2.6
- `construction/u12-audit/nfr-requirements/reliability-requirements.md` — NFR1.8, NFR1.9
- `construction/u12-audit/functional-design/functional-spec.md` — WF6
