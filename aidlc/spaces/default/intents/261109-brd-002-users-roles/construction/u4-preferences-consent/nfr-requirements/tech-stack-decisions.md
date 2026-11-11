# Tech Stack Decisions — u4-preferences-consent

**Upstream:** `codekb/crypto-pocket-butler/technology-stack.md`, `functional-spec.md`, `project.md`, `services/identity/migrations/0001_init.up.sql`.

The stack is fixed by project rule. This file records what this unit adds, leans on, and **what already exists**.

## What already exists (checked against the migrations)

| Fact | Source | Consequence |
|---|---|---|
| `consent_records` already exists with `purpose` (analytics, marketing, disclosures), a `granted` boolean, `recorded_at`, `source` | `0001_init.up.sql` | Migration 0005 **alters** this table, it does not create it. See the mapping below. |
| `identity_app` already holds `SELECT, INSERT, DELETE` on `consent_records` | `0001_init.up.sql` | 0005 must `REVOKE UPDATE, DELETE FROM identity_app` explicitly; without that, NFR23.2's append-only claim is false. |
| `users` already carries `locale` (en/vi), `base_currency`, `timezone` as `NOT NULL DEFAULT` | `0001_init.up.sql` | These are u1's provider-derived profile fields (FR14.4). U4 must not create a second source of truth for them. |
| `chrono-tz` is declared in the workspace `Cargo.toml` but in no Identity crate | workspace manifest | One manifest change is needed in the crate that validates IANA zones. |

## Decisions

| Concern | Choice | Rationale | Alternative rejected |
|---|---|---|---|
| **Locale, base currency, timezone: one owner** | `users.locale`, `users.base_currency`, `users.timezone` stay **u1's** (provider-derived). U4's `user_preferences` holds the user's **explicit choice** for the same three names, nullable. The effective value is `COALESCE(user_preferences.<field>, users.<field>, <literal default>)` | Two stores already exist; the precedence rule makes them one value with one owner per layer, and "never set" survives the registry's non-null defaults | Copying the values into `user_preferences` and writing them on first read — rejected: it makes the two stores drift |
| **Who writes which** | u1 writes `users.*` from provider claims; u4 writes only `user_preferences.*`. Neither writes the other's columns | The registry refresh can never overwrite a choice, because it cannot reach the preferences row | — |
| **Consent migration** | `0005_preferences_consent` alters `consent_records` to the event shape, mapping existing rows, and creates `user_preferences` and `consent_audit_outbox` | The table exists; pretending it is new would fail at migrate time | Dropping and recreating — rejected: it destroys recorded consent |
| **Consent kinds** | `terms`, `risk`, `age`, `analytics`, **`marketing`** | `marketing` gets its own kind: folding it into `analytics` would let a marketing opt-out read as an analytics withdrawal, and the unit's output is consent evidence |
| **Row mapping** | an existing `(purpose, granted=true)` row becomes a `granted` event for the kind that maps one-to-one from `purpose` (`analytics`→`analytics`, `marketing`→`marketing`, `disclosures`→`terms`); `granted=false` becomes a `withdrawn` event; `recorded_at` becomes `occurredAt`; `source` is preserved in `policyVersion` as `legacy:<source>`. **One-to-one, so no two legacy rows can land on one kind and no collision rule is needed** | No recorded consent is lost, and the mapping is invertible for the down |
| **DDL steps in 0005** | 1. add `kind`, `action`, `occurred_at`, `policy_version`, `seq`; 2. backfill from `purpose`/`granted`/`recorded_at`/`source`; 3. `ALTER COLUMN purpose DROP NOT NULL`, drop its CHECK; 4. drop `source`'s CHECK; 5. add the four-value-plus-marketing `kind` CHECK and the `action` CHECK; 6. drop `consent_records_latest_idx` (keyed on `purpose`) and create the two new indexes; 7. give `id` a default (`gen_random_uuid()`) and `seq` a `bigserial`; 8. `REVOKE UPDATE, DELETE FROM identity_app`; 9. create `identity_retention`; 10. install the raising trigger | The existing constraints and index block the new shape; this is the exact order in which they are removed and replaced |
| **Down migration** | restores the old columns and shape, collapsing each (user, kind) to its newest event with the inverse one-to-one mapping; **events of a kind the old shape cannot hold (`risk`, `age`) are copied into `consent_records_legacy_archive` before the columns are narrowed**, so the down completes and the provenance survives. The archive table is created by the down and left in place | The release job's mandated rollback must be able to finish; failing loudly on real data would block it |
| **Append-only** | `REVOKE UPDATE, DELETE FROM identity_app`; a `BEFORE UPDATE OR DELETE` trigger raising for `identity_app`; a new `identity_retention` NOLOGIN role that alone may delete, for the erasure job | Privilege alone is one migration away from being widened; the trigger is the second lock, and the retention role is the one sanctioned exception (NFR22.3) | Application-level enforcement only — rejected: one bug would rewrite history |
| **Ordering** | `occurredAt` written by the database (`now()`), plus a `bigserial seq` column | Three replicas share one database clock; the sequence removes ties | Application timestamps — rejected: replica skew could invert state |
| **Consent-state query** | one `DISTINCT ON (kind) … ORDER BY kind, occurred_at DESC, seq DESC` read | One statement, four rows, no history scan | Reducing the full history in Rust — rejected: it grows with the account |
| **Audit durability** | `consent_audit_outbox` written in the same transaction as the consent event; a drainer posts to C9 with an idempotency key | The append-only row cannot be compensated, so the intent must commit with it (NFR19.1–19.3) | Calling C9 inside the request — rejected: a 202 does not prove an append, and a failure leaves an unauditable withdrawal |
| **Timezones** | `chrono-tz` for IANA validation | Already in the workspace | A hand-written zone list — rejected: it goes stale |

## Dependencies added by this unit

One manifest change: `chrono-tz.workspace = true` in the Identity crate that validates IANA
zone names. Everything else — `sqlx`, `chrono`, `serde`, `thiserror`, `tracing`, `reqwest` — is
already declared by the service.

## Constraint check

- Dependencies point inward: entities and rules in `domain`, use cases in `application`, repositories and the C9 client in `infrastructure`, DTOs and routes in `http`.
- One schema, one role (plus the new retention role, which exists only to delete on erasure); no cross-schema access; no RLS; no message queue.
- The migration ships as a versioned up/down pair, as project rule requires.
