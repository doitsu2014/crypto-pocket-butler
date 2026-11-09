# Reliability Design — u7-ledger

Inputs: `nfr-requirements/reliability-requirements.md`.

- **Ingest idempotency**: canonical_key unique; replayed C5 pages store nothing new.
- **Corrections**: transaction insert commits, then C10 with 3 attempts (1 s/2 s/5 s); a persistent
  failure increments `ledger_audit_failures_total` and pages; the correction stays.
- **Reconcile**: lease 300 s, one run, one retry after 10 min; a missed run is visible.
- **Integrity**: lots never go negative; user-recorded trades over holdings refused; ingested sells
  always stored with a coverage gap.
- **Dependencies**: C5 read (1 s/3 s/10 s, 3 attempts) failures degrade ingest only; C4 resolve
  failure fails the write with a problem document; Postgres failure returns 503.
- **Health**: `/healthz` liveness; `/readyz` Postgres only.
- **Backup**: Supabase regime; RPO ≤ 24 h, RTO ≤ 4 h; rehearsal before launch.

NFR references: NFR1.1–NFR1.7.
