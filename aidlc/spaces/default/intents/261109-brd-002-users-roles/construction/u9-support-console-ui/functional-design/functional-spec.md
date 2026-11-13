# Functional Spec — U9 Support Console UI

**Unit:** `u9-support-console-ui` · **Stage:** Functional Design · **Kind:** ui (embedded in
`support-console/`) · **Iteration:** unit-major, serial

**Stories this unit presents:** US3.1–US3.6 (`inception/user-stories/stories.md`), assigned to U9
by the cross-cutting note in `inception/units-generation/unit-of-work-story-map.md`. The
acceptance criteria themselves belong to U3's service rules; U9 is their surface.

**Requirements this unit presents:** `requirements.md` — FR16.1–FR16.6 (support access), FR15.4
(admin-only grant approval), FR15.5/FR15.6 (ownership and non-disclosure on every read),
FR15.9 (client gating is presentational only), NFR22 (identifier handling), NFR24 (role and status
freshness), NFR25 (accessibility) and NFR26 (localisation). The rules behind them are U3's; this
spec is the screen that hears their answer.

**Upstream contracts:** C5 Support Access (`POST /v1/support-grants`, the pending list,
`/approve`, `DELETE /{id}`, `GET /{id}/metadata`, the Grant and SupportMetadata shapes) and C1/C2
(the gateway's `/v1` surface). The console renders the rules U3 owns; it adds none.

## 1. Purpose

The support agent's whole surface in `support-console/`:

1. **Request a grant** for one user record with a reason.
2. **See the active-grant banner** — status, reason, target, approver, time remaining, and the
   metadata-only reminder — on every section of the console.
3. **Read the closed metadata projection** under an active grant: the target's display name,
   email-verification status, account summaries (id, label, source type, sync status, last sync)
   and portfolio summaries (id, label). Nothing else exists on the screen.

It is presentation only. No screen, export, or control exposes or resets a credential, and no
field carries holdings, balances, valuations, transactions, or orders (FR16.4, AC3.3.2,
AC3.4.1, AC3.4.2). Support is never an owner: portfolio contents and valuations have no route,
so the console has nothing to call (AC3.6.1).

The unit also reconciles the console's existing support surface with the contract that U3
shipped. The console today speaks an older model — a 1–480-minute duration, a pre-named second
approver, an account *search* route, a per-account metadata route and a per-read reason header —
none of which the gateway or Identity serve. This spec fixes the console to the shipped contract
rather than the reverse.

## 2. Journey and screen flow

The console has one signed-in support session (support or admin role, BR1.1). The shell keeps
the session's **own** grant; admin-facing grant approval lives in the Admin users section (U10).

### 2.1 Request a grant

1. A support agent opens `#grants` with no live grant. The shell shows the no-grant callout:
   "No support grant is active. Request one, or ask an administrator to approve a pending
   request, before reading account metadata."
2. The agent enters the **target user id** and a **reason** and submits.
3. `POST /v1/support-grants` answers `201` with the grant in `pending_approval`. The form clears
   and a `role="status"` line reads: "Requested. A different administrator must approve it within
   an hour; the grant then lasts at most 60 minutes."
4. The banner switches to *Pending approval* and states that no metadata reads are allowed yet.
   The shell re-reads the agent's own grants when the agent presses **Check status** (or on the
   next page load) and moves the banner to *Active* once an approval lands.

### 2.2 Approve (admin, in the Admin users section)

U10's AdminApprovals owns this screen. A different administrator approves; the server sets
`expiresAt = approval time + 60 minutes` (BR4.1, AC3.2.1); a self-approval is refused (`403`,
AC3.2.2). Nothing on the support agent's screen can approve its own request, and the approval
queue no longer renders in the `grants` section (§3.4).

### 2.3 Read the metadata projection

1. With an active grant, the agent opens `#metadata` (the old `#search` hash redirects there).
2. The panel reads `GET /v1/support-grants/{grantId}/metadata` once. The response is the whole
   closed projection for the grant's target user — there is no second, per-account read.
