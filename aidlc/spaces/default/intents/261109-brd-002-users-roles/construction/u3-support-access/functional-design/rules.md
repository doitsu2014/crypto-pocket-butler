# Business Rules — u3-support-access

**Upstream:** `requirements.md` (FR16.1–FR16.6, NFR19.1, NFR21), `stories.md` (US3.1–US3.6,
US7.3), `contract-summary.md` (C1, C2, C5), `components.md` (SupportAccess).

A support grant is a narrow, named, short-lived permission to read a metadata list — not a role,
not an ownership override. Every rule below either narrows who may hold or use a grant, fixes what
a grant may return, or makes an action auditable. The one thing no rule does is let a grant stand
in for ownership.

## Source of truth

```yaml
rules:
  - id: BR3.1
    statement: Only a caller whose validated claims carry the Support role may request a grant, for one target user record, with a reason of 1–500 characters after trimming.
    category: authorization
    applies_to: SupportGrant
    trigger: a grant request
    logic: "IF role != support THEN 403; IF userId is absent or malformed THEN 400; IF reason.trim().len() == 0 THEN 422 (reason required); IF reason.trim().chars().count() > 500 THEN 422 (reason too long); ELSE insert pending_approval with requestedAt = now"
    violation_behaviour: refused with the named problem and nothing persisted; the refusal is audited (BR3.13)
    source: FR16.1, FR16.2, AC3.2.3

  - id: BR3.2
    statement: A request's target must be an existing user record that is neither deleted nor pending deletion, and must not be the requester.
    category: policy
    applies_to: SupportGrant.userId
    trigger: a grant request
    logic: "load the target; absent -> 404 (user not found); deleted -> 404; pending_deletion -> 403 and audited (interim until BRD-019); equal to requestedBy -> 422"
    violation_behaviour: refused and audited; no grant row is written
    source: FR16.1, requirements OQ-B

  - id: BR3.3
    statement: A support agent and a target user hold at most one live grant at a time.
    category: constraint
    applies_to: SupportGrant
    trigger: a grant request while a pending_approval or active grant exists for the same (requestedBy, userId)
    logic: "one transaction: first expire the pair's stale pending row (requested_at <= now - 60 minutes, status = expired), then insert; a partial unique index on (support_agent_id, user_id) WHERE status IN ('pending_approval','active') enforces the invariant atomically, so a concurrent insert that wins raises a unique violation and the loser answers 409"
    violation_behaviour: refused and audited; the reason and window of an existing grant are never extended or replaced by a new request
    source: FR16.1

  - id: BR3.4
    statement: Only an admin who is not the requester may approve, and only a pending_approval grant may be approved.
    category: authorization
    applies_to: SupportGrant
    trigger: an approval
    logic: "IF role != admin THEN 403 (checked before the grant is loaded, so a non-admin cannot probe ids); IF approver == requestedBy THEN 403 (self-approval); UPDATE ... SET approved_by, approved_at, expires_at, status = 'active' WHERE id = :id AND status = 'pending_approval' — rows_affected = 0 -> 409, so two concurrent approvals cannot both win; the update and its outbox row commit together"
    violation_behaviour: refused and audited; no field changes
    source: FR16.2, AC3.2.2

  - id: BR3.5
    statement: A request may be approved for at most 60 minutes after it was made; a later approval is refused and the request is exited as expired.
    category: policy
    applies_to: SupportGrant
    trigger: an approval of a pending request
    logic: "IF now - requestedAt >= 60 minutes THEN persist status = expired with approved_by, approved_at, expires_at and revoked_at all null (permitted by the relaxed approval-shape constraint of migration 0007) and return 409 (request expired)"
    violation_behaviour: refused and audited; an old reason can never be activated later
    source: FR16.2

  - id: BR3.6
    statement: Approval opens a window of exactly 60 minutes from the approval instant, and the grant is usable only while the clock is before its expiry.
    category: policy
    applies_to: SupportGrant.expiresAt
    trigger: a successful approval; every later access attempt
    logic: "expiresAt = approvedAt + 60 minutes; the domain accepts any window of 1..=60 minutes and the service always passes 60; usable <=> status == active AND now < expiresAt"
    violation_behaviour: an approval that cannot set the window is refused; an access at or after expiresAt is refused
    source: FR16.2, AC3.2.1

  - id: BR3.7
    statement: Grant validity is decided on every request from the stored status and the clock; a pending, revoked, expired or unknown grant is refused, and an observed expiry is persisted.
    category: policy
    applies_to: SupportGrant
    trigger: any access under a grant
    logic: "pending_approval -> 403 grant-pending (for the grant's holder); revoked -> 403 grant-revoked (holder); active with now >= expiresAt -> persist expired, 403 grant-expired (holder); a caller who is not the grant's requester, and an id that names no grant, both get one uniform 403 support-access-refused, so a grant id cannot be told from a non-existent one by anyone but its holder"
    violation_behaviour: refused and audited — the audit context carries the precise code even when the response is uniform; a revocation is effective at the next request, without any cache refresh
    source: FR16.5, AC3.1.2, AC3.5.1, AC3.5.2

  - id: BR3.8
    statement: Only the support agent named as a grant's requester may read under it.
    category: authorization
    applies_to: SupportGrant.requestedBy
    trigger: any access under a grant
    logic: "IF caller.role != support THEN 403 support-role-required; IF caller.userId != requestedBy THEN 403 support-access-refused (the same uniform answer an unknown grant id gets, so existence is not disclosed)"
    violation_behaviour: refused and audited; an admin is not a reader under a grant, only an approver or revoker
    source: FR16.1, US3.6

  - id: BR3.9
    statement: The metadata projection contains exactly the agreed fields — user id, display name, email-verification status, account id/label/source type/sync status/last-synced-at, and portfolio id/label — and nothing else.
    category: constraint
    applies_to: SupportMetadata
    trigger: a successful metadata read
    logic: "build the response from the allowlist only; userId is C5's field and an explicit extension of FR16.3 (it names the record the grant already binds); email address, venue, provider account id, error detail and every account or portfolio content field are not fields of the projection"
    violation_behaviour: a response carrying any other field is a defect; there is no 'extra field' path because the projection is a closed shape
    source: FR16.3, AC3.3.1

  - id: BR3.10
    statement: No support surface returns holdings, balances, valuations, transactions, orders, credentials, secrets, tokens, or the provider subject, and no support route can read or reset a credential.
    category: security
    applies_to: SupportMetadata, the support route set
    trigger: any support request
    logic: "the Account and Portfolio support endpoints return only the BR3.9 fields; Identity has no support route that names a credential, holding, balance, valuation, transaction or order; a support caller on such a route is refused 403"
    violation_behaviour: a refusal, audited; the route set is the enforcement, not a response filter
    source: FR16.4, AC3.3.2, AC3.4.1, AC3.4.2

  - id: BR3.11
    statement: Support is never an owner; a grant never satisfies an ownership check, and portfolio contents and valuations stay refused under a valid grant.
    category: authorization
    applies_to: the whole support path
    trigger: any support attempt to act as the target's owner
    logic: "grants are consulted only on the support metadata path; the ownership predicates of the Account, Portfolio and Identity services never accept a grant; a support caller with no valid grant is refused every user-record read"
    violation_behaviour: 403 and audited; no grant path calls the owner-scoped read of another service
    source: FR16.6, AC3.6.1, AC3.6.2

  - id: BR3.12
    statement: Only an admin may revoke a grant, a terminal grant cannot be revoked again, and a revocation ends access at the next request.
    category: authorization
    applies_to: SupportGrant
    trigger: a revocation
    logic: "IF role != admin THEN 403; IF status is already expired or revoked THEN 409; ELSE set revokedAt and status = revoked"
    violation_behaviour: refused and audited; the grant row is kept as evidence and is never deleted
    source: FR16.5, AC3.5.2

  - id: BR3.13
    statement: Every request, approval, revocation, access and refusal is audited with actor, action, target, timestamp, context and outcome; an accepted mutation cannot exist without its audit record, and a read that cannot be audited is not served.
    category: constraint
    applies_to: SupportAuditEvent
    trigger: every support action
    logic: "accepted mutation: the grant row and a support_audit_outbox row commit in one transaction, and the mutation is refused if the outbox row cannot be written; refused mutation: its outbox row is written in its own transaction before the refusal is returned, and a failed write answers 503 audit-unavailable instead of the refusal; read: identity.support_read.served (after the projection is assembled, before it is returned) or identity.support_read.denied, appended synchronously, with 503 if the append fails"
    violation_behaviour: the action fails rather than completing without a record — a refusal that cannot be recorded is surfaced as 503, never returned as a clean 403; a delivery failure after commit is retried by the outbox worker and surfaced by its pending-age metric
    source: US7.3, AC3.1.1, AC3.5.1, AC7.3.1, AC7.3.2
```

