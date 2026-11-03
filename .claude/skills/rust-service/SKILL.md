---
name: rust-service
description: >
  Scaffold a new Crypto Pocket Butler backend service as a Clean Architecture
  Cargo workspace (domain, application, infrastructure, http crates), with its
  Supabase Postgres schema and role, health/metrics endpoints and the internal
  run endpoint for scheduled jobs. Use when creating or extending one of the
  Rust services in docs/architecture (Identity, Account, Asset Registry, Market
  Data, Portfolio, Reporting, Job Orchestration, Audit) or the API Gateway.
argument-hint: "[service name or description]"
---

# Scaffold a Rust Service

Every backend service in this repo follows one shape. Do not invent a new layout.

## 1. Workspace layout

Create `services/<name>/` as a Cargo workspace of exactly four crates:

```
services/<name>/
  Cargo.toml            # [workspace] members = ["domain", "application", "infrastructure", "http"]
  domain/               # entities, value types, business rules. No framework deps.
  application/          # use cases + ports (traits). Depends on domain only.
  infrastructure/       # sqlx repositories, Vault, OKX/RPC clients, OTLP. Implements ports.
  http/                 # Axum router, DTOs, middleware. Depends on application (+ domain).
```

Allowed `Cargo.toml` dependency edges (the CI boundary check enforces these):

| Crate | May depend on |
| --- | --- |
| `domain` | external crates only; **never** a sibling crate, Axum, sqlx, reqwest, tokio, or serde derive-free transport types |
| `application` | `domain` |
| `infrastructure` | `application`, `domain` |
| `http` | `application`, `domain` |

## 2. Crate contents

- `domain`: entities and value types with `rust_decimal::Decimal` for every quantity and
  value — never `f64` for money or asset amounts.
- `application`: one use-case module per BRD functional requirement; ports are traits defined
  here (or in domain), implemented in `infrastructure`; no DTOs, no Axum types.
- `infrastructure`: sqlx repos against the service's **own schema and Postgres role only**;
  secret reads go through Supabase Vault, never env vars; outbound HTTP clients with bounded
  retries and idempotency keys.
- `http`: Axum router; DTOs map to/from domain at the boundary; every handler validates the
  forwarded Supabase JWT and enforces owner scoping; DTO types never appear in inner crates.

## 3. Standard endpoints (in `http`)

- `GET /healthz` and `GET /readyz` — liveness and readiness.
- `GET /metrics` — Prometheus metrics.
- `POST /internal/run/<job>` — the scheduled run endpoint the Job Orchestration Service calls
  with the shared service token; must be single-flight (Postgres advisory lock) and
  idempotent. Skip only if the service has no scheduled work.

## 4. Database

The service owns one Supabase Postgres schema and one role named after the service
(identity, account, registry, market, portfolio, reporting, job, audit). Migrations live in
the service; grants give the role access to its schema only. No cross-schema grants, no RLS.

## 5. Testing floor

One happy-path test per use case in `application` (port fakes), one integration test per
adapter against a real schema. Wire the service into the Swarm stack definition when it
exists.

After scaffolding, run the `clean-architecture` skill to verify boundaries.
