# Unit–Story Map — Crypto Pocket Butler

One row per user story, keyed `USx.y`. A unit is named on a row only when it implements part of
that story. The first named unit is the primary implementation for the traceability target.

| Story | Unit(s) | Directory |
|---|---|---|
| US1.1 | U3; U2 | `u3-identity`; `u2-web-app` |
| US1.2 | U1; U4; U6 | `u1-walking-skeleton`; `u4-account`; `u6-portfolio` |
| US1.3 | U4; U2 | `u4-account`; `u2-web-app` |
| US1.4 | U7; U4; U2 | `u7-ledger`; `u4-account`; `u2-web-app` |
| US2.1 | U4; U2 | `u4-account`; `u2-web-app` |
| US2.2 | U4; U2 | `u4-account`; `u2-web-app` |
| US2.3 | U4; U2 | `u4-account`; `u2-web-app` |
| US3.1 | U6; U2 | `u6-portfolio`; `u2-web-app` |
| US3.2 | U6; U2 | `u6-portfolio`; `u2-web-app` |
| US3.3 | U5; U6; U2 | `u5-market-data`; `u6-portfolio`; `u2-web-app` |
| US3.4 | U6; U5; U2 | `u6-portfolio`; `u5-market-data`; `u2-web-app` |
| US3.5 | U3; U2 | `u3-identity`; `u2-web-app` |
| US4.1 | U7; U2 | `u7-ledger`; `u2-web-app` |
| US4.2 | U7; U2 | `u7-ledger`; `u2-web-app` |
| US4.3 | U7; U2 | `u7-ledger`; `u2-web-app` |
| US4.4 | U6; U2 | `u6-portfolio`; `u2-web-app` |
| US4.5 | U7; U2 | `u7-ledger`; `u2-web-app` |
| US5.1 | U6; U2 | `u6-portfolio`; `u2-web-app` |
| US5.2 | U6; U2 | `u6-portfolio`; `u2-web-app` |
| US5.3 | U6; U2 | `u6-portfolio`; `u2-web-app` |
| US5.4 | U6; U7; U2 | `u6-portfolio`; `u7-ledger`; `u2-web-app` |
| US5.5 | U6; U2 | `u6-portfolio`; `u2-web-app` |
| US5.6 | U9; U6 | `u9-notification`; `u6-portfolio` |
| US6.1 | U8; U2 | `u8-reporting`; `u2-web-app` |
| US6.2 | U8; U9 | `u8-reporting`; `u9-notification` |
| US6.3 | U9; U2 | `u9-notification`; `u2-web-app` |
| US6.4 | U9 | `u9-notification` |
| US7.1 | U3; U2 | `u3-identity`; `u2-web-app` |
| US7.2 | U3; U2 | `u3-identity`; `u2-web-app` |
| US7.3 | U3; U12; U2 | `u3-identity`; `u12-audit`; `u2-web-app` |
| US7.4 | U3; U12; U2 | `u3-identity`; `u12-audit`; `u2-web-app` |
| US7.5 | U3; U10; U2 | `u3-identity`; `u10-analytics`; `u2-web-app` |
| US7.6 | U3; U2 | `u3-identity`; `u2-web-app` |
| US8.1 | U13; U2 | `u13-support-console`; `u2-web-app` |
| US9.1 | U11 | `u11-job-orchestration` |
| US9.2 | U1; U12 | `u1-walking-skeleton`; `u12-audit` |
| US9.3 | U13; U12; U3 | `u13-support-console`; `u12-audit`; `u3-identity` |

## Cross-cutting stories

- US7.3 (export) and US7.4 (deletion) span Identity (orchestration), Audit (records), and the
  Web App (the request and confirmation screens). Every service holding personal data is called
  by Identity; none of them implements the story on its own.
- US9.1 (scheduled jobs) is owned by Job Orchestration; every other service exposes the run
  endpoint it calls.

## Implementation order within each unit

Each unit is delivered as one buildable, testable piece: domain types and rules first (test
first), then application use cases against port fakes, then infrastructure adapters and the http
surface, then the deployment manifest and migrations. The unit is complete when its own tests,
the tests of the units it depends on, and the project build pass.

## Coverage verification

- All 37 stories are assigned to at least one unit.
- Every unit U1–U13 appears on at least one row.
- No story is assigned to a unit that does not exist.
