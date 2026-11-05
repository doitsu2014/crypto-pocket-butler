# Feasibility & Constraints Questions

## Q1. What delivery shape should the feasibility assessment assume?

A. A small greenfield team (3–5 engineers) building one service at a time behind the API Gateway, targeting an MVP in 6–9 months. [Recommended]
B. A larger team (6–10 engineers) building several services in parallel, targeting an MVP in 3–6 months.
C. A solo or two-person team, targeting an MVP in 9–12 months.
D. Not yet defined.
E. None.
X. Other (describe)

[Answer]: A. A small greenfield team (3–5 engineers) building one service at a time behind the API Gateway, targeting an MVP in 6–9 months.

## Q2. What budget envelope constrains the build?

A. Under USD 150,000 for build and launch infrastructure, funded from existing resources. [Recommended]
B. USD 150,000–400,000, consistent with the build-vs-buy TCO estimate.
C. Above USD 400,000, with dedicated delivery and go-to-market funding.
D. Not yet defined.
E. None.
X. Other (describe)

[Answer]: A. Under USD 150,000 for build and launch infrastructure, funded from existing resources.

## Q3. Which hosting target applies? The project rules fix Supabase Postgres/Auth and Docker Swarm.

A. Supabase-managed Postgres and Auth, with the Rust services and SPA deployed to Docker Swarm on commodity cloud VPS instances; no AWS dependency. [Recommended]
B. Supabase-managed Postgres and Auth, with Docker Swarm on owned or bare-metal hardware.
C. AWS-managed equivalents (ECS/RDS), which departs from the project's Docker Swarm rule.
D. Not yet defined.
E. None.
X. Other (describe)

[Answer]: A. Supabase-managed Postgres and Auth, with the Rust services and SPA deployed to Docker Swarm on commodity cloud VPS instances; no AWS dependency.

## Q4. Which launch jurisdictions and legal gating apply?

A. EU (MiCA) and UK first, then the US; jurisdiction gate before data collection; external counsel validates disclosures. [Recommended]
B. US first, then EU and UK.
C. Global phased launch with per-market legal review, as BRD-001 states.
D. Not yet defined.
E. None.
X. Other (describe)

[Answer]: A. EU (MiCA) and UK first, then the US; jurisdiction gate before data collection; external counsel validates disclosures.

## Q5. The USD 150,000 budget conflicts with the 3–5 engineer × 6–9 month plan (about 18–45 engineer-months, roughly USD 180,000–675,000 fully loaded). How should feasibility treat the budget?

A. The budget excludes existing salaries: the team is already funded, and USD 150,000 covers vendors, infrastructure, legal and contingency. [Recommended]
B. The budget is a hard cap including salaries; reduce scope or extend the timeline to fit it.
C. The budget is hard and the build TCO must be re-baselined with lower rates or more reuse.
D. Not yet defined.
E. None.
X. Other (describe)

[Answer]: A. The budget excludes existing salaries: the team is already funded, and USD 150,000 covers vendors, infrastructure, legal and contingency.

## Q6. The 6–9 month / 3–5 engineer target does not close against the estimated 25–44 engineer-months. Which delivery shape should the plan record?

A. Option B: 10 launch services, 5 engineers, a 9-month target with a written cut list; full committed scope is 10–11 months. [Recommended]
B. Keep the full committed Must scope and accept a 10–11 month planning date.
C. Option A: fold cost basis into Portfolio and defer paid-plan depth to hit 6–9 months; weakens the P&L differentiator.
D. Not yet defined.
E. None.
X. Other (describe)

[Answer]: A. Option B: 10 launch services, 5 engineers, a 9-month target with a written cut list; full committed scope is 10–11 months.

## Q7. The full EU+UK legal workstream puts the non-salary budget at $130k–165k. How should it be resolved?

A. Keep EU+UK first; cap legal at ~$50k fixed fee so total stays ≤$150k. UK goes live only when counsel signs off. [Recommended]
B. Keep EU+UK first and raise the non-salary envelope to about $165k.
C. Launch EU first; UK is pre-positioning only (no data, no marketing) until the UK opinion lands.
D. Not yet defined.
E. None.
X. Other (describe)

[Answer]: A. Keep EU+UK first; cap legal at ~$50k fixed fee so total stays ≤$150k. UK goes live only when counsel signs off.

## Q8. Notification, Analytics and the Support Console are BRD Must scope but the 10-service plan defers them. What should launch include?

A. Defer Notification, Analytics and Support Console to post-MVP with documented degraded paths; record as an explicit BRD deviation. [Recommended]
B. Keep Notification and Support Console at launch, defer Analytics only; add ~1 month.
C. Keep all three BRD Must areas at launch: 12 services, ~11–12 months.
D. Not yet defined.
E. None.
X. Other (describe)

[Answer]: A. Defer Notification, Analytics and Support Console to post-MVP with documented degraded paths; record as an explicit BRD deviation.
