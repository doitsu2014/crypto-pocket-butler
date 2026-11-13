# Performance Requirements — u10-admin-users-ui

**Upstream:** `functional-spec.md` (§2 screen states, §3.1–§3.5, §8 A1′/A2/A3/A4/A5′/D1/D2, §10),
`frontend-components.md` (§3 data flow and hooks, §6 testing surface), `requirements.md`
(NFR24–NFR26), `contract-summary.md` (contract timeouts, RFC 7807 envelope), codekb
`technology-stack.md` (console tooling and versions).
**Prefix note:** the latency and screen-response rows inherit the BRD-001 intent's `NFR2`
prefix (dashboard response time, p95 < 2 s) and refine it per path for this UI unit; the
freshness rows refine this intent's inception `NFR24`. Sub-numbers are unit-local — the same
`NFRx.y` in two units is two requirements.

## Screen and network budgets

| ID | Path | Target | Basis |
|---|---|---|---|
| NFR2.1 | Admin lookup — A1′ `GET /v1/admin/users?email=` (gateway → Identity → provider metadata read → answer) | p95 < 2 s perceived, from submit to the record or the `404` line. Composition: the provider hop's own ceiling is 3 s (connect 1 s, total 3 s, one attempt — the console's existing provider client, `role_writer.rs`), so the target holds while the provider's p95 share is ≤ 1 s and gateway + Identity + Postgres stay inside the remaining ~1 s. The record renders only after the whole answer arrives — no partial record | Inherits BRD-001 NFR2; `functional-spec.md` §3.1/§3.2 (W1 reads the provider at lookup time) |
| NFR2.2 | Every U10 call — A1′, A2, A5′, D1, D2 | Carries a **15 s client deadline** (an `AbortController` deadline on the call), aligned to the contract's connect 5 s + read 10 s. When it fires the section leaves its loading state and renders the inline retryable error that names the timeout with a retry control, keeping the typed email, the selected role and the typed reason. No spinner outlives the deadline. The console's shared `request()` has **no timeout today** (`support-console/src/api/client.ts`), so the deadline is this unit's change (tech-stack T5) | `contract-summary.md` timeout rules; spec §10 "Network failure" |
| NFR2.3 | Role change — A2 `PUT /v1/admin/users/{userId}/role` | p95 < 2 s. One attempt only (mutations are `retry: false` in the console's query client). The transition ("admin → support") and **"This change is audited."** render from the mutation's own answer; the follow-up record re-read updates the badges when it lands and does not gate the completion message, so the perceived completion stays inside one call's budget | spec §3.3; `queryClient.ts` (`mutations.retry: false`) |
| NFR2.4 | Disable / restore — D1 `POST …/disable`, D2 `POST …/restore` | p95 < 2 s, one attempt, same render-then-re-read split as NFR2.3. Nothing is sent before **Confirm** | spec §3.4; C7 answers |
| NFR2.5 | Approvals list — A5′ `GET /v1/support-grants?status=pending_approval` | p95 < 2 s for ≤ 100 rows (the route's own bound, spec §4 W5). Read with `staleTime: 0` and refetched after an approve or revoke, so the list is never a stale view of what is pending | spec §3.5, §4 W5; `queryClient.ts` defaults |
| NFR2.6 | Approve / revoke — A3, A4 (existing routes, reused by the mounted queue) | Unchanged from the `grants` section: one attempt, the shared per-IP budget, the shell's `onGrantChanged` refresh after success. U10 adds no budget and no retry to these | spec §3.5, §11 |

**The two-call completions.** NFR2.3–NFR2.5 each involve a second call (the record re-read, or
the list refetch). The second call is not given a combined target and never blocks the completion
the server already answered; it corrects the display when it returns, and on its own failure it
renders the inline error without erasing the change that was just confirmed.

## Retry and load behaviour

| ID | Requirement |
|---|---|
| NFR2.7 | **No U10 call retries automatically.** The console's query client already sets `retry: false` for every query and every mutation, and this unit keeps that: the lookup is an audited action (`useAdminLookup`, `retry: false`), the three mutations are one-shot, and the pending-grant read is refetched only on an approve, a revoke or the user's own retry. The only retry is the person pressing the retry control, which is a new request |
| NFR2.8 | A `429` from any U10 route is rendered as its own state — the not-retried, "too many requests, retry after N seconds" line, N taken from the `Retry-After` header the gateway already sets — and is never retried automatically. The gateway answers `429` as an RFC 7807 body carrying the correlation id (`middleware.rs` `rate_limited`) |
| NFR2.9 | No background polling and no refetch-on-focus for the admin section: the record stays until another lookup replaces it, the section is left, or a mutation's re-read answers. The pending-grant list refreshes on an approve/revoke, not on a timer (a timer would be an unattended read of grant data on an admin's screen) |
| NFR2.10 | The looked-up record and its query are not kept in the console cache after the section is left (`queryClient.removeQueries(["admin"])` on unmount, spec §3.2 of `frontend-components.md`): a later lookup is always a fresh provider read, which is also what keeps NFR24.1's freshness promise true across visits |

## Resource constraints

- **No new runtime or dev dependency** (tech-stack T1/T6). The section is a lazy route chunk
  (`React.lazy`, spec §2.1), so the console's initial entry is not grown by this unit's code.
- **The console has no size budget** — `support-console/package.json` defines `build`, `test`,
  `lint`, `typecheck`, `format:check`, and no `size-limit` gate (`web/` is the package with
  budgets, 300/150/150 kB gzip). This unit therefore states the chunk intent and does not
  invent a gate: `yarn build`'s chunk report is the only measurement, and adding a console-wide
  budget is a separate decision for the console's owner.
- No business or compliance computation in the browser: nothing here values, allocates or
  authorises. The role shown is the provider's answer, the status shown is the server's answer,
  and the admin check is the server's (spec §2.1, §7).

## Measurement ownership

The client half is this unit's and is measured in the MSW-backed Vitest suites: the deadline
behaviour (a delayed handler must produce the named-timeout surface), the no-auto-retry rule
(a failing handler is called exactly once), the render-then-re-read split, and the `429`
surface with its `Retry-After`. **The server hops are not this unit's**: the gateway's
routing and limits, Identity's provider read (connect 1 s / total 3 s, one attempt) and the
grant-list query are owned by those units and measured by their metrics and their own NFR
records. A latency regression in any of them shows up there, not in this suite.

## Benchmarks

No benchmark suite is introduced. NFR2.1–NFR2.5 are guarded by the MSW-backed suites'
deadline, retry and re-read assertions; NFR2.6 is the console's existing behaviour, already
covered by `GrantApprovalQueue`'s suite. There is no automated check for the perceived-latency
figures themselves — they are targets for review and for the service metrics, not a harness
this unit builds.