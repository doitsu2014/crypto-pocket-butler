# Tech Stack Decisions — u11-job-orchestration

| Concern | Decision | Alternative rejected |
|---|---|---|
| Language and framework | Rust + Axum + tokio (project mandate) | Node scheduler (out of stack) |
| Storage | Supabase Postgres, one `job_orchestration` schema, `job_orchestration_svc` + `_migrator` roles | A queue (forbidden) or a file lock (no single flight across replicas) |
| Ticker | In-process `tokio::time::interval` guarded by an OrchestratorLease row | Supabase Cron (no unit may own a second scheduler; the lease must be observable) |
| Outbound calls | `reqwest` with per-job timeout, 32-way bounded concurrency, W3C trace propagation | Blocking calls inside the tick (starves the heartbeat) |
| Retry model | One run row per attempt, retry through `nextAttemptAt` | Mutating a failed row (destroys attempt history) |
| Identity | Supabase JWKS validation in this service; `admin`/`support` role from claims | Trusting the gateway alone (forbidden by project rules) |
| Alerting | Prometheus/Alertmanager rules on heartbeat age and failure counters | C9 user alerts (C9 requires a userId and does not serve operators) |
| Migrations | sqlx up/down pairs; the runtime role holds no DELETE | In-service retention (no DELETE grant) |
