# Security Requirements — u3-identity

Inputs: `functional-design/functional-spec.md` (WF1–WF9), `functional-design/rules.md`
(BR1.2, BR1.4, BR2.1–BR2.3, BR4.1–BR4.4, BR5.2–BR5.3, BR6.2–BR6.5, BR8.3),
`inception/requirements-analysis/requirements.md` (FR1, FR8, NFR9–NFR13, NFR19), and
`inception/contract-design/contract-summary.md` (C3, C10).

## Authentication

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR10.1 | Every request presents a Supabase JWT validated against JWKS (signature, issuer, audience, expiry) | Identity validates the forwarded JWT itself and derives the principal from it; the gateway also validates | NFR10 |
| NFR10.2 | Identity never returns or stores a token | The SPA's in-memory-only rule is delivered by u2-web-app; Identity keeps no token, factor, or recovery code | NFR10 |
| NFR10.3 | MFA is required for any user with a stored exchange credential | Session below AAL2 is refused portfolio data; enrolment before access | NFR10 |
| NFR10.4 | Identity stores no tokens, MFA factors, or recovery codes | Supabase Auth is the only holder | NFR10 |

## Authorization

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR11.1 | Identity exposes only the authenticated API plus the pre-auth sign-up gate; every other endpoint requires a validated JWT | The gateway applies the per-user/IP/region limits and the stricter auth limits | NFR11 |
| NFR11.2 | MFA attempts are throttled by Supabase Auth's auth rate limits | Configured in the Auth project, on the direct SPA-to-Auth path; Identity records the outcome | NFR11 |
| NFR10.5 | Role comes only from `app_metadata.role`; unknown or missing is `user` | No admin or support capability from any other source | FR1.5 |
| NFR10.6 | Ownership is enforced on every user-owned read and write | Not-found or forbidden; no data returned | FR1.6 |
| NFR10.7 | Support access is read-only, metadata-only, reason-recorded, admin-approved, and time-bound (60 minutes) | Distinct approver; every read audited before the answer | FR11.2 |

## Data protection

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR9.1 | TLS on all production traffic | No plaintext listener | NFR9 |
| NFR9.2 | No credential, token, or full address in responses, logs, errors, analytics, or support screens | Log redaction and a test per denial path | NFR9 |
| NFR9.3 | The per-subject crypto-shredding key lives in Supabase Vault | Identity never reads the key material; only refers to it | NFR9, NFR13 |
| NFR13.1 | Data minimisation: Identity stores only the fields its workflows need | Entity model is the allowlist | NFR13 |
| NFR13.2 | Consent is deny-by-default and no purpose is pre-ticked | ConsentRecord history | NFR13 |
| NFR13.4 | Deletion anonymises personal fields, keeps labelled legal-retention records, and destroys the key | Completion only after every service answers | NFR13 |

## Audit

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR19.1 | Every access, role, support, privacy, and export event is recorded with actor, action, target, timestamp, context, and outcome | Written through the Audit service before the answer where the rule requires it; a C10 change request makes actor, timestamp, and context mandatory | NFR19 |
| NFR19.2 | Audit records carry no credentials, tokens, or full addresses | Context limited to identifiers | NFR19 |
| NFR19.3 | Audit storage is append-only through an insert-only role | Identity writes events only through the Audit API | NFR19 |

## Threat considerations

- **IDOR**: every user-owned record is read by owner id derived from the principal, never from a
  caller-supplied id alone (BR1.4).
- **Privilege escalation**: role is taken from the signed claim only; support grants require a
  distinct admin approver, so a support user cannot widen their own access (BR4.1).
- **Support overreach**: reads are read-only and metadata-only, time-bound, and audited before the
  answer (BR4.2–BR4.4).
- **Jurisdiction bypass**: the gate must be enforced server-side before Supabase Auth creates an
  account; a client-only check is not sufficient (see the open contract gap in the functional
  design review).
- **Deletion leakage**: deletion removes or anonymises the Supabase Auth identity and factors as
  well as the internal `User`, or the person remains identifiable (open finding R-07).
- **Export leakage**: the archive is owner-only, contains no secrets, and its signed link expires
  (BR5.2–BR5.3).

## Supply chain

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR12.1 | Dependency and container-image vulnerability scanning before release | Fails the release on a known critical | NFR12 |
