# Security Requirements — u2-authorisation

**Upstream:** `functional-spec.md` (W1–W5), `rules.md` (BR2.1–BR2.14), `requirements.md` (FR15, FR20, NFR22–NFR24), `contract-summary.md` (C4).

## Authentication

| ID | Requirement |
|---|---|
| NFR9.1 | Every service validates the provider token against Supabase JWKS — signature, issuer, audience, expiry — before any decision. The token is forwarded unchanged from the gateway (project rule). A failure is a 401 produced by the caller's middleware, never by the decision. |
| NFR9.2 | The C4 internal endpoint accepts only a per-service token **and** the caller on the allowlist for that operation, and **every** call also carries the user's own JWT, which the Identity service validates again against JWKS. The allowlisted service token proves which service is asking; only the user's validated JWT establishes the caller's subject and role. A service token alone never authorises a user-scoped action and never asserts a caller identity. |
| NFR10.1 | On a role change the acting admin's own JWT is forwarded and validated beside the service token, so the actor on the audit record is a person, not a service. |

## Authorisation model

| ID | Requirement |
|---|---|
| NFR10.2 | The role is read only from `app_metadata.role`; absent or unrecognised values are `owner`; any other role-shaped claim is ignored (BR2.2). |
| NFR10.3 | Deny-by-default: an operation absent from the catalogue is denied and audited (BR2.1). |
| NFR10.4 | Ownership is checked on every read and write; collections are filtered to the caller's rows (BR2.5). |
| NFR10.5 | A refusal is indistinguishable from a nonexistent resource: both answer 404 with no field of the resource (BR2.6). |
| NFR10.6 | Support is never an owner and never reads portfolio contents; administrative operations are admin-only (BR2.4, BR2.14). |

## Data protection and least privilege

| ID | Requirement |
|---|---|
| NFR22.1 | The decision holds no personal data beyond the identifiers it is given, writes none to disk, and logs no token, subject plaintext, credential, secret, or full wallet address. |
| NFR23.1 | The decision reads only what the caller passes and the u1 status contract; it gains no new Postgres role, schema access, or cross-schema read. |
| NFR23.2 | The only write the unit performs outside the audit trail is the provider role claim, through Supabase Auth's admin API. |

## Audit and fail-closed behaviour

| ID | Requirement |
|---|---|
| NFR19.1 | Every denial is audited by the decision before it returns: actor (or system), action, target, timestamp, context, outcome (BR2.11). A role change records **two entries in the C9 contract's `success`/`denied`/`failed` vocabulary**: `identity.role.change.requested` written before the provider write with `outcome: success` (the request was accepted and is being attempted, carrying the previous and intended role in `context`), then `identity.role.changed` with `outcome: success` or `failed`. There is no `attempted` or `applied` value on the wire; the pair is what makes the change auditable before it happens. |
| NFR19.2 | An audit write that fails turns the denial into a 503 rather than an unaudited refusal, and fails a role change before any write (BR2.10, BR2.11). |
| NFR19.3 | No audit record carries a credential, token, secret, or full wallet address; a value that would is refused or redacted before the write (BR2.12). |

## Threat considerations

- **Privilege via claim:** a role-shaped claim anywhere outside `app_metadata.role` has no effect; the caller is treated as owner.
- **Caller lies about ownership:** the calling service supplies the owner id from the row it loaded; a service that lies is a defect in that service, and the C4 consumer list bounds who may ask.
- **Enumeration:** `not_owner` and a nonexistent id answer identically, so a caller cannot probe which ids exist.
- **Audit suppression:** a caller cannot return a denial without the decision having attempted the audit entry, because the decision does not return until it has.
- **Replay of an admin action:** a role change requires the admin's own JWT, so a stolen service token alone cannot change a role.
