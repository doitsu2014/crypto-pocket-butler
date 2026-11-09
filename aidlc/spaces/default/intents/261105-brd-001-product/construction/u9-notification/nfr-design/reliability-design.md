# Reliability Design — u9-notification

Inputs: `nfr-requirements/performance-requirements.md`, `security-requirements.md`, `scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`, `tech-stack-decisions.md`, and `functional-design/functional-spec.md`.

- **Idempotency**: unique idempotency_key; a repeat returns deduplicated.
- **Delivery**: 3 attempts (initial + 2 after 1/5 min); abandoned recorded; mandatory abandonment
  pages.
- **C3 contact**: a missing/unverified address is permanent (`address`); a network/5xx error is
  transient and retried.
- **Audit**: commit alert, then C10 with 3 attempts (1/2/5 s); persistent failure pages.
- **Dependencies**: provider 2 s/10 s; C3 150 ms one attempt on the delivery path.
- **Health**: `/healthz`; `/readyz` Postgres only.
- **Backup**: Supabase regime; RPO ≤24 h, RTO ≤4 h.

NFR references: NFR1.1–NFR1.7.
