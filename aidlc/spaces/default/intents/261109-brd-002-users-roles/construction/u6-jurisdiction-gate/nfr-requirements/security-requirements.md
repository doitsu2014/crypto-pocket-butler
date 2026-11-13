# Security Requirements — u6-jurisdiction-gate

**Upstream:** `functional-spec.md` (W1–W3), `rules.md` (BR1.4–BR1.10), `requirements.md`.

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR9.1 | Transport security | All calls use TLS; no plaintext transport in any environment | BRD-001 NFR9; BRD-002 NFR (TLS) |
| NFR10.1 | Token validation | Authenticated operations validate the Supabase JWT against JWKS (signature, issuer, audience, expiry) | BRD-001 NFR10 |
| NFR23.1 | Least privilege | The API role reads the list and writes audit only; publishing requires a separate admin role | NFR23 |
| NFR23.2 | Publish authentication | Publishing requires an authenticated admin and is audited with actor, target, version, outcome | NFR23; FR20.3 |
| NFR23.3 | IP trust boundary | The gate accepts the IP only from the trusted internal caller (the gateway derives it from the connection); a client-supplied header is never used. IP-based signals remain a best-effort control against VPNs and proxies, and this limitation is stated to the user-facing unit | BR1.3; NFR23 |
| NFR9.2 | Input validation | `countryCode` must match `^[A-Z]{2}$`; the IP must parse as IPv4 or IPv6; invalid input never reaches the provider | BR1.4 |
| NFR19.1 | No personal data | The raw IP is never persisted or logged; evidence holds resolved country codes, the signals used, the list version, and the outcome | BR1.7; BRD-001 NFR19 |
| NFR19.2 | List tamper evidence | A checksum detects a partial or altered list; a mismatch refuses to serve | BR1.6 |
| NFR23.4 | Fail closed | An unavailable provider or list returns `503`; the gate never allows on uncertainty, including on continued-use checks | BR1.5, BR1.8 |

Threat considerations: knocking the provider offline cannot force an allow (fail closed); evidence reveals no address or identity (no raw IP stored); a compromised API role cannot rewrite the list (publish is a separate role); a forged `X-Forwarded-For` cannot substitute an allowed address (the IP comes only from the trusted caller's observed connection).
