# Security Design — u3-identity

Inputs: `nfr-requirements/performance-requirements.md`, `security-requirements.md`,
`scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`,
`tech-stack-decisions.md`, and `functional-design/functional-spec.md`.

## Authentication architecture

1. The SPA signs in through Supabase Auth (C13) and holds the access token in memory only.
2. Every gateway call forwards the JWT unchanged.
3. **Identity validates the JWT itself** against Supabase JWKS: signature, issuer, audience, and
   expiry. A failure returns 401 and writes nothing. The principal it returns is derived from that
   validated token.
4. MFA is Supabase Auth's: the `aal` claim and `app_metadata.has_exchange_credentials` are read
   from the validated JWT. Identity refuses portfolio data while an exchange-credential user is
   below `aal2`.
5. The pre-sign-up gate returns a short-lived signed gate token (HS256 with a key in Vault);
   Supabase Auth's pre-sign-up hook verifies it. A forged or expired token is refused.

## Authorization model

- Role is taken only from `app_metadata.role` (`user`, `admin`, `support`); unknown or missing is
  `user`. No other source grants a capability.
- Every user-owned record is read by the owner id derived from the principal, never from a caller
  id alone (BR1.4).
- Support grants require a reason and a distinct admin approver; the metadata read carries the
  `grantId`, and the grant must be active and unexpired. Reads are metadata-only and audited before
  the answer.
- Admin endpoints are deny-by-default; the gateway checks the role and Identity re-checks it.

## Encryption

| Data | Protection |
|---|---|
| In transit | TLS on every production listener and every internal call |
| Identity personal fields | Encrypted and decrypted inside Postgres by a SECURITY DEFINER function (`identity.protect` / `identity.unprotect`) backed by pgsodium and a Vault key id; Identity passes the key id only, and the service role has no direct key-material read |
| Other services' retained records | Outside this key; retained under their own legal basis |
| Export archive | Private Supabase Storage bucket, encrypted at rest, owner-only signed URL, 24 h |
| Gate token | Signed, short-lived, verified by the pre-sign-up hook |

## Secrets management

- All secrets come from Supabase Vault or the Swarm secret mechanism; never from env files or
  source.
- Credentials, tokens, and full addresses are never logged, returned, or written to the audit
  trail; log fields are allow-listed.
- The crypto-shredding key is referenced by id only; Identity never reads its material.

## Input validation

- Validate at the system boundary: email/age/jurisdiction codes, preference enums, ISO 4217 and
  IANA names, pagination cursors, and every id.
- Reject unknown fields and over-long strings; return RFC 7807 `application/problem+json` with a
  `correlationId`.
- Rate-limit the pre-auth gate and the MFA path (Supabase Auth's own limit).

## Audit and compliance

- Every access, role, support, privacy, and export event is written through C10 with actor,
  action, target, timestamp, context, and outcome; the C10 amendment makes those fields required.
- The audit trail is append-only through an insert-only role; existing rows are never rewritten.
- Export, deletion, consent, and retention controls implement NFR13/NFR14; the deletion destroys
  the Auth identity and the key and labels retained categories.

## Defense in depth

- Gateway and Identity both validate the JWT; Identity re-checks every authorization decision.
- Failure domains are separate from the data store and Audit; a compromised token gives only the
  user's own records until expiry.
- No endpoint accepts a caller-supplied actor identity for a user-owned write.

NFR references: NFR9.1, NFR9.2, NFR9.3, NFR10.1, NFR10.2, NFR10.3, NFR10.4, NFR10.5, NFR10.6,
NFR10.7, NFR11.1, NFR11.2, NFR12.1, NFR13.1, NFR13.2, NFR13.4, NFR17.1, NFR17.2, NFR17.3,
NFR17.4, NFR17.5, NFR19.1, NFR19.2, NFR19.3.
