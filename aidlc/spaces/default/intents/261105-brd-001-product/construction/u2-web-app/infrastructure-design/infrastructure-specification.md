# Infrastructure Specification — u2-web-app

Inputs: `nfr-design/*`, `functional-design/`, `inception/domain-design/components.md` (the SPA
container and edge), `web/` (existing image), C1.

## Deployment

| Facet | Choice | Rationale |
|---|---|---|
| Compute | The existing `web` image (nginx) serves the built SPA; no new service | Static assets behind the CDN/WAF |
| Routing | `/` and app routes → SPA `index.html`; `/v1/*` → gateway; `/support` → the U13 bundle (edge table above) | One origin, gateway same-origin for CSP |
| Build-time config | `VITE_GATEWAY_URL` (default same-origin) baked at build; no runtime secrets | Static bundle |
| Cache | `web/nginx.conf` gains `location /assets/` (`public, max-age=31536000, immutable`) and `location = /index.html`, `= /sw.js`, `= /manifest.webmanifest` (`no-store`), each repeating the security headers | Stale shell never served |
| PWA | `manifest.webmanifest` + root-scoped `sw.js` from the build; bypass `/support`, `/v1`, Supabase, WalletConnect | NFR9.5 |
| Security headers | `web/nginx.conf` gains in this unit's code generation: `default-src 'self'; connect-src 'self' https://*.supabase.co wss://relay.walletconnect.com wss://relay.walletconnect.org https://verify.walletconnect.com https://verify.walletconnect.org; frame-src https://verify.walletconnect.com https://verify.walletconnect.org; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src https://fonts.gstatic.com; script-src 'self'; base-uri 'none'; object-src 'none'; frame-ancestors 'none'; report-to csp`, plus HSTS, nosniff, `Referrer-Policy: strict-origin-when-cross-origin` and a `Reporting-Endpoints` collector. The headers test fetches the running container's headers | NFR-SEC-01, NFR9.5 |
| Environments | Production only | Team decision |

## Infrastructure Services

| Service | Role | Configuration | Notes |
|---|---|---|---|
| CDN/WAF edge | TLS, caching, WAF | Existing edge; `/v1` proxied to the gateway | NFR-SEC-01 |
| `web` image | Static host | nginx (unprivileged), `read_only: true` added to the stack with tmpfs for `/tmp`, `/var/cache/nginx` and `/var/run`; gzip enabled in `web/nginx.conf`; a `stub_status` location plus `nginx-prometheus-exporter` sidecar (or access-log exporter) added in this unit | Existing stack |
| Metrics sidecar | Edge SLI source | `nginx-prometheus-exporter` on `/metrics`, scraped by the observability stack | Edge availability and 5xx alerts |
| U13 support bundle | Separate static bundle | Served at `/support` by the same nginx from a second root | Isolation |
| Gateway | API | `/v1/*` same origin | CSP `connect-src 'self'` |
| Supabase Auth | Auth | C13 direct calls | CSP allows `*.supabase.co` |
| WalletConnect relay | C14 | `wss://relay.walletconnect.com`, `wss://relay.walletconnect.org`, verify origins | CSP allows exactly those |
| Telemetry ingress | `POST /v1/telemetry/client-errors` | Public, rate-limited C1 route (amendment) | Pre-sign-in failures accepted |

## Cost and recovery

- Static hosting on the existing edge; no incremental compute.
- A bad release is rolled back by redeploying the previous image digest; the SPA is stateless.
