# Requirements Traceability

How the anchor goals in BRD-001 map to the requirement set, and where the acceptance evidence lives. This is a **goal-level** trace; the FR-to-AC matrix is completed per BRD during backlog refinement.

## Goal trace

| Goal | Primary BRDs | Key requirements | Acceptance evidence |
|------|--------------|------------------|---------------------|
| G-01 Consolidate holdings | 004, 005, 006, 007 | FR-ACC-01–18, FR-AST-01–14, FR-VAL-01–02 | AC-ACC-01–19, AC-VAL-02 |
| G-02 Trustworthy valuation | 005, 007, 012 | FR-AST-07–10, FR-VAL-01–06, NFR-ACC-01–04 | AC-AST-04–12, AC-VAL-01–14 |
| G-03 Visible drift | 006, 007 | FR-PORT-11–12, FR-VAL-04 | AC-PORT-06–07, AC-VAL-03 |
| G-04 Answer “up or down?” | 013, 020 | FR-TXN-07–12, FR-PERF-01–09 | AC-TXN-02–07, AC-PERF-01–07 |
| G-05 Turn the view into action | 006, 008 | FR-PORT-23, FR-ORD-01–19 | AC-ORD-01–11 |
| G-06 Remove manual reporting | 009, 015 | FR-REP-01–24, FR-NOT-01–20 | AC-REP-01–12, AC-NOT-01–10 |
| G-07 Protect credentials and privacy | 003, 004, 012, 017 | NFR-SEC-02–25, FR-ACC-18–19, FR-PRV-01–22 | AC-ACC-10–11, AC-PRV-01–10 |
| G-08 Preserve history | 007, 013 | FR-VAL-07–15, FR-TXN-20 | AC-VAL-05–08, AC-TXN-01–10 |
| G-09 Work globally | 005, 007, 018 | FR-AST-16–18, FR-VAL-16–18, FR-L10N-01–20 | AC-VAL-10–11, AC-L10N-01–10 |
| G-10 Retain users | 014, 015, 021 | FR-ONB-01–22, FR-NOT-01–20, FR-ANL-04–07 | AC-ONB-01–10, AC-NOT-01–10, AC-ANL-01–10 |
| G-11 Trustworthy and lawful | 002, 003, 012, 017, 019 | NFR-LEG-01–10, NFR-PRV-01–15, FR-SUP-01–20 | AC-SUP-01–10, AC-PRV-01–10 |
| G-12 Sustain the business | 016 | FR-BIL-01–20 | AC-BIL-01–10 |

## Retail value loop

| Loop step | BRDs | What the user gets |
|-----------|------|--------------------|
| See all assets | 004, 005, 006, 007 | One consolidated, base-currency view with freshness and coverage |
| Know P&L | 013, 020 | Net invested, realized/unrealized P&L, returns, benchmark |
| Get alerted | 015, 009 | Drift, price, sync, key, brief, and security alerts |
| Act | 006, 008 | Target, guardrails, order intent, trade journal, rebalancing proposal |
| Report and export | 009, 017, 020 | In-app/email/PDF brief, CSV/PDF export, tax-ready data |
| Trust | 003, 012, 017, 019 | MFA, read-only verification, privacy rights, support, status |

## Cross-BRD dependency map

| BRD | Depends on | Depended on by |
|-----|------------|----------------|
| 001 | — | All |
| 002 | 003 | 004–019 |
| 003 | 002 | All authenticated surfaces |
| 004 | 005, 010 | 006, 007, 013 |
| 005 | 011, 018 | 004, 007, 013, 020 |
| 006 | 004 | 007, 008, 009 |
| 007 | 005, 006, 013 | 008, 009, 020 |
| 008 | 006, 007, 012 | 009 |
| 009 | 007, 013, 015, 020 | — |
| 010 | 004, 005, 016, 021 | All admin surfaces |
| 011 | 004, 005, 013, 015, 016, 017 | 007, 009, 019 |
| 012 | — | All |
| 013 | 004, 005, 018 | 007, 009, 020 |
| 014 | 003, 004, 016, 018 | 021 |
| 015 | 003, 009, 018 | 009, 019 |
| 016 | 002, 012, 018 | 010, 014 |
| 017 | 002, 012 | 013, 015, 019, 021 |
| 018 | 005, 012 | 007, 009, 013, 014, 015, 016, 019, 020 |
| 019 | 002, 012, 015, 017 | 021 |
| 020 | 007, 013, 018 | 009, 021 |
| 021 | 012, 017 | 001, 014, 019 |

## Quality checks before construction

- Every `Must` FR has at least one acceptance criterion.
- Every acceptance criterion names a measurable outcome and a test layer (domain, application, adapter, or end-to-end).
- Every BRD has an owner and a last-updated date.
- Every open question has an owner; decisions are recorded in `DECISIONS.md`.
- Every cross-BRD dependency is reflected in both documents.
- Every NFR target is measurable and has an alert or test.
