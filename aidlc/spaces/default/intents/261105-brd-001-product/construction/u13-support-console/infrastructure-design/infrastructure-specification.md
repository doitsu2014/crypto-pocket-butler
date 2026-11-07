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
| CSP | `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src <gateway> <supabase-auth>; frame-ancestors 'none'; base-uri 'none'; object-src 'none'`, applied by the gateway on `/support` | XSS and clickjacking defence |
| Service worker | `/support` is excluded from the customer PWA service-worker scope and served with `Cache-Control: no-store` for HTML, long cache for hashed assets | Separates the support and customer privilege levels |
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
