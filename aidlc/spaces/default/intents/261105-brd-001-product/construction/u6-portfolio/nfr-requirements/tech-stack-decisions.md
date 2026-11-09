# Tech Stack Decisions — u6-portfolio

| Decision | Choice | Rationale |
|---|---|---|
| Language/framework | Rust, Axum, tower, tokio | Project rule |
| Persistence | sqlx, schema `portfolio`, role `portfolio_svc` | One schema and role per service |
| Money | `rust_decimal` for quantities, values, percentages | No binary floating point |
| Valuation | in-process over C5 holdings + C6 quotes/FX | Backend-only math |
| Snapshots | immutable rows with frozen inputs + SHA-256 digest | Reproducibility |
| Scheduling | C11 valuation run | No message queue |
| Observability | tracing + OTLP, Prometheus exporter | Existing stack |

## Rejected alternatives

- Computing valuation in the SPA: forbidden by project rules.
- Storing only the total: history needs per-asset lines with provenance.
