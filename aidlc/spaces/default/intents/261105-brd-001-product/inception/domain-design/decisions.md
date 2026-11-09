# Architecture Decisions — Domain Design

## ADR-001: One component per documented service, minus Billing and Observability

**Context.** The C4 model in `docs/architecture/README.md` names 13 Rust containers including
the gateway, plus the SPA, the Support Console, and the Observability stack. The requirements
interview removed billing, plans, entitlements, and quotas. Domain Design must record logical
building blocks with distinct data ownership and change rates.

**Decision.** We record one component per documented service, except Billing (removed by the
interview) and the Observability stack (infrastructure, not application code). The result is 12
backend components plus WebApp and SupportConsole.

**Consequences.** Positive: every component owns one schema and one Postgres role, matching the
data-ownership rule; each changes for a different reason; ownership maps directly to the C4
model. Negative: 12 backend deployables to operate, each with health checks and a pipeline; the
team accepts that cost to keep independent scaling and clear boundaries.

**Alternatives Rejected.**
- *Fewer, coarser components* (collapse Ledger into Portfolio, Analytics into Notification):
  rejected because it mixes transaction history with valuation and consent-aware analytics with
  delivery, breaking per-service data ownership.
- *Finer components inside each service*: rejected because the extra boundaries would own no
  data and no distinct lifecycle, adding indirection without benefit.

## ADR-002: A separate Ledger component for transactions, cost basis, and P&L

**Context.** Transactions, dedupe, transfer pairing, cost basis, and realized/unrealized P&L are
the retail value loop. They evolve independently of valuation and snapshots, and BRD-013 makes
them a distinct domain.

**Decision.** Transactions, imports, cost basis, and P&L live in a separate Ledger component
with its own schema. Portfolio reads P&L over HTTP.

**Consequences.** Positive: the ledger can evolve and scale independently; reconciliation has a
clear owner; the append-only transaction rule is enforced in one place. Negative: Portfolio
needs a synchronous call to Ledger for performance views, and the two must agree on
reconciliation.

**Alternatives Rejected.**
- *Inside Portfolio*: rejected because valuation and transaction history change for different
  reasons and mixing them would couple the snapshot path to import logic.
- *Inside Account*: rejected because Account owns venue connectivity and credentials, not
  financial history.

## ADR-003: Billing, entitlements, and quotas are removed from this release

**Context.** BRD-016 and the C4 Billing Service define plans, entitlements, quotas, checkout,
and dunning. The requirements interview decided the product is free and open source for this
release.

**Decision.** No Billing component exists in this release, and no component performs
entitlement or quota checks.

**Consequences.** Positive: removes the merchant-of-record integration, subscription tax, and
plan enforcement from launch scope. Negative: the gateway no longer has an entitlement gate, so
abuse protection relies on rate limits alone; adding billing later requires a new component and
gateway checks.

**Alternatives Rejected.**
- *Keep usage limits without billing*: rejected because the interview chose to remove plans and
  quotas entirely.
- *Build Billing but ship it disabled*: rejected because it would carry dead code and a schema
  no path exercises.

## ADR-004: Notification owns every channel and delivery

**Context.** Alerts arrive from Account, Portfolio, and Reporting, and must honour preferences,
dedupe, retries, and mandatory security email.

**Decision.** The Notification component owns every channel and delivery. Other components send
alerts to it over internal HTTP with bounded retries.

**Consequences.** Positive: one place for preferences, dedupe, deliverability, and the delivery
log. Negative: a Notification outage delays every alert; the component becomes a dependency for
otherwise independent services.

**Alternatives Rejected.**
- *Each component sends its own notifications*: rejected because preference and dedupe logic
  would be duplicated and inconsistent.
- *Reporting owns brief email, Notification the rest*: rejected because it splits one channel's
  consent and retry behaviour across two owners.

## ADR-005: The SPA and the Support Console are separate UI components

**Context.** The customer app and the support tool have different audiences, different data
exposure, and different security rules. Support access must never reach credentials or portfolio
contents.

**Decision.** WebApp and SupportConsole are separate components, each calling only the gateway.

**Consequences.** Positive: support access is a distinct security boundary with its own audit
path. Negative: a second UI to build and deploy.

**Alternatives Rejected.**
- *Support as an admin section of the SPA*: rejected because it would put support capability in
  the same bundle customers load and weaken the least-privilege boundary.
- *No UI components in domain design*: rejected because the UI owns real behaviour (memory-only
  tokens, accessibility, localization) and must be designed.

## ADR-006: A separate Audit component with an append-only store

**Context.** BRD-012 requires append-only audit records with actor, action, target, timestamp,
context, and outcome, enforced by an insert-only role.

**Decision.** Audit is a separate component owning the append-only schema. Other components
emit events to it synchronously for security-relevant events and best-effort for domain events.

**Consequences.** Positive: one enforcement point for the insert-only role and retention; audit
queries and exports have a clear owner. Negative: every component depends on Audit for its
security events; a failed call must be handled explicitly rather than silently dropped.

**Alternatives Rejected.**
- *A shared library writing to each component's own schema*: rejected because it fragments the
  audit trail and makes the insert-only guarantee per-schema and unverifiable in one place.
- *Both a library and a component*: rejected as unnecessary indirection for this release; the
  synchronous call is simple enough.

## ADR-007: Synchronous internal HTTP only, with bounded retries and idempotency

**Context.** The architecture removed message queues in the 2026-11-02 simplification and
repairs missed work through scheduled reconciliation.

**Decision.** Every component-to-component call is internal HTTP: synchronous where the caller
needs an answer, fire-and-forget with bounded retries where it does not. Retries carry
idempotency keys or version checks.

**Consequences.** Positive: one communication model, simpler debugging, no broker to operate.
Negative: no natural fan-out or back-pressure; each fire-and-forget path re-implements retry;
reconciliation jobs must repair missed notifications.

**Alternatives Rejected.**
- *Message queues*: rejected by the project's development rules and the architecture history.
- *An event bus with an outbox*: rejected because it reintroduces the machinery the
  simplification removed.

## ADR-008: The API Gateway is the only application entry point, forwarding JWTs unchanged

**Context.** The SPA must not call services directly, and every service must validate the user's
identity.

**Decision.** The SPA and Support Console call only the gateway (plus Supabase Auth and
WalletConnect as browser-external exceptions). The gateway validates the JWT against Supabase
JWKS, resolves the principal through Identity, and forwards the user's JWT unchanged; every
service validates it again.

**Consequences.** Positive: one enforcement point for routing, rate limits, and admin gating;
defence in depth because services validate independently. Negative: the gateway is on every
request path and must stay stateless and horizontally scalable; each service needs JWKS access
and clock-skew tolerance.

**Alternatives Rejected.**
- *Direct service calls with signed internal assertions*: rejected because it reintroduces key
  management the architecture removed.
- *Gateway-only validation*: rejected because a misconfigured route would then bypass identity
  checks entirely.
