# Refined Mockups — Questions

The classic scope skips rough mockups, so these refined mockups are designed directly from the BRD-002 stories and requirements. The questions settle the design scope and patterns before the screens and interaction specification are written.

Answer each question with a letter after `[Answer]:`, or pick `X` and say what you want instead.

## Q1. [designer] Which surfaces should get refined mockups?

BRD-002 touches four new or changed surfaces: account settings and security, the support console (grant request/approval plus metadata panel), admin user management, and the sign-up gates (terms, age, jurisdiction).

- A. Those four BRD-002 surfaces, including deletion and recovery inside settings
- B. Only the highest-risk pair — support console and admin user management — with the rest specified as component notes
- C. Every screen the stories touch, including existing portfolio screens affected by preferences
- X. Other (please specify)

[Answer]: A — Those four BRD-002 surfaces, including deletion and recovery inside settings.

## Q2. [designer] What fidelity should the mockups be?

- A. Mid-fidelity structural wireframes with annotations, no visual styling
- B. High-fidelity descriptions mapped to Parchment Ledger tokens with real copy
- C. Component specifications only, with no full-screen layouts
- X. Other (please specify)

[Answer]: A — Mid-fidelity structural wireframes with annotations, no visual styling.

## Q3. [designer] How should account deletion be confirmed?

Deletion is the most destructive action in this stage, and the 30-day window is the recovery path.

- A. A dedicated deletion page that names what is affected and what is kept, then an explicit confirm step
- B. A confirmation modal naming what is affected, with the 30-day window explained as the recovery path
- C. Two-step: a confirmation modal, then typing the account email to confirm
- X. Other (please specify)

[Answer]: A — A dedicated deletion page naming what is affected and what is kept, then an explicit confirm step.

## Q4. [designer] How should the layouts respond across screens?

The SPA is a mobile-first PWA; the support console and admin surfaces are desktop tools.

- A. Mobile-first for the SPA surfaces; desktop-first for the support console and admin
- B. Mobile-first everywhere, including the support console and admin
- C. Desktop-first everywhere
- X. Other (please specify)

[Answer]: A — Mobile-first for the SPA surfaces; desktop-first for the support console and admin.

## Q5. [designer] How much of the existing design system should the mockups reuse?

The Parchment Ledger design system already defines tokens, figure conventions, and core components.

- A. Reuse the existing tokens and components; add only what BRD-002 needs (role badge, status pill, grant banner)
- B. Introduce new components for the permission matrix and grant banner, and reuse tokens
- C. Tokens only; design the components freely
- X. Other (please specify)

[Answer]: A — Reuse the existing tokens and components; add only a role badge, status pill, and grant banner.
