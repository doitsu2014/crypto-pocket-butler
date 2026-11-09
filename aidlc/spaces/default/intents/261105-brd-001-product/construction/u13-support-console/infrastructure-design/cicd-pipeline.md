# CI/CD Pipeline — u13-support-console

Inputs: `infrastructure-specification.md`, `memory/team.md`.

| Stage | Command / action | Gate |
|---|---|---|
| Install | `corepack enable && yarn --cwd support-console install` | Lockfile honoured |
| Lint | `yarn --cwd support-console lint` (ESLint + Prettier) | Clean |
| Typecheck | `yarn --cwd support-console typecheck` (tsc --noEmit) | Clean |
| Unit/component tests | `yarn --cwd support-console test` (Vitest + RTL) | Pass |
| Accessibility | axe check in the component tests plus a manual keyboard and screen-reader sign-off recorded on the release | No serious/critical axe violations and a signed manual pass |
| Build | `yarn --cwd support-console build` | Bundle produced, initial ≤ 200 KB gzip |
| Scan | dependency scan (Yarn audit) | No high/critical |
| Deploy | the bundle is copied into the `web` image; the stack update rolls `/support` | `/support` returns 200 |

## Rollback

- Roll back the `web` image digest; `/support` serves the previous bundle. No migrations.

## Secrets

- None. The console uses only the public Supabase anon key and the user's JWT.
