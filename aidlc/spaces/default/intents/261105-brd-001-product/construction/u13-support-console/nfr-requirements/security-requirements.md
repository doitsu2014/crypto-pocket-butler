# Security Requirements — u13-support-console

Inputs: `functional-design/functional-spec.md` (BR1–BR4), NFR13, and C2.

| ID | Requirement | Target |
|---|---|---|
| NFR9.1 | Transport | TLS only; the support console is served under `/support` behind the CDN/WAF; the CSP `connect-src` allows the same-origin gateway and Supabase Auth (a different gateway origin is added explicitly) |
| NFR9.2 | Authentication | Supabase Auth JWT in memory only; never in localStorage or sessionStorage; session ends on tab close |
| NFR9.3 | Authorization | role `support`/`admin` from `app_metadata.role`; every C2 call carries the JWT; the gateway enforces the role |
| NFR9.4 | Least privilege | metadata only; no credential, holding, valuation, portfolio content, requester label, or ticket conversation content in any DTO, screen, log, or analytics event |
| NFR9.5 | Grant binding | active, unexpired, target user equals the record owner for search, metadata, and the ticket list; enforced server-side, mirrored in the UI |
| NFR9.6 | Audit | every read and incident action recorded via U1's support API; reason hash, never free text. The free-text reason travels only in `X-Support-Reason`, never a URL, and is never logged |
| NFR9.7 | Client storage | no PII in localStorage/sessionStorage; TanStack Query cache cleared on sign-out, grant expiry/revocation, and 401/403 |
| NFR12.1 | Supply chain | dependency and bundle scanning in CI; no CDN script without SRI |

## Notes

- The console is a separate static app with its own CSP; `connect-src` allows only the same-origin
  gateway and Supabase Auth. If deploy configures a different gateway origin, that origin is added
  to `connect-src`.
- A root-scoped customer service worker must bypass `/support` and `/v1`; the console also
  unregisters any registration whose scope covers `/support` at startup.
