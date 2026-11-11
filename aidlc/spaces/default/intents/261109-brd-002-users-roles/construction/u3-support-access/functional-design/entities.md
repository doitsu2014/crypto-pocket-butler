# Entities — u3-support-access

**Upstream:** `unit-of-work.md` (U3 `u3-support-access`), `components.md` (SupportAccess),
`stories.md` (US3.1–US3.6, US7.3), `requirements.md` (FR16.1–FR16.6), `contract-summary.md` (C5).

U3 owns the support grant — its lifecycle, its binding to one support agent and one target user —
and the metadata projection a grant authorises. It owns no user, account or portfolio record: the
grant points at a `User` owned by U1, and the projection is assembled from fields owned by
Identity (the user's own row) and by the Account and Portfolio services, never by reading their
tables.

## Source of truth

```yaml
entities:
  - name: SupportGrant
    description: >
      One time-bound, reason-recorded grant that lets a named support agent read the metadata of
      one user record. The aggregate root of this unit; every lifecycle edge is decided here.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
        constraints: "grant identifier; assigned on insert, never changed"
      - name: userId
        type: uuid
        required: true
        constraints: "the one target user record; fixed at request, never changed"
      - name: requestedBy
        type: uuid
        required: true
        constraints: "the support agent who asked; this is the only caller who may read under the grant"
      - name: reason
        type: string
        required: true
        constraints: "1–500 characters after trimming; recorded once, never edited; never a credential or token"
      - name: status
        type: enum
        required: true
        allowed: [pending_approval, active, expired, revoked]
        constraints: "the stored state; an active grant whose expiresAt has passed reads as expired (rules.md BR3.7)"
      - name: requestedAt
        type: timestamp
        required: true
        constraints: "database clock, the moment the request committed"
      - name: approvedBy
        type: uuid
        required: false
        constraints: "an admin user id, distinct from requestedBy; set once by approval"
      - name: approvedAt
        type: timestamp
        required: false
        constraints: "set with approvedBy"
      - name: expiresAt
        type: timestamp
        required: false
        constraints: "approvedAt + 60 minutes; set once by approval; the window's end"
      - name: revokedAt
        type: timestamp
        required: false
        constraints: "set once by an admin revocation"
    entity_constraints:
      - "One live grant per (requestedBy, userId): no new request while a pending_approval or active grant exists for the pair (BR3.3)."
      - "requestedBy must hold the Support role at request time; approvedBy must hold the Admin role at approval time and must differ from requestedBy (BR3.1, BR3.4)."
      - "userId must differ from requestedBy and must name a user record that is neither deleted nor pending_deletion (BR3.2)."
      - "Every state change writes a SupportAuditEvent row in the same transaction (BR3.13)."
    relationships:
      - to: User
        cardinality: many-to-one
        direction: "the target; owned by u1-user-registry, read only"
      - to: User
        cardinality: many-to-one
        direction: "the requester (support agent); owned by u1-user-registry, read only"

  - name: SupportAuditEvent
    description: >
      One durable audit intent for a support action, written with the action and drained to the
      audit service (C10). The context carries identifiers only, never a credential or token.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: actor
        type: uuid
        required: true
        constraints: "the acting user id — every event this unit writes is caused by a caller, so there is no system actor"
      - name: action
        type: enum
        required: true
        allowed:
          - identity.support_grant.requested
          - identity.support_grant.approved
          - identity.support_grant.revoked
          - identity.support_read.served
          - identity.support_read.denied
      - name: target
        type: string
        required: true
        constraints: "the affected user id or grant id; identifiers only"
      - name: occurredAt
        type: timestamp
        required: true
        constraints: "database clock"
      - name: context
        type: map
        required: true
        constraints: "identifier-only key/values: grant id, target user id, refusal code; never a reason body, credential, token or address"
      - name: outcome
        type: enum
        required: true
        allowed: [success, denied, failed]
        constraints: "C9's vocabulary — `failed` covers an operation that could not complete, including a metadata source that could not answer"
      - name: deliveryState
        type: enum
        required: true
        allowed: [pending, delivered, dead]
        constraints: "pending until the worker loop delivers it to C10; `dead` after the attempt budget — the shipped outbox schema and drainer vocabulary (NFR1.4)"
    entity_constraints:
      - "An accepted mutation writes its event in the same transaction as the grant change (BR3.13)."
      - "A refusal has no mutation to share, so its event is written in its own transaction before the refusal is returned; a failed write answers 503 audit-unavailable instead of the refusal (BR3.13)."
      - "Delivery is at-least-once with an idempotency key; the worker tolerates a repeat."
    relationships:
      - to: SupportGrant
        cardinality: many-to-one
        direction: "every event about a grant names it in context; a read event names the grant it used"

  - name: SupportMetadata
    description: >
      The read-only projection a valid grant authorises. Not persisted by U3: assembled per request
      from the Identity user row and the Account and Portfolio services, and returned in exactly
      this shape. The allowlist is the security boundary of AC3.3.1/AC3.3.2.
    attributes:
      - name: userId
        type: uuid
        required: true
        constraints: "the grant's target"
      - name: displayName
        type: string
        required: false
        constraints: "the target's display name; genuinely absent is null, never a placeholder"
      - name: emailVerified
        type: boolean
        required: true
        constraints: "verification status only — the address itself is not in the projection"
      - name: accounts
        type: list<AccountSummary>
        required: true
        constraints: "may be empty; never carries holdings, balances, values, transactions or orders"
      - name: portfolios
        type: list<PortfolioSummary>
        required: true
        constraints: "may be empty; never carries contents or valuations"
    entity_constraints:
      - "No other field may be added to this shape without a requirements change (BR3.9)."
      - "`userId` is C5's field and an explicit extension of FR16.3's list: it names the record the grant already binds, which the holder supplied at request time — recorded here rather than left implicit."
      - "The provider subject, any credential or token, and any address are absent by construction."

  - name: AccountSummary
    description: One account's metadata inside the projection.
    attributes:
      - name: id
        type: uuid
        required: true
      - name: label
        type: string
        required: true
        constraints: "the user's label for the account"
      - name: sourceType
        type: enum
        required: true
        allowed: [exchange, wallet, walletconnect, csv]
        constraints: "the connection kind (`accounts.kind`); never the venue, the provider credential or the provider account id"
      - name: syncStatus
        type: enum
        required: true
        allowed: [active, syncing, error, disconnected]
        constraints: "the account's stored status; the error detail is not part of the projection"
      - name: lastSyncedAt
        type: timestamp
        required: false
        constraints: "null when the account has never synced"

  - name: PortfolioSummary
    description: One portfolio's metadata inside the projection.
    attributes:
      - name: id
        type: uuid
        required: true
      - name: label
        type: string
        required: true

  - name: GrantCheck
    description: >
      The decision value a grant produces for one access attempt: valid, or refused with a code.
      The read path answers only through this value so every refusal is a named outcome, not an
      absence.
    attributes:
      - name: grantId
        type: uuid
        required: true
      - name: targetUserId
        type: uuid
        required: true
      - name: requesterId
        type: uuid
        required: true
        constraints: "the grant's requestedBy — the caller must equal it"
      - name: valid
        type: boolean
        required: true
      - name: refusalCode
        type: enum
        required: false
        allowed:
          - grant-unknown
          - support-role-required
          - not-grant-holder
          - grant-pending
          - grant-expired
          - grant-revoked
          - target-not-accessible
        constraints: "present exactly when valid is false"
```

## Summary

**SupportGrant** is the aggregate. It is created by a support agent's request (one target, one
reason), becomes usable only through a distinct admin's approval, and carries a window of at most
60 minutes from that approval. It never becomes an ownership credential: no reader of this entity
may treat `active` as "is the target's owner".

