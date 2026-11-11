# Entities — u4-preferences-consent

**Upstream:** `unit-of-work.md` (U4 `u4-preferences-consent`), `components.md` (PreferencesConsent), `stories.md` (US6.1, US6.2, US6.5), `requirements.md` (FR19).

U4 owns two entities inside the Identity service's schema: the user's preferences, one row per user,
and the consent record, which is **append-only** — one row per grant or withdrawal, never updated.

## Source of truth

```yaml
entities:
  - name: UserPreferences
    description: One user's explicit choices. A field is null until the user sets it, so an explicit choice is always distinguishable from a default.
    attributes:
      - name: userId
        type: uuid
        required: true
        unique: true
        constraints: "primary identifier; one preferences row per user, created on first write"
      - name: locale
        type: enum
        required: false
        nullable: true
        allowed: [en, vi]
        constraints: "null means the user has never chosen; the effective value is then the default `en`"
      - name: baseCurrency
        type: enum
        required: false
        nullable: true
        allowed: [USD, EUR, GBP, VND]
        constraints: "null means never chosen; the effective value is then `USD`. A display and aggregation unit only — no stored financial value is re-based (BR4.9)"
      - name: timezone
        type: string
        required: false
        nullable: true
        constraints: "IANA zone name; null means never chosen, effective value `UTC`"
      - name: notifications
        type: object
        required: false
        nullable: true
        constraints: "keys restricted to `email` and `push`; a missing key is null, not enabled; an unknown key is refused (BR4.2)"
      - name: updatedAt
        type: timestamp
        required: true
    entity_constraints:
      - "A null field is an unset preference; a non-null field is the user's explicit choice and is never overwritten (BR4.1)."
      - "A partial write changes only the fields it supplies; every other field, set or null, is left as it is."
      - "A read before the first write returns the defaults with source `default` for every field and creates no row."
      - "A row belongs to exactly one user and is only ever read or written for its owner (BR4.3)."
    relationships:
      - to: User
        cardinality: one-to-one
        direction: "belongs to exactly one user; the user record is owned by u1-user-registry"

  - name: ConsentRecord
    description: One grant or one withdrawal of one consent kind, recorded as an immutable event.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
        constraints: "event identifier; assigned on insert and never changed"
      - name: seq
        type: bigint
        required: true
        unique: true
        constraints: "database-generated monotonic sequence; the ordering tie-breaker when two events share a timestamp"
      - name: userId
        type: uuid
        required: true
        constraints: "the consenting user; ownership checked on every read"
      - name: kind
        type: enum
        required: true
        allowed: [terms, risk, age, analytics]
        constraints: "the consent kind this event is about"
      - name: action
        type: enum
        required: true
        allowed: [granted, withdrawn]
        constraints: "the event itself; a row is never updated to represent both"
      - name: occurredAt
        type: timestamp
        required: true
        constraints: "assigned by the database clock on insert, never by a client and never by an application replica"
      - name: policyVersion
        type: string
        required: true
        constraints: "the version of the text the person acted on. A grant supplies it; a withdrawal carries forward the version of the newest grant for that kind, resolved server-side"
    entity_constraints:
      - "Append-only: an UPDATE or DELETE on a consent row is refused at the repository and by the schema's trigger."
      - "Current state for a kind is the newest event for that (user, kind), ordered by occurredAt then seq; there is no stored 'current' column to drift."
      - "Every event belongs to exactly one user."
    relationships:
      - to: User
        cardinality: many-to-one
        direction: "each event belongs to one user; the user record is owned by u1-user-registry"
```

## Summary

| Entity | Identifier | Purpose |
|---|---|---|
| UserPreferences | userId | The user's explicit locale, base currency, timezone, and notification choices; null means unchosen |
| ConsentRecord | id (+ seq) | One immutable grant or withdrawal event per consent kind |

## Decision recorded here

Consent is stored as events, not as state. A `granted`/`withdrawn` pair of rows answers both
"what is the state now" (the newest row) and "what happened and when" (the rows themselves), so
there is no separate history table to keep in sync and no `withdrawnAt` column that a failed
update can leave half-written. **This replaces the catalogue's `grantedAt`/`withdrawnAt` shape
on `ConsentRecord`**; the contract's C6 must carry the event shape, not the catalogue's
two-timestamp row, and the catalogue is corrected at the next domain-design pass rather than
silently diverging.

Preferences get their own table rather than columns on `users`: the user row belongs to u1's
registry, and a preferences write from this unit must not race a profile refresh from the
registry on the same row. Every preference column is nullable so "never set" is representable —
a default is applied on read, never written over a choice.
