# Tech Stack Decisions — u8-reporting

| Decision | Choice | Rationale |
|---|---|---|
| Language/framework | Rust, Axum, tower, tokio | Project rule |
| Persistence | sqlx, schema `reporting`, role `reporting_svc` | One schema and role per service |
| Money | `rust_decimal` when rendering figures | No binary floating point |
| Composition | reads C7/C8; stores rendered sections | No business math recomputed |
| Email | C9 only, `channel: email`, key `brief:{briefId}:email` | Notification owns preferences and provider; no second in-app entry |
| Scheduling | C11 run | No message queue |
| Observability | tracing + OTLP, Prometheus exporter | Existing stack |

## Rejected alternatives

- Sending email directly: duplicates Notification's preference/address logic.
- Storing only a link to live data: briefs must be immutable historical records.
