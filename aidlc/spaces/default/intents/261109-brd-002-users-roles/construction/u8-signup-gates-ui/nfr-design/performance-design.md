# Performance Design — u8-signup-gates-ui

**Upstream:** `nfr-requirements/performance-requirements.md` (NFR2.1–NFR2.7),
`nfr-requirements/security-requirements.md` (NFR24.1 freshness), `functional-spec.md`
(revision 2 — §2.1 entry routing, §2.4 submit order, §2.6 partial failure),
`contract-summary.md` (timeout/retry rules), codekb `technology-stack.md`.
**Unit kind:** ui — this document designs the client side only; the server hops (C8 → C3 → C6)
are designed in U6/U1/U4's own NFR records (measurement-ownership split, performance
requirements).

## 1. Budget enforcement map

| Requirement | Design mechanism | Enforced by |
|---|---|---|
| NFR2.1 route chunk ≤ 150 kB gzip | Lazy gate route (§3) + static country module inside that route chunk (§4) | `web/scripts/check-size.mjs` (every non-initial chunk, mechanical) + the size-limit `route-*.js` glob, which requires the page module to be named `*Page` (§3) |
| NFR2.2 entry p95 < 2 s, never hangs | Parallel entry resolution (§2) behind a 15 s client deadline (§5) | MSW-backed suite (delayed handler → full-page retry surface) |
| NFR2.3 submit p95 < 2 s, `submitting` < 100 ms | Synchronous `isPending` transition (§6), 15 s deadline → `incomplete` surface | MSW-backed suite (delayed handler → inline error, values kept) |
| NFR2.4 combobox < 100 ms/keystroke | Precomputed search keys, memoised filter, no virtualisation (§4) | Guidance — devtools profiling pass in Build and Test |
| NFR2.5 no main-thread task > 200 ms | Component split + memoisation so keystrokes never re-render the form (§7) | Guidance — same profiling pass |
| NFR2.6 retry discipline | Query retry predicate + mutation `retry: 0` (§5) | MSW-backed suite (no auto-resubmit; 4xx never retried) |
| NFR2.7 freshness, no polling | `staleTime: 0`, no `refetchInterval`, no persistence (§8) | MSW-backed suite + code review |

## 2. Entry resolution (D-P1): one hook, parallel reads

`useGateStatus` is the single resolution point the router and the screens consume; it exposes
one `GateStatus` value (`loading | passed | gates-required | jurisdiction-blocked |
owned-elsewhere | error`) per `frontend-components.md`.

- **Design decision.** When G1 (read-only principal) and G2 (gate state) remain two endpoints,
  the hook issues **both queries in parallel** and resolves when both settle; routing never
  waits for the sum of two round trips. NFR2.2's budget ("p95 < 2 s perceived") is written
  against the entry resolution as a whole and holds under parallel issue: total entry latency
  becomes max(G1, G2) + render, not G1 + G2 + render. The cost is one wasted read-only G2 call
  for visitors routed elsewhere (verify-email, MFA, U7 screens); that call is cheap, idempotent
  and side-effect-free, and buying it keeps the first-visit majority (no record → Gate Screen)
  on the shorter path. *This refines NFR2.2's "sequential reads" wording — recorded in the
  stage diary as a tradeoff.*
- **Robust to the fold.** If open item 2 resolves by folding G2 into G1 (or into the existing
  `GET /v1/me/consent/state`), the hook's shape does not change: it consumes one query instead
  of two and still emits the same `GateStatus`. No screen or guard depends on the call count.
- **Both reads are read-only, and "no record" is data, not an error.** Entry never targets a
  provisioning path: the existing `GET /v1/me/consent/state` handler provisions on read and
  422s for a first-time visitor, so it is not usable for entry as it stands — the confirmed
  read-only surface is dependency 1 in `security-design.md` §11. On that surface, `client.ts`
  maps the defined no-record answer (404-equivalent per spec §6 open item 1) to typed sentinels
  (`principal: null`, `gateState: null`) inside the client methods, so the query *resolves*
  with "Gate Screen" semantics (spec §2.1) and the retryer never sees it (NFR2.2: never
  retried; NFR2.6: 404 excluded from retry by predicate anyway). A *rejected* query is the only
  road to `GateStatus.error` — sentinel and failure are different types at the client layer,
  so the hook cannot confuse them.
