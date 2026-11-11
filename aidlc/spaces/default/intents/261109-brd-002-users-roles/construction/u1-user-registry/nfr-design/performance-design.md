# Performance Design — u1-user-registry

**Upstream:** `performance-requirements.md` (NFR2.1–NFR2.4), `functional-spec.md`, `contract-summary.md` (C3, C8, C9).

## Latency budget

| Path | Budget | Design |
|---|---|---|
| Resolve an existing user | p95 ≤ 100 ms | One indexed read by subject hash; no join |
| Provision a new user | p95 ≤ 2 s | C8 gate call inside its 1.5 s cap; key reference + user row in one transaction ≤ 500 ms; audit write 300 ms with one retry |
| Admin lookup | p95 ≤ 300 ms at 1M users | Indexed exact match on email or id |
| Profile update | p95 ≤ 100 ms | Single row update |

## Optimisations

- **Indexes:** `users(provider_subject_hash)` unique, `users(id)` primary, `users(email)` for admin exact match.
- **No N+1:** the principal is built from one row; role comes from the token and is not queried.
- **Connection pool:** min 2, max 10 per replica.
- **Dependency budgets:** the C8 gate and C9 audit each have their own timeout and retry budget; the registry's own work is capped so a slow dependency cannot exceed the end-to-end target.
