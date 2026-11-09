# Security Requirements — u4-account

Inputs: `functional-design/functional-spec.md`, `functional-design/rules.md`,
`inception/requirements-analysis/requirements.md` (FR1, FR2, NFR9–NFR13, NFR19), and
`inception/contract-design/contract-summary.md` (C1, C3, C4, C5, C9, C10, C11, C15).

## Authentication and authorization

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR10.1 | User requests (C1) carry a Supabase JWT validated against JWKS by Account itself; internal requests (C5, C11, C15) carry the shared service token from Vault and are authorised per operation | 401 on a failed JWT or token; no unverified caller | NFR10 |
| NFR10.2 | Account persists no token, key material, or seed phrase | Vault references only; a key is held in memory only for the provider call | NFR10 |
| NFR10.3 | Storing an exchange credential requires an MFA-enrolled principal (`aal2`) | Below `aal2`, the key is refused | NFR10, FR1.2 |
| NFR10.4 | A user-owned read needs the owner or an active support grant; an internal service read is authorised by the service identity and the operation | No blanket admin; no unauthenticated internal read | FR1.6, FR11.2 |
| NFR10.5 | An admin without a support grant cannot read another user's accounts | Refused and audited | FR1.5, US9.3 |
| NFR10.6 | Support reads are metadata-only and audited before the answer | Grant active, unexpired, reason-recorded | FR11.2 |

## Credential protection

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR9.1 | Exchange credentials live in Supabase Vault | Never persisted outside Vault, never returned, never logged; held in memory only for the duration of the provider call | NFR9 |
| NFR9.2 | Credentials are verified read-only before use and periodically | Withdrawal or trading permission rejects and destroys the secret | FR2.1 |
| NFR9.3 | No credential, token, or full wallet address in logs, errors, or audit | Allow-listed fields only | NFR9, NFR19 |
| NFR9.4 | The claim `app_metadata.has_exchange_credentials` is maintained and cleared | Set on store; cleared on rejection, removal, revocation | FR1.2, NFR10 |
| NFR9.5 | TLS on every production path | Terminates at the edge/CDN and gateway; internal hops use the encrypted Swarm overlay | NFR9 |

## Data protection and privacy

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR13.1 | Data minimisation: only the fields the workflows need | Entity model is the allowlist | NFR13 |
| NFR13.2 | Privacy export and anonymise for Identity | Export returns account data; anonymise removes labels/addresses and labels retention | FR8.1, FR8.2 |
| NFR13.3 | No full wallet address in any audit record | account id and label only | NFR19 |

## Audit and retention

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR19.1 | Every credential, account, sync, and import event is written through C10 with actor, action, target, timestamp, context, and outcome | identifiers only; never the full address | NFR19 |
| NFR19.2 | No credential, token, or full address in an audit record | account id and label only | NFR19 |
| NFR19.3 | Account writes only through the Audit API and holds no grant on the audit schema | insert-only role at the Audit service | NFR19 |
| NFR14.1 | Anonymise keeps labelled financial-retention rows and returns the categories | two-year basis supplied by the owning service | NFR14 |

## Supply chain

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR12.1 | Dependency and container-image vulnerability scanning before release | Fails on a known critical | NFR12 |

## Threat considerations

- **Credential theft**: keys live in Vault, are verified read-only, and are never returned or logged.
- **IDOR**: every account read derives the owner from the principal (BR1.1).
- **Support overreach**: metadata-only, grant-gated, audited before the answer.
- **Malicious CSV**: rows are parsed defensively, bad rows are reported, and the file never executes.
- **Provider compromise**: responses are validated and resolved through the registry; spam is
  labelled, never trusted.

NFR references: NFR9.1–NFR9.5, NFR10.1–NFR10.6, NFR12.1, NFR13.1–NFR13.3, NFR14.1,
NFR19.1–NFR19.3.
