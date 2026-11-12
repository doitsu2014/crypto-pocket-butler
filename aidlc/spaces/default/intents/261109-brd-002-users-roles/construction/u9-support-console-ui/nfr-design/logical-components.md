# Logical Components — u9-support-console-ui

**Upstream:** `functional-spec.md` (§2 shell and gating, §3 screens, §4 W1–W7, §8 S1–S6, §12 open
items), `frontend-components.md` (§1 hierarchy, §2 types and props, §3 client and hooks, §5
presentation, §6 testing surface, §7 files), `nfr-design/performance-design.md`,
`nfr-design/security-design.md`, `contract-summary.md` (C1/C2, C5, C9).
**Unit kind:** ui — the whole inventory lives inside `support-console/`; this unit adds no
deployable, no schema, no container and no service-side component. S2's route is U3's
(tech-stack T1) and appears here only as an external dependency.

## 1. Component inventory

### 1.1 Console (`support-console/src/`)

| Logical component | Kind | Responsibility | Depends on | Failure domain |
|---|---|---|---|---|
| `GrantRequestForm` (`src/components/GrantRequestForm.tsx`, reworked) | form component | Owns `targetUserId`, `reason`, field errors and the create mutation; no duration and no approver input; on `201` clears both fields, shows the success line and calls `onCreated`; `409` offers **Check your current grant** through the shell-supplied `onCheckStatus?: () => void` | `useCreateGrant`, `src/domain/grants.ts` validators | The form panel only |
| `GrantBanner` (`src/components/GrantBanner.tsx`, reworked) | component | Renders the session's grant: status badge, remaining time in text, the pending/active/expired/revoked copy and the metadata-only reminder; receives `now` from the shell and the **session's** grant only | `grantStatusPresentation`, `StatusBadge` | The banner strip; the sections below it still render |
| `SupportMetadataPanel` (`src/components/SupportMetadataPanel.tsx`, **new**) | component | Owns the filter text; renders the complete projection (user, accounts table, portfolios list), the empty states, the read-audit statement and the error/refresh surfaces; mounted only with a live grant (tech-stack T6) | `useSupportMetadata`, `pickSupportMetadata` | The metadata section; the form and banner are independent |
| Shell wiring (`src/screens/SupportShell.tsx`, reworked) | routing and session state | Holds the session's active grant, the shared clock tick, the `mine` read and adoption rule; maps the `search` hash to `metadata`; renders the form only for `support` and the admin link for `admin`; renders the no-grant callout itself | hooks, banner, panel, form | Shell-wide state; a failure is contained to the affected section |
| `useMyGrants` (`src/hooks/useMyGrants.ts`, **new**) | hook (query) | The role-gated own-grant read: `["support", "grants", "mine"]`, `enabled` only for a support session, explicit `refetchOnWindowFocus: false` / `refetchInterval: false`, `retry: false`; refetched by **Check status** only | `client.ts`, query client | One read |
| `useSupportMetadata` (`src/hooks/useSupportMetadata.ts`, **new**) | hook (query) | The projection read: `["support", "metadata", grant.id]`, `enabled` on the clock-derived `active` status, `staleTime: 10_000`, `retry: false`; a `403` calls `onAccessEnded` and clears the support keys | `client.ts`, query client | One read |
| `useCreateGrant` (mutation hook) | hook (mutation) | One attempt, 15 s deadline, no retry; renders the answered grant through the form's callback | `client.ts` | One request |
| `useGrantClock`, `useGrantAccess` (`src/hooks/`, existing) | hooks | The 30-second tick and expiry timeout, and the clock-derived status shared by the shell, banner and panel; no network | — | Presentational only |
| `client.ts` changes (`src/api/client.ts`) | transport | `listMyGrants` and `fetchSupportMetadata` added with `deadlineMs: 15_000`; `pickSupportGrant` accepts `grantId ?? id`; remove `getSupportGrant`, `searchAccounts`, `fetchAccountMetadata` and the `X-Support-Reason` header | gateway | Shared module — additive except the retirements, which no surviving caller uses |
| `queryClient.ts` + `useClearSupportCacheOnErrors.ts` (changed) | cache policy | `queryClient.ts` gains the scoped `clearSupportQueries` and keeps `clearSupportCache`'s full `client.clear()`; the existing `useClearSupportCacheOnErrors` is **unchanged**, so U10's admin queries keep full clear on `401` and `403`; the same hook file gains `useClearSupportQueriesOnRouteErrors`, wired only by the two U9 queries (`401` → full clear, `403` → scoped) (security design §2) | TanStack Query | Shared cache policy — additive: one new function, one new hook, no existing caller moves |
| `types.ts` changes | types | `GrantStatus`, `SupportAccessGrant`, `GrantRequestInput`, `SupportMetadata`; remove `AccountMetadata`, `AccountSearchResult`, `MetadataReadInput`, `AccountSearchInput`, `GrantRequestValues` | — | Compile-time only |
| `src/domain/grants.ts` / `metadata.ts` (changed) | domain | The reason bound becomes the spec's **1–500 characters**, replacing the shipped 20-character minimum `MIN_REASON_LENGTH = 20` (`src/domain/grants.ts:4`), so the console never refuses a reason the server accepts; clock-derived expiry and fail-closed status, the four-status presentation, the metadata whitelist and picker | — | Pure functions |
| MSW handlers + suites | test infra | Handlers for S1/S2/S3 including delayed, `409`, `403`, `429` and `503` variants; `jest-axe` on every rendered state | `msw` (dev-only) | Tests only; never in the production bundle |

