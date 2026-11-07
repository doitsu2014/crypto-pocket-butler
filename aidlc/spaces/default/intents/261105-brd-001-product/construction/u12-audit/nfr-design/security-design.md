# Audit — Security Design

## Defence in depth, layer by layer

```
Swarm overlay network + TLS
  └── mTLS/service token: only registered services may append
       └── Supabase JWT validated in-process for every operator path
            └── role check: admin (query/export/revoke), support (sessions)
                 └── scope check: the record belongs to this caller
                      └── Postgres grant: the role's columns and statements
                           └── trigger on the workflow records: legal transitions only
```

Each layer refuses on its own. A bug in the handler does not become a write the database accepts.
The innermost trigger covers the workflow-record tables; on the trail itself the grant is the
innermost control, and `audit_retention` is the only grant that can delete there.

## Authentication

| Path | Mechanism |
|---|---|
| Append (`POST /v1/events`) | Internal service token (the C11 scheme). The token identifies the emitting service, which is also half of the idempotency key. No user JWT is accepted here. |
| Operator paths | Supabase JWT validated against JWKS in-process (signature, issuer, audience, expiry), then `app_metadata.role`. |
| Retention sweep | Its own credential from Supabase Vault, held only by the retention service. |

## Authorisation

- Deny by default: a route with no explicit role rule is not served.
- `admin` — audit query, export, session revocation.
- `support` — create a session request naming a subject; read only within an active session whose
  scope matches.
- `actorKind: system` is accepted only with no `actorId` and only for the system-action allow-list
  (job runs, retention runs, scheduled syncs); both together, or an action outside the list, is
  `422`. The entity model's `system` default is a schema convenience, not a permission: the service
  sets it only on those paths.
- Every denial writes a security `AuditEvent` before the `403` goes out.

## Data protection

- TLS everywhere, including service-to-service inside the Swarm.
- No encryption of the audit trail beyond the database's own at-rest encryption: an audit record
  that only some key holders can read is not much of an audit record. This is a deliberate
  trade-off, and it is why BR1.3 forbids secrets in the trail in the first place — the trail is
  readable by design, so nothing secret may enter it.
- Vault holds the three role credentials; nothing is read from an env file.

## Rate limiting and its counter store

The gateway is the primary control and it already holds per-user and per-IP counters. The service's
own per-caller limits — 30 admin queries a minute, 5 exports an hour, 1 000 appends a minute per
emitting service — are enforced per replica, because replicas share no cache and the project
forbids adding one. The honest consequence is that the effective ceiling is three times the stated
figure at three replicas, and the stated figures are therefore chosen as a third of the intended
per-user ceiling. This is written down rather than glossed over: a shared counter store would mean
a cache tier or a counter table on the write path, and neither is worth the availability cost for a
control the gateway already provides.

Exports are additionally counted in `audit_exports` itself (`requestedBy`, `requestedAt`), so the
export-spike alert queries that table — a per-caller label on a metric would break the
low-cardinality rule and put a user id in the observability stack, which the logging discipline
forbids.

## Size bounds on free text

`context` is capped at 2 KB and at the five allowed keys (correlation id, request id, source
service, rule id, session or run id); `target` at 240 characters; `reason` and `failureReason` at
500. The append handler refuses anything larger with `422` before the insert, so an emitter cannot
turn the trail into a payload store.

## Input validation — the secret detector

One shared helper, used on every free-text and map field before an insert:

| Shape | Detection |
|---|---|
| EVM / BSC address | `0x` + 40 hex |
| Solana address | base58, 32–44 chars, decodes to 32 bytes |
| JWT | three dot-separated base64url segments |
| API key | the project secret-prefix list |
| Seed phrase | 12 or 24 space-separated words from the word list |
| DSN / connection string | scheme `://`, userinfo, or `password=` |

Rejection names the field path and never the value. `failureReason` gets the same helper with a
replacement marker rather than a rejection, because the run still has to record that it failed.

## The append-only guarantee

- `audit_app`: `INSERT`, `SELECT` on `audit_events`; no `UPDATE`, no `DELETE`.
- `audit_readonly`: `SELECT` only.
- `audit_retention`, used only by the separate retention service: `SELECT` and `DELETE` on
  `audit_events` and its partitions, `CREATE TABLE … PARTITION OF` on `audit_events` (so it can
  pre-create partitions), `INSERT` on `audit_events` (its own run events and its privacy-deletion
  events), and `INSERT`/`UPDATE` on `retention_runs` and `privacy_deletions`. It holds no grant on
  support sessions or exports. `audit_app` holds `INSERT` on `retention_runs` and
  `SELECT, INSERT` on `privacy_deletions` only so the report and deletion-request paths can
  record what they received; insert-shape triggers refuse any row other than a completed
  `reported` run (owner not audit) or a `requested` deletion with zero counts, and `audit_app`
  holds no `UPDATE` or `DELETE` on either. The append-serving credential therefore cannot forge
  an `audit_logs` run or a completed deletion. The audit-log sweep also removes the
  `append_idempotency` rows whose event is past the same window, so the idempotency table is
  bounded with the trail.
- Workflow-record tables: column-scoped `UPDATE` plus a trigger that refuses a write to a row
  already in a terminal state, so a completed run or export is immutable too.
- `audit_events` carries no trigger, because no role has `UPDATE` or `DELETE` on it except
  `audit_retention`. That is the honest statement of the layering: on new events the grant **is**
  the control, and the retention role's use of its `DELETE` is what the `RetentionRun` record makes
  visible. The trigger is a second control on the workflow records only.

## Privacy deletion

A deletion request removes in-window records through the retention service, not through the
append-serving service: the retention credential is the only one with `DELETE`, so a privacy
deletion is an audited operation performed by the component that already holds the destructive
grant. Each deletion writes a `PrivacyDeletion` row and an audit event naming the request, the
count deleted, the count kept, and the legal basis of everything kept. A record with a
`legalBasis` is never deleted by this path.

## Threat model summary

| Threat | Control |
|---|---|
| Forged or downgraded append | Service token, per-token rate limit, server-derived event class |
| Trail tampering | Insert-only grant, separate retention credential and workload |
| Support over-reach | Second approver, scope, expiry, per-read recording, deny when the check cannot answer |
| Export exfiltration | Recorded before produced, rate-limited, capped, deleted after 24 h |
| Secret leakage into the trail | Pre-insert detector on every free-text field; error text sanitised |

## Sources

- `construction/u12-audit/nfr-requirements/security-requirements.md` — NFR9.1 to NFR19.4
- `construction/u12-audit/nfr-requirements/reliability-requirements.md` — NFR1.4, NFR1.5, NFR1.7
- `construction/u12-audit/functional-design/rules.md` — BR1.1, BR1.3, BR3.1, BR3.2, BR5.1
- `construction/u12-audit/functional-design/functional-spec.md` — WF1 to WF5, append authentication
- `inception/contract-design/contract-summary.md` — C10, C11
