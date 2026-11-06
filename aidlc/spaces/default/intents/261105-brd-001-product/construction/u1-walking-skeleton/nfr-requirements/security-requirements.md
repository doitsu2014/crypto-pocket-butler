# Security Requirements — u1-walking-skeleton (Review artifact)

This Unit is the gateway and the Asset Registry. It holds no credentials and no user personal
data; its security obligations are transport, token validation, routing, rate limiting, audit of
registry changes, and supply chain.

| ID | Requirement | Target | Parent | Verification |
|---|---|---|---|---|
| NFR9.1 | All production traffic uses TLS; plaintext is rejected | 100% of endpoints; HSTS on the edge | NFR9 | TLS scan, plaintext request refused |
| NFR9.2 | Provider or database secrets come from Supabase Vault, never source or env files | zero secrets in the repository | NFR9 | Secret scan in CI |
| NFR9.3 | No credential, token, or full address appears in a response, log, error, or audit record | zero occurrences | NFR9 | Log and response sampling in the integration test |
| NFR10.1 | The gateway validates the Supabase JWT against JWKS (signature, issuer, audience, expiry) before routing | every request; invalid → 401 | NFR10 | Integration test with forged, expired, and wrong-audience tokens |
| NFR10.2 | The SPA shell holds session tokens in memory only, never in persistent storage | zero tokens in `localStorage`/`sessionStorage` | NFR10 | Browser storage inspection test |
| NFR11.1 | Rate limits per user, IP, and region at the gateway, stricter for auth and manual triggers | 429 above the limit with `Retry-After` | NFR11 | Load test above the limit |
| NFR12.1 | Dependencies and container images are scanned before release | zero critical or high without an accepted risk | NFR12 | CI scan gate |
| NFR19.1 | Every registry administration change and every denied request is audited with actor, target, time, and outcome | 100% | NFR19 | Audit query for a change and a denial |
| NFR19.2 | Audit records never include credentials, tokens, or full addresses | zero occurrences | NFR19 | Audit record inspection |

## Threat considerations

- **Token forgery** — rejected by JWKS validation (NFR10.1); the token is forwarded unchanged so
  every service validates independently.
- **Route enumeration** — deny-by-default routing answers 404 to undeclared routes (BR4.2).
- **Credential exposure** — this Unit stores no credentials; the registry holds public contract
  data only.
- **Unresolved-token flooding** — quarantine writes are rate-limited with the gateway limits and
  bounded per account.
- **Supply chain** — image and dependency scans gate release (NFR12.1).

## Explicit non-goals for this Unit

No user personal data, no payments, no wallet keys. Ownership checks for user-owned resources
belong to the units that own those resources.
