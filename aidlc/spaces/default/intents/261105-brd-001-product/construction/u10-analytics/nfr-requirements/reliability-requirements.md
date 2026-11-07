# Reliability Requirements — u10-analytics

Inputs: `functional-design/functional-spec.md`, `functional-design/rules.md`, and
`inception/requirements-analysis/requirements.md`.

| ID | Requirement | Target |
|---|---|---|
| NFR1.1 | Ingest availability | SLI: good = 202 + 403; bad = every 5xx including the C3-failure 503; SLO 99.9% over 30 days |
| NFR1.2 | C3 dependency | Consent call timeout 80 ms, one attempt; on timeout or error refuse with 503 rather than accept unconsented |
| NFR1.3 | Aggregation | Runs once nightly; a failed run retries once after 10 minutes; a missed run is visible as no fresh aggregate |
| NFR1.4 | Idempotency | An event has no natural key; duplicate events are accepted and counted, never deduplicated by content |
| NFR1.5 | Erasure | A failed erase is reported to Identity and retryable; partial deletes are retried from the start |
| NFR1.6 | Backup | Supabase backup, RPO ≤ 24 h, RTO ≤ 4 h; a restore rehearsal before launch |
| NFR1.7 | Health | `/healthz` liveness, `/readyz` checks Postgres and Vault salt availability |
