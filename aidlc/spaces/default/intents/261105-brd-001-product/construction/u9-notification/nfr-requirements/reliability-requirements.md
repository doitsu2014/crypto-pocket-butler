# Reliability Requirements — u9-notification

| ID | Requirement | Target |
|---|---|---|
| NFR1.1 | Alert intake availability | 99.9% of submits answered over 30 days |
| NFR1.2 | Idempotency | a repeated key returns deduplicated and never sends twice |
| NFR1.3 | Delivery retries | 3 attempts (initial + 2 after 1 and 5 min); abandoned is recorded and surfaced |
| NFR1.4 | Mandatory email | a security/key_revoked email is attempted even when general notifications are off; a C3 network failure retries, a missing/unverified address is a permanent `address` error |
| NFR1.5 | Audit ordering | the alert commits first, then C10 with 3 attempts (1 s, 2 s, 5 s); a persistent failure increments `notification_audit_failures_total` and pages |
| NFR1.6 | Backup | Supabase regime, RPO ≤ 24 h, RTO ≤ 4 h |
| NFR1.7 | Health | `/healthz` liveness, `/readyz` Postgres; provider/C3 outages do not fail readiness |
