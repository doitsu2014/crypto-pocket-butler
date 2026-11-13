# Unit Story Map — Users, Roles & Permissions

Every upstream story is assigned to the unit(s) that implement it. Rows naming more than one unit are cross-cutting.

| Story | Unit(s) | Notes |
|---|---|---|
| US1.1 | U1 | One record per provider subject |
| US1.2 | U1 | Stable id across claim changes |
| US1.3 | U1 | Exactly one owner per user-owned resource; U2 supplies the ownership contract |
| US1.4 | U1 | Profile fields |
| US1.5 | U1 | No stored passwords, MFA secrets, or tokens |
| US1.6 | U1 | Recovery through the provider; no bypass |
| US2.1 | U2 | Deny by default |
| US2.2 | U2 | Role from `app_metadata.role`; missing value is Owner |
| US2.3 | U2 | Admin additive |
| US2.4 | U2 | Admin-only management |
| US2.5 | U2 | Ownership on lists, searches, exports |
| US2.6 | U2 | No existence disclosure |
| US2.7 | U2 | Disabled and deleted denied |
| US2.8 | U2 | Jobs never mix users |
| US2.9 | U2 | Server authoritative |
| US3.1 | U3 | Support metadata under a grant, audited |
| US3.2 | U3 | ≤60 minutes, different-admin approval, reason 1–500 |
| US3.3 | U3 | Metadata field boundary |
| US3.4 | U3 | No credentials, no holdings; no credential fields |
| US3.5 | U3 | Expired and revoked grants refused |
| US3.6 | U3 | Support is never an owner |
| US4.1 | U5; U1 | U5 owns transitions; U1 holds and reports the field |
| US4.2 | U5 | Admin disable without erasing history |
| US4.3 | U5 | Deletion request and 30-day recovery |
| US4.4 | U5 | Legal-hold-aware erasure per BRD-017 |
| US4.5 | U5 | Deleted denied; idempotent erasure |
| US5.1 | U2 | Ownership and no-double-count rule; the account/portfolio linkage mechanics remain in the existing Account and Portfolio services |
| US5.2 | U2 | Never link another user's portfolio |
| US5.3 | U2 | Connection removal honours ownership; the connection record itself belongs to the existing Account service |
| US6.1 | U4 | Preferences and consent |
| US6.2 | U4; U8 | Terms, risk-disclosure, and age acceptances recorded as consent kinds; U8 presents and names a missing acceptance |
| US6.3 | U6; U8 | Jurisdiction at sign-up and continued use |
| US6.4 | U6 | Restricted sign-up stores nothing |
| US6.5 | U4; U8 | Age self-declaration recorded |
| US7.1 | U10; U1; U2 | Admin user lookup and role change; U2 writes the role through the Supabase Auth admin API |
| US7.2 | U2; U5 | Role-change audit (U2) and status-change audit (U5) |
| US7.3 | U2; U3 | Denial audit and support-access audit |
| US7.4 | U2 | Audit content excludes credentials, tokens, and full addresses |
| US8.1 | N/A | Won't for the MVP; no unit implements shared portfolios |

## Coverage verification

- Every upstream ID (`US1.1`–`US7.4`) is assigned to at least one unit.
- Every unit appears on at least one row: U1 (US1.x, US4.1, US7.1), U2 (US2.x, US5.x, US7.x), U3 (US3.x, US7.3), U4 (US6.1, US6.2, US6.5), U5 (US4.x, US7.2), U6 (US6.3, US6.4), U7 — see note below, U8 (US6.2, US6.3, US6.5), U9 — see note below, U10 (US7.1).
- **Cross-cutting note:** the UI units U7 (`u7-settings-ui`) and U9 (`u9-support-console-ui`) present journeys whose acceptance criteria belong to the service rows above; they are the surface for US1.4, US4.3, US6.1 (U7) and US3.1–US3.6 (U9). Their presentation behaviour is traced in Functional Design, not as separate story rows.

## Implementation order within each unit

Each unit builds outward from its domain rules: domain tests first, then application use cases against port fakes, then adapters against a disposable Postgres schema, then the HTTP boundary. UI units build the component contract first (states and accessibility), then the screens, then the MSW-backed tests.
