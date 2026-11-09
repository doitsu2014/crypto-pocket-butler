# Reliability Requirements — u6-portfolio

| ID | Requirement | Target |
|---|---|---|
| NFR1.1 | Portfolio availability | 99.9% of reads answered over 30 days; a degraded valuation is labelled, not an error |
| NFR1.2 | Degraded valuation | C5 holdings unavailable → `status = partial`, `holdingsUnavailable = true`, no fabricated lines; C6 prices unavailable → unpriced lines; only Postgres failure is 503 |
| NFR1.3 | Snapshot immutability | a snapshot is never updated or deleted, enforced by `portfolio_svc` having SELECT/INSERT only on the snapshot table (no UPDATE/DELETE grant); reproduction verifies its digest |
| NFR1.4 | Valuation run | leased, one run, retry once after 10 minutes; a missed run is visible. Dependency timeouts on the read path: C5 200 ms one attempt, C6 150 ms one attempt, C7 300 ms one attempt; the 800/900 ms budgets include them |
| NFR1.5 | Recorded trade | Ledger failure refuses the trade (no Portfolio-only trade); after the write, C10 is attempted 3 times (1/2/5 s), its failure increments `portfolio_audit_failures_total` and pages, and it never extends the read path's 900 ms budget |
| NFR1.6 | Backup | Supabase regime, RPO ≤24 h, RTO ≤4 h |
| NFR1.7 | Health | `/healthz` liveness, `/readyz` Postgres only |
