# Refined Mockups — Questions

The classic scope skipped Ideation, so there are no rough wireframes or user-flow inputs
(`ideation/rough-mockups/` is absent by design). These mockups are therefore designed directly from
the user stories and requirements, and from what the SPA already renders today.

Answer each with a letter (A–D), or **X** plus your own words if none fits.

## Q1. [designer] How far should these mockups go?

The SPA already has real screens for sign-in, sign-up gates, MFA, verify-email, settings and the
limited session. Some of what the stories need exists and some does not.

- A. Document every screen the stories touch — existing pages as "current state", plus the new and
  changed ones (recommended)
- B. Only the screens that do not exist yet; treat existing pages as settled
- C. Only the sign-in and MFA screens, on the theory that the rest is form work
- D. Skip mockups and let the interaction spec describe everything in prose
- X. Other (please specify)

[Answer]: A — document every screen the stories touch: existing pages as current state, plus the new and changed ones. **Mode:** guided

## Q2. [designer] How should the mockups be presented?

These are markdown artifacts, so "mid-to-high fidelity" has to mean something concrete.

- A. Annotated ASCII wireframes per screen with all five states, plus measured token values
- B. Structured layout descriptions (regions, hierarchy, tokens) with no ASCII drawing
- C. ASCII wireframes for the new screens only, descriptions for changed ones
- D. Link to the live components and annotate only what changes
- X. Other (please specify)

[Answer]: A — annotated ASCII wireframes per screen with all five states, plus measured token values. **Mode:** guided

## Q3. [designer] Where does the session/device list live in the information architecture?

There is no sessions screen today. Settings has four destinations: profile, preferences, consent,
privacy. The stories need a device list with last-seen times and revoke actions.

- A. A fifth settings destination, "Sessions", beside the existing four
- B. A section inside the existing Profile panel
- C. A section inside Privacy, grouping it with data-control actions
- D. A top-level page at `/sessions`, linked from the user menu rather than settings
- X. Other (please specify)

[Answer]: A — a fifth settings destination, "Sessions", beside the existing four. **Mode:** guided

## Q4. [designer] How should the limited-session state present itself?

An exchange-credential user without a second factor is confined to the MFA and settings screens. The
requirement is that they know exactly what to do; the risk is that it feels like an error.

- A. A persistent banner plus a redirect: the MFA screen opens with an explanation of why the
  restriction exists and what lifts it (recommended)
- B. A blocking modal on every attempt to reach another page
- C. A dedicated interstitial page the user cannot dismiss
- D. Silent redirect to the MFA screen with no explanation
- X. Other (please specify)

[Answer]: A — a persistent banner plus a redirect: the MFA screen opens with an explanation of why the restriction exists and what lifts it. **Mode:** guided

## Q5. [designer] How should destructive session actions be confirmed?

Revoking the current session and "sign out everywhere else" both end access. The design system pairs
every destructive action with a confirmation naming what is affected.

- A. A confirmation dialog for both, naming how many sessions end and which, and requiring an
  explicit confirm for the current session (recommended)
- B. Confirmation only for the current session; revoke-all is undoable through a toast instead
- C. Confirmation for both, with no undo
- D. No confirmation; rely on the action being reversible by signing in again
- X. Other (please specify)

[Answer]: A — a confirmation dialog for both, naming how many sessions end and which, and requiring an explicit confirm for the current session. **Mode:** guided

## Q6. [designer] How much should the mockups say about copy?

Several criteria depend on wording: the non-revealing sign-in error, the "link no longer valid"
message, the outage message, and the revocation-window help text.

- A. Write the exact user-facing copy for every state, including the non-revealing variants
  (recommended)
- B. Give the intent of each message and leave the wording to implementation
- C. Write copy only where the requirement constrains it (errors, refusals, warnings)
- D. Reuse the existing `web/src/i18n` keys and add no new copy
- X. Other (please specify)

[Answer]: A — write the exact user-facing copy for every state, including the non-revealing variants. **Mode:** guided
