# Logical Components — u1-walking-skeleton

This inventory is the bridge to Infrastructure Design: where each NFR pattern applies, the
failure domains, and the blast radius of each component.

| Logical component | Responsibility | Runtime | Failure domain | Blast radius if it fails |
|---|---|---|---|---|
| Gateway router | JWT validation, deny-by-default routing, rate limits, correlation ID | Stateless replica | One replica | One replica; Swarm restarts it |
| JWKS cache | Holds Supabase signing keys in memory | In-replica | One replica | Requests fail closed after the 10-minute staleness window |
| Principal resolver client | Calls Identity to resolve the caller | In-replica HTTP client | One replica | Requests needing a role fail; cached principals serve briefly |
| Registry API | Asset, chain, contract, source, quarantine operations | Stateless replica | One replica | One replica |
| Registry cache | In-memory asset resolution cache, polled change version | In-replica | One replica | Resolution falls through to the schema |
| Registry schema | Canonical registry data | Supabase Postgres, schema `registry` | The schema | Whole registry read/write path |
| Audit client | Emits audit events, bounded retry | In-replica HTTP client | One replica | Events fall back to the local structured log and an error metric |
| SPA shell | Minimal walking-skeleton UI | Static SPA | The SPA | Customer UI, not the API |

## Shared resources

- **Supabase Postgres** is shared by every service but isolated per schema and role; this Unit
  uses only `registry` and never another schema.
- **JWKS** is read-only and shared; an outage affects every gateway replica at once.
- **Swarm routing mesh** distributes traffic; a node failure moves replicas.

## Component isolation

- Each replica is stateless except its in-process cache; no local disk state.
- The release process versioned-migrates the schema before the new image takes traffic.
- The audit dependency never blocks a user request: it is fire-and-forget with bounded retry.

## Blast radius summary

| Failure | Scope | Degradation |
|---|---|---|
| One replica | single | none visible |
| Registry schema | all registry reads | callers keep last-known identity, labelled stale |
| JWKS source | all routing after 10 min | 503, fail closed |
| Audit store | audit trail | local log fallback plus error metric |
| Swarm node | replicas on that node | rescheduled |
