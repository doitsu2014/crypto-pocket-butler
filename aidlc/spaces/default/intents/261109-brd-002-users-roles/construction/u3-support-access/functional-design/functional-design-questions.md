# Functional Design Questions — u3-support-access

Unit: `u3-support-access` (U3) · Stage: construction/functional-design

This run is autonomous Construction: the person is not stopped for stage questions. The answers
below were chosen by the architect role from the requirements, the contract, the components and
the existing code, and each one records why. They are decisions of this design, not the person's
words; anything the person must still settle is listed in `functional-spec.md` § "Open items".

## Q1. [Architect] Where the metadata projection is assembled (the C5 open question)

- A. Identity (U3) serves the projection and aggregates it from new internal support-metadata
  endpoints on the Account and Portfolio services, called with Identity's own service token.
- B. The gateway composes the projection from the grant check plus owner-surface reads.

[Answer]: A. The gateway's declared route table has no Account or Portfolio upstream at all, so B
cannot run today without giving the edge two new upstreams and business assembly logic. The grant,
its audit trail and the refusal decision all live in Identity, and C5 already assigns the metadata
path to U3. A keeps the gateway a router and the projection an auditable identity decision. The
cost is two new service-token-guarded endpoints owned by services outside this intent; recorded
as an open dependency, not hidden.

## Q2. [Security] May support read a `pending_deletion` target (requirements OQ-B)?

- A. Refuse it (403, audited) until BRD-019 settles the deletion policy.
- B. Allow it, because the record still exists.

[Answer]: A. U5's own rule denies a pending-deletion account every action until its policy lands;
granting support a read of a record already scheduled for erasure is the wider hole. The refusal
is recorded as an interim rule and an open item against BRD-019.

## Q3. [Product] Is the C5 `denied` grant status needed?

- A. No. Four statuses — `pending_approval`, `active`, `expired`, `revoked`. An admin who will not
  approve simply leaves the request pending (it expires); C5's `denied` is dropped.
- B. Add an explicit admin deny action and a `denied` status.

[Answer]: A. No FR or AC requires a deny action, and the domain aggregate has no such edge.
Inventing one adds a fifth terminal state no story uses. The C5 enum edit is recorded as an open
contract item.

## Q4. [Security] When is a grant's validity evaluated?

- A. On every request, against the clock; an observed expiry is persisted. No lifecycle sweeper.
- B. A background job flips expired rows, and reads trust the stored status.

[Answer]: A. This is what the domain already does (`effective_status`, `expire_if_due`), and it
cannot be stale. B adds a window in which an expired grant reads as active. A worker still runs,
but only to drain the audit outbox, never to decide access.

## Q5. [Product] Who may read under a grant?

- A. Only the support agent who requested it.
- B. Any support agent holding a valid grant id.

[Answer]: A. The grant records its requester; binding use to that agent makes every access
attributable to a person who asked for that reason. It also means a leaked grant id is useless to
another agent. The existing check path does not enforce this; the design requires it.

## Q6. [Product] May a stale pending request be approved indefinitely?

- A. No — approval is eligible for 60 minutes after the request; later approval is refused and the
  request is expired.
- B. Yes, indefinitely.

[Answer]: A. Without a bound, an old request plus a distracted admin produces an access window
hours after the reason was written. The bound mirrors the access window and keeps the reason
current. Marked as a derived rule, not a new product requirement.

## Q7. [Architect] How the audit records of grant mutations are made durable

- A. A `support_audit_outbox` row commits in the same transaction as the grant change; the existing
  worker loop drains it to C10, as U4 and U5 already do. Reads append synchronously and fail
  closed.
- B. Synchronous C10 append after the mutation, returning 503 if it fails.

[Answer]: A. B leaves a committed grant with no audit record when C10 is down — the state change
survives and only the error is surfaced. A gives every accepted mutation a durable audit intent,
and the read path keeps the strict "record before serve" behaviour AC7.3.2 asks for.