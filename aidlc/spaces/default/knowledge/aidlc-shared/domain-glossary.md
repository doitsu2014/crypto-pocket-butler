# Domain glossary

The words this product uses in its own way. When an artifact, a test name or a UI label uses one
of these, it means what is written here and nothing looser. Terms are grouped; within a group the
definition is one or two sentences, because a glossary an agent skims is worth more than one it
skips.

## Identity

- **Principal** — the identity a request is allowed to act as: an internal user id, a role, and a
  status, resolved from a validated Supabase token. `/v1/principals` answers it; every service
  that needs "who is this" asks for the principal rather than re-reading the token's claims.
- **User status** — `pending` (email or required MFA incomplete; portfolio data blocked), `active`
  (verified and allowed), `disabled` (set by an administrator; history kept), `pending_deletion`
  (deletion requested, recovery window open), `deleted` (anonymised, terminal). `pending` is a
  stored status this project added alongside the four the BRD names.
- **Profile override** — a field the user edited in the SPA (`display_name_overridden`,
  `email_overridden`). The refresh from the identity provider must skip an overridden field; it
  never overwrites a user's edit.
- **Shredding key** — the per-user pgsodium key that encrypts `auth_subject`. Destroying the key
  is how erasure makes the stored subject unreadable; it is not a re-encryption.
- **Jurisdiction gate** — the check that refuses provisioning for a jurisdiction that is not on the
  launch list. A refusal is `422`, matching C8. The gate is called in-process by the registry; the
  C8 HTTP endpoint exists for callers outside Identity.

## Support access

- **Support grant** — a time-boxed permission for a support agent to see one user's *metadata*.
  States: `pending_approval`, `active`, `expired`, `revoked`. Approval must come from a **distinct**
  administrator; a request that expired before approval is a legitimate terminal shape.
- **Metadata boundary** — what a grant exposes: display name, email-verification status, account
  status, and ids. Never portfolio contents, never holdings, never a credential. Any artifact that
  widens this is wrong even if the code would allow it.
- **One live grant per pair** — at most one `pending_approval` or `active` grant per
  (support agent, user). A terminal row does not occupy the slot, so a new request is allowed after
  a revoke or an expiry.

## Portfolio

- **Holding / position** — a quantity of an asset. Quantities and values are `rust_decimal`, never
  binary floating point.
- **Valuation** — the market value of a holding or a portfolio at a point in time.
- **Allocation** — a holding's share of the portfolio, as a percentage.
- **Drift** — the distance between a target allocation and the current one.
- **Order intent** — an *advisory* buy or sell intention carrying a projected allocation and drift,
  with an explicit shortfall when a sell exceeds the holdings. Nothing places an order; no external
  order call exists anywhere in the system.

## Audit and delivery

- **Audit outbox** — a per-service table (`support_audit_outbox`, `consent_audit_outbox`,
  `status_audit_outbox`, …) written in the same transaction as the change it records, so the change
  and its audit intent commit together.
- **Drainer** — the worker that delivers outbox rows to the audit service (C9) with an idempotency
  key, exponential backoff, and a three-attempt dead letter. A dead-lettered row is surfaced, never
  dropped silently.
- **Contract id** — `C1`–`C9` name the internal/external REST contracts in
  `inception/contract-design/contract-summary.md`. `C1`/`C2` are the gateway surfaces for the SPA
  and the support console; `C3` user registry; `C4` authorisation; `C5` support access;
  `C6` preferences and consent; `C7` account lifecycle; `C8` jurisdiction gate; `C9` audit write.
- **Traceability ids** — `BR<n>.<n>` business rules, `AC<n>.<n>.<n>` acceptance criteria,
  `FR-…`/`NFR-…` requirement ids, `U<n>` units. Code cites the `BR` ids it serves in its module
  header; tests and traceability rows cite the `AC` ids.

## Environment

- **Disposable database** — the throwaway PostgreSQL container (the pinned `supabase/postgres`
  image) the `#[ignore]` adapter and migration suites run against. Never a shared or production
  database: those suites create and drop roles and reverse migrations.
- **Local stack** — `deployment/localhost`, a Docker Compose stack that runs the whole system on one
  machine. Developer convenience only; it is not an environment tier and not a promotion step.