3. The panel renders the user header, the accounts table and the portfolio list, and states that
   the read is recorded in the audit log (AC3.1.1).
4. **Refresh** re-reads within the same active grant. Every read is a new audited access.
5. When the window ends, the client clock turns the banner to *Expired* and the read control is
   disabled before any call. A read that still reaches the server after expiry, or one whose
   grant was revoked mid-session, is refused with `403`; the panel shows the access-ended state
   and the shell marks the local grant expired and clears the support query cache (AC3.1.2,
   AC3.5.1, AC3.5.2).

### 2.4 State machine (the grant as the console sees it)

```
(none) --request 201--> pending_approval --approve--> active --expiresAt--> expired
                                    \--revoke--> revoked        \--DELETE--> revoked
```

- The only writes the console sends are **request** (support) and **approve/revoke** (admin, in
  the Admin users section). The agent's screen never mutates a grant after creating it.
- The console's local status is a clock-derived view of the server's answer; the server is
  authoritative on every read. An `active` grant whose `expiresAt` is null or unparseable is
  treated as expired (fail closed).
- `denied` appears in C5's status enum but has no path in the shipped lifecycle (U3's domain has
  four statuses). The console models the four it can receive (§12, open item 3).

## 3. Screen rules

### 3.1 The `grants` section — request

- The request form renders **only for the `support` role**. An admin-only session sees one line:
  "A support agent requests a grant; an administrator approves it in Admin users." with a link to
  the admin section. The server refuses a non-support request with `403` regardless.
- Fields: **Target user id** (required, free text) and **Reason** (required, 1–500 characters
  after trimming, with a `{count}/500` counter). There is **no duration field** (the window is
  fixed at 60 minutes from approval) and **no approver field** (any different administrator may
  approve; the server enforces the difference).
- Submit is disabled while the request is in flight. On `201` the form clears both fields and the
  success line appears in a `role="status"` region.
- `409` (a live grant already exists for this agent and user) clears nothing, states "You already
  have a live grant for this user.", and offers **Check your current grant**, which re-reads the
  agent's own grants and shows it. A stale pending request older than an hour is expired by the
  server before the new one is written, so the same submit succeeds again after that hour.
- `404` means no user has that id: "No user has that id." The typed values are kept.
- `400` (a malformed user id) is a field error on the target id: "Enter a valid user id."
- The form never pre-fills a reason and never stores one outside component state.

### 3.2 The active-grant banner

The banner is the shell's, rendered above every section, and it always shows:

- The status as a label **plus** tone and shape (`StatusBadge`), never colour alone.
- The reason as the banner heading — this is the reason the access was granted for.
- Target user id, approved by (or an em dash), and the expiry date-time (or an em dash).
- For an active grant, **time remaining** — "42 minutes left", "less than a minute left" — derived
  from the same 30-second clock the console already uses (`useGrantClock`).
- The metadata-only reminder: "Metadata only. Holdings, balances, valuations, transactions and
  orders are never visible to support."

Per-status copy:

| Status | Copy | Reads |
|---|---|---|
| `pending_approval` | "Waiting for a different administrator to approve. Requests expire one hour after they are made." | Disabled |
| `active` | "Access ends at {expiresAt}, {n} minutes left. Every metadata read is recorded in the audit log." | Enabled |
| `expired` | "Access ended at {expiresAt}. Request a new grant to continue." | Disabled |
| `revoked` | "This grant was revoked. Request a new grant to continue." | Disabled |

- **Check status** re-reads the session's own grants; **Request new grant** moves to `#grants`.
- The banner is presentational: it never claims access the server would refuse, and it never
  renders a ProblemDetails `detail`.

### 3.3 The `metadata` section — the closed projection

The section is renamed **Metadata** (hash `#metadata`; `#search` redirects to it). It replaces
both the old account search and the per-account panel.

