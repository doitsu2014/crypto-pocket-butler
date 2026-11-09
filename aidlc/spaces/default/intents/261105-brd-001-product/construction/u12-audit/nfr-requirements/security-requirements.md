# Audit — Security Requirements

This is the unit the rest of the system trusts to keep an honest record. Its own security is
therefore part of everyone else's threat model.

## Authentication and authorisation

| ID | Requirement |
|---|---|
| NFR9.1 | Append callers authenticate with the internal service token issued to a registered service (the contract C11 scheme), never with a user JWT. A caller without a registered token is refused before its payload is examined. |
| NFR9.2 | Operator paths require a validated Supabase JWT: `app_metadata.role = admin` for queries, exports, and revocation; `support` for support sessions. Anything else is denied by default. |
| NFR10.1 | Every JWT is validated against Supabase JWKS — signature, issuer, audience, and expiry — inside the audit service itself; the gateway's validation is not trusted on its own. |
| NFR10.2 | `actorKind: system` is accepted only when the payload carries no `actorId`, and only for actions on the system-action allow-list (job runs, retention runs, scheduled syncs). A payload with both an `actorId` and `actorKind: system` is refused with 422; an action outside the allow-list is refused the same way. |

## Data protection

| ID | Requirement |
|---|---|
| NFR9.3 | TLS on every hop, including service-to-service calls inside the Swarm. |
| NFR9.4 | No stored value in this schema carries a credential, a token, a seed phrase, or a full wallet address. The detector named in BR1.3 runs before the insert on every free-text and map field the schema stores, not only `AuditEvent.target` and `context`: it also covers `AuditExport.filter` (whose target is operator-supplied), `SupportAccessRecord.reason`, and `RetentionRun.failureReason`. A rejected event is never stored in a rejected-payload table. |
| NFR9.6 | Error text stored in `RetentionRun.failureReason` is sanitised before it is written: connection strings, passwords, tokens, and URLs with embedded credentials are replaced by a fixed marker, keeping the failure's shape without its secrets. |
| NFR13.1 | `context` is limited to correlation id, request id, source service, rule id, and session or run identifiers; the schema rejects a context larger than 2 KB. |
| NFR9.5 | Nothing in the service logs an event payload, a token, or a JWT. Log lines carry the correlation id and the event id only. |
| NFR13.2 | An error response never echoes the offending value, only the field path. |

## Storage roles (enforced in the schema, not in code alone)

The schema has two surfaces, and the role matrix treats them differently: the **trail** is
append-only, and the **workflow records** move through their state machines.

| Surface | Tables | Rule |
|---|---|---|
| Append-only trail | `audit_events` and its partitions | `INSERT` and `SELECT` only, for every role except retention |
| Workflow records | `support_access_records`, `retention_runs`, `audit_exports` | `INSERT`, `SELECT`, and a column-scoped `UPDATE` that a trigger limits to legal transitions |

| Role | May |
|---|---|
| `audit_app` | `INSERT` and `SELECT` on `audit_events`; `INSERT`, `SELECT`, and column-scoped `UPDATE` on the workflow-record tables — no `DELETE` anywhere |
| `audit_readonly` | `SELECT` only, for operator queries and exports |
| `audit_retention` | `DELETE` on `audit_events` partitions only, and only through the retention procedure |

Column-scoped `UPDATE` means the grant names columns: `support_access_records` may change `state`,
`approvedAt`, `endedAt`, `outcome`; `retention_runs` may change `state`, `finishedAt`,
`deletedCount`, `outcome`, `failureReason`; `audit_exports` may change `state`, `completedAt`,
`rowCount`, `outcome`, `fileRef`, `expiresAt`. A trigger refuses any other column and any write to
a row already in a terminal state, so a completed run or export is as immutable as an audit event.

| ID | Requirement |
|---|---|
| NFR19.1 | `audit_events` holds no `UPDATE` and no `DELETE` for the application or read-only role; the database refuses the attempt even if application code is wrong. |
| NFR19.2 | The retention role is separate from the append role, exists for nothing but deleting past-window partitions, and every use of it is recorded. The sweep runs as its own Swarm service with its own credential from Vault; the append-serving service holds no retention credential and cannot delete. |
| NFR19.3 | No role granted here reaches another service's schema, and the service holds no grant on one. |
| NFR19.4 | One-role-per-service is the project rule, and the audit service is its sanctioned exception: the insert-only-audit-role requirement cannot be met with a single role that also deletes. The exception is recorded here and in the infrastructure design; each of the three roles gets its own credential from Supabase Vault, and none can be used from another service. |

## Supply chain and abuse

| ID | Requirement |
|---|---|
| NFR12.1 | Dependency and container-image vulnerability scanning runs before release, and a release does not proceed on an unresolved high-severity finding. |
| NFR11.1 | Rate limits are enforced at the gateway per user and per IP. The audit service additionally limits admin queries to 30 per minute per caller and exports to 5 per hour per caller, because an export is the cheapest way to pull a large slice of the trail out. |

## Threat considerations

- **Forged append.** A compromised service could inject events. Mitigation: per-service tokens,
  the source service recorded in every event, and an append rate per token that a single
  compromised service cannot silently exceed.
- **Trail tampering.** Erasure needs the retention credential, which the append-serving process
  does not hold: the sweep runs as its own Swarm service with its own Vault credential, and every
  use of it is recorded. A stolen application credential therefore cannot erase history. The
  residual risk is stated rather than hidden: whoever can deploy a new service definition can mount
  the retention credential, so the separation is an operational control backed by deployment
  rights, not a cryptographic one. Database backups are the recovery path if that boundary is
  crossed.
- **Support over-reach.** The session check is the single gate for support reads; it is answered
  only for an active session inside its scope, and a check that cannot be answered is a deny.
- **Export exfiltration.** Every export is recorded before it is produced, is capped in size and
  rate, and expires from storage after 24 hours.

## Sources

- `construction/u12-audit/functional-design/functional-spec.md` — WF1 through WF7, and the append-authentication paragraph in section 3
- `construction/u12-audit/functional-design/rules.md` — BR1.1 to BR1.3, BR3.1 to BR3.3, BR5.1, BR5.2
- `inception/requirements-analysis/requirements.md` — NFR9, NFR10, NFR11, NFR12, NFR13, NFR19
- `inception/contract-design/contract-summary.md` — C10 and the C11 service-token scheme
