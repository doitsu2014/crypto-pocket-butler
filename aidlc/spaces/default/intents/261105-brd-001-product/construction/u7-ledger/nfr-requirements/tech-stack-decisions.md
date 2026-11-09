# Tech Stack Decisions — u7-ledger

| Decision | Choice | Rationale |
|---|---|---|
| Language/framework | Rust, Axum, tower, tokio | Project rule |
| Persistence | sqlx, schema `ledger`, role `ledger_svc` | One schema and role per service |
| Money | `rust_decimal` for quantity, price, cost, P&L | No binary floating point |
| Cost basis | FIFO lots in Postgres, consumed in a transaction | Auditable and reconcilable |
| Dedupe | unique `canonical_key` and `external_key` indexes | Cross-source identity |
| Append-only | role grants SELECT/INSERT only on transactions/corrections | Mechanical enforcement |
| Scheduling | C11 reconcile run | No message queue |
| Observability | tracing + OTLP, Prometheus exporter | Existing stack |

## Rejected alternatives

- Average-cost basis: simpler but not reconcilable to specific disposals and harder to audit.
- Event sourcing with an external store: the append-only tables already provide the audit trail.
