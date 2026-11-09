# Infrastructure Specification — u13-support-console

Inputs: `nfr-design/*`, `functional-design/`, `inception/domain-design/components.md`, and C1/C2.

## Deployment

| Facet | Choice | Rationale |
|---|---|---|
| Compute | Static bundle `support-console/` packaged into the existing `web` image and served at `/support` | No new runtime service |
| Networking | CDN/WAF → gateway → static assets; the console calls only the gateway and Supabase Auth | Same edge |
| Storage | None; no database, no browser persistence | Static UI |
| Environments | Production only | Team decision |
| IaC | Existing `web` service in `infra/swarm-stack.yml`; the bundle is a build artefact | No new service |
| CSP | `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; img-src 'self' data:; font-src https://fonts.gstatic.com; connect-src 'self' https://*.supabase.co; frame-ancestors 'none'; base-uri 'none'; object-src 'none'`, applied by the gateway on `/support`. `connect-src 'self'` requires the API gateway to be same-origin through the CDN host; if deploy configures a different gateway origin, that origin is added to `connect-src`. | XSS and clickjacking defence |
| Service worker | U2's customer worker is root-scoped and would control `/support` and the same-origin `/v1` calls; it must bypass both paths. `/support` is excluded from the customer PWA service-worker scope, served with `Cache-Control: no-store` for HTML and long cache for hashed assets, and the console also unregisters any registration whose scope covers `/support` at startup (`src/pwa/isolate.ts`). | Separates the support and customer privilege levels |
| Access logs | Gateway and nginx access logs record the URL, status, and timing only. They must never record the `X-Support-Reason` request header (the free-text read reason); no `$http_x_support_reason` in the nginx log format. | No console PII in logs |
| Health | The existing `web` `/healthz`; no separate service health | Static asset |
| Scaling | CDN | Static |

## Shared Infrastructure

| Resource | Owner | Consumers | Boundary |
|---|---|---|---|
| CDN/WAF + gateway | u1 | u2, u13 | `/support` path and C2 |
| Support API (U1) | u1 | u13 | Grants, incidents, metadata |
| Supabase Auth | external | u2, u13 | Sign-in only |

## Cost and recovery

- No incremental compute; the bundle shares the existing web image and CDN.
- Recovery is the web deployment rollback; no data to recover.

## Non-resources

- No database, message queue, cache, or server-side session.
