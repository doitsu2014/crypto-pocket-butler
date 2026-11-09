# CI/CD Pipeline — u2-web-app

Inputs: `infrastructure-specification.md`, `monitoring-design.md`, `memory/team.md`.

| Stage | Command / action | Gate |
|---|---|---|
| Install | `yarn install --immutable` | Lockfile honoured |
| Lint | `yarn lint` + `yarn format:check` | Clean |
| Types | `yarn build` (tsc -b) | No type errors |
| Unit/component tests | `yarn test` (Vitest + Testing Library) with axe | Pass |
| Browser matrix | Playwright projects: bundled Chromium, Firefox, WebKit; Chrome and Edge channels; mobile Chrome and mobile Safari; `@axe-core/playwright` scan per project | Pass |
| Lighthouse | Lighthouse CI with the mobile preset, 4× CPU, Fast 3G; publishes to Grafana | NFR2.1 budget |
| Bundle budget | `yarn size` (size-limit: initial chunk set ≤ 300 kB gzip, route chunks ≤ 150 kB) | Budget met |
| Audit | `yarn npm audit --severity high` (Yarn 4 command) | No high |
| Image build | `web` image digest-pinned (`image@sha256` recorded for rollback) | Build |
| Scan | Trivy on the image | No high/critical |
| Headers test | fetches the running container's headers and asserts the full CSP, `Referrer-Policy`, HSTS and each cache rule | Exact policy |
| Deploy | `docker stack deploy` web service with the digest substituted into the stack | nginx healthy |
| Smoke | `/` 200 with the app shell; `/support` served by the U13 bundle; `/v1/healthz` through the gateway and **not** the SPA shell | Green or rollback |

## Deployment strategy

- Immutable hashed assets; `index.html` and `sw.js` `no-store`; rolling `start-first` on the web
  service with `failure_action: rollback`.

## Rollback

1. Redeploy the previous image digest.
2. The service worker version changes with the build, so clients pick up the previous shell on
   their next navigation.
3. Complete when the smoke passes.
