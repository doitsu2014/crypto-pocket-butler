# U1 Identity Auth Domain — Security Requirements

**Unit:** U1 `u1-identity-auth-domain`  
**Source:** Requirements Analysis NFR5, NFR6; FR3–FR8; BR-AUTH rules; Functional Design.

| ID | Requirement | Target / verification |
|---|---|---|
| NFR6.1 | Every protected Identity operation derives the caller from a validated Supabase JWT and internal user projection; caller-supplied subject, role, email-verification or MFA state is never authority. | Negative HTTP tests reject missing/invalid identity context and forged authority fields; disabled/deleted users are refused. |
| NFR6.2 | User-owned sessions and recovery records are protected by ownership checks, with privileged cross-user access limited to the existing admin/support-grant path. | Tests cover own-user access, another-user denial, authorized admin access and non-enumerating refusal. |
| NFR6.3 | Provider credentials, access/refresh tokens, TOTP seeds/codes, recovery codes and full device fingerprints are absent from logs, errors, URLs, analytics, support views, audit payloads and persistent Identity records. | Zero occurrences in dedicated redaction tests and reviewed logs; allowed one-time enrolment response is not persisted. |
| NFR6.4 | Security-sensitive Identity state changes and their sanitized audit intent commit atomically. | For each covered mutation, transaction tests show either both local state and outbox intent commit or neither commits. Delivery remains append-only through Audit. |
| NFR6.5 | Support-assisted recovery is non-enumerating and requires an authorized support actor plus a distinct authorized administrator under the current support-grant policy. | Known/unknown requester responses are indistinguishable; self-approval and unauthorized actions are refused and audited without revealing account existence. |
| NFR6.6 | MFA and exchange-credential policy use trusted provider assurance and trusted Identity state. | Tests prove single-factor users with an exchange credential are restricted; verified AAL2 lifts that restriction; client-asserted AAL or factor status has no effect. |
| NFR6.7 | Provider and notification integration failures are mapped to stable, safe RFC 7807 errors. | Problem responses include correlation id, stable type and safe field reasons; never include provider text, rejected values, keys or upstream identifiers. |
| NFR6.8 | Auth-specific public operations are protected by the gateway's stricter per-route rate budgets and Identity-side rate policies where specified. | Route/configuration checks verify auth limits are stricter than ordinary reads; repeated recovery and email-verification attempts return a safe 429 with retry guidance. |
| NFR6.9 | All provider-facing operations fail closed when validation or required provider state is unavailable; no break-glass or fallback authentication is added. | Fault-injection tests cover JWKS/provider unavailable and stale-key conditions; only already-validated, unexpired access tokens continue to work. |

## Security boundaries

- Supabase Auth owns passwords, email verification, token issuance/rotation, TOTP secrets and provider session validity.
- Identity owns application policy and its privacy-minimized device/session view, but not provider secrets.
- The gateway forwards the user's JWT unchanged; Identity independently validates it. The separate service token is used only for authorized internal operations.
- No shared deny-list, cross-schema data access, browser token persistence, or message queue is introduced.