- With no grant, or a grant that is not active, it renders one callout and **performs no read**:
  "Metadata reads open once a different administrator approves your grant." The server would
  answer `403` anyway.
- With an active grant it reads the projection once and renders:
  - **User**: user id (mono), display name (em dash when absent), email verification as
    **Verified** / **Not verified** — never the address itself.
  - **Accounts** — a table with a caption, columns **Label**, **Source**, **Sync status**, **Last
    synced**. Source values are `exchange`, `wallet`, `walletconnect`, `csv`; sync status is
    `active`, `syncing`, `error`, `disconnected`; a missing sync time is an em dash. Empty:
    "No accounts."
  - **Portfolios** — a list of labels with their identifiers; each row is a label and an id, not
    a link and not a value. Empty: "No portfolios."
- A **filter** input narrows the already-returned accounts and portfolios by label, id, or source.
  It performs **no request** and cannot widen the projection; it is client-side presentation only.
- **Refresh** re-reads. The panel states: "This read is recorded in the audit log."
- The panel never renders a field outside the whitelist in §4 W3, even if the server over-sends.
- An empty projection is a legitimate answer and is reported as such, not as an error.

### 3.4 Where the grant surfaces sit

- **Grants** (`#grants`): the support agent's request form and the banner's entry point. It no
  longer mounts the approval queue; a support session has no approval control anywhere.
- **Admin users** (`#admin-users`, U10): the approvals panel — the pending list, **Approve** and
  **Revoke** — is the one place an administrator manages grants (FR20.1). The earlier duplication
  U10 recorded is resolved here (§12, open item 7).
- The `GrantApprovalQueue` component itself is unchanged in behaviour except that its **Approve**
  button is disabled with an explanatory line when the session's role is not `admin`; it is now
  mounted only by the admin section.

## 4. Contract and console rules this unit adds

U9 is presentational, but its surface cannot be built on the routes the console calls today. Each
rule below is the smallest change that lets the validated contract serve the screen; none adds a
grant rule, table, or state.

- **W1 — The request body is `{userId, reason}`.** The duration and the pre-named approver are
  gone from the console and from the request. The window is fixed at 60 minutes **from approval**
  by the approving administrator (BR4.1); asking an agent to choose it would misrepresent the
  contract. The client type `GrantRequestInput` shrinks to the two fields.
- **W2 — The grant answer is keyed `grantId`.** `POST /v1/support-grants`, `/approve` and
  `DELETE /{id}` answer `{grantId, userId, status, reason, requestedAt, approvedBy?, approvedAt?,
  expiresAt?}`; the pending list answers rows keyed `id` with `requestedBy`. The console's grant
  mapper accepts `grantId` or `id`, so one mapper serves every grant answer. Today the mapper
  requires `id` and would throw on a create or approve response — a real defect this unit fixes.
- **W3 — The metadata read is the grant route.** `GET /v1/support-grants/{grantId}/metadata`
  answers the C5 `SupportMetadata`: `userId`, `displayName` (nullable), `emailVerified`, and the
  `accounts[]` / `portfolios[]` projections. The console retires `/support/accounts?grantId…`,
  `/support/accounts/{id}/metadata?grantId` and the `X-Support-Reason` header; none of those
  paths or headers exists at the gateway or in Identity. The free-text per-read reason goes with
  them — the grant's own reason is what the server records.
- **W4 — The session reads its own grants.** The console cannot recover the agent's grant after a
  reload and cannot learn that a pending request became active, because no route returns the
  caller's grants (the only list is the admin-only pending list, and no by-id read exists). U9
  adds `GET /v1/support-grants/mine`: the caller's own grants, newest first, bounded to 20 rows,
  in the C5 grant shape plus `requestedBy`/`requestedAt`/`approvedBy`/`approvedAt`. Identity
  serves it from U3's grant store through a read-only use case that requires the support role; the
  gateway declares it with the shared per-token budget and no admin requirement, and the service
  re-checks the role from the forwarded JWT.
