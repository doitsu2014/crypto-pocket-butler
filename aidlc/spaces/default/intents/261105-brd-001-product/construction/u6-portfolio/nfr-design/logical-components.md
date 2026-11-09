# Logical Components — u6-portfolio

Inputs: `nfr-requirements/performance-requirements.md`, `security-requirements.md`, `scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`, `tech-stack-decisions.md`, and `functional-design/functional-spec.md`.

| Logical component | Responsibility | Scaling | Failure domain |
|---|---|---|---|
| Portfolio API | C1 routes, valuation read, previews, trades | 2 replicas | One task |
| Valuation engine | quantity × price, FX, allocation, drift | In-process | Degrades labelled |
| Snapshot writer | immutable row + inputs digest | Per valuation | A failed write retried by next run |
| Guardrail evaluator | crossing state and C9 alert | In-process | One alert per crossing |
| Valuation worker | hourly snapshots, guardrails, retention | 1 leased replica | Missed run visible |
| `portfolio` schema | portfolios, membership, targets, guardrails, snapshots, overlays, intents, trades | Supabase | Service-owned |
| C5/C6/C7/C9/C10 clients | holdings, quotes/FX, trades, alerts, audit | Per call | Bounded timeouts |

## Boundaries

- No cross-schema read; all math here; no external order calls.
NFR references: NFR20.1, NFR9.3, NFR1.2.
