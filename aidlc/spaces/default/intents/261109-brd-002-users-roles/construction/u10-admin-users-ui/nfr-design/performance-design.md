# Performance Design — u10-admin-users-ui

**Upstream:** `nfr-requirements/performance-requirements.md` (NFR2.1–NFR2.10),
`nfr-requirements/security-requirements.md` (NFR24.2 freshness), `nfr-requirements/tech-stack-decisions.md`
(T5 deadline, T4 list read), `functional-spec.md` (§2.2 screen states, §3.1–§3.5, §10 error handling),
`frontend-components.md` (§1 lazy section, §3 client and hooks), `contract-summary.md` (timeout rules,
RFC 7807 envelope).
**Unit kind:** ui — the client half is designed here. The server hops (gateway routing, Identity's
provider read, the grant-list query) are owned by their units and measured there; §9 states which
half of each target this unit can actually gate.

## 1. Budget enforcement map

| Requirement | Design mechanism | Enforced by |
|---|---|---|
| NFR2.1 lookup p95 < 2 s, no partial record | One call, one render point: the record renders only after the whole answer arrives (§5) | Server metrics (gateway + Identity); the client suite asserts the states, not the p95 |
| NFR2.2 every U10 call carries a 15 s deadline | `deadlineMs` on the shared `request()` with an `AbortController` and a typed `DeadlineError` (§2) | MSW suite: a delayed handler produces the named timeout surface and one attempt |
| NFR2.3 role change p95 < 2 s, one attempt | Render from the mutation's own answer, then a non-blocking re-read (§4) | MSW suite: transition and "audited" render without waiting for the re-read |
| NFR2.4 disable/restore, nothing before Confirm | Same render-then-re-read path; the confirm step gates the call itself (§4) | MSW suite: zero requests until Confirm; timeout/busy handling |
| NFR2.5 approvals list p95 < 2 s, ≤ 100 rows | One bounded read, `staleTime: 0`, refetch only after approve/revoke (§3) | MSW suite: refetch on approve/revoke, error state never an empty list |
| NFR2.6 approve/revoke unchanged | The existing `GrantApprovalQueue` and its calls, unchanged (§7) | The queue's existing suite |
| NFR2.7 no automatic retry | The console's query client already sets `retry: false` for queries and mutations (`queryClient.ts`); the U10 hooks inherit it and add none (§3) | MSW suite: a failing handler is called exactly once |
| NFR2.8 `429` rendered from `Retry-After` | `ApiError` gains `retryAfterSeconds`, parsed from the response header (§6) | MSW suite: a 429 with `Retry-After` renders the retry-after sentence, no second request |
| NFR2.9 no polling, no refetch-on-focus | **`refetchOnWindowFocus: false` and `refetchInterval: false` set explicitly on both U10 queries** (§3) — the console's default is TanStack Query's `true` | MSW suite: a window `focus` event issues no second request |
| NFR2.10 no cached record after the section is left | `queryClient.removeQueries({ queryKey: ["admin"] })` in the section's unmount effect (§3) | MSW suite: leaving and returning issues a fresh lookup, never a cache hit |

## 2. The client deadline (NFR2.2, T5)

`support-console/src/api/client.ts`'s `request()` passes straight through to `fetch` today — no
timeout, no abort. The change is additive and local to that module:

There is no helper to extend: `request()` performs the `fetchImpl(...)` call inline, so the change
restructures that one function (the sketch shows the shape, not a replacement file):

```ts
// design sketch (≤15 lines) — implementation belongs to Code Generation
export class DeadlineError extends Error { readonly name = "DeadlineError"; }

async function request<T>(path: string, options: RequestOptions & { deadlineMs?: number }): Promise<T> {
  const controller = options.deadlineMs ? new AbortController() : undefined;
  const timer = controller
    ? setTimeout(() => controller.abort(), options.deadlineMs)
    : undefined;
  try {
    // the existing body of request(), with its fetchImpl(...) call gaining
    // `signal: controller?.signal` and its catch checking the abort first
  } finally {
    if (timer !== undefined) clearTimeout(timer);
  }
}
```

- **Scope.** Only the five U10 client functions pass `deadlineMs: 15_000`; every other console call
  passes nothing and behaves exactly as it does today. Widening the deadline console-wide is the
  console owner's decision, not this unit's (tech-stack T5).
- **Typed failure.** In `request()`'s existing `catch`, `controller?.signal.aborted === true` becomes
  `DeadlineError`; every other transport failure stays `NetworkError`. The section renders the
  timeout as the retryable inline error named in NFR2.2 and keeps the typed email, the chosen role
  and the typed reason; no spinner outlives the deadline.
- **Cleanup.** The timer is cleared on every exit (success, HTTP error, abort), so a settled request
  never aborts a later one.

## 3. Query behaviour the requirement depends on (NFR2.7–NFR2.10)

The console's shared query client sets only `retry`, `staleTime` and `gcTime`
(`support-console/src/api/queryClient.ts`). `refetchOnWindowFocus` is therefore **TanStack Query's
default `true`** — a behaviour no existing suite covers. The U10 hooks must turn it off explicitly;
the requirement's promise is carried by these exact options, not by a comment:

| Hook | Key | Options set by the hook |
|---|---|---|
| `useAdminLookup` | `["admin", "user", email]` | `enabled` only once a submit sets the email; `staleTime: 0`; `retry: false`; `refetchOnWindowFocus: false`; `refetchInterval: false` — the query's only automatic read is the one its `enabled` transition starts |
| `usePendingGrants` | `["admin", "pending-grants"]` | `staleTime: 0`; `retry: false`; `refetchOnWindowFocus: false`; `refetchInterval: false`; refetched explicitly after an approve or revoke (`invalidateQueries`) |

