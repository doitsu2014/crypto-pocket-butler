# Security Design — u4-account

Inputs: `nfr-requirements/performance-requirements.md`, `security-requirements.md`,
`scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`,
`tech-stack-decisions.md`, and `functional-design/functional-spec.md`.

## Authentication and authorization

- **User path (C1)**: the Supabase JWT is validated by Account itself against JWKS (signature,
  issuer, audience, expiry); `aal2` is required to store an exchange credential.
- **Internal path (C5, C11)**: each calling service holds its own token in Vault (identity,
  portfolio, ledger, job-orchestration). It presents `Authorization: Bearer <token>`; a
  service-token registry maps the token to the caller identity, and each operation has an
  allow-list (Identity for privacy, Portfolio/Ledger for holdings, Job Orchestration for the run
  endpoint). One shared secret is not used.
- **Outbound C15**: the Supabase Auth admin call authenticates with the service-role key from
  Vault; it is an outbound dependency, not an inbound path.
- Ownership: a user-owned read needs the owner or an active, reason-recorded support grant verified
  through the C3 grant check. No blanket admin.

## Credential protection

- The key is stored only as a Vault secret reference. It is decrypted into process memory for the
  provider call and dropped immediately; it is never persisted outside Vault, returned, or logged.
- Verification: a read-only balances call and a permissions check on add and every 6 hours (±10%
  jitter, worst-case exposure 6 hours); a key with withdrawal or trading permission is rejected and
  its secret destroyed.
- Account maintains `app_metadata.has_exchange_credentials` through the Supabase Auth admin API and
  clears it on rejection, removal, or revocation.

## Encryption and transport

- TLS terminates at the edge/CDN and gateway; the internal Swarm overlay is encrypted.
- Wallet addresses are public but are never written to an audit record; only the account id and
  label are.

## Input validation

- Validate venue, chain (C4 supported-chains read), address format, labels, CSV headers and rows,
  and every id at the boundary; RFC 7807 errors with a correlation id.
- CSV parsing never executes file content; a bad row is an `ImportRowError`, not a crash.

## Audit

- Credential, account, sync, and import events are written through C10 with actor, action, target,
  timestamp, context (identifiers only), and outcome.

## Threat considerations

- **IDOR**: owner derived from the principal; internal callers by service identity.
- **Credential leak**: Vault-only storage, in-memory use, redacted logs.
- **Support overreach**: metadata-only, grant-gated, audited.
- **Malicious CSV/provider data**: defensive parsing and registry resolution; spam labelled.

NFR references: NFR9.1–NFR9.5, NFR10.1–NFR10.6, NFR13.1–NFR13.3, NFR17.1, NFR17.2, NFR17.3,
NFR17.4, NFR17.5, NFR19.1–NFR19.3.
