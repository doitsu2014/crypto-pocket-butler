# User Stories — Plan and Questions

This stage turns the BRD-002 requirements (FR14–FR20, NFR21–NFR26) into personas and user stories with Given/When/Then acceptance criteria, traceable back to each requirement.

**Persona development approach (draft):** derive personas from the BRD-002 actor table — Portfolio Owner, Admin, Support, and Anonymous visitor — and treat System as a non-persona actor whose behaviour is captured by job stories only if needed.

**Story format:** `As a [persona], I want [goal], so that [benefit]`, each INVEST-checked, with 3–6 acceptance criteria in Given/When/Then form and a stable `USx.y` / `ACx.y.z` ID.

**Breakdown approach (draft):** by requirement group, so each story group maps to one of FR14–FR20 and traceability stays direct.

Answer each question with a letter after `[Answer]:`, or pick `X` and say what you want instead.

## Q1. [product manager] Which personas should the stories use?

BRD-002 names five actors, and the post-MVP Analyst/Viewer is marked Won't for the MVP.

- A. Four personas — Portfolio Owner, Admin, Support, Anonymous visitor — with System treated as a non-user actor
- B. Five personas, including System, with job stories for scheduled work
- C. Add Analyst/Viewer as a post-MVP persona so the forward design stays visible
- X. Other (please specify)

[Answer]: A — Four personas, with System treated as a non-user actor.

## Q2. [product manager] How should the stories be broken down?

- A. By requirement group — one story group per FR14–FR20
- B. By persona journey — sign-in to profile, support access, admin work
- C. By BRD feature area — roles and authorisation, support access, preferences, deletion, administration
- D. Vertical slices by user-facing capability, independent of the requirement grouping
- X. Other (please specify)

[Answer]: A — By requirement group, one story group per FR14–FR20.

## Q3. [product manager] How granular should the stories be?

There are 38 functional sub-requirements in FR14–FR20.

- A. One story per sub-requirement (about 38 stories), with exact traceability
- B. Coarser — group related sub-requirements into about 15–20 stories
- C. Per user-journey step, about 10–12 stories
- X. Other (please specify)

[Answer]: A — One story per sub-requirement, about 38 stories.

## Q4. [product manager] How should priorities be assigned?

- A. Every BRD Must is Must Have; the Won't item (shared portfolios) is Won't Have; no Should/Could in this set
- B. Mark the open items (support access during pending deletion, the pending-deletion UI) as Should Have
- C. Use Must only for security-critical rules; preferences and admin conveniences become Should Have
- X. Other (please specify)

[Answer]: A — Every BRD Must is Must Have; shared portfolios is Won't Have; no Should/Could in this set.

## Q5. [designer] How detailed should each persona card be?

- A. Full cards — role, goals, pain points, context, tech comfort, frequency
- B. Brief cards — role, goals, key constraints
- C. Labels only, with the BRD actor table as the reference
- X. Other (please specify)

[Answer]: A — Full cards, with role, goals, pain points, context, tech comfort and frequency.
