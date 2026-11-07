<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-11-07T08:12:00Z — u4-account: the stage's AWS-oriented guidance is applied to the project's actual platform (Docker Swarm + Supabase), not to AWS services; the platform choice is fixed by team memory, so service selections map to Swarm services, Supabase Postgres/Vault, and the self-hosted observability stack.

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-11-07T08:12:00Z — u4-account: sync and CSV import were pulled into a separate `account-worker` Swarm service instead of running inside the API. It costs a second image and a stop-first update path, but it keeps the API stateless and makes the single-writer run lease explicit.

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
