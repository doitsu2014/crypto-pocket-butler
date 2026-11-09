# Reliability Requirements — u3-identity

Inputs: `functional-design/functional-spec.md`, `functional-design/rules.md`,
`inception/requirements-analysis/requirements.md`, and `inception/contract-design/contract-summary.md`.

## Targets

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR1.2 | Identity availability contributes to the product's 99.9% monthly target | Health and readiness checked; rolling update gated on the health check | NFR1 |
| NFR18.1 | RPO ≤ 24 h and RTO ≤ 4 h for the Identity schema | Supabase backup regime; restore rehearsed before launch | NFR18 |
| NFR14.1 | Financial records are retained for two years | Recorded as a labelled retention category on deletion | NFR14 |
| NFR14.2 | Audit logs are retained for six months | Audit service enforces the window; Identity records events only | NFR14 |
| NFR6.2 | Sign-up gate and first principal survive a provider blip | Bounded retry with an idempotency key; no partial user | NFR6 |

## Fault tolerance

- **Supabase Auth unavailable**: sign-in and MFA fail with a clear error; the session list serves the
  last known `DeviceSession` view with its last-activity time, and revoke is refused with the reason.
  No local fallback ever accepts an unverified identity.
- **Audit unavailable**: a security-sensitive read (support read) is refused rather than served
  unrecorded; ordinary reads log the drop and continue where the rule allows.
- **Export contribution fails**: the request is marked `failed` with the reason and can be retried;
  a partial archive is never presented as ready.
- **Deletion step fails**: the request is marked `failed` with the reason and stays retryable; a
  retry resumes the remaining steps without repeating completed destruction.
- **Worker dies mid-execution**: each worker claims a record with a lease (owner id +
  `lease_until`). A record whose lease has expired is re-claimed by a compare-and-set on the expired
  lease, so a restart or another instance resumes the remaining steps without a double execution.
  The status stays `building`/`in_progress` until the re-claim; the observability alert for a stuck
  record is "lease expired and no re-claim within two lease windows".

## Graceful degradation

- Principal resolution is the critical path: it degrades to a clear 503 for the gateway rather than
  serving an unverified principal.
- Consent reads degrade to "not granted" (deny) rather than serving a stale allow.
- Export and deletion are already asynchronous; a backlog delays them but does not fail the
  request path.

## Data durability

- The Identity schema is backed up under the project's Supabase backup regime; the per-subject
  crypto-shredding key is held in Vault with its own backup and access policy.
- A completed deletion destroys the key only after every service confirms anonymisation.