## Summary

| Rule | Statement | Category | Source |
|---|---|---|---|
| BR3.1 | Support role, one target, reason 1–500 | authorization | FR16.1–2, AC3.2.3 |
| BR3.2 | Target must exist, not be deleted or pending deletion, not self | policy | FR16.1, OQ-B |
| BR3.3 | One live grant per (agent, target) | constraint | FR16.1 |
| BR3.4 | Distinct admin approves, pending only | authorization | FR16.2, AC3.2.2 |
| BR3.5 | Approval eligible for 60 minutes after request | policy | FR16.2 |
| BR3.6 | Window is exactly 60 minutes from approval | policy | FR16.2, AC3.2.1 |
| BR3.7 | Read-time validity; refusals named | policy | FR16.5, AC3.1.2, AC3.5.1, AC3.5.2 |
| BR3.8 | Only the requesting agent reads | authorization | FR16.1, US3.6 |
| BR3.9 | Closed metadata allowlist | constraint | FR16.3, AC3.3.1 |
| BR3.10 | No contents, credentials or secrets, ever | security | FR16.4, AC3.3.2, AC3.4.1, AC3.4.2 |
| BR3.11 | Support is never an owner | authorization | FR16.6, AC3.6.1, AC3.6.2 |
| BR3.12 | Admin-only revocation, terminal grants kept | authorization | FR16.5, AC3.5.2 |
| BR3.13 | Audit with the action; reads fail closed | constraint | US7.3, AC7.3.1, AC7.3.2 |

