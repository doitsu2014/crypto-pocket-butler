<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-11-06T07:39:06Z — [unit u12-audit] read "admin queries, exports, and retention" as three separate operator workflows rather than one; each has its own entity because each records a different thing
- 2026-11-06T07:39:06Z — [unit u12-audit] treated the four shared stories (US7.3, US7.4, US9.2, US9.3) as audit-side contributions only; each non-audit acceptance criterion is marked N/A with the delivering unit named, not GAP

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-11-06T07:52:00Z — [unit u12-audit] after the reviewer's NOT-READY, narrowed RetentionRun to audit_logs plus reported runs instead of deleting financial records; the two-year window belongs to Ledger, and a cross-schema delete would break the one-schema-one-role rule
- 2026-11-06T07:52:00Z — [unit u12-audit] named Audit the single owner of the support-session record and removed the parallel lifecycle from the design; this needs contract C10 extended and C2 amended, which is a change to an approved inception artifact and is surfaced for the person rather than made silently

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-11-06T07:39:06Z — [unit u12-audit] rejected a silent-redaction rule for secret-shaped values in favour of rejecting the whole event (BR1.3); a redacted record hides that an emitter is leaking, and the emitter is the thing that must be fixed

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-11-06T07:39:06Z — [unit u12-audit] who holds the second-approver role for a support session is a product decision left open in the requirements; the rule only requires that the approver differ from the agent
- 2026-11-06T07:39:06Z — [unit u12-audit] the legal basis wording for records kept past a deletion request is still open per requirements open question OQ-5; BR4.3 stores a label rather than fixing the text
