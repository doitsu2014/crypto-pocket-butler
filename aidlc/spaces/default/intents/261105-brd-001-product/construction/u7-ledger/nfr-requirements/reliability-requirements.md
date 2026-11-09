# Reliability Requirements — u7-ledger

| ID | Requirement | Target |
|---|---|---|
| NFR1.1 | P&L availability | 99.9% of requests answered over 30 days; a partial history returns labelled partial, never an error |
| NFR1.2 | Idempotent ingest | replaying the same C5 page stores no duplicate (canonicalKey) |
| NFR1.3 | Reconcile run | lease-claimed, one run at a time; a failed run retries once after 10 minutes |
| NFR1.4 | Corrections and audit | the correction row is committed first, then the C10 event is written with bounded retry; a persistent audit failure is surfaced and paged, and the correction is never rolled back once committed (a correction surviving without its audit is visible, not silent) |
| NFR1.5 | Backup | Supabase regime, RPO ≤ 24 h, RTO ≤ 4 h; restore rehearsal before launch |
| NFR1.6 | Health | `/healthz` liveness, `/readyz` checks Postgres only; a C5 outage degrades ingest and is surfaced as an ingest-level signal, not by failing readiness |
| NFR1.7 | Data integrity | a lot's remaining quantity never goes negative; a **user-recorded** trade exceeding known holdings is refused with a field-level error; an **ingested** venue/CSV sell is always stored and any shortfall becomes a coverage gap |

## Notes

- A failed C5 read degrades the ingest run, not the read path; the last computed P&L remains
  served with its `asOf`.
