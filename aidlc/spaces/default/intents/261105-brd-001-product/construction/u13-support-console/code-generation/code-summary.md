# Code Summary — u13-support-console

## What was built

The support console as a separate React 18 + TypeScript 5 SPA under `support-console/`, served at
`/support` by the existing `web` image. It is frontend-only: every data operation is a C2 call to
U1's support API, which owns the grant/incident store, the C10 audit write, and the C9
notification.

| Layer | Responsibility |
|---|---|
| `src/api` | One C2 client module (bearer token, RFC 7807 mapping, 401/403, grant-scoped search/tickets/metadata guards), the TanStack Query client, and the narrow DTO types |
| `src/auth` | In-memory session and role resolution; Supabase Auth adapter with `persistSession: false` and an in-memory storage adapter |
| `src/domain` | Grant expiry/state and request validation (BR1.2–BR1.3), incident validation/severity and the 15-minute SEV-1 rule (BR3.2), metadata/ticket/grant/incident field whitelists (BR1.4), status presentations |
| `src/hooks` | Grant clock (refresh at `expiresAt` + 30 s poll), grant access, cache clearing on 401/403, grant read |
| `src/pwa` | `isolate.ts` unregisters any service-worker registration whose scope covers `/support` (defence in depth for U2's root-scoped worker) |
| `src/components` | Sign-in/role gate, grant banner, grant request form, approval/revocation queue, grant-scoped search, metadata panel, grant-scoped ticket queue, incident form/list/console, help panel, status footer |
| `src/screens` | `SupportApp` and the sectioned `SupportShell` (hash routes, admin-only incidents, sign-out, expiry cache clearing) |
| Build | `web/Dockerfile` builds both bundles into one image; `web/nginx.conf` serves `/support` with `no-store` HTML, immutable assets, a support-specific CSP/security headers, and a no-request-header access-log rule |

### Behaviour delivered

- **BR1.1 role gate** — only `app_metadata.role` of `support` or `admin` reaches the shell; a
  customer or missing role shows "Not authorised" and loads no data.
- **BR1.2 grant request** — reason ≥ 20 trimmed characters, duration 1–480 whole minutes, and a
  second approver who differs from the requester; the approval queue disables self-approval.
- **BR1.3 expiry and grant scope** — `grantStatusAt` treats an `active` grant past `expiresAt` as
  expired (and fails closed on a missing/invalid expiry). Search, metadata, and the ticket list
  refuse a missing grant id in the client before any request. The shell clears the query cache at
  expiry and disables reads.
- **BR1.4 metadata only** — account fields are whitelisted to `id, kind, label, venue, status,
  lastSyncedAt`; tickets to `id, subject, status, updatedAt, requesterRef` (pseudonymous, never a
  requester label or conversation); grants and incidents use the same narrow pickers. Any extra
  field the API over-sends is stripped before it reaches a component.
- **BR2.1–BR2.2 audit** — no audit write path exists in the SPA; reads are recorded by U1's support
  API. The free-text read reason is sent only in the `X-Support-Reason` request header, never in a
  URL, so it cannot reach access logs, browser history, or referrers; U1 hashes it for C10. A test
  pins the absence of an audit export.
- **BR3.1–BR3.3 incidents** — admin-only console; severity and status are separate fields; a SEV-1
  shows the 15-minute notice rule; the C9 outcome is shown as sent/pending/failed with a detail.
- **BR4.1 help** — the same articles are rendered read-only and the customer help channel is linked.
- **NFR9 / NFR15** — in-memory session, cache clearing on sign-out/expiry/401/403, CSP and
  security headers on `/support`, visible focus ring, keyboard tab order, ARIA live regions, and
  every status as a label plus a shape. Axe runs in every component and screen suite.
- **Service-worker isolation** — `src/pwa/isolate.ts` runs at startup and unregisters any
  registration whose scope covers `/support`, because a root-scoped customer worker would otherwise
  control `/support` and the same-origin `/v1` calls. U2 must also make its worker bypass
  `/support` and `/v1` (recorded as a U2 dependency).

## Review findings addressed (code-generation review-01)

| Finding | Fix |
|---|---|
| R-01 Major: free-text reason in the URL | The metadata read now sends `X-Support-Reason` and only `grantId` in the URL. `client.test.ts` asserts the header is present and the URL contains neither `reason=` nor the reason text; `AccountMetadataPanel.test.tsx` asserts the same at the component boundary. C2, the functional spec, the security design, the monitoring design, the infrastructure spec, and `web/nginx.conf` record the header and the no-header-log rule. |
| R-02 Major: `/support` controlled by the customer worker | Added `src/pwa/isolate.ts` and `isolate.test.ts` (5 tests), called at console startup in `main.tsx`. It unregisters root-scoped and `/support/`-scoped registrations but leaves unrelated scopes alone, surfaces read failures to the caller, and treats a missing Service Worker API as a no-op. The U2 dependency (worker must bypass `/support` and `/v1`) is recorded in the security design, infrastructure spec, README, code summary, and nginx comment. |
| R-03 Minor: weak traceability targets | AC9.3.3/BR2.1/BR2.2/NFR9.6 are `N/A` because U1 owns the audit and the console has no write path; AC8.1.2/AC8.1.3 are marked joint with U1/U2; NFR1.1 targets the monitoring design as a joint infrastructure surface; NFR2.2–NFR2.5 point at the exact components and cache/call behaviour; NFR12.1 points at the lockfile and Dockerfile that the CI scan reads. |
| R-04 Minor: unscoped ticket list with requester label | `listTickets` now requires `grantId`, sends it on C2, and `pickTicketMetadata` whitelists `requesterRef` instead of `requesterLabel`. `TicketQueue` requires a grant, shows a grant-required callout otherwise, and is passed the active grant by the shell. Client and component tests cover the grant scope and the stripped label/conversation. |
| R-05 Minor: CSP gateway origin | `web/nginx.conf`, the infrastructure spec, the security design, and the README state that `connect-src 'self'` requires the gateway to be same-origin, or its configured origin must be added at deploy. |
| R-06 Minor: raw grant/incident responses | Added `pickSupportGrant` and `pickIncident` (with a notification picker) to `src/domain/metadata.ts`; every grant and incident client call now returns a whitelisted object. `metadata.test.ts` covers extra-field stripping and invalid status/severity rejection. |
| R-07 Minor: manual accessibility sign-off | The code summary and NFR15.2 record that the manual keyboard and screen-reader release sign-off is still pending; automated axe and keyboard-order tests pass. |

## Verification output

All commands were run in this worktree.

```
$ yarn --cwd support-console test
 Test Files  29 passed (29)
      Tests  180 passed (180)
```

Test volume by suite:

| Area | Tests |
|---|---|
| components (13 files) | 75 |
| api (client 12, queryClient 3) | 15 |
| auth (session 7, supabase 11) | 18 |
| domain (grants 11, incidents 11, metadata 12, tickets 2) | 36 |
| hooks (clock 3, access 3, cache 4, grant 3) | 13 |
| pwa (isolate) | 5 |
| screens (shell 9, app 6) | 15 |
| styles | 3 |
| **total** | **180** |

```
$ yarn --cwd support-console lint
(no output, exit 0)

$ yarn --cwd support-console typecheck
(no output, exit 0)

$ yarn --cwd support-console build
✓ 142 modules transformed
../web/dist/support/index.html                            1.13 kB │ gzip:  0.53 kB
../web/dist/support/assets/index-CPFOhVKa.css            14.06 kB │ gzip:  3.84 kB
../web/dist/support/assets/IncidentConsole-C4-24XEI.js    6.40 kB │ gzip:  2.25 kB
../web/dist/support/assets/index-C-FtZX5f.js             39.59 kB │ gzip: 11.34 kB
../web/dist/support/assets/vendor-F_sOxvLw.js           184.42 kB │ gzip: 58.38 kB
../web/dist/support/assets/supabase-D5xPhma_.js         228.75 kB │ gzip: 59.50 kB
✓ built in 981ms (exit 0)

$ docker build -f web/Dockerfile -t cpb-web:u13-support-console .
... both yarn installs, web build, support build, runtime export ... (exit 0)

$ docker run ... cpb-web:u13-support-console
/healthz -> "ok"
/support/ -> <title>Crypto Pocket Butler — Support</title>
/support/ headers -> Cache-Control: no-store; X-Frame-Options: DENY; support CSP present
```

Initial shell budget: html 0.53 + css 3.84 + index 11.34 + vendor 58.38 + supabase 59.50 =
**133.6 KB gzipped**, under the 200 KB performance-design budget. The incident console is a lazy
chunk (2.25 KB gzipped) and is not loaded until an admin opens it.

### TDD evidence

The first run before implementation failed at module resolution for all five client-model suites
(`Failed to resolve import "../auth/session"`, no tests collected). Each subsequent layer was
written red first (API client, query/cache hooks, components) and then made green; the count rose
48 → 59 → 75 → 92 → 108 → 128 → 150 → 164 → 168 → 180 as each layer and the review fixes landed.
The final suite is green.

## Deviations and gaps

1. **`requestedBy` is an additive grant field.** The C2 `SupportAccessGrant` schema has no
   requester id, but BR1.2 requires the second approver to differ from the requester. The console
   reads `requestedBy` when present and disables self-approval; the support API remains the
   enforcement point. U1 should add the field to C2 (recorded in the C2 amendment).
2. **No pending-grant list route in C2.** The approval queue receives its grants from the route's
   state (`initialPendingGrants`) or from a grant just requested. An operator who lands directly on
   the console cannot enumerate pending grants. A `GET /v1/support-grants?status=pending_approval`
   route is needed in U1; the console does not invent one.
3. **CSP is enforced by the web nginx as well as the gateway.** The infrastructure design says the
   gateway applies the `/support` CSP; in this repository nginx is the actual server for `/support`.
   `web/nginx.conf` enforces the support CSP and security headers, and the gateway should keep its
   own copy when configured. `connect-src 'self'` requires the gateway to be same-origin, or its
   configured origin must be added.
4. **Google Fonts are allowed in the support CSP.** The infrastructure design's earlier CSP omitted
   the design system's font origins. The Parchment Ledger system loads Fraunces, Instrument Sans,
   and JetBrains Mono from Google Fonts, matching the customer SPA; `style-src` and `font-src`
   allow exactly those origins.
5. **Search/metadata use TanStack Query rather than mutations.** Search caches for 30 s and
   metadata for 10 s per grant/account/reason key. Re-submitting the same value inside that window
   reuses the cached result instead of making a second C2 read. The cache is cleared on sign-out,
   expiry/revocation, and 401/403.
6. **The account and ticket `status` values are provider-defined strings.** The console renders
   them with a neutral label+shape badge instead of inventing a state machine; the C2 contract does
   not close the set.
7. **MFA is not enforced in this SPA.** BRD-012 NFR-SEC-11 requires MFA for users with exchange
   credentials; that is an Identity/Auth (C15) control, not a console screen.
8. **Locale formatting.** `formatDateTime` uses the browser locale; tests assert the
   "access ends/ended at" structure, not a locale-specific string.
9. **The manual keyboard and screen-reader release sign-off is still pending.** Automated axe
   checks and keyboard-order tests pass; the security design's manual pass is a recorded release
   step that has not been run.
10. **The TypeScript coverage threshold is unresolved by the team**, so none is enforced. The test
    strategy floor is met (five to eight tests per component, 180 in total) and every suite covers
    a happy path plus error/edge cases.
11. **The C2 backend is not implemented here.** Grant/incident storage, the C10 audit event, the
    C9 notification, and the public status read remain U1's support API (and U9 for notification).
12. **The support build writes outside its package root.** `support-console/vite.config.ts` sets
    `outDir: "../web/dist/support"`, so `yarn --cwd support-console build` requires `web/` to exist
    (it does in this repo) and the web build must run first. The Dockerfile encodes that order.
13. **U2 owns the customer service worker.** U2's worker must bypass `/support` and `/v1`; the
    console's `src/pwa/isolate.ts` unregisters any registration whose scope covers `/support` at
    startup as defence in depth, but it cannot change U2's worker for routes it never sees. This is
    a U2 dependency, recorded in the security design, infrastructure spec, README, and nginx
    comment.
14. **The ticket list has no list-all route for the console.** It is grant-scoped and requires the
    active grant id, supplied by the shell. U1 owns the ticket store and the grant binding.

## Unexecuted tests

- No automated suite was skipped; every test file listed in `source-manifest.json` ran in this
  worktree (29 files, 180 tests).
- The C2 API was tested against msw handlers, not a live U1 support API; the U1 backend is not in
  this unit.
- U2's worker bypass of `/support` and `/v1` cannot be exercised here because the worker belongs to
  U2; the console-side isolation is tested.
- The manual keyboard and screen-reader release sign-off has not been run; axe and keyboard-order
  tests passed.

## Build and Test loop-back repair (2026-11-09)

One failing test was repaired in `support-console/src/domain/grants.test.ts`: the expired-grant
assertion used `expiredGrant()`, whose fixture builds `expiresAt` from the real `Date.now()`, while
the test's clock is fixed at `2026-11-08T10:00:00Z`; the grant therefore still read as active. The
assertion now supplies an explicit `expiresAt` of `2026-11-08T09:59:00.000Z` relative to that
fixed clock. The fixture and every other assertion are unchanged.

Result: 180/180 tests pass, lint is clean, and the build succeeds.
