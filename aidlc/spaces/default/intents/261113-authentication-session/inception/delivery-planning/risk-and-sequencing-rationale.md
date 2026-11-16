# Risk and Sequencing Rationale — Authentication & Session Management

**Chosen heuristic:** risk-first through the approved dependency graph, then deliver the highest user
value available at each point. The Unit responsibilities and direct DAG are specified in
`unit-of-work.md` and `unit-of-work-dependency.md`; this rationale chooses the economic path through
them, not different boundaries or topology. A **Bolt** is one build pass over a Unit that ends in a running,
reviewable result. The sequence does not reorder the DAG: risk-first is the economic rationale within
the only topologically valid chain.

No numeric WSJF score is used. WSJF (Weighted Shortest Job First) ranks work by value, urgency and
risk reduction divided by estimated size; this intent has no agreed person-day estimates or measured
reach for its Units, so numeric scores would imply precision we do not have.

**Sequencing decision (interview Q1):** risk-first through the strict dependency graph, then user value.
**Bolt sizing (Q2):** one Unit per Bolt. **Completion test (Q3):** every in-scope SPA auth path has one
Identity handler, is declared at the gateway, and browser journeys pass; report unresolved review
findings explicitly.

## Sequence rationale

| Order | Bolt / Unit | Risk targeted early | Value delivered / evidence |
|---|---|---|---|
| 1 | U1 — Identity auth domain | Trusted-side policy: gates, AAL2/limited session, revocation, support recovery and audit. Invalid policy here would make every UI path misleading. | Use-case behavior can be tested against port fakes before HTTP and UI consumers depend on it. |
| 2 | U2 — Identity and gateway boundary | User JWT/service-token boundary, deny-by-default route completeness, ownership and admin access, rate limits and provider failure mapping. | Routes become callable only when each owning handler is declared; this enforces the team's agreed completion test. |
| 3 | U3 — SPA auth and sessions | Browser token handling, callback/PKCE, MFA and destructive session actions. | Users receive working sign-in, session and recovery journeys over the settled gateway contract. |
| 4 | U4 — Auth integration proof | Cross-boundary failures: route drift, stale/revoked sessions, retries, audit sanitization, provider outage. | Final evidence shows the SPA calls only declared routes and the journeys work end to end. |

## Why not other heuristics?

- **Value-first** could put a visible SPA flow before its gateway handlers and provider semantics exist;
  the approved strict DAG forbids that and the UI would be mocked against unstable contracts.
- **Foundation-first without risk framing** would still put backend first, but would not explicitly test
  the uncertain policy and provider error cases before the edge/UI work consumes them.
- **WSJF** requires estimated size and value/urgency scores; no evidence-supported estimates are
  available. The human selected risk-first then value instead of inventing numeric inputs.
- **Walking-skeleton-first** is not selected as a separate ceremony: the existing Asset Registry
  walking skeleton is already built. U4 is a final integration proof, not an early skeleton marker.

## Risk register

| Risk | Likelihood | Impact | Mitigation / owner | Bolt affected |
|---|---|---|---|---|
| Session absolute-cap anchor (initial sign-in vs last refresh) remains open in requirements Q3 | Medium | High | Security owner resolves in Functional Design before U1 session policy is frozen | U1 |
| Accepted revocation residual window up to one hour weakens BR-AUTH-10's literal next-request wording | High (known behavior) | High | Keep explicit UI notice, test the bound, and require human disposition of the reviewed deviation before implementation | U1, U3, U4 |
| FR4.7 admin list/revoke lacks a story criterion and is a named review finding | Medium | High | Product owner resolves coverage before U2 implementation; U2 route/handler and U4 authorization proof only proceed against an approved criterion | U2, U4 |
| Contract review says recovery continuation and admin-session APIs are under-specified | Medium | High | Close the Contract Design findings before U2 interface implementation; identify authorized recovery approver and admin route payloads | U2 |
| Provider outages interrupt sign-in, refresh or TOTP | Medium | High | Bound retries, show non-revealing 503, let existing users continue only to access-token expiry; test no retry storm | U1–U4 |
| Refresh-token rotation races across tabs | Medium | Medium | Single-flight refresh and retry-with-current-session behavior; test concurrent refresh | U1, U3, U4 |
| Notification service outage after a successful sign-in | Medium | Medium | Idempotent alert call; sign-in remains successful; preserve retry/outbox intent and surface missed delivery | U1, U2, U4 |
| New/unusual-location definition is open (requirements Q5) | Medium | Medium | Decide measurable rule before implementing alert classifier; new-device-only remains the minimum specified trigger | U1, U4 |
| Email verification resend authentication mode conflicts in Contract Design review | Medium | Medium | Choose one auth class and align OpenAPI, gateway route and handler before U2 | U2, U3 |
| Contrast ratio for `--accent-on` on `--accent` is below AA in mockup review | High (measured) | Medium | Design owner selects compliant CTA text/fill treatment from approved token system before U3 UI implementation | U3 |
| External QR generator or provider SDK behavior differs from assumptions | Low | Medium | Confirm pinned Supabase SDK behavior and QR dependency at U2/U3 design; manual key remains fallback | U2, U3 |
| Disposable Postgres gate is not provisioned by a committed script | Medium | High | Operations/quality owner provides the approved local provisioning command before U1/U4 DB tests; no production DB used | U1, U4 |
| Windows path length may block tooling at this workspace location | Medium | Medium | Move workspace to a shorter path or enable Windows long paths before running tools that materialize deep test/build paths | All |

## Explicit non-goals and deferred risk

- OAuth, passkeys and phone OTP remain outside this intent (requirements interview decisions).
- The provider-initiated logout question (OQ-002), unusual-location heuristic, recovery approver,
  TypeScript coverage target and session-retention policy remain owned open questions, not assumptions
  that construction may silently decide.
- The observability stack is a target, not deployed. This plan does not claim metrics or traces are
  collected today.