- **Error path.** Either query failing (after the bounded retry of §5 exhausts, or on deadline)
  yields `GateStatus.error` → the full-page retry surface; the router has no branch that enters
  the app on an unresolved status (spec §2.1 first row).

## 3. Chunking and load path

- The gate screen ships as its own **lazy route** (`React.lazy` + the router's suspense
  boundary), consistent with the shell's existing route-chunk strategy; nothing from the gate
  feature enters the 300 kB initial entry chunk. The page module is named `SignupGatesPage`
  (ending in `Page`) so Vite's `chunkFileNames` convention keeps it inside the size-limit
  `route-*.js` glob; `web/scripts/check-size.mjs` measures every non-initial chunk regardless.
- The generated country module (§4) is imported **only** from inside the lazy route, so Vite
  places it in the same route chunk; the 150 kB gzip route budget is enforced mechanically by
  `check-size.mjs` — **no new size-limit config entry is needed** (NFR2.1).
- Chunk-load failure (offline reload, failed deploy): the router today has **no** route-level
  `errorElement` — the root `ErrorBoundary` would catch the failure and replace the whole app
  with "The app could not start", a shell-wide blast radius for a gate-route problem. This unit
  therefore ships a **route-level error boundary on the gate route** as part of its route-table
  registration: a gate-chunk load failure renders the full-page retry *inside* the gate route
  and leaves the rest of the shell mounted.

## 4. Country module and combobox filtering (NFR2.4)

- **Module shape.** A generated, frozen array of ~250 entries
  `{ code, en, vi, search }`, where `search` is the lowercased, diacritic-folded concatenation
  of both locale names, precomputed at generation time — filtering never allocates or
  normalises strings per keystroke.
- **Filter design.** Substring match over `search` for the active locale + English fallback;
  O(n) over 250 entries is well under 1 ms per keystroke, so no index structure and **no
  virtualisation** (revisit only if the list ever exceeds ~500 rows). The filtered slice is
  `useMemo`'d on (query, locale); the open listbox renders at most a bounded window of matches.
- **Budget evidence.** Two short names × ~250 entries as a JSON/TS module is estimated
  10–20 kB gzip (performance requirements); `yarn size` on the route chunk is the first real
  measurement, in Build and Test.

## 5. Deadlines and retry policy (NFR2.2, NFR2.3, NFR2.6)

One transport-layer design in `web/src/api/client.ts`, shared by the three gate calls:

```ts
// design sketch (≤15 lines) — implementation belongs to Code Generation
const DEADLINE_MS = 15_000; // contract: connect 5s + read 10s
async function callWithDeadline(req: RequestInfo, init?: RequestInit) {
  const ctl = new AbortController();
  const timer = setTimeout(() => ctl.abort(new DeadlineAbort()), DEADLINE_MS);
  try {
    return await fetch(req, { ...init, signal: ctl.signal });
  } finally {
    clearTimeout(timer);
  }
}
// auto-retry only FAST failures; a deadline is terminal for the attempt sequence
const isAutoRetryable = (e: unknown) =>
  !(e instanceof DeadlineAbort) && (isNetworkError(e) || is5xx(e)); // never 400/403/404/422
```

- **Queries (G1/G2):** `retry: (count, e) => count < 2 && isAutoRetryable(e)` — at most 3
  attempts for *fast* failures (connection refused, quick 5xx), with a **custom jittered**
  `retryDelay` (`attempt => Math.min(1000 * 2 ** attempt, 8000) * (0.5 + Math.random() / 2)`):
  TanStack Query v5's default backoff is exponential *without* jitter, and the contract's
  jitter clause does not apply at the browser edge unless the client adds it.
