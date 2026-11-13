# Performance Design — u9-support-console-ui

**Upstream:** `nfr-requirements/performance-requirements.md` (NFR2.1–NFR2.11),
`nfr-requirements/security-requirements.md` (NFR10.4 cache scope, NFR24.2 freshness),
`nfr-requirements/tech-stack-decisions.md` (T1 S2 in U3, T2 gating and adoption, T3 mapper),
`functional-spec.md` (§2 journeys, §3.1–§3.4 screen rules, §8 S1–S3, §10 errors, §12 open items),
`frontend-components.md` (§3 client and hooks, §5 presentation, §6 testing surface),
`contract-summary.md` (connect 5 s + read 10 s, RFC 7807 envelope).
**Unit kind:** ui — the client half is designed here. The server hops (the gateway route, U3's
grant read, the Account/Portfolio fan-out) are owned and measured by their units; §8 states which
half of each target this unit can gate.

## 1. Budget enforcement map

| Requirement | Design mechanism | Enforced by |
|---|---|---|
| NFR2.1 request p95 < 2 s, one attempt | The form issues one mutation on submit, disables Submit while it is in flight, and renders from the `201` body; nothing is optimistic and nothing follows the answer automatically (§4) | MSW suite: one request per submit, Submit disabled, success line from the answer |
| NFR2.2 every U9 call carries a 15 s deadline | The shipped `RequestOptions.deadlineMs` on `support-console/src/api/client.ts:165` with its `AbortController` (`:182-186`) and typed `DeadlineError` (`:54`); every U9 client function passes `deadlineMs: 15_000`, as the five U10 calls already do (`client.ts:281-343`) | MSW suite: a delayed handler produces the named-timeout surface, the typed target id and reason kept, and exactly one attempt |
| NFR2.3 `mine` read p95 < 2 s for ≤ 20 rows | One bounded read on mount for a support session and on **Check status**, with explicit `refetchOnWindowFocus: false` / `refetchInterval: false` (§3) | MSW suite: a window `focus` event and a clock tick issue no request; the 20-row bound is U3's server constant |
| NFR2.4 metadata read p95 < 2 s perceived | One read per active grant plus **Refresh**; the panel renders the complete projection or an error, never a partial one (§5) | MSW suite: one request per mount, Refresh issues a second, `503` renders the refreshable error |
| NFR2.5 banner updates with no network | The shipped `useGrantClock` 30-second tick plus its expiry timeout; the banner renders remaining minutes as text and flips to *Expired* locally (§6) | MSW suite: the clock advance changes the copy and makes no request |
| NFR2.6 sign-in out of scope | The shell reuses the console's existing session module; U9 adds no auth call and no refresh loop | Existing session suite |
| NFR2.7 no automatic retry | The console's query client sets `retry: false` for queries and mutations; every U9 hook inherits it and adds none (§3) | MSW suite: a failing handler is called exactly once |
| NFR2.8 `429` rendered from `Retry-After` | Shipped `ApiError.retryAfterSeconds` (`client.ts:41`, parsed in `parseProblem`); U9 renders the retry-after sentence and offers no second request (§7) | MSW suite: `429` + `Retry-After` renders the seconds, handler called once |
| NFR2.9 no polling, no refetch-on-focus | Explicit `refetchOnWindowFocus: false` and `refetchInterval: false` on both U9 queries; `staleTime: 0` for the `mine` read and 10 s for the metadata read (§3) | MSW suite: focus and timer assertions |
| NFR2.10 no projection after access ends | A route-level `403` clears the support query keys **and** `removeQueries` drops the metadata key; the panel is unmounted with the grant (§8 of the security design, §5 here) | MSW suite: after access ends, remounting issues no metadata read and renders no cached rows |
| NFR2.11 filter is client-side; ≤ 20 rows | The filter narrows already-returned rows in component state; no request and no server field (§5) | MSW suite: typing in the filter issues no request |

## 2. The deadline already exists — U9 adds calls, not a timeout mechanism

`client.ts`'s `request()` gained `deadlineMs` with U10 (`support-console/src/api/client.ts:165-199`),
so this unit changes no shared transport behaviour:

- `requestSupportGrant` and the two new functions (`listMyGrants`, `fetchSupportMetadata`) pass
  `deadlineMs: 15_000`, matching the five admin calls. A call without the option behaves exactly as
  today, so no other section's timeout behaviour moves.
- The failure is the shipped `DeadlineError`; the form and the panel render the inline timeout
  error with a manual retry control and keep the typed target id and reason. No spinner outlives
  the deadline.
- **No new transport option is added.** A console-wide default deadline is the console owner's
  decision (tech-stack T7/T8); this unit states its own calls only.

## 3. Query behaviour the requirements depend on (NFR2.3, NFR2.7, NFR2.9)

The console's query client sets only `retry`, `staleTime` and `gcTime`
(`support-console/src/api/queryClient.ts`), so `refetchOnWindowFocus` is TanStack Query's default
`true`. The U9 hooks therefore set their behaviour explicitly:

| Hook | Key | Options |
|---|---|---|
| `useMyGrants(token, role)` | `["support", "grants", "mine"]` | `enabled: token !== null && role === "support"`; `staleTime: 0`; `retry: false`; `refetchOnWindowFocus: false`; `refetchInterval: false`; refetched only by **Check status** (`refetch()`) and by `onCheckStatus` from the form's `409` |
| `useSupportMetadata(grant, token, clock)` | `["support", "metadata", grant.id]` | `enabled` only while the clock-derived status is `active` and a token is present; `staleTime: 10_000`; `retry: false`; `refetchOnWindowFocus: false`; `refetchInterval: false`; refetched by **Refresh** only |
| `useGrantClock(expiresAt)` | — | shipped; 30-second tick plus an expiry timeout; **no request** |
| `useGrantAccess(grant)` | — | shipped; the clock-derived status the banner and the panel share |

- **The `mine` read is gated on the role, not only on the token.** `Role::is_support()` is exactly
  `Support` (`services/identity/domain/src/claims.rs:72-74`), so an admin session that called S2
  would receive a guaranteed `403`; the hook never runs for `role === "admin"` (tech-stack T2).
- **Adoption needs no identifier comparison.** S2 is already caller-scoped — U3's
  `list_by_support_agent` filters `support_agent_id = $1` — so every row it returns belongs to the
  signed-in support agent and the shell adopts the newest one unconditionally, or the grant the
  `201` answered. A client-side match on `requestedBy` would be wrong: that field is Identity's
  internal user id (`services/identity/http/src/dto.rs:786`, column
  `support_agent_id uuid NOT NULL` at `services/identity/migrations/0001_init.up.sql:110`), while
  the console's only identity is the Supabase auth id (`support-console/src/auth/session.ts:63-77`)
  and nothing in the package resolves one to the other, so such a predicate would match no row and
  W4's recovery would be silently dead. Approve/revoke answers are keyed `grantId` and carry no
  `requestedBy` (`services/identity/http/src/dto.rs:714-734`); they are never adoption inputs.
- **No automatic read follows an adoption.** The banner renders from state; the panel enables its
  query when the grant is active, which is the one intended read.

## 4. The request path (NFR2.1, NFR2.7, NFR2.8)

- **One attempt, one request.** Submit is the only trigger; it is disabled while the mutation is
  pending, so a double click cannot double-send. The mutation is `retry: false`; a `409`, `404`,
  `403`, `422`, `429`, `503` or a timeout each renders its own state and **none re-sends**.
- **Render from the answer.** On `201` the form clears both fields, shows the success line, and
  hands the answered grant to `onCreated`; the banner draws from it without a further read.
- **`409` offers a read, not a retry.** **Check your current grant** calls the shell's
  `onCheckStatus`, which refetches the `mine` query — a user-initiated read of a different route,
  not a second create attempt.
- **Ambiguity is reported, never retried.** A `503` (audit not recorded) and a `429` render their
  sentences with the correlation id and a manual control; the typed values stay so the person can
  resubmit deliberately.

## 5. The metadata path (NFR2.4, NFR2.10, NFR2.11)

