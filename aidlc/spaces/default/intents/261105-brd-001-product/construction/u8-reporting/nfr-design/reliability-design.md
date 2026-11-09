# Reliability Design — u8-reporting

Inputs: `nfr-requirements/performance-requirements.md`, `security-requirements.md`, `scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`, `tech-stack-decisions.md`, and `functional-design/functional-spec.md`.

- Exactly once via unique `(user_id, period, period_start)`; failed or hour-old pending retried in place.
- Failure rule: no valuation → failed + reason; degraded → generated partial/unavailable.
- Run: 1,800 s lease renewed every 60 s; one retry after 10 min.
- Email: C9 `email` channel, key `brief:{briefId}:email`; submission failures recorded and retried.
- Dependencies: C3 2 s, C8 800 ms, C7 400 ms, C9 1/3/10 s ×3, C10 1/2/5 s ×3.
- Health/backup: `/healthz`, `/readyz` Postgres; RPO ≤24 h, RTO ≤4 h.

NFR references: NFR1.1–NFR1.7.
