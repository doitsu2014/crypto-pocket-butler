<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-11-09T03:20:00Z — Treated the resumed build as one in-flight attempt: checked each plan step against the project, ticked what was already built, and carried on from the first gap instead of rebuilding the SPA.
- 2026-11-09T03:42:16Z — [unit u2-web-app] The build dispatch ended on an upstream API error after the work was already written to disk; the unit was resumed through the pending review request (retry-pending) rather than rebuilt, since every plan step was ticked and the unit's files were present. The developer agent also authored the Step 5 files itself as its final plan step; the conductor verified them against the workspace.

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-11-09T03:20:00Z — PWA config uses `srcDir: "src/pwa"` + `filename: "sw.ts"`; vite-plugin-pwa 2.0 fails the design's nested `pwa/sw.ts` output with `ENOENT dist/pwa/sw.mjs`. The worker still ships at root `dist/sw.js` with scope `/`.
- 2026-11-09T03:20:00Z — No Playwright/Lighthouse job files and no nginx exporter sidecar or `nginx -t` run: no CI platform and no container runtime are configured in this repository, and installing them is not named by the approved plan.
- 2026-11-09T03:20:00Z — Kept `setAccessToken` in `web/src/auth/session.ts` because the pre-existing `web/src/App.tsx` asset-resolver demo still uses it; the routed SPA never calls it.

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-11-09T03:20:00Z — The API client resolves its fetch implementation per request rather than capturing it at construction, so a test-supplied `fetchImpl` is honoured without a production branch.

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-11-09T03:20:00Z — Should the legacy `web/src/App.tsx` asset-resolver surface be retired now that the router mounts the real SPA shell?
- 2026-11-09T03:20:00Z — Do the Playwright browser-matrix and Lighthouse gaps (NFR6.1, NFR2.6) need to close before Build and Test, or wait for the CI platform decision?
