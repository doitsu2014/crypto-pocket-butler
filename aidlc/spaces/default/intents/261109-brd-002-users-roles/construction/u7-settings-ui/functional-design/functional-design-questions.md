# Functional Design Questions — u7-settings-ui

**Unit:** U7 — Settings UI (`u7-settings-ui`)
**Stories:** US1.4, US4.3, US6.1 · **Requirements:** FR14.4, FR17.3, FR19.1
**Upstream:** `unit-of-work.md` (U7), `unit-of-work-story-map.md`, `stories.md`, `requirements.md`,
`components.md` (`AccountSettingsUI`), `contract-summary.md` (C1/C2, C3, C6, C7), `mockups.md` §1
and §1a, `interaction-spec.md`, `design-system-mapping.md`, `accessibility-checklist.md`

The requirements and the refined mockups settle what the settings surface must show. These
questions cover only what they leave open — mostly where the screens meet a backend that does not
yet expose the call they need.

## Q1. [Engineering] How much of the missing backend may this unit add?

The screens need three things the backend does not expose today. First, a way to save a display
name the person has chosen: the user registry holds the override it needs but nothing sets it,
and the current page calls a route that does not exist. Second, a way to read the person's open
deletion request, without which the recovery screens cannot show the date the account is restored
until. Third, any gateway route at all for deletion: the `/v1` route table declares no deletion
path, so the call the account lifecycle service already serves cannot be reached from the browser.

- A. This unit may add the missing owner-scoped routes and their gateway entries inside the
  Identity service — no new service, no new transport, no new table — and the plan records each
  as a contract addition.
- B. This unit stays presentation-only: it renders only what the backend exposes today, and each
  missing route becomes a follow-up change, with the acceptance criteria it cannot reach recorded
  as deferred rather than claimed.
- C. This unit stays presentation-only except for the one read the recovery screens cannot work
  without — the person's own open deletion request — which it adds.

[Answer]: A — This unit adds the missing owner-scoped routes and their gateway entries inside the
Identity service, with no new service, transport, or table; the plan records each as a contract
addition.

## Q2. [Product] What does saving a display name mean?

The provider supplies a display name and the requirements say the profile keeps it "unless I have
overridden them" (AC1.4.1). The registry already records whether a field was overridden, so a
later provider refresh cannot overwrite a person's choice, but nothing in the product sets that
marker today.

- A. The display name is editable, saving it marks the field as overridden so later provider
  refreshes leave it alone, and a "use my provider name again" action clears the override.
- B. The display name is editable and saving marks it overridden, with no reset control in this
  release.
- C. The display name is read-only here and always shows the provider's value.

[Answer]: A — Editable; saving marks the field overridden so later provider refreshes leave it
alone, and a "use my provider name again" action clears the override.

## Q3. [Product] Which notification switches does the screen offer?

The settings mockup draws three toggles — Daily brief, Alerts, Digest. The preferences service
stores two channels only, email and push, and nothing in it names a brief, alert, or digest. A
save can persist only what the service stores.

- A. Two channel switches, email and push — exactly the values that exist and can be saved.
- B. The three topic toggles as drawn, which means the preferences service must gain those topics
  first; the panel ships once it does.
- C. Two channel switches now, with a short line stating that per-brief controls are not available
  yet.

[Answer]: A — Two channel switches, email and push: exactly the values that exist and can be
saved.

## Q4. [Product] What can a person change about their consent from settings?

There are four consent kinds: terms, risk, and age, which gate connecting a source, and analytics,
which is optional. The sign-up flow records the first three along with the policy version the
person read. The settings mockup shows analytics with the date it was withdrawn.

- A. Full self-service: every kind's current state and history is shown, any kind can be
  withdrawn, and a withdrawn kind can be granted again with the current policy version.
- B. Analytics only: the person can turn analytics on and off here, while terms, risk, and age are
  shown read-only because they gate source connections and are re-accepted in that flow.
- C. Read-only: settings shows the consent history with no controls, and every record is made in
  the sign-up gate flow.

[Answer]: B — Analytics only: the person can turn analytics on and off here, while terms, risk,
and age are shown read-only because they gate source connections and are re-accepted in that flow.

## Q5. [Product] Where does deleting an account live, and does it ask for a reason?

The mockup gives deletion its own page with a what-happens list, a what-is-kept list, an optional
reason, and a second explicit confirmation. The deletion call takes no request body, so an
optional reason cannot be stored; its response does carry the recovery date and the categories of
records that are kept.

- A. A dedicated page matching the mockup, with the reason field left out because nothing can
  store it, and the what-is-kept list filled from the categories the service returns.
- B. A dedicated page with the optional reason, which means the deletion call gains that field
  first.
- C. Deletion stays where it is in the privacy page, with the confirmation dialog it already has.

[Answer]: A — A dedicated page matching the mockup, with the reason field left out because nothing
can store it, and the what-is-kept list filled from the categories the service returns.

## Q6. [Product] How does a returning pending-deletion person learn when their account goes?

Once the person reloads or signs in on another device, nothing reads their open deletion request,
and the principal's next step never reports a restore. So a recovery banner has no date to show
today.

- A. The banner shows the exact date read from the open request, with Restore as the primary
  action.
- B. No extra read: the banner states the 30-day rule and offers Restore, without a date.
- C. The banner shows the date the person was told when they requested deletion, remembered in
  this browser, and states the rule elsewhere.

[Answer]: A — The banner shows the exact date read from the open request, with Restore as the
primary action.

## Q7. [Engineering] What happens to the two settings pages that already exist?

The SPA already has a settings page and a privacy page, and both call routes that do not exist on
the server: a display-name save, preferences, consent, and sessions, none of which match what the
Identity service serves. The settings page also shows device sessions, which is outside this
unit's scope.

- A. Rewire in place: the existing settings and privacy pages become this unit's surface against
  the real routes, and the dead calls are removed.
- B. Leave them as they are and build this unit's surface in new modules; the old pages are
  removed in a later cleanup.
- C. Keep the old pages alongside the new surface.

[Answer]: A — Rewire in place: the existing settings and privacy pages become this unit's surface
against the real routes, and the dead calls are removed.