# Frontend testing

Two packages, `web/` and `support-console/`, with their own lockfiles and their own Vitest setup.
Both use the same stack: **Vitest 3**, Testing Library (`@testing-library/react` 16, `user-event`,
`jest-dom`), **jest-axe** for automated accessibility, and **MSW 2** for gateway responses. There is
no browser engine in the loop — no Playwright, no Chromium — so nothing here proves real rendering,
only behaviour under jsdom.

## Commands

```bash
yarn --cwd web test          # vitest run
yarn --cwd web lint          # ESLint 9 flat config
yarn --cwd web build         # tsc -b && vite build (this is the type check)
yarn --cwd web format:check  # Prettier
yarn --cwd web size          # size-limit + scripts/check-size.mjs
# same four for support-console; no size budget there
```

`test:watch` exists but a gate reads `yarn test`. Use `yarn` only — `npm`, `npx` and `pnpm` are
forbidden.

## How the tests are built here

- Colocated `*.test.ts(x)` next to the thing under test.
- Shared MSW handlers live under `src/test/` (`support-console/src/test/supportHandlers.ts` with its
  `fixtures.ts`). A handler answers only routes the app actually calls, and the options interface
  lets a test choose the response shape — including the failure shape — per case. Nothing under
  `src/test/` is imported by the app, so none of it reaches the production bundle.
- Accessibility: call `expectNoAxeViolations(container)` (the helper at `web/src/test/axe.ts`)
  on every rendered surface; a violation fails with the rule id and the offending node.
- Assert through **roles, accessible names, labels and visible text**. A query that reaches for a
  class name, a test id used as a shortcut, or an internal state variable is testing the
  implementation, not the requirement.

## What a good assertion looks like in this product

- A missing financial value renders an **em dash**, never `0` or `$0.00`.
- A status is never carried by colour alone: assert the label and the shape too, because that is
  the rule the design system imposes (`design-system/DESIGN.md`).
- Financial figures use the tabular mono style — assert the element carries it, not the exact class
  string if a token is involved.
- A refusal renders the RFC 7807 `title`/`detail` the user is meant to read, with a correlation id
  available, not a raw status code.
- A mutation shows its pending, success and failure states; a test that only covers success misses
  the state users complain about.

## MSW and the console's failure paths

The console's interesting tests are refusal paths: a grant that is already revoking, a metadata read
that returns `no-record`, a cached read invalidated after an error. Model each as an MSW handler
variant, then assert what the user sees and that the query cache was cleared or refetched — the
`useClearSupportCacheOnErrors` hook exists for exactly this and has its own test.

## What to report when a frontend gate fails

The failing test name, the assertion that failed, and the diff in the rendered output. If the
failure is pre-existing formatting drift under the locked Prettier version, say so and leave the
package-wide reformat out of an unrelated change — a sweeping reformat hides the real diff.