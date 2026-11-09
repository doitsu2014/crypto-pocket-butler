# Entity Model — u8-reporting

Inputs: U8 in `inception/units-generation/unit-of-work.md`,
`inception/requirements-analysis/requirements.md` (FR7.1), the Reporting component in
`inception/domain-design/components.md`, and C7/C8/C9/C10.

```yaml
entities:
  - name: Brief
    description: One daily or weekly brief for one user.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: userId
        type: uuid
        required: true
      - name: period
        type: enum
        required: true
        allowed_values: [daily, weekly]
      - name: periodStart
        type: timestamp
        required: true
      - name: periodEnd
        type: timestamp
        required: true
      - name: status
        type: enum
        required: true
        allowed_values: [pending, generated, failed]
        default: pending
      - name: failureReason
        type: string
        required: false
        description: A non-secret reason shown to the user when generation fails.
      - name: asOf
        type: timestamp
        required: false
        description: The valuation/P&L as-of time of the data used.
      - name: dataStatus
        type: enum
        required: false
        allowed_values: [complete, partial, unavailable]
      - name: sections
        type: map
        required: false
        description: The generated figures and narrative (valuation, drift, P&L with coverage).
      - name: generatedAt
        type: timestamp
        required: false
    constraints:
      - (userId, period, periodStart) is unique.
      - A brief is either generated with asOf/dataStatus or failed with a reason; never both.
      - Every figure in sections carries the brief's asOf and dataStatus; a missing figure is an em dash, never zero.

  - name: BriefDelivery
    description: One delivery of a brief on one channel.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: briefId
        type: uuid
        required: true
      - name: channel
        type: enum
        required: true
        allowed_values: [in_app, email]
      - name: status
        type: enum
        required: true
        allowed_values: [pending, sent, failed]
        default: pending
      - name: alertId
        type: string
        required: false
        description: The C9 alert id for the email channel.
      - name: attemptedAt
        type: timestamp
        required: true
    constraints:
      - In-app delivery is the stored brief itself; email goes through C9 type `brief`.
      - Email respects the user's brief email preference; in-app is always shown.
```

## Summary

Reporting owns briefs and their deliveries. A brief is generated from Portfolio's valuation and
snapshot data (C8) and Ledger's P&L (C7), carries an as-of time and a data status on every figure,
and either succeeds or fails with a visible reason. Email is handed to Notification (C9), which
applies the user's preference.
