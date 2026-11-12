# Performance Design — u7-settings-ui

**Upstream:** `nfr-requirements/performance-requirements.md` (NFR2.1–NFR2.9),
`nfr-requirements/security-requirements.md` (NFR24.x freshness and the non-denying reads),
`nfr-requirements/tech-stack-decisions.md` (T1–T6), `functional-spec.md` (§2 shell and entry
routing, §3 screens, §8 D0/P1/P2/D1/D2/D3), `contract-summary.md` (timeout rules), codekb
`technology-stack.md`.
**Unit kind:** ui — this document designs the client side. The server hops the settings calls
traverse belong to the gateway and Identity's own NFR records (measurement-ownership split in
`performance-requirements.md`).

## 1. Budget enforcement map

| Requirement | Design mechanism | Enforced by |
|---|---|---|
| NFR2.1 route chunk ≤ 150 kB gzip | Lazy `/settings/*`, `/account/restore`, `/account/blocked` routes; no bundled dataset (T2) | `web/scripts/check-size.mjs` (every non-initial chunk) — the `*Page` module-name convention keeps the page modules in the size-limit route glob |
| NFR2.2 D0 entry p95 < 2 s, never hangs | `useAccountStatus` — one read behind the shared 15 s deadline, single 5xx retry (§2, §5) | MSW-backed suite: delayed handler → full-page retry within the deadline |
| NFR2.3 shell paints before panel data | Shell renders its `nav` from route state only; each panel owns its query (§3) | MSW suite: every panel handler delayed → nav still rendered |
| NFR2.4 saves: pending < 100 ms, one attempt | Synchronous `isPending` transition, `retry: false` (§4) | MSW suite: disabled-while-pending, single request, value kept on failure |
| NFR2.5 D3/D2 single-flight | Same synchronous transition + `isPending` gate on the control (§4) | MSW suite: double activation issues one request |
| NFR2.6 recovery banner p95 < 2 s, `410` terminal | D1 query + `410` state swap without a second round trip (§3) | MSW suite |
| NFR2.7 one retry policy, mutations never retried | The app's `shouldRetry` and `mutations: retry: false`, used unchanged (§5) | MSW suite request-count assertions |
| NFR2.8 D1 `404` is data; `410`/`409` not retried | `nullOnNoRecord` + pinned sentinel; `410`/`409` mapped to states, never errors to retry (§3, §6) | MSW suite |
| NFR2.9 routing read always fresh | `staleTime: 0` on D0, invalidation after restore/deletion (§7) | MSW suite: refetch on entry, invalidation after mutation |

## 2. Entry routing read (D0)

`useAccountStatus()` is the one resolution point the route guard and the blocked/restore screens
consume. It issues `GET /v1/me/account-status` (D0) through the shared client and maps the answer
to the `Destination` union in `frontend-components.md` §2 (`gateStatus.ts`).

- **One read, no fan-out.** Routing needs the status only; the profile, preferences, consent and
  deletion reads belong to the panels behind it and are not on the entry path. Entry latency is
  `max(D0) + render`, so a slow panel cannot delay routing.
- **Fresh, not cached-as-authoritative.** `staleTime: 0` and a refetch on every authenticated
  entry (first load and hard navigation). A cached status never unlocks a route: the guard
  re-renders from the resolution it has, and the server re-decides on every call (NFR24.3).
- **Failure is a destination, not a guess.** A deadline abort, a network failure, or an exhausted
  retry yields the full-page retry surface. The guard has no branch that routes into the app on
  an unresolved status, and no branch that treats `disabled`/`deleted` as anything but the blocked
  screen.
- **D0 is not the denied principal read.** `/v1/me/principal` denies a `pending_deletion` account
  by design (BR1.7), so routing reads D0, which resolves the stored record (NFR24.1). See
  `security-design.md` §3 for the bounds on that read.

## 3. Panel load and independent failure

- The shell (`SettingsPage` layout + `SettingsNav`) reads nothing: it renders the labelled `nav`
  and an `<Outlet/>`, so the navigation is painted before any panel's data and survives a failing
  panel.
- Each panel owns exactly one query — `["me","profile"]` (P1), `["me","preferences"]` (X2),
  `["me","consent"]` (X3/X4), `["me","deletion"]` (D1) — and renders its own three states
  (loading skeleton, content, inline error with retry). No panel awaits another, and no shared
  suspense boundary makes one slow panel blank the page.
- **D1's empty answer is a state, not an error.** The query resolves `null` for the pinned
  no-record `404` and renders the "no open request" panel; only a rejected query renders the
  error state. A `410` on D2 swaps the recovery surface for its terminal view with no second
  round trip.

## 4. Save and destructive-action timing (NFR2.4, NFR2.5)

- Activation flips the mutation to `isPending` synchronously in the same event, so the control
  disables and the progress label renders in the same frame — inside the 100 ms visibility target
  with no debounce and no deferred transition.
- Saves keep the entered value on failure (NFR25.3), so a retry costs no re-entry. The display
  name input keeps its draft while `dirty` and re-renders from the fresh `ProfileView` after a
  successful save (`frontend-components.md` §2).
