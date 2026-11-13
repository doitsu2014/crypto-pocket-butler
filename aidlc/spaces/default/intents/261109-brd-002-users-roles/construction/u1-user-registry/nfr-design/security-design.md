# Security Design — u1-user-registry

**Upstream:** `security-requirements.md` (NFR22.1–NFR19.1), `functional-spec.md`, `contract-summary.md` (C3).

## Trust boundaries

| Boundary | Trusted party | Rule |
|---|---|---|
| Gateway → registry (C3) | The API Gateway forwarding the user's JWT | The registry re-validates the JWT against JWKS; the subject comes only from the validated token |
| Service-to-service (u5 status transition) | The Identity service's own internal caller | Service token plus the caller allowlist; no public route |
| Admin lookup | `admin` role from `app_metadata.role` | Role checked from the token, never from the request |
| Registry → Postgres | `identity_app` role | Own schema only |
| Registry → Audit | Internal service | TLS, bounded retry |

## Data protection

- **Subject**: SHA-256 hash for lookup plus deterministic pgsodium ciphertext for the encrypt/decrypt path; plaintext exists only in memory during a request and is never logged, returned, or audited (NFR22.1).
- **Personal data**: email, display name, and email-verification status are personal data; they are returned only to their owner or to an admin within the metadata boundary.
- **No secrets**: BR1.5 and NFR9.1 forbid storing or emitting credentials, MFA secrets, or tokens.
- **Least privilege**: the API role has `SELECT/INSERT/UPDATE` on `users` and `shredding_keys` only (NFR23.1).

## Input validation

- Display name ≤ 200 characters; email in RFC 5322 shape; status in the allowed set; the subject hash is lowercase hex of length 64.

## Fail-closed paths

| Condition | Behaviour |
|---|---|
| Jurisdiction gate denies or is unavailable | Refuse provisioning; nothing stored |
| Hash collision (two subjects, one hash) | Refuse with a validation error, log a security event, create and return no record |
| Unknown status value | Refuse the write |
| Audit write fails | Surface the failure; do not report success |

## Threat considerations

- **Subject-existence probing:** the by-subject endpoint is internal-only and requires a validated token; an outsider cannot enumerate subjects.
- **Privilege via claim:** role is read only from `app_metadata.role`; a client-supplied role header is ignored.
- **Admin overreach:** the admin lookup returns the metadata boundary and never portfolio contents.
- **Hash collision:** treated as a security event and fail-closed.
