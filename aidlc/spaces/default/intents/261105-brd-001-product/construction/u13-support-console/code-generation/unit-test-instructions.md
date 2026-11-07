# Unit Test Instructions — u13-support-console

## Framework

Vitest + React Testing Library + `jest-axe`, jsdom environment.

## Exact command for THIS unit

```
yarn --cwd support-console test
```

Targeted run:

```
yarn --cwd support-console test -- src/components/GrantRequestForm.test.tsx
```

## Coverage and volume

Five to eight tests per component; target 45+ tests. The TypeScript coverage threshold is not set
by the team yet, so no threshold is enforced for this unit.

## Mocking

- `src/api/client.ts` is mocked with `msw` handlers for C2 responses, including 401/403 and RFC 7807
  problems.
- Supabase auth is faked with a memory session (`persistSession: false`).
- Time is faked with Vitest timers for expiry tests.

## Test data

A `support` user, an `admin` user, a `pending`/`active`/`expired` grant, one account
metadata record, and incident fixtures for sev1/sev2/sev3.
