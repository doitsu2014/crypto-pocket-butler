# Units Generation — Questions

These questions settle how the components become implementable units. Answer each with a letter
after `[Answer]:`, or pick `X` and say what you want instead.

## Q1. [Architect] What boundary strategy should the units use?

- A. One unit per component from the domain design, plus a first walking-skeleton unit that proves the end-to-end path
- B. One unit per deployable service, grouping the two UI components into one
- C. Group by capability slice — fewer, larger units that each deliver a user journey end to end
- X. Other (please specify)

[Answer]: A — One per component

## Q2. [Architect] How coarse should each unit be?

- A. Coarse — a unit is a whole component, delivered as one buildable and testable piece
- B. Fine — split each component into API, business-logic, and persistence units
- C. Mixed — coarse for simple components, finer for the ledger and portfolio
- X. Other (please specify)

[Answer]: A — Coarse

## Q3. [Architect] How should dependency order be treated?

- A. Strict topology only — nothing starts before its dependencies are complete
- B. Allow parallel work between units with no dependency between them
- C. Parallel with an integration checkpoint after each wave
- X. Other (please specify)

[Answer]: A — Strict topology

## Q4. [Architect] What deployment model should the units target?

- A. Each backend unit deploys independently, plus the SPA and Support Console — as the C4 model says
- B. One backend deployable plus the SPA
- C. Hybrid — core services together, edge services separate
- X. Other (please specify)

[Answer]: A — Independent services
