# Business Rules — u8-reporting

```yaml
rules:
  - id: BR1.1
    statement: A brief is generated once per user, period, and period start.
    category: constraint
    applies_to: Brief
    trigger: The scheduled generation runs.
    logic: IF a generated brief exists for (userId, period, periodStart) THEN skip; IF one is failed or has been pending for over an hour THEN retry that same brief; ELSE create it pending and generate.
    violation: The same period never yields two briefs, and a failed brief is retried.
    source: FR7.1, AC6.1.1

  - id: BR1.2
    statement: A generated brief states its period, as-of time, and data status.
    category: constraint
    applies_to: Brief
    trigger: A brief is opened.
    logic: IF status is generated THEN period, asOf, and dataStatus are present on the brief and on every figure.
    violation: A figure is never shown without its time and status.
    source: FR7.1, AC6.1.3

  - id: BR1.3
    statement: A missing figure is an em dash, never zero.
    category: constraint
    applies_to: Brief
    trigger: A figure is unavailable.
    logic: IF a value is absent or unpriced THEN render an em dash with the data status; never substitute 0.
    violation: A brief never reports a fabricated figure.
    source: design-system, FR3.5

  - id: BR2.1
    statement: Generation failure is explicit with a reason.
    category: policy
    applies_to: Brief
    trigger: Generation runs.
    logic: IF no portfolio valuation can be read at all THEN set status failed with a non-secret reason (valuation unavailable); IF a source is degraded but usable THEN generate with dataStatus partial/unavailable.
    violation: The user never sees a silently missing brief.
    source: AC6.1.2

  - id: BR2.2
    statement: Email respects the brief email preference; in-app always shows.
    category: policy
    applies_to: BriefDelivery
    trigger: A brief is generated.
    logic: Reporting ALWAYS submits a C9 `brief` alert (email channel, idempotency key `brief:{briefId}:email`) and keeps the in-app brief; Notification suppresses the email when the user's brief email preference is off. Reporting does not read preferences.
    violation: A disabled brief email sends nothing; the in-app brief always exists.
    source: FR7.1, AC6.2.3

  - id: BR2.3
    statement: The email goes to the verified address and delivery outcomes are recorded.
    category: policy
    applies_to: BriefDelivery
    trigger: An email is attempted.
    logic: IF C9 accepts the alert THEN record pending/sent with the alert id; IF it fails THEN record failed and retry through C9's bounded retry.
    violation: Every delivery attempt has an outcome.
    source: AC6.2.1–6.2.2

  - id: BR3.1
    statement: Briefs are owner-scoped and never expose another user's data.
    category: authorization
    applies_to: Brief
    trigger: Any read.
    logic: IF the caller is the owner THEN allow; ELSE not-found.
    violation: No cross-user exposure.
    source: FR1.6
```

## Rules summary

| Rule | Category | Statement |
|---|---|---|
| BR1.1 | constraint | One brief per period |
| BR1.2 | constraint | Period, asOf, dataStatus on every figure |
| BR1.3 | constraint | Missing figure is an em dash |
| BR2.1 | policy | Explicit failure with a reason |
| BR2.2 | policy | Email preference respected; in-app always |
| BR2.3 | policy | Verified address, outcomes recorded |
| BR3.1 | authorization | Owner-scoped |
