# Tech Stack Decisions — u2-authorisation

**Upstream:** `codekb/crypto-pocket-butler/technology-stack.md`, `functional-spec.md` (where the decision lives), `project.md` (Mandated/Forbidden).

The stack is fixed by project rule. This file records only what this unit adds or leans on, and
why.

| Concern | Choice | Rationale | Alternative rejected |
|---|---|---|---|
| Decision implementation | A module inside the Identity service's `application` crate | The decision is business logic over validated claims; it belongs behind the service's own ports, and `unit-of-work.md` forbids a shared crate | A new shared crate linked by every service — rejected: it crosses service workspaces and breaks the boundary gate |
| External boundary | The C4 `/internal/authorize` endpoint, an Axum route on the Identity `http` crate | Callers outside the Identity workspace (Account, Portfolio) need a surface; the contract already declares one | Making every caller embed the library — rejected: TypeScript surfaces cannot, and other services have their own workspaces |
| Internal HTTP client | `reqwest` 0.12 with `rustls-tls` and the contract's 5s connect / 10s read / three-attempt policy | Already the stack's internal client; no new dependency | `hyper` directly — rejected: more code for the same behaviour |
| Token validation | `jsonwebtoken` 9 against Supabase JWKS, as every service already does | One validation path across the platform (NFR9.1) | Validating in the decision itself — rejected: the caller's middleware already owns it, and duplicating it doubles the trust surface |
| Audit sink | The existing internal audit service client (C9) behind an `AuditSink` port | Keeps the fail-closed behaviour testable with a fake; no new transport | Writing audit rows directly to the audit schema — rejected: cross-schema access is forbidden |
| Role storage | Supabase Auth `app_metadata.role` through the admin API | One source of truth; the role arrives on every token (NFR24) | A roles table in the Identity schema — rejected: it would be a second source of truth and go stale against the token |
| Cache | An in-process bounded map with a 30-second TTL | No new infrastructure; bounded staleness during a u1 outage | A distributed cache — rejected: it would add a dependency whose failure mode is worse than the blip it covers |
| Persistence | None | The unit owns no entity | — |
| Frontend | None; the SPA and support console consume the gateway's decisions | U2 is a service unit | — |

## Dependencies added by this unit

None beyond what the Identity service already declares. The decision uses `async-trait` for the
audit port, `tracing` for logs and spans, `thiserror` for the typed denial reason, and the
existing `reqwest`/`jsonwebtoken` stack for the C4 client and token validation.

## Constraint check

- Dependencies point inward only: the decision lives in `application`, the C4 router in `http`,
  the Supabase Auth adapter in `infrastructure` (`project.md` Mandated).
- No ORM other than sqlx; no new schema; no message queue.
- The boundary gate (`scripts/check-boundaries.sh`) gains no new edge.
