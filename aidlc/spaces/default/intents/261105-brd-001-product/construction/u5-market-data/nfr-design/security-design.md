# Security Design — u5-market-data

Inputs: `nfr-requirements/security-requirements.md`, `functional-design/rules.md` (BR5), and C4/C6/C10/C11.

## Authentication and authorization

- **C6**: per-caller service tokens in `Authorization: Bearer`. A SHA-256 hash lookup in
  `market_data.service_token_registry` yields the caller; the operation allow-list is
  Portfolio/Reporting/Job Orchestration for reads, plus the read-only `smoke-reader` caller used by
  the release smoke. A token that resolves to no caller gets 401; an
  operation outside its allow-list gets 403.
- **C11**: the shared internal run token from Vault; the job name is in the path
  (`price-refresh-top100`, `price-refresh-tail`).
- **C4**: Market Data is a client; it presents the shared internal service token to the Asset
  Registry.

## Secrets

- Provider API keys live in Vault. The `market_data` schema owns
  `read_provider_secret(source text)` as `SECURITY DEFINER`, granted to `market_data_svc`, so the
  service never holds a cross-schema grant.
- A key is decrypted only for the provider call and dropped; it is never logged, echoed in a
  problem document, or written to an observation.
- The manual refresh uses the Job Orchestration token. The C10 event records `actorKind: system`
  for a scheduled run and the service identity for a manual one, plus `trigger: manual|scheduled`
  in the context, so a human trigger is distinguishable.

## Transport and data at rest

- TLS terminates at the edge/CDN and gateway; the `internal` Swarm overlay is encrypted; provider
  and C4 calls use `reqwest` with rustls and HTTPS only. Postgres uses a TLS DSN
  (`sslmode=require`).
- Prices and rates are public data; the only secret at rest is the Vault reference.

## Input validation

- `assetIds` is non-empty and ≤ 1,000; currencies are three uppercase letters; `base != quote`.
- RFC 7807 problems carry a correlation id and no provider detail.
- A source payload that embeds a credential is discarded as a `data` rejection.

## Audit

- Failed runs write a C10 event with actor `system` (scheduled) or the service identity (manual),
  action `price_refresh_failed`, target the run id, context (job, source, trigger), and outcome.
  No key, token, or full URL appears.
- A failed audit write increments `market_data_audit_failures_total` and pages; it is never
  silently dropped.

## Threat considerations

- **Token replay**: tokens are per-caller and revocable by deleting the registry row.
- **Provider compromise**: a malformed or credential-bearing payload is rejected, never stored.
- **Price poisoning**: only configured sources are read; a source that returns an implausible price
  is still stored as an observation, but selection is by configured priority, and the admin can
  disable the source in C4.

NFR references: NFR9.1–NFR9.7, NFR12.1, NFR9.4.
