# Security Requirements — u2-web-app

Sources: `functional-design/functional-spec.md`, `functional-design/rules.md`.

| ID | Requirement | Measure |
|---|---|---|
| NFR9.1 | The Supabase client is the single source of truth for the session: `persistSession: true` with a custom storage adapter that keeps every key in memory except the PKCE `${storageKey}-code-verifier` (sessionStorage until the callback exchange, then removed); `autoRefreshToken: true`, `detectSessionInUrl: true`, `flowType: 'pkce'`. Access and refresh tokens never leave memory. `persistSession: false` is not used because it replaces a custom adapter and loses the verifier. The API client reads the token from the supabase session, and a 401 signs out | Unit tests: token keys absent from web storage; a `?code=` callback with a stored verifier establishes a session; verifier cleared afterwards |
| NFR9.2 | The SPA calls only the gateway plus Supabase Auth (C13) and WalletConnect (C14) | API-client test enumerating base URLs |
| NFR9.3 | No credential, token, seed phrase or full address is rendered or logged | Component tests |
| NFR9.3a | The support form refuses detectable secrets locally: 12/24-word mnemonics, `0x` + 64 hex private keys, `xprv`/`xpub` prefixes, and exchange key prefixes (`sk-`, `pk_live_`, `AKIA`); a normal sentence containing "key" must pass | Unit test per pattern plus one false-positive case |
| NFR9.4 | Every rendered problem shows the correlation id; the client sends a fresh `x-correlation-id` per request | Client test |
| NFR9.5 | A root-scoped service worker bypasses `/support/*`, `/v1/*` and every cross-origin Supabase/WalletConnect request in its fetch handler; it caches only the app shell and static assets | Handler test with representative URLs; registration scope assertion |
| NFR10.1 | MFA: a user with exchange credentials is routed to the MFA enrolment panel when the session assurance is `aal1`; enrolment and recovery-code display are the only MFA surfaces this unit adds (JWKS validation is server-side) | Component test with an `aal1` session |
| NFR10.2 | The SPA enforces no authorization of its own; every owner check is the gateway's, and 401/403 render the refusal | Mocked 401/403 tests |
| NFR12.1 | Dependencies are pinned in `yarn.lock`; a dependency change is reviewed and scanned in CI | `yarn audit`/review |
| NFR17.1 | Uncaught errors, failed chunk loads and failed bootstrap post one structured telemetry event (correlation id, route, error code; no PII, no token) to the gateway's telemetry route, visible in Loki/Grafana; the nginx container exposes a health metric | Beacon test with a stub endpoint; nginx healthcheck |
