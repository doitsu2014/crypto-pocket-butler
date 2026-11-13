# Functional Design Questions — u8-signup-gates-ui

**Unit:** U8 — Sign-up Gates UI (`u8-signup-gates-ui`)
**Stories:** US6.2, US6.3, US6.5 · **Requirements:** FR19.2, FR19.3, FR19.5
**Upstream:** `unit-of-work.md`, `unit-of-work-story-map.md`, `stories.md`, `requirements.md`, `components.md` (`SignupGatesUI`), `contract-summary.md` (C1/C2, C3, C6, C8)

The requirements settle what the gates must enforce. These questions cover only what the
requirements leave open for the screen design.

## Q1. [Product] Where do the gates sit in the sign-up journey?

The requirements say the acceptances are required "before the first source is connected" and
the jurisdiction check happens "at sign-up". The SPA already signs in through Supabase Auth,
so the gates can run either as a dedicated step after sign-in or inside the add-source flow.

- A. A dedicated gate screen after sign-in, before any other product screen: the visitor accepts terms, risk disclosure, and age, declares country, and only then reaches the app.
- B. A multi-step wizard inside the first-source flow: the visitor starts "connect a source", passes the gates, then continues to the source form.
- C. A modal over the first-source screen, shown the first time they attempt to connect and not dismissible until every gate passes.
- X. Other (please specify)

[Answer]: A — A dedicated gate screen after sign-in, before any other product screen.

## Q2. [Product] Which countries can a visitor select?

The jurisdiction gate owns the restricted list (U6). The screen still has to decide what it offers
in its country selector.

- A. The full ISO 3166-1 list: the selector never becomes a second, drifting copy of the restricted list; a restricted choice is refused by the gate on submit.
- B. Only the launch markets the product supports: the selector is a short list, and a restricted or unsupported country is not offered at all.
- C. A free-text country field: the visitor types their country and the gate normalises it.
- X. Other (please specify)

[Answer]: A — The full ISO 3166-1 list; a restricted choice is refused by the gate on submit.

## Q3. [Product] How does the age confirmation work?

FR19.5 says age assurance is a self-declaration with no documents in this release. The screen
still has to choose the control.

- A. A single checkbox — "I confirm I meet the minimum age for this service" — recorded as the age consent when the visitor proceeds.
- B. A month-and-year-of-birth entry, checked against the minimum age, with no document upload.
- C. A checkbox plus a separate minimum-age statement the visitor must acknowledge (two controls, one recorded acceptance).
- X. Other (please specify)

[Answer]: A — A single checkbox, recorded as the age consent when the visitor proceeds.
