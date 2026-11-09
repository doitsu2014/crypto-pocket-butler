# Tech Stack Decisions — u10-analytics

Inputs: `functional-design/functional-spec.md`, `functional-design/rules.md`, and
`inception/requirements-analysis/requirements.md`.

| Decision | Choice | Rationale |
|---|---|---|
| Language/framework | Rust, Axum, tower, tokio | Project rule |
| Persistence | sqlx, schema `analytics`, role `analytics_svc` | One schema and role per service |
| Hashing | `hmac` + `sha2` crates, salt from Vault | HMAC-SHA256 pseudonymisation |
| Aggregation | SQL over `analytics.events`, upserted into `metric_aggregates` | No separate analytics store at launch |
| Flags | Postgres table, deterministic bucket | Simple, auditable |
| Secrets | Vault via a `SECURITY DEFINER` function | No cross-schema grant |
| Scheduling | C11 run endpoints | No message queue |
| Observability | tracing + OTLP, Prometheus exporter | Existing stack |

## Rejected alternatives

- A third-party product-analytics vendor: consent and PII constraints require in-house control.
- A separate OLAP store: 1 M events/month fits Postgres with the 90/180-day pruning.
