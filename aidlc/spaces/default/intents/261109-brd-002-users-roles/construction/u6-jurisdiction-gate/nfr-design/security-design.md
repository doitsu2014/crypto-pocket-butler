# Security Design — u6-jurisdiction-gate

**Upstream:** `security-requirements.md` (NFR9.1–NFR23.4), `functional-spec.md`, `contract-summary.md` (C8).

## Trust boundaries

| Boundary | Trusted party | Rule |
|---|---|---|
| Caller → gate (`/internal/jurisdiction/check`) | The API Gateway and the User Registry, service-to-service | The gate accepts the request only from a trusted internal caller; the IP is the connection address observed by that caller, never a client-supplied header (NFR23.3) |
| Gate → geolocation provider | External third party | TLS; the response is treated as untrusted input and validated |
| Gate → Audit service | Internal service | TLS; the event shape is validated before the write |
| Admin → publish | Authenticated admin | Role check plus audit (NFR23.2) |

## Fail-closed matrix

| Condition | Response | Never |
|---|---|---|
| Provider timeout or budget exhausted | `503`, retry offered | Allows the check |
| List unreadable or checksum mismatch | `503`, startup error logged | Serves a partial list |
| Audit write fails after retry | `503` | Completes the decision without evidence |
| Invalid country code or IP | `422` | Calls the provider |
| Continued-use IP unavailable | `503`, blocks until re-evaluated | Extends a stored allowance |

## Data protection

- **No raw IP at rest or in logs** — the IP is used in memory, reduced to a country code, and, for the cache key only, a salted hash (NFR19.1).
- **Evidence content** — actor `system` (or the calling service), action `jurisdiction.check`, target the country code, list version, signals used, outcome. No address, no user identifier.
- **Least privilege** — the API role reads `jurisdiction_rules`/`jurisdiction_list_versions` and writes audit; publish uses a separate admin role (NFR23.1).
- **Tamper evidence** — a checksum over the ordered rules refuses a partial list (NFR19.2).

## Input validation

- `countryCode`: `^[A-Z]{2}$`, uppercase, ISO 3166-1 alpha-2.
- `ip`: parsed as IPv4 or IPv6; invalid values never reach the provider.
- Payload size capped; unknown fields ignored (additive-change rule).

## Residual limitations

IP geolocation is a best-effort signal: VPNs, corporate egress, and satellite links can disguise a location. The self-declared country and the maintained list remain the primary control; this limitation is passed to the UI unit so the notice is honest.
