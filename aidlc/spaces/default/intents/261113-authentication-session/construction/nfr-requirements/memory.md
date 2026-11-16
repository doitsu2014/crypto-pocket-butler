<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-11-14T11:52:55Z [unit u1-identity-auth-domain] — Kept the existing end-to-end session-list latency targets as U1's target, measured at the Identity API boundary for the 99.9% availability objective; alerts separate provider outage failures from other auth errors.
- 2026-11-14T11:52:55Z [unit u1-identity-auth-domain] — Chose deterministic two-actor concurrency and idempotency tests plus database-backed revocation conflict coverage; no load-test concurrency threshold was established by the requirements.

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
