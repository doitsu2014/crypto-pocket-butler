# Performance Requirements — u7-settings-ui

**Upstream:** `functional-spec.md` (this unit — §2.1 entry routing, §3 screens, §8 API integration
points D0/P1/P2/D1/D2/D3, X1–X7), `requirements.md` (NFR21–NFR26 and the FRs the screens serve),
`contract-summary.md` (C1/C2 gateway surface; C3/C6/C7 internal edges; timeout and error-envelope
rules), codekb `technology-stack.md` (size-limit budgets, Vitest/MSW tooling, TanStack Query).
**Prefix note:** the latency and screen-response rows inherit the BRD-001 intent's `NFR2` prefix
(screen response p95 < 2 s) and refine it per path for this UI unit; the bundle budgets are the
repository's existing size-limit gates. Sub-numbers are unit-local — the same `NFRx.y` in two
units is two requirements.

## Screen and network budgets

| ID | Path | Target | Basis |
|---|---|---|---|
| NFR2.1 | Settings route chunk (`/settings/*`, the delete page, `/account/restore`, `/account/blocked`) | ≤ 150 kB gzip for the route chunk; the 300 kB initial entry chunk and the 150 kB stylesheet budget are unchanged by this unit | Existing `web/` size-limit budgets (`yarn size`); the unit adds screens and no dataset |
| NFR2.2 | Entry status resolution (D0 `GET /v1/me/account-status`) | p95 < 2 s perceived. Each attempt is bounded by the app's existing per-request deadline of 15 s (`GATE_DEADLINE_MS`, `callWithDeadline` in `web/src/api/client.ts`), derived from the contract's connect 5 s + read 10 s. A `5xx` gets the app-wide single retry (`shouldRetry` in `web/src/api/queryClient.ts`), so the worst case is two attempts, about 30 s plus backoff; a network failure or a deadline abort is **not** retried, and the full-page retry appears at once. On any of those outcomes the full-page retry is guaranteed to appear — the screen never hangs unresolved and never routes into the app on an unknown status. A `401` is a signed-out result, not a retryable error | Inherits BRD-001 NFR2; `client.ts`; `queryClient.ts`; `contract-summary.md` timeout rules |
| NFR2.3 | Settings shell first paint | the labelled `nav` renders before any panel's data, so a slow or failing panel never blanks the navigation; each panel's own read (P1, X2, X3/X4) is p95 < 2 s and bounded by the same 15 s deadline | Spec §2.2 (the shell reads nothing itself) |
| NFR2.4 | Save actions (P2 display name, X2 preferences, X5/X6 consent) | the pending state is visible within 100 ms of activation and the control is disabled for the round trip; p95 < 2 s; one attempt bounded by the 15 s deadline (mutations never retry), after which the inline retryable error appears with the entered value kept | Spec §3.1–§3.3, §5 |
| NFR2.5 | Deletion request (D3) and cancellation (D2) | pending state within 100 ms, single-flight (the control is disabled while the request is in flight), one attempt bounded by the 15 s deadline and never retried; on deadline the surface shows the inline retryable error and re-reads D1 rather than assuming the outcome | Spec §3.5/§3.6 — an irreversible action must never appear to have succeeded when its answer was lost |
| NFR2.6 | Recovery surface (D1 read + D2 restore) | the banner with the exact recovery date renders p95 < 2 s after the route is reached; a `410` swaps it for the terminal state without a second round trip | Spec §3.6; AC4.3.1/AC4.3.2 |

## Retry and load behaviour

| ID | Requirement |
|---|---|
| NFR2.7 | **One client policy, taken from the app as it is built.** The 15 s deadline is **per attempt** (`GATE_DEADLINE_MS` / `callWithDeadline`, `web/src/api/client.ts`). Queries use `createQueryClient`'s existing `retry: shouldRetry` unchanged: **one** retry, and only when the failure is an `ApiError` whose status is `5xx` (`web/src/api/queryClient.ts`) — never on a `4xx`, and never on a `NetworkError` or a `DeadlineAbort`, because a retry there costs another full deadline for a failure the person must see. Worst case for one query is therefore two attempts: about 30 s plus backoff. Mutations use the app's `mutations: retry: false` unchanged: exactly one attempt. No route in this unit sets a different `retry`, and no route sets `retry` higher than the default | `client.ts`; `queryClient.ts`; spec §10 |
| NFR2.8 | D1's `404` is a **normal answer** ("no open request"), not a retryable error: it resolves to `null` and renders the no-request state. Because the client mapping (`nullOnNoRecord` → `isNoRecordProblem`) accepts only the pinned `https://crypto-pocket-butler.example/problems/no-record` problem type, **Identity must emit that `type` for D1's no-record `404`** — the existing `not_found` helper renders `https://errors.cpb/identity/not-found`, which would surface as an error instead of the empty state. That sentinel value is a contract addition recorded in `tech-stack-decisions.md` §3, not an assumption. `410` on D2 is terminal and is never retried. `409` on X5/X6 and D3 is re-read, never retried blindly | `client.ts` (`isNoRecordProblem`, `NO_RECORD_PROBLEM_TYPE`); `services/identity/http/src/problem.rs`; spec §3.5 |
| NFR2.9 | The routing read is always fresh: D0 sets `staleTime: 0` on its own query, explicitly overriding `createQueryClient`'s app-wide `staleTime: 30_000`, and is re-fetched on every authenticated entry (first load and hard navigation) and invalidated after a restore or a deletion mutation. No polling and no background refetch loop on any settings route | `queryClient.ts`; spec §2.1 |

## Resource constraints

- **No new runtime dependency.** MSW 2, Vitest 3, Testing Library, jest-axe and jsdom are already
  present in `web/package.json`; the timezone list comes from `Intl.supportedValuesOf("timeZone")`
  rather than a bundled dataset (`tech-stack-decisions.md` T1/T2).
- **No business or compliance computation in the browser** (project mandate): the SPA renders what
  the service returns. It computes no valuation, drift, allocation or financial figure, and it
  re-derives no preference default and no consent decision (spec §1).
- **No new service.** The endpoints U7 adds are routes on the existing Identity service; the
  performance of their query paths belongs to Identity's own NFR record, not this one.

## Measurement ownership

The p95 < 2 s targets split by ownership. **This unit owns the client side** — issuing the calls
promptly, the 15 s deadlines, the pending/error surfaces, the freshness rules, and no added
latency of its own. It is measured in the MSW-backed Vitest suite, which asserts the deadline
behaviour (a delayed handler must produce the retryable-error surface and must not strand the
screen), the disabled-while-pending rule, and the no-auto-retry rule. **The server hops belong to
Identity and the gateway** (C1/C2 → C3/C6/C7) and are measured by their own service metrics; a
latency regression there shows up in the gateway's metrics, not in this unit's suite.

## Benchmarks

No benchmark suite is introduced. NFR2.1 is guarded mechanically by the existing `yarn size`
size-limit gate; NFR2.7–NFR2.9 by the MSW-backed Vitest suite. NFR2.2–NFR2.6's perceived-latency
figures are guidance checked by a devtools profiling pass during Build and Test review — no
automated check exists for them and none is assumed.