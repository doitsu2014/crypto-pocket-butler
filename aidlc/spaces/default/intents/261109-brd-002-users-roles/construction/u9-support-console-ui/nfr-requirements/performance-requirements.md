# Performance Requirements — u9-support-console-ui

**Upstream:** `functional-spec.md` (§2 journeys, §3.1–§3.3 screen rules, §8 S1–S3, §10 error
handling, §12 open items), `frontend-components.md` (§3 client, hooks and shell wiring; §6 testing
surface), `requirements.md` (NFR24–NFR26), `contract-summary.md` (C1/C2 gateway limits, C5),
codekb `technology-stack.md`.
**Prefix note:** the latency and screen-response rows inherit the BRD-001 intent's `NFR2` prefix
(dashboard response time, p95 < 2 s) and refine it per path for this UI unit; the freshness rows
refine this intent's inception `NFR24`. Sub-numbers are unit-local — the same `NFRx.y` in two
units is two requirements.

## Screen and network budgets

| ID | Path | Target | Basis |
|---|---|---|---|
| NFR2.1 | Request a grant — S1 `POST /v1/support-grants` | p95 < 2 s from submit to the success line or an error surface. One attempt only (every mutation is `retry: false` in the console's query client). Submit is disabled while in flight, so the form cannot double-send | `functional-spec.md` §3.1; `support-console/src/api/queryClient.ts` |
| NFR2.2 | Every U9 call — S1, S2 and S3 | Carries a **15 s deadline** through the console's existing `RequestOptions.deadlineMs` (`AbortController`; `support-console/src/api/client.ts:165-190`). When it fires the section leaves its loading state and renders the inline retryable error that names the timeout with a retry control; the typed target id and reason are kept. No spinner outlives the deadline | `contract-summary.md` connect 5 s + read 10 s; spec §10 |
| NFR2.3 | The session's own grants — S2 `GET /v1/support-grants/mine` | p95 < 2 s for the 20-row bound (W4). Read once on mount for a **support-role** session and again when the person presses **Check status**; it is not a polling read. The banner renders from the newest row | spec §3.2, §4 W4; `frontend-components.md` §3.2 |
| NFR2.4 | Metadata projection — S3 `GET /v1/support-grants/{grantId}/metadata` | p95 < 2 s perceived. The server's fan-out to Account and Portfolio is U3's budget (its own 500 ms deadline, answered `503` rather than a partial projection); this unit adds no second call and renders only the complete answer. One read per active grant, plus **Refresh** | spec §8 (S3), §12 item 1; C5 |
| NFR2.5 | Active-grant banner | No network. The remaining-minutes text updates on the console's existing 30-second clock (`useGrantClock`) and an expiry timeout turns the banner to *Expired* locally before any call; "less than a minute left" replaces `0`. The local clock is presentational, the server remains authoritative on every read | spec §3.2; `frontend-components.md` §3.2 |
| NFR2.6 | Sign-in | Out of scope. U9 adds no sign-in path and no token-refresh loop; the shell reuses the console's existing Supabase Auth session | `support-console/src/auth/session.ts` |

**The no-grant and ended states do no work.** With no active grant, the banner and the metadata
section issue no request (the panel is not mounted, or its query is disabled) — "no call" is the
required behaviour, not an optimisation. After a `403` from S3 the shell clears the support query
cache and issues no further read until a new grant is active (spec §2.3 step 5, §3.3).

## Retry and load behaviour

| ID | Requirement |
|---|---|
| NFR2.7 | **No U9 call retries automatically.** The console's query client already sets `retry: false` for every query and mutation, and this unit keeps it: the request is a one-shot mutation, the `mine` read is refetched only on mount or **Check status**, and the metadata read only on mount, **Refresh**, or a new grant. The only retry is the person pressing a control, which is a new request |
| NFR2.8 | A `429` from any U9 route is rendered as its own state — the not-retried, "too many requests, retry after N seconds" line, N taken from the gateway's `Retry-After` header — and is never retried automatically. The gateway answers `429` as an RFC 7807 body carrying the correlation id |
| NFR2.9 | No background polling and no refetch-on-focus: the `mine` read runs on mount and on **Check status**, never on a timer; the metadata read runs once per active grant and on **Refresh**; the banner's 30-second tick is local rendering, not a request. The metadata query's `staleTime` is 10 s so a remount within that window does not re-read |
| NFR2.10 | When access ends (expiry, revoke, or an S3 `403`), the support query cache is cleared and the projection is not kept or re-rendered from cache: a support session that no longer holds a live grant cannot read the last projection from memory |
| NFR2.11 | The metadata filter is client-side only over the already-returned rows; it performs no request and cannot widen the projection. The `mine` read is bounded by the server to 20 rows, and the banner reads the newest one (W4) |

## Resource constraints

- **No new runtime or dev dependency** (tech-stack T1/T8). The work reuses the console's existing
  React, TanStack Query, Vitest, Testing Library, MSW and `jest-axe`.
- **The console has no size budget** — `support-console/package.json` defines `build`, `test`,
  `lint`, `typecheck`, `format:check`, and no `size-limit` gate (`web/` is the package with
  budgets, 300/150/150 kB gzip). U9 adds no budget of its own; `yarn build`'s chunk report is the
  only measurement, and a console-wide budget is the console owner's decision.
- The retired surfaces shrink the package: `AccountSearch`, `AccountMetadataPanel`,
  `getSupportGrant` and `useSupportGrant` are removed, and the request type loses `durationMinutes`
  and `approverId` (W1, W3, W7) — the net dependency footprint falls.
- No business math in the browser: the console renders the server's status, expiry and projection
  and computes nothing about holdings, balances or valuations (spec §1, §7).

## Measurement ownership

Each figure names the series that settles it, so none is left to "review" alone:

| Row | Measured by | Owner |
|---|---|---|
| NFR2.1, NFR2.4 | Identity's existing duration histograms — `support_write_duration_seconds` for the grant write and `support_read_duration_seconds` for the metadata read — plus the gateway's request counters (status class only). Neither carries a route label today, so they bound the whole support surface rather than one row | Identity/U3 for the write and read histograms; gateway for the counters |
| NFR2.3 (S2) | **No duration series exists today**: the gateway records status classes only (`services/gateway/http/src/metrics.rs`), and Identity's two support histograms have no route label, so a list read would land in neither. **This is a recorded measurement gap**, not an instrumented figure. If the list read is to be measured, a per-route duration histogram (or a label on Identity's support-read histogram) is a named deliverable for U3 alongside the route; otherwise the figure stays an unreviewed design budget | U3 (route owner); this unit owns the client behaviour only |
| Client-visible share | The MSW-backed suite's behaviour assertions — deadline, loading, one request per action, no spinner past the deadline — because a jsdom suite cannot measure a real p95 | U9 |
| NFR2.2 | The console's Vitest suite: a delayed MSW handler must produce the named-timeout surface with the typed values kept, and no spinner past the deadline | U9 |
| NFR2.5 | The suite's 30-second clock and expiry assertions; no network involved | U9 |
| NFR2.7–NFR2.11 | The suite's "handler called exactly once", `429` + `Retry-After`, and "no call without an active grant" assertions | U9 |

**The client-visible p95 is a budget, not a harness.** No real-browser performance suite exists in
this repository and U9 does not add one; NFR2.1–NFR2.4 are satisfied when the server series stay
inside their share (C5's 500 ms fan-out, the gateway's own budget) and the client does exactly one
request per action with no retry, which the suite proves. **The server hops are not this unit's**:
the gateway's route budget, Identity's grant read and the Account/Portfolio fan-out are owned by
the gateway and by U3, and are measured by their own metrics and NFR records. A latency regression
there shows up in their suites and their series, not in this one.

## Benchmarks

No benchmark suite is introduced. NFR2.1–NFR2.5 are guarded by the MSW-backed suites' deadline,
retry and clock assertions plus the server series in the table above; a benchmark harness would be
new infrastructure for a unit whose work is rendering one answered read.