**Removed whole:** `AccountSearch.tsx` / `.test.tsx`, `AccountMetadataPanel.tsx` / `.test.tsx`,
`useSupportGrant.ts` / `.test.tsx`, and the `getSupportGrant` / `searchAccounts` /
`fetchAccountMetadata` client functions with their cases. No surviving component imports them
(tech-stack T4, W3, W7).

### 1.2 Server-side touches

**None owned by this unit.** The one console-side documentation edit outside the package is the
comment at `services/gateway/infrastructure/src/upstream.rs:88-90`, which documents the retired
header; U9 edits it, changes no behaviour and adds no gateway test (tech-stack §3).

### 1.3 External dependency (U3's work, consumed here)

| Component | Owner | What U9 assumes |
|---|---|---|
| `GET /v1/support-grants/mine` | U3 | Support-role only; a bare array of C5 grant rows, ≤ 20, newest first; `route_label` arm `support.grants.mine`; no per-route rate budget |

**U9 is blocked on this row.** As the code stands, the path exists in no contract, no gateway
`Route` entry and no Identity handler, and U3's work has already landed; so the console's reload
recovery (W4) and its **Check status** control have no server half and no frozen response shape
until that delivery ships. Code Generation of `useMyGrants` does not start before it (security
design §11 items 1–2).

## 2. Failure domains and blast radius

- **The create fails** (`400`, `403`, `404`, `409`, `422`, `429`, `503`, timeout) → the form's own
  inline state with the correlation id; the banner and the metadata section are untouched.
- **The `mine` read fails** → the banner has no session grant to show and the form stays usable; a
  `403` means the person is not a support agent in the server's view, rendered as the not-allowed
  line. Blast radius: the banner.
- **The metadata read fails** → its own error and **Refresh**; the last projection is not rendered
  from cache after an access-ended answer. Blast radius: the metadata section.
- **Access ends mid-read** → `403` → `onAccessEnded` marks the grant ended, clears the support
  query keys and drops the metadata key; the banner turns *Expired* locally and the panel is
  unmounted. Nothing else clears (an admin's queries in the same session survive).
- **The session's role says support but the server disagrees** → the server answers `403`; the
  screen shows the not-allowed state, never an empty successful surface (NFR24.3).
- **The projection malformed** → `pickSupportMetadata` throws, the panel renders the retryable
  error, and no partial table is drawn.

## 3. Isolation strategy

- The feature lives in `src/components/` (three components), `src/hooks/` (two new hooks) and the
  existing `src/domain/` and `src/api/` modules; the only shell file touched is
  `src/screens/SupportShell.tsx`.
- **Additive where shared.** `client.ts` gains two functions and one mapper change; the shared
  `request()` is unchanged this unit (its `deadlineMs` already exists). `queryClient.ts` gains one
  scoped function and keeps the existing full clear. No other section's request, retry or caching
  behaviour moves.
- **No new context provider, no shared state with another section, no storage API, no telemetry
  call** — the absences are part of the design (security design §2, §5).
- The admin surface is unchanged: U10's `AdminApprovals` keeps the pending list and the queue; U9
  only stops the shell from holding an unused pending list (tech-stack T2, T5).

## 4. Shared resources

| Resource | Shared with | Isolation |
|---|---|---|
| `src/api/client.ts` + `request()` | Every console section | Two additive functions and one mapper rule; existing call sites untouched; `deadlineMs` passed per call |
| Query client and cache | Every console section | U9 keys namespaced under `["support", …]`; the scoped clear matches that prefix; the existing full clear is unchanged |
| `useClearSupportCacheOnErrors` | Every console section | A second, route-level hook beside it (`useClearSupportQueriesOnRouteErrors`); the existing hook's behaviour for `401` **and** `403` is untouched, so no admin caller moves |
| `SupportShell` session state | Every section | Holds the session's own grant only; the admin section's `onGrantChanged` no longer feeds it |
| `grantStatusPresentation` (`src/domain/grants.ts`) | Admin queue, banner, badges | One mapping for four statuses; U9 adds no second presentation table |
| Session module (`src/auth/`) | Shell | Read-only reuse (token, role); U9 adds no token handling |
| Gateway `Route` table | Every route | S1 and S3 unchanged; S2 is a new U3 entry this unit only consumes |

## 5. Data-layer note (NFR23.1)

This unit holds no schema, no Postgres role and no data-access code. Its only new read is S2,
which runs inside Identity under Identity's own role and is U3's method and route (tech-stack T1).
The least-privilege position is unchanged; NFR23.1's N/A row remains accurate for U9.

## 6. Bridge to Infrastructure Design

Nothing in this unit changes infrastructure. The support section ships inside the existing console
build; S1 and S3 ride the existing gateway and Identity services; S2 adds one gateway route entry
and one Identity handler, neither of which changes a replica count, secret, volume or Swarm
service. Two facts the next stage should carry:

- **The console's static edge is the `web` service** (`web/nginx.conf` serves `/support/`), and the
  production `/v1` edge is still undefined in this repository; U9 adds no edge configuration and
  depends on none.
- **S2's missing duration series** (performance design §8) is a monitoring gap, not an
  infrastructure change: if U3 adds a route-labelled histogram it is instrumentation inside the
  existing services, with no new component.