# Unit Test Instructions — u2-web-app

## Framework

Vitest + Testing Library (jsdom) for components and the API client; axe checks through
`jest-axe`/`@axe-core/react` in jsdom; Playwright and Lighthouse run in the recorded CI pipeline.

## Exact command for THIS unit

```
yarn --cwd web test
yarn --cwd web lint
yarn --cwd web build
yarn --cwd web size
```

## Coverage and volume

Component-level happy path plus two error/edge cases per feature; the TypeScript coverage
threshold is unresolved (team memory), so this unit records coverage without gating it.

## Mocking

The API client is mocked at the module boundary with MSW-style handlers returning RFC 7807
problems for 401/403/409/422/500; Supabase Auth is mocked with a fake in-memory client exposing
`getSession`, `onAuthStateChange` and `signOut`.

## Test data

A signed-in user with one portfolio/10 assets, a stale and an unpriced price, a partial valuation,
a failed brief with `failureReason`, a P&L with `marksUnavailable` and `fxEstimated`, a duplicate
trade (409), a covered-window manual run (409), and a valid + expired WalletConnect session.
