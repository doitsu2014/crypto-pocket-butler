# Reliability Requirements — u6-jurisdiction-gate

**Upstream:** `functional-spec.md` (W1–W3), `contract-summary.md` (C8, C9), `requirements.md`.

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR1.1 | Gate availability | 99.9% monthly for the gate endpoint over a 30-day rolling window, measured excluding provider-caused failures and reported separately | BRD-001 NFR1 |
| NFR1.2 | Provider dependency SLO | A separate provider-dependency SLO tracks provider success (99.5% monthly); provider outages consume that budget, not the gate's | NFR1.1; BR1.5 |
| NFR1.3 | Fail closed | A provider outage blocks the check with a retry path rather than allowing it; the outage is visible in metrics | BR1.5, BR1.8 |
| NFR1.4 | Continued-use on provider failure | A continued-use check whose IP signal cannot be evaluated returns `503` and blocks until re-evaluated; a stored allowance is never silently extended | BR1.8 |
| NFR1.5 | List publish atomicity | A publish activates a complete, checksum-valid version or leaves the previous one active | BR1.6, W3 |
| NFR1.6 | Version durability | Published versions are kept readable; recovery restores the latest active version | W3 |
| NFR18.1 | Recovery objectives | RPO ≤ 24 h, RTO ≤ 4 h for the list store | BRD-001 NFR18 |
| NFR1.7 | Audit write behaviour | The Audit call has a 300 ms timeout, one retry, and an idempotency key; if it still fails, the gate returns `503` rather than completing a decision without evidence (BR1.7) | BR1.7; C9 |

The provider and audit dependencies each have their own budget and failure behaviour; neither is allowed to turn into an allow. The unit has no scheduled job of its own.