- **W5 — Four statuses.** The console's `GrantStatus` stays `pending_approval | active | expired |
  revoked`, matching the shipped lifecycle. C5's `denied` has no producer; modelling it would add
  a dead branch (§12, open item 3).
- **W6 — One approvals home.** The approvals queue moves out of `grants` and lives in Admin users
  only. The support surface gains no approval control, which is also AC3.2.2 at the screen.
- **W7 — No by-id grant read.** `getSupportGrant` and `useSupportGrant` in the console call a
  route that does not exist; U9 removes both together with the search/metadata calls and types
  they replace.

## 5. Form and field rules

| Screen | Field | Rule | Failure shown |
|---|---|---|---|
| Request | Target user id | Required after trimming | "Enter the target user's id." |
| Request | Target user id | The server's id format; a malformed value is `400` | "Enter a valid user id." / `400` |
| Request | Reason | Required after trimming; 1–500 characters | "Enter a reason." / `422` `reason` |
| Request | Reason | Counter `{count}/500`; over-length is refused before submit | "Use 500 characters or fewer." |
| Request | — | No duration and no approver field exist | — |
| Metadata | Filter | Free text; narrows rendered rows only | — |

Client-side checks are presentation only; the server re-checks every one and its answer wins. A
`422` renders per-field messages from the response's `errors[].field` and never the problem
`detail` text.

## 6. Accessibility and localisation

- **WCAG 2.2 AA (NFR25):** every section is a labelled region with a visible heading; every input
  has a visible label and its error is tied with `aria-describedby`; the urgency of a nearly
  expired grant is carried in text, not colour; the request result, the status change, and the
  metadata read completion are announced in a `role="status"` live region; an access-ended or
  failed state uses `role="alert"`; the banner's status uses `StatusBadge` (label + tone + shape);
  the accounts table has a `<caption>` and column headers with `scope="col"`; focus is always
  visible and never colour-only.
- **Parchment Ledger tokens only** (`support-console/src/styles.css`): `--accent`, `--surface`,
  `--surface-warm`, `--border`, `--border-soft`, `--ink`, `--ink-2`, `--muted`, `--success`,
  `--warn`, `--danger`, through the console's existing `ui.ts` helpers. No raw colour, spacing, or
  type value.
- **Missing values** render as an em dash, never `0` or `$0.00`. Dates go through `Intl`; the
  remaining-minutes text is a whole number, and "less than a minute" replaces `0`.
- **English and Vietnamese (NFR26):** the console's copy is presentation-level and follows the
  console's existing strings. The console has no i18n module today — every section renders English
  strings — so U9 adds no second localisation mechanism (§12, open item 6).

## 7. Security posture

- The session's access token lives in memory only (`support-console/src/auth/session.ts`,
  BRD-012 NFR-SEC-10). U9 stores nothing in `localStorage`, `sessionStorage`, a cookie, or
  IndexedDB — not the grant id, the reason draft, the filter, or the projection.
- Every call goes through the API Gateway through the one client module
  (`support-console/src/api/client.ts`); the console never calls a service directly.
- The request body carries the reason in JSON, never in a URL or a header, so it does not reach
  access logs, browser history, or referrers.
- The console never renders a credential, token, provider subject, wallet address, or the target's
  email address. Email verification is a boolean, not the address (FR16.4).
- The projection is whitelisted in the console (`SUPPORT_METADATA_FIELDS`); a field the server
  over-sends cannot become a component prop (BR1.4, AC3.3.2).
- Cancelling an expired or revoked grant is not an action: there is no revoke control on the
  support surface, and the server refuses a read under a dead grant (`403`, audited).
- No screen renders ProblemDetails `detail`; the status and the correlation id reach the UI, which
  is what keeps internal denial codes out of reach.

## 8. API integration points (gateway `/v1` surface)

All through `support-console/src/api/client.ts`, which owns every URL.

| Id | Call | Use | Answers |
|---|---|---|---|
| S1 | `POST /v1/support-grants` | request (§3.1) | `201` grant · `400` malformed user id · `403` not a support agent · `404` no such user · `409` a live grant exists · `422` reason · `503` audit unavailable |
| S2 | `GET /v1/support-grants/mine` | the session's own grants (W4) | `200` array (≤20, newest first) · `403` not a support agent |
| S3 | `GET /v1/support-grants/{grantId}/metadata` | the close projection (§3.3) | `200` `SupportMetadata` · `401` · `403` support role, grant not held, expired, revoked, pending, or target not accessible · `503` the account/portfolio fan-out did not answer |
| S4 | `POST /v1/support-grants/{grantId}/approve` | admin section (U10) | `200` grant · `403` self-approval or not an admin · `404` · `409` request expired or no longer pending |
| S5 | `DELETE /v1/support-grants/{grantId}` | admin section (U10) | `200` grant · `403` · `404` · `409` already terminal |
| S6 | `GET /v1/support-grants?status=pending_approval` | admin approvals (U10) | `200` pending rows · `403` · `422` any other status |

Gateway table: S1 is `POST /v1/support-grants` at 20/min per token (UR3); S2 is a new route at the
shared per-token budget, no admin requirement; S3 is `GET /v1/support-grants/{grantId}/metadata`
at 60/min per token (U3); S4, S5, S6 are admin. S1 and S3 already exist; S2 is this unit's
addition. Every service route re-validates the forwarded JWT against Supabase JWKS.

The metadata read carries a 15-second deadline, matching the rest of the console's data calls;
the server's own fan-out to Account and Portfolio has its own budget, and a fan-out failure is
`503`, never a partial projection.

## 9. Story coverage

| Story AC | Where satisfied in this spec |
|---|---|
| AC3.1.1 — a reason-bearing access is audited | §3.1 (the reason is required and shown), §3.2 (the reason is the banner heading), §3.3 ("This read is recorded in the audit log"), S1/S3. The audit write itself is U3's; the console never writes it. |
| AC3.1.2 — a request after the window ends is refused | §2.3 step 5, §3.3 (client gate + `403` → access-ended), §3.2 (expired copy disables reads) |
| AC3.2.1 — a different admin's approval yields at most 60 minutes | §2.2, §3.1 (no duration field), §3.2 (expiry + remaining minutes), W1 |
| AC3.2.2 — self-approval is refused | §3.4 (no approval control on the support surface), §3.2 (the pending copy names the different administrator), W6; the server refuses with `403` |
| AC3.2.3 — a reason outside 1–500 characters is refused | §5, §3.1 (counter and client check), S1 `422` |
| AC3.3.1 — only the agreed metadata fields are returned | §3.3 (the exact rendered fields), W3, `SUPPORT_METADATA_FIELDS` |
| AC3.3.2 — holdings, balances, valuations, transactions, orders are absent | §3.3 (no such field is rendered), §7 (whitelist) |
| AC3.4.1 — no screen or export exposes a credential, holding, balance, or valuation | §1, §3.3, §7 |
| AC3.4.2 — a credential view or reset is unavailable and refused | §1 (no such control), §3.3 (no such route is called); no API offers one |
| AC3.5.1 — an expired grant's read is forbidden and audited | §2.3 step 5, §3.3 (`403` → access-ended), S3 |
| AC3.5.2 — a revoked grant is refused on the next request | §2.3 step 5, §3.3 (`403` → access-ended; the shell marks the local grant expired and clears the cache) |
| AC3.6.1 — portfolio contents and valuations are denied with a grant | §1, §3.3 (labels and identifiers only; no contents or valuation route exists) |
| AC3.6.2 — no grant means no user record | §2.1, §3.3 (no read without an active grant), S3 (`403`) |

## 10. Error handling summary

| Failure | Classification | Surface |
|---|---|---|
| S1 `400` | Recoverable | Field error on the target id; typed values kept |
| S1 `404` | Not an error | "No user has that id."; values kept |
| S1 `409` | Not an error (one live grant) | "You already have a live grant for this user." + **Check your current grant** |
| S1 `422` | Recoverable | Per-field error from `errors[]`; values kept |
| S1 `403` | Recoverable (session) | "Your session is not a support agent." with the correlation id |
| S1 `503` | Ambiguous | "The request could not be recorded right now." + correlation id + retry; never a claim that nothing was written |
| S2 `403` | Recoverable (session) | The banner keeps its last known state; one line with the correlation id; no crash |
| S2 other failure | Recoverable | The banner states the status could not be checked; the local clock stays authoritative |
| S3 `403` | Not an error (access ended) | Access-ended state; local grant marked expired; support cache cleared |
| S3 `401` | Recoverable (session) | "Sign in again to continue." |
| S3 `503` | Recoverable (upstream) | "The metadata cannot be read right now." + correlation id + **Refresh** |
| S3 network/deadline | Recoverable | Inline error with **Refresh**; no partial projection is rendered |
| Unexpected ProblemDetails | Recoverable | Inline error with the correlation id; surfaced, never swallowed |

## 11. Shared components

- **`GrantBanner`** and **`GrantRequestForm`** already exist in `support-console/src/components/`
  and are reworked here rather than duplicated; both are package-local because their consumers
  live in this package.
- **`SupportMetadataPanel`** is new and is the only metadata surface; it replaces `AccountSearch`
  and `AccountMetadataPanel`, both of which call routes that do not exist.
- **`StatusBadge`** and the `StatusPresentation` vocabulary are reused for every grant status; the
  mapping to label, tone and shape is in `frontend-components.md` §5.1.
- **`GrantApprovalQueue`** is unchanged as a component contract except for the non-admin disabled
  state; it is mounted by the Admin users section only (U10's `AdminApprovals`).
- The console's `ui.ts` class helpers and `cardClass` / `calloutClass` / `alertClass` /
  `fieldErrorClass` / `inputClass` / `labelClass` / button classes are used as they are.

## 12. Open items (recorded, not resolved here)

1. **Contract additions.** S2 (`GET /v1/support-grants/mine`) is new, and W2's `grantId` key plus
   W3's metadata route are the shapes the console must read. `contract-summary.md` C1/C2 and C5
   must carry them in the change that lands them; the C4 model and `docs/architecture/README.md`
   move together only if a container or relationship changes (these are route additions).
2. **How an agent obtains the target user id.** There is no support-facing lookup route; the admin
   exact-email lookup is admin-only, and the ticket list carries a pseudonymous requester
   reference with no resolution route. The form therefore takes a pasted internal user id. If
   support needs identifier resolution, it is a new route with its own authorisation and audit —
   not something this surface can invent.
3. **C5's `denied` status has no producer.** U3's domain models `pending_approval`, `active`,
   `expired`, `revoked`; no code path denies a grant. The console matches the implementation. If
   `denied` is intended, U3 owns adding it and the console adds the branch then.
4. **No by-id grant read.** `GET /v1/support-grants/{id}` does not exist at the gateway or in
   Identity; `getSupportGrant`/`useSupportGrant` are removed rather than left pointing at a `404`.
   S2 is the supported replacement.
5. **Grant discovery depends on S2.** Without it a reload loses the session's grant entirely,
   because nothing lists the caller's own grants. This is why S2 is part of the unit rather than a
   follow-up.
6. **The console has no i18n module.** Every existing section renders English strings; NFR26.2
   stays Deferred for the console, as recorded for U7 and U10.
7. **Approvals were in two sections.** U10 recorded the duplication and left the consolidation to
   this unit; §3.4 puts approvals in Admin users only and removes the support-session Approve
   button that could only answer `403`.
8. **The per-read reason is retired.** The console sent `X-Support-Reason` on every metadata read;
   the server has no such header and audits the grant's own reason. No screen asks for a second
   reason.