- **A deadline is never auto-retried.** Retrying a 15 s abort under `count < 2` would leave
  entry unresolved for ~45 s plus backoff — a hang in everything but name. The first
  `DeadlineAbort` ends the attempt sequence and surfaces the error state at once. **Stated
  bounds:** a hung gateway reaches the full-page retry surface in **≤ 15 s** (one attempt);
  fast failures exhaust ≤ 3 attempts within a few seconds. Manual retry from the full-page
  surface restarts the same bounded sequence. This is a deliberate refinement of NFR2.6's
  "≤ 3 attempts" ceiling — the contract's retry budget is a maximum, and NFR2.2's
  never-hangs rule sets the actual bound (recorded in the stage diary).
- **Mutation (G3):** `retry: 0` — retries are manual only (spec §2.6). An abort at 15 s leaves
  `submitting`, renders the `incomplete` inline error, keeps field values and the idempotency
  key (§6).
- **No request outlives its deadline:** every gate call goes through `callWithDeadline`, so a
  hung gateway is bounded on the client even if a proxy never answers.

## 6. Submit path timing (NFR2.3)

- Activation flips the mutation to `isPending` **synchronously** in the same React event, so
  Continue disables and the progress announcement renders in the same frame — far inside the
  100 ms visibility target; no debounce, no deferred transition.
- On settle: `200` → invalidate + re-resolve (§8); `403` → blocked notice; `422` → re-fetch G2
  and re-render from its `missing` list; `503` → `nothing-saved` inline error; 5xx/network/
  deadline → `incomplete` inline error. Form state is **never cleared by a failure path**, so a
  retry costs no re-entry (spec §2.6).

## 7. Render performance (NFR2.5)

- `CountrySelect` is `React.memo`-wrapped and owns its own query-string state, so combobox
  keystrokes re-render only the listbox — not the three checkboxes, the summary, or the submit
  control; conversely checkbox toggles never re-filter the list.
- Validation summary is computed only on a submit attempt (four presence checks — trivially
  cheap) and is not re-derived per keystroke; no validation-on-type anywhere on the screen.
- No new context providers inside the route; the idempotency-key holder is a module-scope
  subscription used only by the submit hook, so holding a key never triggers renders.
- Guidance-level targets (keystroke-to-render < 100 ms, no task > 200 ms) are checked by the
  devtools profiling pass in Build and Test — no mechanical gate is claimed (NFR2.4/2.5).

## 8. Cache design (NFR2.7, NFR24.1)

- Gate-state queries: `staleTime: 0`, default `gcTime`, **no** `refetchInterval`, no
  `refetchOnWindowFocus` override beyond the shell default; the query client for this feature
  is never paired with a persister — the cache is memory-only (also a security rule, NFR10.1).
  Query keys are namespaced with the signed-in session's user id (`["signup-gates",
  <sessionUserId>, …]`), so a user switch in the same tab can never read the previous
  session's entries even though the shell's manual sign-out clears only the token
  (`security-design.md` §2).
- Submit success invalidates the gate-state keys; the re-resolve then routes into the app
  (spec §2.4). Every retryable failure and every 422 triggers a G2 re-fetch rather than
  trusting local field state (spec §2.6).
- A cached "passed" status is never an unlock: the route guard re-renders from the fresh
  resolution on every authenticated entry (hard navigation included).

## 9. Performance verification design

| Check | Mechanism | Stage |
|---|---|---|
| Route-chunk budget | `web/scripts/check-size.mjs` (every non-initial chunk) + the size-limit `route-*.js` glob via the `*Page` module name — no config change | Build and Test (mechanical) |
| 15 s deadline surfaces (entry + submit), deadline never auto-retried | MSW `server.delay`/hung handler; assert the error surface within the deadline and a single attempt (request-count assertion) | Build and Test (mechanical) |
| `incomplete` submit surface, values kept | MSW hung/5xx handler on G3 | Build and Test (mechanical) |
| No auto-retry of G3; 4xx never retried | MSW request-count assertions | Build and Test (mechanical) |
| `submitting` visible < 100 ms | Assert disabled + progress state in the same act() as activation | Build and Test (mechanical) |
| Keystroke/task budgets (NFR2.4/2.5) | Devtools profiling pass | Build and Test review (guidance) |
| Server-hop p95 budgets | Gateway/Identity service metrics — **not** this unit's suite | U1/U4/U6 + gateway |
