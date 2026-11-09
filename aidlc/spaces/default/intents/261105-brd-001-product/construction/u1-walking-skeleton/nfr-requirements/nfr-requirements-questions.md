# NFR Requirements — u1-walking-skeleton

The inception NFRs set the system targets; these questions settle this Unit's share of them.
Answer each with a letter after `[Answer]:`, or pick `X` and say what you want instead.

## Q1. [Architect] What latency budget does asset resolution get?

The dashboard target is p95 <2 s end to end (NFR2).

- A. p95 <50 ms server-side for a single resolution, leaving the rest of the budget to the caller
- B. p95 <200 ms
- C. No separate target; inherit the dashboard target
- X. Other (please specify)

[Answer]: B — p95 <200 ms

## Q2. [Architect] How should asset resolution handle repeated lookups?

- A. In-process cache with a short TTL and explicit invalidation when a registry change lands
- B. No cache; query the schema on every request
- C. An external cache service
- X. Other (please specify)

[Answer]: A — In-process cache

## Q3. [Architect] What should the walking skeleton alert on?

- A. Availability burn below the 99.9% target, error rate above 1%, and a missed heartbeat — all set below the SLO
- B. Only the error rate
- C. No alerts yet; add them later
- X. Other (please specify)

[Answer]: A — Three signals
