# Units Generation — Plan and Questions

**Design boundaries accepted:** Identity owns app-facing auth policy and related lifecycle; Supabase Auth
remains the external identity authority; the API Gateway owns public routing/validation; the SPA owns
presentation. Units must not imply a new service split or contradict these boundaries.

**Story plan:** group implementation by end-to-end user outcome and existing ownership, not by
technical layer. Each Unit is independently buildable/testable, carries its requirements/stories, and
names direct dependencies only. The dependency DAG is topology only; Delivery Planning chooses the
implementation/economic sequence.

**Plan choices visible in this draft:**
- U1 — Identity domain/auth policy: TOTP policy and limited-session decisions, session and recovery
  use cases, gate policy, and the Notification port from Identity.
- U2 — Identity HTTP/infrastructure and gateway contracts: trusted-side routes, provider/admin
  adapters, route-table entries, rate budgets, and ownership checks.
- U3 — SPA auth/session screens: sign-in and magic link, gates, MFA, sessions, recovery entry, alerts,
  outage/error copy, and route guards.
- U4 — End-to-end closure: in-app alert delivery and the cross-boundary acceptance/E2E proof that
  every SPA-called route is declared and has an owning handler.

Proposed DAG (not an execution order): U2 depends on U1; U3 depends on U2; U4 depends on U1, U2, and
U3. `U4` is verification/integration work, not a fourth deployable service. The existing Identity,
gateway, and `web` deployables remain unchanged.

Answer each with a letter (A–D), or **X** plus your own words if none fits.

## Q1. [architect] Is this boundary strategy right for the four Units?

- A. Keep the four proposed outcomes: Identity domain, Identity/gateway boundary, SPA surfaces, then integrated proof (recommended)
- B. Three Units: combine Identity domain and HTTP/infrastructure, then SPA, then integrated proof
- C. Two Units: all backend (Identity + gateway) and all SPA, no separate integration Unit
- D. One Unit for the complete authentication vertical slice
- X. Other (please specify)

[Answer]:

## Q2. [architect] Is the proposed granularity right?

- A. Four Units as proposed: each is a bounded, reviewable slice (recommended)
- B. Finer — split the SPA into sign-in/gates, sessions/MFA, and recovery/alerts
- C. Coarser — two Units, backend and SPA
- D. Let implementation decide later; keep one broad Unit now
- X. Other (please specify)

[Answer]:

## Q3. [delivery lead] Should any of these Units be independent of the others?

A Unit can run independently only if its code and tests pass with its own changes, its declared
prerequisites, and the existing codebase. No dependency between two Units means they can be developed
in parallel in separate copies.

- A. Keep the strict chain U1 → U2 → U3 → U4 (recommended)
- B. U1 and U3 can start in parallel against agreed contracts; U2 integrates them
- C. U1 and U2 can be built together; U3 waits for both; U4 last
- D. No parallelism; all Units depend on the previous one
- X. Other (please specify)

[Answer]:

## Q4. [architect] Should U4 remain a verification Unit, or belong inside U3?

- A. Keep U4 as a separate verification/integration Unit so the route-table invariant and E2E proof have a named owner
- B. Merge it into U3; the SPA Unit owns the end-to-end proof
- C. Merge it into U2; the backend/gateway Unit owns the route completeness proof
- D. Do not create an integration Unit; make verification a cross-cutting condition of every Unit
- X. Other (please specify)

[Answer]:

## Q5. [architect] Is there a walking-skeleton Unit here?

The existing Asset Registry walking skeleton is already built. This intent closes an auth/session slice
through the existing gateway and Identity; the team practice says the slice is complete when its SPA
calls are declared and served.

- A. No separate walking-skeleton Unit; this is feature work that grows the existing slice (recommended)
- B. Make U1 the new auth walking skeleton and prove sign-in → gateway → Identity end to end before the remaining features
- C. Make U4 the walking-skeleton proof after all feature Units
- D. Treat the existing Asset Registry slice as sufficient and do no auth-specific integrated proof
- X. Other (please specify)

[Answer]:
