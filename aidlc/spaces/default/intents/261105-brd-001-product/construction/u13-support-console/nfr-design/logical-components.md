# Logical Components — u13-support-console

Inputs: `nfr-requirements/*`, `functional-design/frontend-components.md`.

| Logical component | Responsibility | Scaling | Failure domain |
|---|---|---|---|
| SupportApp shell | Route-split static bundle, CSP | CDN | One bundle |
| RoleGate | Reads `app_metadata.role`; hides the shell | In-browser | No data loaded |
| GrantClient | Request/approve/revoke/status through C2 | In-browser | Pending state shown |
| MetadataClient | Search and metadata reads through C2 | In-browser | 403 reason shown |
| IncidentClient | Create/update incidents through C2 (U1 backend) | In-browser | Submit error shown |
| Support API (U1) | Grant/incident store, C10 audit, C9 notify | Backend | Owned by U1, not this unit |
| Gateway | JWT validation, role enforcement, `/support` serving | Existing | 401/403 |

## Boundaries

- The console never holds a credential, service token, or audit write path.
- No business data is stored in the browser beyond the in-memory query cache.
- The public status page is U2's; the console only feeds the incident store.

## Blast radius

- A compromised console session exposes only the metadata the active grant allows, for its target
  user.
- A console outage does not affect customers; the support API and status page are separate.

NFR references: NFR9.3, NFR9.4, NFR20 (CDN).
