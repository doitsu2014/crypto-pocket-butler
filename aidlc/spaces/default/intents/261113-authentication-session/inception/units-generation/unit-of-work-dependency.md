# Unit of Work Dependency — Authentication & Session Management

**Purpose:** record the direct dependency DAG only. This file does not choose the order in which
Delivery Planning should execute independent work and does not identify a critical path.

## Direct dependencies

```yaml
units:
  - name: u1-identity-auth-domain
    kind: service
    depends_on: []
  - name: u2-identity-gateway
    kind: service
    depends_on: [u1-identity-auth-domain]
  - name: u3-spa-auth-sessions
    kind: ui
    depends_on: [u2-identity-gateway]
  - name: u4-auth-integration-proof
    kind: spec
    depends_on: [u1-identity-auth-domain, u2-identity-gateway, u3-spa-auth-sessions]
```

## Dependency rationale

| Unit | Direct dependency | Why the Unit's own code or tests need it |
|---|---|---|
| `u1-identity-auth-domain` | none | Extends the existing Identity-owned policy/use-case layer; it can be implemented and tested against current ports and fakes. |
| `u2-identity-gateway` | `u1-identity-auth-domain` | HTTP handlers and gateway route declarations depend on the use cases, ports and policy introduced by U1. U2 tests need the U1 contracts and implementation available. |
| `u3-spa-auth-sessions` | `u2-identity-gateway` | The SPA calls the concrete route set and response contracts supplied by U2; its route-mocked tests must match those declared paths and errors. |
| `u4-auth-integration-proof` | U1, U2, U3 | End-to-end route and browser proof exercises the integrated implementation from all three Units. |

## Integration points

| Boundary | From | To | Interaction | Data crossing |
|---|---|---|---|---|
| Browser API | U3 | U2 | HTTPS through the single public gateway; user JWT forwarded unchanged | Gate choices, MFA requests, session list/revoke, recovery request; RFC 7807 responses |
| Identity HTTP | U2 | U1 | In-process application use cases behind inward ports | Validated request context, domain commands/queries, typed application errors |
| Supabase Auth | U1/U2/U3 | External Supabase Auth | Browser SDK for sign-in; Identity adapter for admin/factor operations | Credentials go only to provider; provider tokens/factor results; no secret in logs |
| Login alert | U1 | Notification | Synchronous internal HTTP through a port; bounded retry and idempotency key | User id, coarse device label/location, time; no token/fingerprint |
| Audit | U1/U2 | Audit | Existing per-service audit outbox and append-only audit API | Actor, action, target, timestamp, context, outcome; no credentials or tokens |

## Parallelism

There are no independent Units in this approved DAG. Every edge is required for the dependent Unit's
code or tests to build and pass. Therefore the topology has one ready Unit at a time. Delivery Planning
may choose an economic path through the DAG but cannot remove these prerequisites without changing
this approved boundary plan.

## Walking-skeleton status

No separate walking-skeleton Unit applies. The existing Asset Registry end-to-end slice is already
built. This intent's authentication slice is proven by U4 only after the Identity/gateway paths and SPA
surfaces exist; U4 is a verification Unit, not a skeleton marker and not an earlier topology node.