## Notes that are decisions, not restatements

- **BR3.5 and BR3.3 are derived rules** — no AC names them. They are recorded as derived (and in
  the stage diary as interpretations) because an unbounded pending request and a duplicate-live
  grant are both real holes in FR16's wording. They narrow behaviour; they never widen it.
- **BR3.7 answers "unknown" and "not yours" with one uniform `support-access-refused`, and gives
  the holder the precise code.** A 404 would let any caller test whether a grant id exists; a
  distinct code for a foreign grant would leak the same fact. The refusal is still audited with its
  precise internal code. Approve and revoke keep 404 on an unknown id because the admin role check
  runs first — only an admin can reach that answer, and an admin is not the enumeration case.
- **BR3.3 and BR3.4 are enforced atomically, not by read-then-write.** A partial unique index
  (`(support_agent_id, user_id) WHERE status IN ('pending_approval','active')`) makes duplicate
  live grants impossible, and approval is a conditional `UPDATE ... WHERE status = 'pending_approval'`
  whose zero rows affected means someone else moved the grant first. Two concurrent requests or
  approvals cannot both pass.
- **BR3.5's persisted edge needs migration 0007.** The existing `support_grants_approval_shape`
  constraint requires `approved_at` for a non-pending status, so an unapproved request cannot be
  stored as `expired` as the schema stands. Migration `0007_support_access` (which also creates
  `support_audit_outbox` and the partial unique index) relaxes that one clause to allow an
  `expired` row with no approval fields; the down migration restores it.
- **BR3.10 is enforced by the route set, not by a filter.** The Account and Portfolio support
  endpoints return the BR3.9 shape; Identity never asks for anything else. A filter over a rich
  response would be one forgotten field away from a leak.
- **BR3.13 uses three mechanisms because the three paths differ.** An accepted mutation has a
  transaction to ride; a refusal has none, so it gets its own; a read has neither and appends
  inline. All three end at the same place: no accepted change, no refusal and no served read
  without a record.