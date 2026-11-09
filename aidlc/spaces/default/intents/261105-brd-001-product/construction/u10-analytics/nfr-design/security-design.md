# Security Design — u10-analytics

Inputs: `nfr-requirements/security-requirements.md`, `functional-design/rules.md`.

## Transport

- TLS terminates at the edge and gateway; the `internal` Swarm overlay is encrypted; C3 and C10
  calls use HTTPS; Postgres uses `sslmode=require`.

## Identity and authorization

- The gateway forwards the user's Supabase JWT unchanged. Analytics validates it itself against
  Supabase JWKS (signature, issuer, audience, expiry) and derives the subject and the role from
  `app_metadata.role`. `X-Auth-Subject` is a convenience header and must equal the token's `sub`;
  a mismatch is refused. Network isolation alone is never trusted as authentication.
- Flag reads: any authenticated subject. Aggregates and flag administration: `admin` role from the
  validated token.
- Erase: the caller's service token is resolved through `analytics.service_token_registry`; only
  the Identity caller's allow-list includes erase; another registered caller gets 403.

## Pseudonymisation

- `userRef = HMAC-SHA256(salt_version, auth_subject)`.
- Salt versions are 32 random bytes in Vault; the `analytics` schema exposes
  `read_analytics_salt(version text)` as `SECURITY DEFINER` granted to `analytics_svc`.
- A version is added only on suspected compromise; all versions are kept 12 months. Erasure
  computes the HMAC under every retained version.

## PII controls

- Closed event catalogue and per-event allow-lists.
- Values matching email, 0x/EVM, base58 32–44, IPv4/IPv6, or the secret regex are dropped; the
  offending `eventName`/`occurredAt` refuses the event.
- Logs never print a property value, userRef, or subject.

## Audit

- C10 events for flag create/update/delete, for an erase request, and for a refused-ingest category
  (consent refused, event rejected), with actor, action, target (flag key or userRef prefix),
  outcome; no PII.

## Supply chain

- The release pipeline runs `cargo audit` and a container-image scan on both images before deploy;
  a high or critical finding blocks the release (CICD stage).

NFR references: NFR9.1–NFR9.7, NFR12.1.