The lifecycle is small: `pending_approval → active → expired|revoked`, plus
`pending_approval → expired` when the request itself ages out, and `pending_approval → revoked`
when an admin cancels it. There is no `denied` state — question Q3 records why, and the C5 enum
edit is an open contract item.

**SupportAuditEvent** is not a log line: it is a row written with the action, so an accepted
mutation cannot exist without its audit intent. Reads take the stricter path — the event is
appended before the data is returned, and a failure refuses the read. Both behaviours are one
rule (BR3.13) because both answer AC7.3.2.

**SupportMetadata** is a read model with an allowlist that is deliberately narrow. Its shape is the
contract: `userId`, `displayName`, `emailVerified`, and the account and portfolio summaries. The
email address, provider subject, credentials, holdings, balances, valuations, transactions and
orders are not "filtered out" — they are never requested from their owners, so there is nothing to
leak by forgetting a filter.

## Entities referenced but not owned

| Entity | Owner | How U3 uses it |
|---|---|---|
| `User` | u1-user-registry | Reads the target's existence, status, display name, and verification flag; reads the requester's and approver's roles from validated claims |
| `Account` | Account service (BRD-003) | Reads the allowlisted summary fields through a new internal support-metadata endpoint |
| `Portfolio` | Portfolio service (BRD-004) | Reads id and label through a new internal support-metadata endpoint |
| `Role` | u2-authorisation / claims | Decides `support` for a request and `admin` for an approval; never inferred from another field |
| `AuditRecord` (C10) | Audit service | Destination of every `SupportAuditEvent`; append-only |

## Divergences from the catalogue, recorded

1. **`components.md` lists eight attributes on `SupportGrant`; this design keeps them and adds
   `revokedAt`** — the revocation timestamp is needed to tell a revoked grant from an expired one
   in the audit trail, and the existing table already carries the column.
2. **C5 lists five statuses; this design uses four.** `denied` is dropped (Q3).
   `components.md` is the second place the deny action appears: it gives `SupportAccess` a
   `deny` edge and says `AdminUsersUI` "approves or denies pending grants". No requirement defines
   a deny action, so the narrowing applies to both — the C5 and components.md edits are recorded
   together as open items, not just the contract one.
3. **`SupportMetadata` is not a stored entity and has no repository.** It is assembled per request;
   caching it would let a grant read a snapshot taken under a different (or expired) grant.
4. **The stored `pending_approval → expired` edge needs a schema change.** The existing
   `support_grants_approval_shape` constraint (`0001_init.up.sql`) requires `approved_at` for every
   non-pending status except `revoked`, so an unapproved request that ages out cannot be stored as
   `expired`. Migration `0007_support_access` (which also creates `support_audit_outbox`) relaxes
   that clause to allow `status = 'expired'` with `approved_by`, `approved_at`, `expires_at` and
   `revoked_at` all null, leaving the constraint's intent — an approved grant always has an approval
   time — intact. The matching down migration restores the original constraint.