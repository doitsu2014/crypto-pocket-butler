# Logical Components — u10-analytics

Inputs: `nfr-requirements/*`, `functional-design/functional-spec.md`.

| Logical component | Responsibility | Scaling | Failure domain |
|---|---|---|---|
| Analytics API | C12 ingest, flag reads/admin, aggregate reads, erase, health/metrics | 2 replicas | One task |
| Consent guard | Per-request C3 check for analytics-class events | In-process | C3 failure → 503, no unconsented accept |
| Pseudonymiser | HMAC-SHA256 under Vault salt versions | In-process | Missing salt → readiness fails |
| Aggregator | Nightly metrics, suppression, upsert | 1 worker | Missed run visible |
| Retention/erase worker | Batched prune, multi-version erase | 1 worker | Retryable |
| `analytics` schema | events, consent_checks, metric_aggregates, feature_flags | Supabase | Service-owned |
| C3 client | Consent | Per request | 503 on failure |
| C10 client | Flag/erase audit | Per event | Logged and counted |

## Boundaries

- No raw identifier, address, or credential is stored.
- The Vault salt is never stored in the analytics schema.
- No message queue; C11 runs aggregate and retention.

## Blast radius

- A failed API task affects its replica only.
- A missing salt blocks ingest (fail closed) but not flag reads.
- An erase failure is retried; aggregates are unaffected.

NFR references: NFR20.1, NFR9.4, NFR1.1.
