# Security Design — u1-walking-skeleton (Review artifact)

## Authentication and authorization

- **Gateway:** validates the Supabase JWT against JWKS (signature, issuer, audience, expiry)
  before any routing. JWKS is cached in memory; on a JWKS outage the cached keys serve for a
  **10-minute staleness window**, then requests fail closed with 503.
- **Forwarding:** the user JWT is forwarded unchanged; the registry validates it again.
- **Routing:** deny by default; only declared routes reach a service. Undeclared routes return
  404 and are audited.
- **Admin routes:** registry administration requires the admin role from `app_metadata.role`;
  a caller without it is denied and the denial is audited.

## Input validation

- Chain and contract addresses are validated against the launch coverage before any lookup.
- Symbol lookups are normalized (case, whitespace) with one documented rule.
- All request bodies are schema-validated at the `http` boundary; invalid input returns
  RFC 7807 with the field named.

## Transport

- TLS everywhere; plaintext rejected.
- HSTS at the edge.

## Secrets

This Unit reads exactly two secrets, both from Supabase Vault: the registry database role
credential and the JWKS configuration. It uses no provider credentials, no wallet keys, no
payment secrets. No secret appears in source, env files, logs, errors, or audit records.

## Data protection

- The registry holds public asset and contract data only. No user personal data.
- Audit records name the actor and the registry target by internal ID; a public contract address
  may appear as a registry target, but user wallet addresses never do.
- Logs never include tokens or credentials; the correlation ID is not a secret.

## Abuse protection

Rate limits at the gateway per user, IP, and region, with stricter limits for auth and manual
triggers; 429 carries `Retry-After`. Quarantine writes are additionally bounded per account.

## Supply chain

Dependency and container-image scans gate release; critical or high findings block without an
accepted risk.

## Security headers

`Strict-Transport-Security`, `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`,
`Content-Security-Policy: default-src 'none'` on API responses.
