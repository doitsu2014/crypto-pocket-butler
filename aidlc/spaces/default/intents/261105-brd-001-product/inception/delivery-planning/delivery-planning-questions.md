# Delivery Planning — Questions

These questions set the build order. A **Bolt** is one build pass over a piece of the work — one
or more Units of Work — ending in something that runs, with a Definition of Done and a
confidence hypothesis (what shipping it will prove). Answer each with a letter after `[Answer]:`,
or pick `X` and say what you want instead.

## Q1. [Delivery lead] What should we build first?

- A. A thin end-to-end slice first — the walking skeleton, which proves the pieces connect — then risk-first for the rest
- B. The riskiest parts first
- C. The most valuable parts first
- X. Other (please specify)

[Answer]: A — Thin slice first

## Q2. [Delivery lead] Should we score and rank the Bolts with a formal model?

A WSJF-style score is (value + urgency + risk reduction) ÷ size; a higher score ships first.

- A. Yes — score every Bolt with WSJF-style weights
- B. No — use risk-first ordering with a written rationale
- C. Value-first ordering
- X. Other (please specify)

[Answer]: A — Use WSJF

## Q3. [Delivery lead] How big should one Bolt be?

- A. One Unit of Work per Bolt
- B. Several related Units bundled into one Bolt
- C. Thin slices that cut across Units
- X. Other (please specify)

[Answer]: A — One Unit per Bolt

## Q4. [Delivery lead] Can several Bolts be built at the same time?

- A. Serial — one Bolt at a time, following the dependency order
- B. Parallel wherever the dependencies allow
- C. Serial until the walking skeleton ships, then parallel
- X. Other (please specify)

[Answer]: C — Skeleton, then parallel

## Q5. [Delivery lead] What outside dependencies could hold us up?

- A. None beyond the exchange, RPC, market-data, email, and Supabase services already in the design
- B. Legal and licensing approvals — jurisdiction, market-data licences, exchange API terms
- C. Other teams or vendors we have not named
- X. Other (please specify)

[Answer]: A — None beyond services

## Q6. [Delivery lead] What worries you most about this build?

- A. Data correctness — decimals, cost basis, and valuation accuracy
- B. Credential security and cross-user leakage
- C. Provider reliability — stale prices and failed syncs
- X. Other (please specify)

[Answer]: A — Data correctness
