# Contract Design — Questions

The approved DAG has three inter-unit edges: U2 depends on U1; U3 depends on U2; U4 depends on U1,
U2 and U3. The gateway-to-Identity route is an external boundary inside the edge, and Identity calls
the existing Notification service for an in-app alert. The browser calls the gateway and Supabase
Auth directly.

The project uses synchronous internal HTTP, forwards the user's Supabase JWT unchanged, forbids
message queues, and requires bounded retries with idempotency keys. The contract specs will be
OpenAPI-style YAML for HTTP; they will not choose database field types or implementation frameworks.

Answer each with a letter (A–D), or **X** plus your own words if none fits.

## Q1. [architect] Which formal specs should this contract summary pin down?

- A. Pin the U1 → U2 Identity use-case/HTTP boundary, U2 → U3 SPA route/API boundary, U1 → Notification boundary, and U4's route-completeness proof contract; record Supabase Auth as an external provider dependency (recommended)
- B. Pin only the public browser → gateway API; internal contracts are implementation detail
- C. Pin one REST contract per existing backend component, including every unrelated Identity API
- D. No formal specs: the Unit descriptions are sufficient
- X. Other (please specify)

[Answer]:

## Q2. [architect] What is the canonical consumer boundary for SPA calls?

The team practice is that every SPA-called path must have a declared gateway route and owning handler.

- A. The SPA contract is with the gateway only; gateway → Identity is a separate internal contract, and Supabase Auth remains the one direct browser exception (recommended)
- B. The SPA may call Identity directly for auth routes, while other data still uses the gateway
- C. Treat SPA → gateway → Identity as one end-to-end contract, with no separate gateway/Identity contract
- D. SPA uses the gateway for reads but may call Notification and Identity directly for writes
- X. Other (please specify)

[Answer]:

## Q3. [architect] How should the user JWT cross the internal Identity boundary?

The project practice says the gateway forwards the user's Supabase JWT unchanged and every service
validates it. Identity's gateway-facing routes need both the user principal and Identity's own
service authority for internal calls.

- A. Forward the user's JWT unchanged in `Authorization`; use the existing Identity service-token mechanism separately for gateway-to-Identity privileged operations (recommended)
- B. Exchange the user JWT for a new internal JWT at the gateway
- C. Put the user subject and role in a trusted request body from the gateway
- D. Use only the Identity service token; do not forward the user token
- X. Other (please specify)

[Answer]:

## Q4. [architect] What retry and idempotency contract applies to revocation and notification?

Revocation and alert creation are side-effecting calls; retry must not revoke or notify twice.

- A. Bounded retry with exponential backoff and jitter; idempotency key stable per operation; retry transient failures only; never retry validation/authorization failures (recommended)
- B. Retry any failure up to three times; no idempotency key
- C. No retry; surface every transient error immediately
- D. Retry revocation only; login alert is best-effort with no retry
- X. Other (please specify)

[Answer]:

## Q5. [architect] What compatibility policy should the HTTP contracts use?

- A. Additive, backward-compatible changes only within `/v1`; consumers ignore unknown response fields; breaking changes require a new version and a coordinated migration (recommended)
- B. Any field or status can change in `/v1` if both Units change together
- C. Version every route independently
- D. No version policy; decide at implementation time
- X. Other (please specify)

[Answer]:

## Q6. [security engineer] Which error envelope and disclosure rule should every contract use?

The project already has an RFC 7807 `application/problem+json` envelope and forbids echoing rejected
values, secrets or upstream details.

- A. Use the existing RFC 7807 shape with stable `type`, title, status, optional safe detail, field-path errors, and correlation id; never include rejected values, tokens, credentials, full device fingerprints, or upstream details (recommended)
- B. Use the simpler `{error:{code,message}}` shape for auth only
- C. Return provider error text verbatim to help support debug
- D. Status codes only, with no body
- X. Other (please specify)

[Answer]:
