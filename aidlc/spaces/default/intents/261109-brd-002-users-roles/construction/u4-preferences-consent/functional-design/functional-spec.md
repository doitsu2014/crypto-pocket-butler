# Functional Specification — u4-preferences-consent

Implements **US6.1**, and the recording half of **US6.2** and **US6.5** from `stories.md`.
U4 owns the user's preferences and the immutable consent event stream inside the Identity service.

## The consent gate is an operation of this unit

The design's centrepiece is a **consent-state query** that any caller can ask without reading
the event history itself:

```
GET /internal/users/{id}/consent/state        (C6)
→ 200 {
    "userId": "…",
    "kinds": { "terms": "granted", "risk": "withdrawn", "age": "granted", "analytics": "none" },
    "missing": ["risk"]                        // the kinds a source connection needs that are not granted
  }
```

`missing` always lists **every** required kind that is not currently granted — `terms`, `risk`,
and `age` — and nothing else; `analytics` is never in it. The same query is exposed to the
person's own surface as `GET /v1/me/consent/state` through the gateway, so the sign-up UI (U8)
can name a missing acceptance without re-deriving state from history.

**Who calls it.** The source-connection path is the existing Account service, which is outside
this intent; it calls C6's consent-state query and refuses the connection with the returned
`missing` list. The gateway exposes the same answer to the SPA for U8. This unit decides and
records; it does not connect anything and does not render.

## Workflows

### W0 — Answer the consent-state query

1. A caller asks for a user's consent state.
2. For each of the four kinds, the newest event decides (`granted`, `withdrawn`, or `none` when
   the user has recorded nothing) — ordered by the database timestamp then the monotonic
   sequence (BR4.5).
3. `missing` is computed: every one of `terms`, `risk`, `age` whose state is not `granted`.
4. The answer is returned. A read failure is a 503, never an empty or permissive answer.

**Error paths:** an unreadable store → 503; the caller never receives a state it cannot trust.

### W1 — Read and update preferences

1. A caller requests its own preferences.
2. **Before the first write** there is no row: the response carries every default with
   `source: "default"`, and no row is created by a read (BR4.1).
3. On a write, only the supplied fields are validated against their closed sets — locale
   `en`/`vi`, base currency `USD`/`EUR`/`GBP`/`VND`, timezone a known IANA zone, notification
   keys `email` and `push` only (BR4.2) — and only those fields change; every other field, set
   or null, is untouched.
4. The response carries each field's value **and** its source (`user` or `default`), so a caller
   can tell a choice from a default.
5. Nothing about the write rewrites a stored financial value (BR4.9).

**Error paths:** an out-of-set value or unknown notification key → 422 naming it, previous
value kept; another user's id → 404, indistinguishable from absent.

### W2 — Grant a consent

1. A caller submits a consent kind and the policy version it read; `policyVersion` is required on
   a grant.
2. The user record must exist (u1's registry) and belong to the caller.
3. One row is inserted: kind, `granted`, database timestamp, policy version, user id, sequence
   (BR4.8).
4. The response carries the kind and its new current state.

**Error paths:** unknown kind → 422; missing policy version → 422; an insert failure returns an
error and never a claimed success.

### W3 — Withdraw a consent

1. A caller submits a withdrawal for a kind — **no policy version is sent**.
2. The current state must be `granted`; withdrawing an already-withdrawn kind is a no-op that
   returns the current state without writing a duplicate event; withdrawing a kind never granted
   is refused with **409** naming the kind.
3. The new row carries the `policyVersion` of the newest `granted` event for that kind, resolved
   server-side (BR4.8), so a withdrawal can never exist without the version it undid.
4. The effect is immediate: the next consent-state query sees the newest event (BR4.7).

**Error paths:** never granted → 409; the audit entry is written for every withdrawal.

### W4 — Gate every source connection

1. The source-connection path calls W0 for the caller.
2. If `missing` is non-empty, the connection is refused and **every** missing kind is named
   (BR4.6, BR4.7). This applies to every attempt, not only the first: a user who withdraws terms
   after connecting one source is refused the next one.
3. If `missing` is empty, the connection proceeds; this unit makes no source record.

**Error paths:** a consent-state read failure fails closed — the connection is refused with an
error, never allowed on a guess.

### W5 — Read the consent history

1. A caller requests the consent events for its own account.
2. The events are returned newest first (`occurredAt`, then `seq`) with kind, action, timestamp,
   and policy version.
3. No event is ever modified or removed (BR4.4).

**Error paths:** another user's history is not reachable; the response is the same as for absent.

## Consent state per kind

```
(none) --grant--> granted --withdraw--> withdrawn --grant--> granted
```

There is no stored state column: the newest event *is* the state. A `withdrawn` kind may be
granted again, which inserts a new `granted` row rather than editing the old one. `analytics` is
optional: it never appears in `missing`, and its withdrawal only stops analytics emission (BR4.11).

## Error catalogue

| Condition | Status | Behaviour |
|---|---|---|
| Value or notification key outside its set | 422 | Field named; previous value kept |
| Unknown consent kind | 422 | Refused; nothing written |
| Missing policy version on a grant | 422 | Refused; nothing written |
| Withdrawal of a kind never granted | 409 | Current state returned; no event written |
| Another user's record | 404 | Indistinguishable from absent |
| Consent-state read fails | 503 | Fail closed; the connection is refused |
| Connection attempt with a missing acceptance | 422 | Every missing kind named |

## Integration points and contract changes required

- **C6 must gain** the consent-state query above; a consent-record schema (id, seq, kind, action,
  occurredAt, policyVersion); the 409, 422, 404 and 503 statuses this catalogue uses; and
  `account-api` (the source-connection path) added to its consumer list beside U7. Until that is
  written into `contract-summary.md`, C6 and this spec disagree, and the disagreement is recorded
  here rather than hidden.
- **`components.md`** still describes `ConsentRecord` with `grantedAt`/`withdrawnAt`; the
  append-only event shape in `entities.md` supersedes it, and the catalogue needs correcting at
  the next domain-design pass.
- **u1-user-registry:** the user must exist and belong to the caller before a preference or
  consent row is written.
- **The audit service (C9):** every withdrawal.
- **The analytics path:** reads `analytics` state from W0; U4 records the withdrawal, that path
  stops emitting.
