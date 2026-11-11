# Delivery Planning — Questions

This stage chooses the order in which the work is built. A **Bolt** is one build pass over a piece of the work, ending in something that runs: one or more Units with a Definition of Done, a confidence hypothesis, and an owner. The dependency graph from Units Generation says what *can* follow what; these questions decide what *should*.

Answer each question with a letter after `[Answer]:`, or pick `X` and say what you want instead.

## Q1. [delivery lead] What should we build first?

- A. Walking-skeleton first — complete a thin end-to-end slice (Jurisdiction Gate → User Registry → Sign-up Gates UI) before breadth
- B. Risk-first — the security-critical units (Authorisation, Support Access, Account Lifecycle) before any UI
- C. Value-first — the owner-facing settings path (User Registry, Preferences & Consent, Settings UI) first
- D. WSJF scoring — score every unit on value, urgency, and risk against size, and follow the order the model produces
- X. Other (please specify)

[Answer]: A — Walking-skeleton first.

## Q2. [delivery lead] How big should one Bolt be?

- A. One Unit of Work per Bolt — the smallest reviewable increments
- B. Bundle related Units into three or four Bolts — fewer, larger review points
- C. Thin slices that cut across Units, each delivering a visible capability
- X. Other (please specify)

[Answer]: A — One Unit of Work per Bolt.

## Q3. [delivery lead] Can several Bolts be built at the same time?

The recorded construction execution is serial (one Unit at a time through its stages).

- A. One Bolt at a time — serial, matching the recorded execution
- B. Several Bolts in parallel where the dependency graph allows
- C. Parallel only within the backend, UI after
- X. Other (please specify)

[Answer]: A — One Bolt at a time, serial.

## Q4. [delivery lead] Is anything outside this team going to hold us up?

Select all that apply. The plan will record each blocker, its owner, and which Bolt it delays.

- A. The Supabase Auth Admin API (needed to write `app_metadata.role`)
- B. An IP geolocation provider (needed by the jurisdiction gate)
- C. The Account and Portfolio APIs (needed for the support metadata projection)
- D. The compliance-owned restricted-jurisdiction list
- X. Other (please specify) — or say "nothing external" if everything is inside this team

[Answer]: A, B, C, D — all four external blockers apply.
