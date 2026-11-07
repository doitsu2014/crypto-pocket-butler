# Reliability Design — u3-identity

Inputs: `nfr-requirements/performance-requirements.md`, `security-requirements.md`,
`scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`,
`tech-stack-decisions.md`, and `functional-design/functional-spec.md`.

## Resilience patterns

| Dependency | Pattern | Configuration |
|---|---|---|
| Supabase JWKS | Cache + retry | 1 h refresh; unknown `kid` triggers one refetch; 3 attempts, 1 s connect / 5 s read |
| Account, Ledger, Portfolio (export/anonymise) | Timeout + retry + isolation | 1 s connect / 5 s read / 30 s total; 3 attempts with jitter; failure marks the request failed, never a partial success |
| Supabase Auth admin | Timeout + retry | 1 s / 5 s / 30 s; 3 attempts; revoke refused on failure |
| Audit (C10) | Timeout, no silent drop | 100 ms p95 / 200 ms timeout on the support-read path; on exhaustion the read is refused with 503 |
| Supabase Storage | Timeout + retry | 1 s / 5 s / 30 s; 3 attempts |

No circuit breaker library is added at launch; the bounded timeout plus the gateway rate limit keep
a failing dependency from exhausting the pool, and the worker isolates failures per record.

## Retry policy

- Exponential backoff with jitter: 100 ms, 200 ms, 400 ms; at most 3 attempts.
- Only transient failures are retried (timeout, connection reset, 5xx, 429). A 4xx is not
  retried.
- Every side-effecting call carries an idempotency key so a retry applies once.

## Health checks

- `GET /healthz`: process liveness, no dependency.
- `GET /readyz`: Postgres reachable; Vault reachable for the key reference. The gateway removes a
  task that fails readiness.

## Graceful degradation

- Supabase Auth unavailable: sign-in/MFA fail clearly; the session list serves the last known
  `DeviceSession` view; revoke is refused.
- Audit unavailable: support reads are refused; ordinary reads log the drop where the rule allows.
- Export contribution fails: request `failed` with the reason; retry opens a new execution.
- Deletion step fails: request `failed` and retryable; the lease lets another worker resume.
- Consent read unavailable: deny (not granted), never a stale allow.

## Backup and recovery

- RPO ≤ 24 h / RTO ≤ 4 h: Supabase backup regime for the Identity schema; Vault key backup under
  its own policy.
- A restore rehearsal runs before launch.
- Deletion destroys the key only after every service confirms anonymisation, so a restore cannot
  resurrect the person's Identity fields after a completed deletion.

NFR references: NFR6.2, NFR10.7, NFR14.1, NFR14.2, NFR18.1.
