# Unit Dependencies — Crypto Pocket Butler

Topology only. This file says what can depend on what; Delivery Planning chooses the economic
order through it.

## Dependency DAG

Directed edge "A depends on B" means A's code or tests need B to build or pass.

- `u1-walking-skeleton` depends on nothing.
- `u12-audit` depends on `u1-walking-skeleton`.
- `u3-identity` depends on `u1-walking-skeleton`, `u12-audit`.
- `u4-account` depends on `u1-walking-skeleton`, `u12-audit`.
- `u5-market-data` depends on `u1-walking-skeleton`, `u12-audit`.
- `u7-ledger` depends on `u1-walking-skeleton`, `u4-account`, `u12-audit`.
- `u9-notification` depends on `u3-identity`, `u12-audit`.
- `u10-analytics` depends on `u3-identity`, `u12-audit`.
- `u6-portfolio` depends on `u1-walking-skeleton`, `u4-account`, `u5-market-data`,
  `u7-ledger`, `u9-notification`, `u12-audit`.
- `u8-reporting` depends on `u5-market-data`, `u6-portfolio`, `u7-ledger`, `u9-notification`,
  `u12-audit`.
- `u11-job-orchestration` depends on `u4-account`, `u5-market-data`, `u6-portfolio`,
  `u7-ledger`, `u8-reporting`, `u9-notification`, `u10-analytics`, `u12-audit`.
- `u2-web-app` depends on `u1-walking-skeleton`, `u3-identity`, `u4-account`, `u5-market-data`,
  `u6-portfolio`, `u7-ledger`, `u8-reporting`, `u9-notification`, `u10-analytics`.
- `u13-support-console` depends on `u1-walking-skeleton`, `u3-identity`, `u12-audit`.

```yaml
units:
  - name: u1-walking-skeleton
    kind: service
    depends_on: []
  - name: u2-web-app
    kind: ui
    depends_on: [u1-walking-skeleton, u3-identity, u4-account, u5-market-data, u6-portfolio, u7-ledger, u8-reporting, u9-notification, u10-analytics]
  - name: u3-identity
    kind: service
    depends_on: [u1-walking-skeleton, u12-audit]
  - name: u4-account
    kind: service
    depends_on: [u1-walking-skeleton, u12-audit]
  - name: u5-market-data
    kind: service
    depends_on: [u1-walking-skeleton, u12-audit]
  - name: u6-portfolio
    kind: service
    depends_on: [u1-walking-skeleton, u4-account, u5-market-data, u7-ledger, u9-notification, u12-audit]
  - name: u7-ledger
    kind: service
    depends_on: [u1-walking-skeleton, u4-account, u12-audit]
  - name: u8-reporting
    kind: service
    depends_on: [u5-market-data, u6-portfolio, u7-ledger, u9-notification, u12-audit]
  - name: u9-notification
    kind: service
    depends_on: [u3-identity, u12-audit]
  - name: u10-analytics
    kind: service
    depends_on: [u3-identity, u12-audit]
  - name: u11-job-orchestration
    kind: service
    depends_on: [u4-account, u5-market-data, u6-portfolio, u7-ledger, u8-reporting, u9-notification, u10-analytics, u12-audit]
  - name: u12-audit
    kind: service
    depends_on: [u1-walking-skeleton]
  - name: u13-support-console
    kind: ui
    depends_on: [u1-walking-skeleton, u3-identity, u12-audit]
```

## Walking skeleton

`u1-walking-skeleton` is the first resolved DAG unit and the integrated slice: SPA shell → API
Gateway → Asset Registry → its own Postgres schema → the Docker Swarm, with health and readiness
endpoints and observability. It runs end to end before later units are built, proving the
architecture, the JWT path, the schema and role isolation, and the deployment model.

## Integration points

| Between | Contract |
|---|---|
| Web App → ApiGateway | HTTPS, Supabase JWT in the Authorization header |
| Support Console → ApiGateway | HTTPS, Supabase JWT with the support role |
| ApiGateway → each service | Internal HTTP, user JWT forwarded unchanged, correlation ID |
| Account → Asset Registry | Internal HTTP asset resolution |
| Portfolio → Account, Market Data, Ledger, Asset Registry | Internal HTTP reads |
| Reporting → Portfolio, Ledger, Market Data | Internal HTTP reads |
| Reporting, Account, Portfolio → Notification | Internal HTTP alert submission with idempotency keys |
| Every service → Audit | Internal HTTP audit event submission |
| Job Orchestration → each service | Internal HTTP run endpoint with the shared service token |
| Every service ↔ Supabase Postgres | One schema and one Postgres role per service, no cross-schema grants |

## Parallel development opportunities

Units with no dependency between them can be built at the same time:

- `u3-identity`, `u4-account`, and `u5-market-data` are mutually independent once
  `u1-walking-skeleton` and `u12-audit` exist.
- `u9-notification` and `u10-analytics` are mutually independent.
- `u13-support-console` can proceed in parallel with `u6-portfolio` and later units once
  `u1-walking-skeleton`, `u3-identity`, and `u12-audit` exist.
- `u2-web-app` is the last consumer: it needs the services it calls.

## Topology notes

- The graph is acyclic.
- `u1-walking-skeleton` and `u12-audit` are the only units with no service dependency of their
  own beyond the skeleton.
- No unit depends on `u11-job-orchestration`; it is a caller of the others.
