# Units of Work — Users, Roles & Permissions

Ten fine-grained units, one per domain component and one per UI surface. Each service-side unit is a slice inside the existing Identity service's four-crate Clean Architecture; each UI unit is a feature module inside the SPA or the support console. Topology only — Delivery Planning chooses the build order.

**Upstream:** `components.md`, `decisions.md`, `requirements.md` (FR14–FR20, NFR21–NFR26), `stories.md` (US1.1–US8.1).

## Unit definitions

| Unit ID | Directory | Kind | Complexity | Deployment model |
|---|---|---|---|---|
| U1 | `u1-user-registry` | service | M | Embedded in the Identity service (`services/identity`) |
| U2 | `u2-authorisation` | service | M | Embedded in the Identity service |
| U3 | `u3-support-access` | service | L | Embedded in the Identity service |
| U4 | `u4-preferences-consent` | service | S | Embedded in the Identity service |
| U5 | `u5-account-lifecycle` | service | L | Embedded in the Identity service |
| U6 | `u6-jurisdiction-gate` | service | S | Embedded in the Identity service |
| U7 | `u7-settings-ui` | ui | M | Embedded in the SPA build (`web/`) |
| U8 | `u8-signup-gates-ui` | ui | S | Embedded in the SPA build (`web/`) |
| U9 | `u9-support-console-ui` | ui | M | Embedded in the support console build (`support-console/`) |
| U10 | `u10-admin-users-ui` | ui | M | Embedded in the support console admin section |

## Responsibilities

### U1 — User Registry (`u1-user-registry`, service)

- One internal user record per provider subject; stable internal id; display name, email, and email-verification status; the status field with its transition guard.
- Subject-hash lookup and deterministic subject ciphertext; shredding key reference creation before the data it protects.
- Its own `/v1/principals` and `/v1/users/{id}` route declarations in the gateway route table.
- **Constraints:** never stores passwords, MFA factors, or tokens; writes a record only after the jurisdiction gate clears; no direct status mutation outside U5.

### U2 — Authorisation (`u2-authorisation`, service)

- Role mapping from `app_metadata.role`; deny-by-default; ownership contract; denial evidence for audit.
- **Constraints:** no stored role entity; a missing or unrecognised role is a plain Owner; denials surfaced to the audit path.

### U3 — Support Access (`u3-support-access`, service)

- Grant request, distinct-admin approval, 60-minute maximum window, revocation, expiry; the metadata projection boundary.
- **Constraints:** never returns holdings, balances, values, transactions, orders, credentials, or secrets; every request, approval, access, and refusal is audited.

### U4 — Preferences & Consent (`u4-preferences-consent`, service)

- Locale, base currency, timezone, notification preferences; consent grant and withdrawal history.
- **Constraints:** provider defaults never overwrite a user's explicit choice; a withdrawal is visible to later stages.

### U5 — Account Lifecycle (`u5-account-lifecycle`, service)

- The four-status machine; admin disable/restore without erasing history; deletion request, 30-day recovery, erasure subject to legal holds; idempotent erasure.
- **Constraints:** the only unit allowed to request a status transition; erasure acts through the shredding key.

### U6 — Jurisdiction Gate (`u6-jurisdiction-gate`, service)

- Restricted-jurisdiction list; self-declared country plus IP geolocation; sign-up and continued-use decisions with evidence.
- **Constraints:** a blocked attempt stores nothing; the stricter signal wins.

### U7 — Settings UI (`u7-settings-ui`, ui)

- Profile, preferences, consent, and deletion surfaces; the role badge and status pill; the blocked-account screen for pending deletion.

### U8 — Sign-up Gates UI (`u8-signup-gates-ui`, ui)

- Terms, risk disclosure, age, and country gates; the blocked-jurisdiction notice; missing-acceptance validation.

### U9 — Support Console UI (`u9-support-console-ui`, ui)

- Grant request, active-grant banner, metadata panel; read-only presentation.

### U10 — Admin Users UI (`u10-admin-users-ui`, ui)

- User search, role and status changes with reason, grant approval; the same metadata boundary as support.

## Implementation notes

- Every service unit follows the four-crate rule (`domain`, `application`, `infrastructure`, `http`) and adds its own routes to the gateway's deny-by-default table.
- Every UI unit uses the single API-client module and the Parchment Ledger tokens; the three shared additions (RoleBadge, StatusPill, GrantBanner) ship with U7 and are reused by U9 and U10.
- Every unit lands with tests in the same change (TDD), including one integration test per adapter against a disposable Postgres schema.
- No unit introduces a new transport, a shared crate, or cross-schema access.
