# U1 Identity Auth Domain — Security Design

**Unit:** U1 `u1-identity-auth-domain`  
**Inputs:** `nfr-requirements/security-requirements.md`, `functional-design/functional-spec.md`, `functional-design/rules.md`, `inception/contract-design/contract-summary.md`. NFR coverage: NFR6.1, NFR6.2, NFR6.3, NFR6.4, NFR6.5, NFR6.6, NFR6.7, NFR6.8, NFR6.9.

## Trust boundaries and identity

1. The gateway validates the Supabase JWT and forwards it unchanged; Identity independently validates signature, issuer, audience, expiry and configured clock-skew allowance on protected routes.
2. Identity derives subject and role from the validated token and its own registry. Request bodies cannot supply caller subject, role, MFA level, email-verification status or device ownership as authority.
3. User-owned session and recovery operations enforce ownership at the Identity application boundary. Cross-user operations require the existing authorized admin/support-grant path and return non-enumerating refusals to unauthorized callers.
4. Supabase Auth is authoritative for passwords, email verification, token issuance/rotation, TOTP factors and provider session validity. Identity stores policy and minimal projections, not provider secrets.

## MFA and sensitive changes

- Require provider-verified AAL2 where the policy requires step-up. Exchange-credential users without a verified factor are restricted to MFA/settings capabilities; trusted-side policy—not SPA route state—enforces this.
- Return pending factor secret material only to the authorized enrolment response, once. Do not persist, log or audit secret, challenge code or recovery codes.
- Sensitive password/MFA/recovery-code changes require current second-factor proof. On success, revoke other sessions; retain only the initiating session. Failed proof changes nothing.

## Recovery and signup gates

- Signup decisions require trusted jurisdiction validation and affirmative explicit consent choices. Gate authorization is opaque, subject/attempt-bound and single-use.
- Recovery entry is public, rate-limited and non-enumerating. The case carries only protected contact reference and reason; identity re-proof evidence and credentials remain outside the application record.
- Require authorized support access and a distinct authorized administrator under the existing support-grant approval policy. Audit accepted and refused actions with sanitized context.

## Data protection and error mapping

- Use the project's RFC 7807 `application/problem+json` structure with stable type and correlation id. Field errors include names/reasons only, never rejected values.
- Prohibited everywhere in logs, errors, audit, analytics, URLs, support views and persistent Identity records: passwords, access/refresh tokens, TOTP seeds/codes, recovery codes, full device fingerprints, raw IP and full user-agent.
- Encrypt the stored provider subject using the existing per-user shredding-key practice; secrets and service credentials are sourced through Vault/Docker secrets and `*_FILE` settings.
- Security-sensitive state mutation and local audit outbox intent share one transaction. Audit delivery uses the existing append-only owner and bounded outbox retry, not a direct cross-schema write.

## Failure behavior

- Provider outage: no break-glass or cached-auth fallback. Continue only already-issued, independently validated access tokens until expiry; map new provider-dependent operations to a safe recoverable service-unavailable result.
- Provider revocation failure: retain local active session state; no audit success. Return a recoverable typed failure.
- Notification failure: do not fail sign-in; record/surface delivery failure without sensitive payload data.
- Maintain the accepted residual window: a revoked stateless access token may remain accepted until its expiry (up to one hour); the design does not add a shared deny-list.