- **Composition.** Gateway route → Identity support read → U3's 500 ms fan-out to Account and
  Portfolio → the complete projection or `503`. U9 adds no second call and no cache of its own; the
  perceived 2 s holds while U3's fan-out stays inside its 500 ms deadline (their record, their
  metric).
- **No partial render.** `pickSupportMetadata` throws on a malformed projection and the panel
  renders the retryable error, never a projection missing an accounts or portfolios field.
- **No read without a live grant.** The panel is mounted only when the shell has an active grant
  (tech-stack T6), and the query is additionally `enabled` on the clock-derived status, so an
  expired grant performs no read even if the component is somehow still mounted.
- **Access ending clears the data, not just hides it.** A route-level `403` from S3 calls
  `onAccessEnded`, which marks the local grant ended, clears the support query keys (security
  design §2) and drops the metadata key; a remount cannot render the last projection from cache
  (NFR2.10).
- **The filter is component state.** It narrows the already-returned rows case-insensitively on
  label and id, issues no request, and cannot widen the projection (NFR2.11).

## 6. The banner's clock (NFR2.5)

- `useGrantClock` ticks every 30 seconds and sets one timeout for `expiresAt`; the shell passes the
  same `now` to the banner so its copy and the panel's `enabled` decision cannot disagree.
- The remaining-minutes text is a whole number, "less than a minute left" replaces `0`, and an
  unparseable or absent `expiresAt` is treated as expired (fail closed, security design §7).
- **The clock is presentational.** It changes copy and the local status only; the server remains
  authoritative on every read, and the next read after expiry is refused `403` by Identity.

## 7. The `429` surface (NFR2.8)

`parseProblem` already turns `Retry-After` into `ApiError.retryAfterSeconds` (seconds or an
HTTP-date, floored at 0, `undefined` when absent or unparseable). U9 renders it as its own state on
both reads and on the create:

- the sentence names the seconds when known ("Too many requests. Try again in 42 seconds.") and
  falls back to "Too many requests. Try again shortly." without a number;
- the retry is a manual control and **no automatic second request is issued** — a retried `429` is
  the amplification the limit exists to stop;
- it is never rendered as the generic failure, so the person is not invited to resubmit into a
  limit they are already in.

## 8. Chunking, retired surfaces and measurement ownership

- **Chunking.** No lazy boundary is added or removed. The support section is the console's default
  surface; this unit shrinks it by removing `AccountSearch`, `AccountMetadataPanel`,
  `getSupportGrant` and `useSupportGrant`, and by dropping `durationMinutes` and `approverId` from
  the request type. The console has no size gate (`support-console/package.json` defines no `size`
  script), so the intent is measured by `yarn build`'s chunk report at review time; adding a
  console-wide budget is the console owner's decision (performance requirements, "Resource
  constraints").
- **Measurement ownership** (corrected to series that exist):

| Row | Measured by | Owner |
|---|---|---|
| NFR2.1, NFR2.4 | Identity's duration histograms `support_write_duration_seconds` and `support_read_duration_seconds`; neither carries a route label, so they bound the support surface rather than one row | Identity/U3; gateway counters for status classes |
| NFR2.3 (the `mine` read) | **No duration series exists today.** The gateway records status classes only (`services/gateway/http/src/metrics.rs`) and Identity's support histograms are not route-labelled, so this figure is an **acknowledged measurement gap**. If it is to be measured, a route-labelled duration series is a named deliverable for U3 alongside the route; until then it is a design budget, not an instrumented figure | U3 (route owner); U9 for the client behaviour |
| Client-visible behaviours (deadline, one attempt, no focus refetch, clock, `429`, no read without a live grant) | This unit's MSW-backed Vitest suites — request-count and surface assertions, not timing tests | U9 |
| Chunk placement | `yarn build`'s chunk report | Review-time check; no console gate exists |

## 9. Benchmarks

No benchmark suite is introduced. The client behaviours above are request-count and surface
assertions in the MSW suite; the p95 figures are budgets for the owning services' series and for
review, not a harness this unit builds.