- **The section's unmount effect** calls `queryClient.removeQueries({ queryKey: ["admin"] })`, which
  drops both keys in one call (the prefix matches) and is idempotent under StrictMode's double
  effect invocation.
- **One intended automatic read remains**: the approvals panel's read when the section mounts
  (`refetchOnMount` default) — that is the panel's job, not an unattended re-read of an open
  account. Nothing else refetches.
- **The lookup is not a retryable failure surface.** `404` is data (the "no user has that address"
  state), so the query resolves and the retryer never sees it; a `403`, a `5xx` or a timeout is the
  inline error with its own retry control, which is a new user-initiated request.

## 4. Mutation paths: one attempt, render-then-re-read (NFR2.3, NFR2.4, NFR24.2)

- **Nothing is optimistic.** No mutation seeds or writes the record; every visible value is a
  server answer (security design §7). The role change renders "admin → support" and
  "This change is audited." from the mutation's own `200` body, and the status change renders the
  answered `status` and the same line — the completion never waits for the follow-up read.
- **The re-read is fired after the answer is rendered**, not before: `onSuccess` first publishes
  the completion, then calls `refetch()` on the lookup query. Its failure renders the inline error
  *in addition to* the confirmed change (a second, non-destructive notice) and never erases what
  the server already answered.
- **Ambiguous answers re-read before reporting.** `503` (audit not recorded, or an uncertain
  provider write) and `409` (the edge moved under the read) trigger the same `refetch()` first;
  the panel then reports the server's state plus the correlation id. There is no second attempt of
  the *change* — a duplicate privileged change is the worse outcome (§6 of the security design).
- **`404` clears the record** and returns the section to the lookup with the target-gone sentence
  (spec §2.2, §10).
- **The confirmation step is part of the timing budget**: the status call is issued only from
  Confirm, so no time is spent on a request the admin has not yet confirmed.

## 5. The lookup path (NFR2.1)

- **Composition.** Gateway route → Identity admin lookup → `UserRepository::reveal_auth_subject` →
  the provider metadata read (Identity's existing `SupabaseRoleWriter` client: connect 1 s, total
  3 s, one attempt — `services/identity/infrastructure/src/authorisation/role_writer.rs`, Identity's
  file, not a console client) → answer. The 2 s p95 target holds while the provider's share stays
  ≤ 1 s; the remaining budget covers gateway, Identity and Postgres.
- **No partial render.** The record view appears only when the whole answer is in hand; a provider
  failure is `503` and the section shows "this user cannot be looked up right now" with a retry —
  never a record with a blank role and never a stored value (security design §7).
- **The lookup is audited server-side**, so the console does not retry it automatically and does
  not repeat it on focus or on a timer (NFR2.7, NFR2.9).

## 6. The `429` surface (NFR2.8)

The gateway answers `429` as an RFC 7807 body with a `Retry-After` header (`middleware.rs`,
`rate_limit.rs`). The client parses the header once, in `parseProblem`:

- integer seconds → `retryAfterSeconds`; an HTTP-date → `ceil((date - now) / 1000)`, floored at 0;
  a missing or unparseable value → `undefined` and the sentence falls back to "Too many requests.
  Try again shortly." without a number.
- The surface is its own state (not the generic error): the sentence names the seconds, offers no
  retry navigation of its own beyond a manual button, and **issues no automatic second request** —
  a 429 retried in code is the amplification the limit exists to stop.
- The same shape serves both reads and the three mutations.

## 7. Chunking and the shared queue (NFR2.5, NFR2.6)

- **`AdminUsersSection` is a lazy section**, loaded exactly as `IncidentConsole` is today
  (`React.lazy` + the shell's `Suspense` boundary; `frontend-components.md` §1). The console has no
  size budget (`support-console/package.json` defines no `size` script), so this unit states the
  intent — the section must not enter the console's initial chunk — and measures it with the chunk
  report `yarn build` already prints. Adding a console-wide size gate is the console owner's
  decision (performance requirements, "Resource constraints").
- The approvals panel reuses `GrantApprovalQueue` unchanged, including its own error/retry
  behaviour. U10 adds no budget and no retry to approve/revoke (NFR2.6); the only change is that
  its `grants` prop now comes from the pending-grant read on mount and after each action.

## 8. Measurement ownership (corrected)

| Target | Measured by | Notes |
|---|---|---|
| 15 s deadline, no auto-retry, render-then-re-read, 429, no focus refetch, no cached record after leaving | This unit's MSW-backed Vitest suites | Each is a request-count or surface assertion, not a timing test |
| Lookup p95 < 2 s (NFR2.1) | Gateway and Identity service metrics (`/metrics`) | The provider hop's 3 s ceiling is Identity's; a regression shows there |
| Approve/revoke budgets (NFR2.6) | The console's existing queue suite; gateway metrics | Unchanged by this unit |
| Pending-grant list p95 < 2 s for ≤ 100 rows (NFR2.5) | **No automated gate.** The query is this unit's new read (`SupportGrantRepository::list_by_status`, tech-stack T4), executed by Identity, so the only observation is Identity's HTTP latency metric on `GET /v1/support-grants` once deployed | Stated as an unmeasured target rather than claimed. The status-filter index is a **required** dependency on U3 (security design §12), not a conditional one |
| Chunk placement of `AdminUsersSection` | `yarn build`'s chunk report | Review-time check; no console size gate exists to enforce it |

## 9. Benchmarks

No benchmark suite is introduced. The client-side behaviours above are request-count and
surface assertions in the MSW suite; the perceived-latency figures are targets for service
metrics and review, not a harness this unit builds.