# Logical Components — u8-reporting

Inputs: `nfr-requirements/performance-requirements.md`, `security-requirements.md`, `scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`, `tech-stack-decisions.md`, and `functional-design/functional-spec.md`.

| Logical component | Responsibility | Scaling | Failure domain |
|---|---|---|---|
| Reporting API | C1 briefs, owner-scoped | 2 replicas | One task |
| Brief composer | reads C8/C7, renders sections with as-of/status | In worker | One brief fails |
| Generation worker | due users, batches, lease, email submit | 1 replica | Missed run visible |
| `reporting` schema | briefs, deliveries | Supabase | Service-owned |
| C3/C7/C8/C9/C10 clients | users, P&L, valuation, email, audit | Per call | Bounded timeouts |

## Boundaries

- No business math; no provider email; no cross-schema read.
NFR references: NFR20.1, NFR9.3, NFR1.2.
