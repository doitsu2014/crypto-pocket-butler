# Delivery Planning — Plan and Questions

Delivery Planning chooses an economic path through the already approved Unit DAG; it does not change
the Unit boundaries or direct dependencies.

## Strategic plan

The four Units are tightly coupled in a strict chain, so the sequence will follow the DAG:

1. **Bolt 1 — Identity auth domain** (`U1`): Identity-owned gates, MFA policy, session/recovery use
   cases, audit and Notification port. Proves the domain/use-case contracts against fakes.
2. **Bolt 2 — Identity and gateway boundary** (`U2`): provider adapters, HTTP handlers, declared
   gateway routes, rate limits and auth contract. Proves user JWT forwarding and the direct internal
   route behavior.
3. **Bolt 3 — SPA auth and sessions** (`U3`): email/password + magic link, gates, MFA/limited session,
   sessions, recovery entry, and in-app alerts. Proves each chosen user journey against the gateway.
4. **Bolt 4 — Auth integration proof** (`U4`): route completeness, browser journeys, cross-boundary
   auth/security tests and alert/recovery delivery failure proof.

This is not a new walking-skeleton ceremony: the Asset Registry slice already exists. U4 demonstrates
that the authentication slice has joined it. The build order is both topological and sequential because
each Unit depends on the previous one; no parallel Bolt grouping is available in the approved DAG.

**Economic heuristic proposed:** risk-first within the DAG, then user value. Early Units settle the
highest security uncertainty (policy and provider boundary), then provide usable flows, with end-to-end
proof last after all producers exist. No numeric WSJF score is proposed because effort and user reach
were not estimated in this stage.

**Execution setting:** checkpoint-enabled, unit-major, serial (the workflow's current setting). Each
Unit's applicable design stages, Code Generation and verification run before the next Unit.

**Verification command proposed:** `cargo test --workspace` plus the appropriate `yarn test` in `web/`
and `support-console/`, and `yarn e2e` in `web/` because the Units touch auth. However, this project
requires the ignored DB-backed suites, coverage and the full ordered gate list. The disposable database
is not provisioned by a committed script, so a single shell command would be misleading; the person
must choose an executable checkpoint command after seeing which levels can run here.

## Strategic questions

Answer each with a letter (A–D), or **X** plus your own words if none fits.

### Q1. [delivery lead] Which sequencing heuristic should Delivery Planning record?

A **Bolt** is one build pass over a piece of the work, ending in something that runs.

- A. Risk-first through the strict DAG, then deliver user value as early as possible (recommended)
- B. Value-first: prioritize the first user-visible sign-in flow even if that means revisiting interfaces
- C. Formal WSJF ranking (value + urgency + risk reduction divided by estimated size); first estimate size per Unit
- D. Foundation-first: complete Identity and gateway before any browser work
- X. Other (please specify)

[Answer]: A — risk-first through the strict DAG, then deliver user value as early as possible. **Mode:** guided

### Q2. [delivery lead] Should one Bolt contain one Unit or several?

- A. One Unit per Bolt; the approved dependency chain already gives four natural, reviewable slices (recommended)
- B. Bundle U1+U2 as one backend Bolt, U3 as the SPA Bolt, U4 as final proof
- C. Bundle all four as one Bolt so the auth slice ships as a single increment
- D. Keep U4 outside the Bolt sequence as a separate QA activity
- X. Other (please specify)

[Answer]: A — one Unit per Bolt; four natural, reviewable slices in the approved dependency chain. **Mode:** guided

### Q3. [delivery lead] What should count as the user-visible result of the last Bolt?

- A. Every in-scope SPA auth path has an owning Identity handler, is declared at the gateway, and the browser journeys pass; explicitly report the unclosed admin-session, contrast and other review findings
- B. The sign-in happy path works; other routes and error cases can follow later
- C. U4 proves tests only; no user-visible route-completeness criterion
- D. Include the deferred OAuth/passkey methods as part of the final proof
- X. Other (please specify)

[Answer]: A — every in-scope SPA auth path has an owning Identity handler, is declared at the gateway, and browser journeys pass; report unresolved findings. **Mode:** guided

### Q4. [delivery lead] Which external dependencies can block this work?

- A. Supabase Auth provider behavior; Notification service availability; disposable PostgreSQL; email delivery; support-approver policy — list each with its owner, what it blocks and a fallback/mitigation (recommended)
- B. Only Supabase Auth; all other dependencies are internal
- C. None; all work is self-contained
- D. The actual external dependency list is unknown; keep the map open
- X. Other (please specify)

[Answer]: A — list Supabase Auth behavior, Notification availability, disposable PostgreSQL, email delivery, and support-approver policy with owner, blocked work, and fallback. **Mode:** guided

### Q5. [security engineer] Which open risks must be resolved before construction starts?

- A. Resolve the session-cap anchor, refresh-versus-public verification behavior, recovery approver, unusual-location rule, and admin-session acceptance criteria before the affected Unit; track contrast and scope questions as explicit risks (recommended)
- B. Start U1 immediately and let all open issues become implementation assumptions
- C. Defer all unresolved items to Functional Design without blocking construction
- D. No open risks block or need tracking
- X. Other (please specify)

[Answer]: A — resolve the session-cap anchor, resend auth mode, recovery approver, unusual-location rule, admin-session criteria and reviewed design findings before affected Units. **Mode:** guided

## Per-Bolt prompts

The four per-Bolt sets will be resolved in `bolt-plan.md` from the approved unit boundaries and story
assignments. Each Bolt will have a Definition of Done, confidence hypothesis, expected demo, and
AI-only ownership (`aidlc-developer-agent`), since Team Formation was skipped by classic scope.
