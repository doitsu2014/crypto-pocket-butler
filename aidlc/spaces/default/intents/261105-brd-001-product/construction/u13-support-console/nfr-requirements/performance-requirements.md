# Performance Requirements — u13-support-console

Inputs: `functional-design/functional-spec.md`, `functional-design/rules.md`, and
`inception/requirements-analysis/requirements.md`.

| ID | Requirement | Target |
|---|---|---|
| NFR1.1 | `/support` availability | 99.9% of requests answered over 30 days |
| NFR2.1 | Console initial load (static bundle, no metadata) | ≤ 2 s on a 10 Mbps link |
| NFR2.2 | Account search response (C2) | p95 ≤ 300 ms |
| NFR2.3 | Metadata read screen | skeleton ≤ 100 ms, data p95 ≤ 500 ms |
| NFR2.4 | Grant approve/revoke round trip | p95 ≤ 400 ms |
| NFR2.5 | Incident declaration to C9 accepted | p95 ≤ 1 s |
| NFR2.6 | SEV-1 subscriber notice delivered | 100% within 15 minutes, measured by the support API and Notification metrics |

## Accessibility

| ID | Requirement | Target |
|---|---|---|
| NFR15.1 | WCAG level | 2.2 AA for every console screen |
| NFR15.2 | Keyboard | every action reachable and operable by keyboard; focus visible |
| NFR15.3 | Contrast | text and controls meet AA contrast on Parchment Ledger tokens |
| NFR15.4 | Status cues | status colour is always paired with a label and a shape, never colour alone |
