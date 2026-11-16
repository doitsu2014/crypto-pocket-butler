<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-11-14T10:58:30Z [unit u1-identity-auth-domain] — Resolved the session-cap anchor as first sign-in; refreshed tokens cannot extend the absolute 30-day limit, following the existing story AC2.1.3.
- 2026-11-14T10:58:30Z [unit u1-identity-auth-domain] — Reused the existing distinct-administrator support-grant approval for recovery and limited login-alert eligibility to first-seen devices; unusual-location detection remains outside this Unit.

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
