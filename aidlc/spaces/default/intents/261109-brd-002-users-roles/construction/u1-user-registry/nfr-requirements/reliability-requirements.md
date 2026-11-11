# Reliability Requirements — u1-user-registry

**Upstream:** `functional-spec.md`, `contract-summary.md` (C3), `requirements.md`.

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR1.1 | Availability | 99.9% monthly for principal resolution over a 30-day window | BRD-001 NFR1 |
| NFR1.2 | Provisioning atomicity | A provision either creates the key reference and the user in one transaction or creates nothing | BR1.1 |
| NFR1.3 | Idempotent provisioning | A retried first request creates no second record and returns the existing principal | BR1.1 |
| NFR1.4 | Fail-closed gate | Provisioning is refused when the jurisdiction gate has not cleared the provider | BR1.1; C8 |
| NFR24.1 | Status freshness | The stored status is read on every request; a status change takes effect at the user's next request. Role is taken from the signed token and takes effect at the token refresh | NFR24 |
| NFR18.1 | Recovery objectives | RPO ≤ 24 h, RTO ≤ 4 h for the identity schema | BRD-001 NFR18 |

A database outage returns `503`; no partial user row is left behind.
