# Domain Design — Questions

The C4 model in `docs/architecture/README.md` already names the services. These questions settle
how they become the logical components this design records. Answer each with a letter after
`[Answer]:`, or pick `X` and say what you want instead.

## Q1. [Architect] How fine-grained should the components be?

- A. One component per documented service, minus Billing (removed by the interview) and the Observability stack (infrastructure, not code) — 12 backend components plus the SPA and Support Console
- B. Fewer, coarser components, collapsing related services (for example Ledger into Portfolio)
- C. More, finer components split inside each service
- X. Other (please specify)

[Answer]: A — One per service

## Q2. [Architect] Where do transactions, cost basis, and P&L live?

- A. A separate Ledger component owning its own schema, as the C4 model says
- B. Inside the Portfolio component
- C. Inside the Account component
- X. Other (please specify)

[Answer]: A — Separate Ledger

## Q3. [Architect] Is Billing a component in this release?

The interview removed billing, plans, entitlements, quotas, checkout, and dunning.

- A. No Billing component at all; no entitlement or quota checks
- B. Keep a minimal usage-limit component to protect capacity
- C. Keep the Billing component built but disabled
- X. Other (please specify)

[Answer]: A — No Billing

## Q4. [Architect] Who owns notification delivery?

- A. The Notification component owns every channel and delivery, as documented; other components ask it
- B. Each component sends its own notifications
- C. Reporting owns brief email; Notification owns the rest
- X. Other (please specify)

[Answer]: A — Notification owns all

## Q5. [Architect] Which UI components belong in the design?

- A. Both the Web App (SPA) and the Support Console, as documented
- B. The Web App only; support is an admin section of it
- C. No UI components in domain design
- X. Other (please specify)

[Answer]: A — SPA + Support Console

## Q6. [Architect] How should audit be structured?

- A. A separate Audit component with an append-only store, as documented; other components send events to it
- B. A cross-cutting library that writes to each component's own schema
- C. Both: a shared library for emission plus the Audit component for queries and retention
- X. Other (please specify)

[Answer]: A — Separate Audit
