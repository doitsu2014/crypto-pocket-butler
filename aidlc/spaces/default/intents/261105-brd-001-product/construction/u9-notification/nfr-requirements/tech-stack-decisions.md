# Tech Stack Decisions — u9-notification

| Decision | Choice | Rationale |
|---|---|---|
| Language/framework | Rust, Axum, tower, tokio | Project rule |
| Persistence | sqlx, schema `notification`, role `notification_svc` | One schema and role per service |
| Email provider | transactional email provider over HTTPS (SMTP not used from the API) | TLS, retries, delivery receipts |
| In-app feed | alerts table read by the SPA through the gateway | No extra channel service |
| Auth | JWT for users; per-caller service tokens for C9/C11 | Internal callers |
| Scheduling | A durable `wake_requested_at` flag set on submit; the single leased delivery worker polls every 5 s (worst-case delay 5 s; mandatory ≤ 30 s including provider call) | No message queue |
| Observability | tracing + OTLP, Prometheus exporter | Existing stack |

## Rejected alternatives

- A message queue for delivery: forbidden by project rules; the alerts table is the queue.
- SMTP from the API: worse TLS and receipt handling than provider HTTPS.