- **D3 and D2 are single-flight by construction:** the confirm control and the cancel control
  both read `isPending` from their own mutation, and the dialog's confirm button is disabled while
  the request is in flight, so a double activation issues one request. On a deadline the surface
  shows the inline retryable error and re-reads D1 rather than assuming the outcome — an
  irreversible action may never appear to have succeeded on a lost answer.
- `409` on D3 ("already open") is re-read through D1 and rendered as the existing pending state.
  `410` on D2 is terminal. Neither is retried.

## 5. Transport, deadlines and retry (NFR2.2, NFR2.7)

The settings calls reuse the client's existing per-request deadline and the app's query policy —
no bespoke retry layer is added.

```ts
// design sketch (≤15 lines) — implementation belongs to Code Generation
// web/src/api/client.ts: the existing gate transport, reused per call
callWithDeadline((signal) => fetch(url, { signal }), GATE_DEADLINE_MS);

// web/src/api/queryClient.ts: unchanged app policy
retry: shouldRetry            // one retry, ApiError with status >= 500 only
mutations: { retry: false }   // every settings mutation, as today
```

- **The deadline is per attempt**, 15 s (`GATE_DEADLINE_MS`, derived from the contract's connect
  5 s + read 10 s). A `NetworkError` or a `DeadlineAbort` is never retried, because a retry costs
  another full deadline for a failure the person must see.
- **Queries therefore reach their error surface in at most two attempts** (a fast `5xx` and its
  one retry); a hung gateway reaches it at **≤ 15 s**. Mutations reach theirs at ≤ 15 s, one
  attempt. These are the stated worst-case bounds behind NFR2.2/NFR2.4/NFR2.5.
- The contract's service-to-service retry clause is **not** the browser rule and is not cited as
  one; the browser rule is the app's `shouldRetry`.
- Settings queries add no `retryOverride`: they inherit `createQueryClient`'s policy, which keeps
  one policy in one place.

## 6. Error and outcome mapping

| Answer | Surface |
|---|---|
| `400`/`422` on P2 or X2 | Inline field error at the offending field (`errors[]` → field map); the entered value is kept |
| `401` | Signed-out result: the shell's existing unauthorized policy clears the cache and returns to `/`; not a retryable error |
| `403` on any settings read | Blocked state for that route (the server's denial, rendered as the blocked screen) |
| `404` on D1 | No-request state (`null`) — the pinned no-record sentinel only |
| `409` on D3, X5/X6 | Re-read the affected resource and render its current state; never retried blindly |
| `410` on D2 | Terminal "window closed" state; no further action offered |
| `429` | Recoverable inline error with the correlation id; the person retries by hand |
| `5xx`, network, deadline | Inline retryable error (panel) or full-page retry (entry); one auto-retry for queries on `5xx` only |

## 7. Cache design (NFR2.9)

- Keys as T6 names them (`["me","account-status"]`, `["me","profile"]`, `["me","preferences"]`,
  `["me","consent"]`, `["me","deletion"]`), namespaced under `me` so a settings cache entry can
  never collide with another feature's keys.
- `staleTime: 0` on D0 only; the rest keep the app default `staleTime: 30_000`. The cache is
  memory-only — the query client is never paired with a persister (NFR10.1).
- Invalidation: P2 → `["me","profile"]`; X2 → `["me","preferences"]`; X5/X6 → `["me","consent"]`
  and `["me","consent","history"]`; D3/D2 → `["me","deletion"]` **and** `["me","account-status"]`,
  because a deletion changes the routing status.
- No `refetchInterval` and no polling on any settings route.

## 8. Resource constraints

- No new dependency (T1): the panels compose existing primitives; the timezone list is
  `Intl.supportedValuesOf("timeZone")` (T2) and adds no bundle weight beyond the code path.
- The unit adds no business or financial computation to the browser: it renders the server's
  values, its `source` labels and its dates.
- The only new server work is the six Identity route additions (D0/P1/P2/D1/D2 + the two export
  gateway entries), which are single indexed reads and writes on the existing schema; their own
  latency belongs to Identity's NFR record.

## 9. Performance verification design

| Check | Mechanism | Stage |
|---|---|---|
| Route-chunk budget | `web/scripts/check-size.mjs` + size-limit route glob via `*Page` module names | Build and Test (mechanical, `yarn size`) |
| 15 s deadline surfaces; deadline not auto-retried | MSW `server.delay`/hung handler; assert the error surface and the request count | Build and Test (mechanical) |
| Saves: pending < 100 ms, value kept on failure | Assert disabled + pending label in the same `act()`; assert the input value after a `422` | Build and Test (mechanical) |
| D3/D2 single-flight | Double activation → exactly one MSW request | Build and Test (mechanical) |
| D1 `404` → no-request state, `410` terminal | MSW responses with the pinned problem type and `410` | Build and Test (mechanical) |
| Retry discipline (`5xx` once, network/deadline never, mutations never) | MSW request-count assertions across the suite | Build and Test (mechanical) |
| D0 freshness + invalidation | Assert refetch on mount and refetch after the restore/deletion mutations | Build and Test (mechanical) |
| Perceived latency p95 targets | Devtools profiling pass in Build and Test | Build and Test review (guidance) |
| Server-hop p95 budgets | Gateway and Identity metrics — **not** this unit's suite | Gateway / Identity |