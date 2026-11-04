# BRD-021 — Product Analytics, Experimentation & Measurement

Draft v1.0 · Product/Data · 2026-11-04. A global product cannot be improved by opinion alone. This BRD defines the event taxonomy, the metric tree, consent-aware analytics, and the guardrails for experiments.

**In scope:** event taxonomy · north-star and supporting metrics · activation, engagement, retention, conversion, trust, and quality metrics · consent-aware analytics · dashboards and alerts · experimentation and feature flags · guardrail metrics · data quality.
**Out of scope:** advertising identifiers · cross-site tracking · selling data · individual-level surveillance · PII in analytics · growth hacking that violates consent.

## Metric tree

| Layer | Metric | Definition |
|-------|--------|------------|
| North star | Weekly active portfolios with a valid valuation | A portfolio with at least one priced asset viewed or refreshed in the week |
| Activation | First value | First portfolio valuation within 24 hours of sign-up |
| Activation | Time to first value | Median minutes from sign-up to first valuation, by path |
| Engagement | Weekly brief opens | Briefs opened ÷ briefs delivered |
| Engagement | Alert action rate | Alerts leading to a portfolio/account view |
| Retention | D1/D7/D30/D90 | User returns within the window |
| Monetization | Free → paid conversion | Paid subscriptions ÷ activated free users |
| Trust | Data coverage | Share of portfolio value with a fresh price |
| Trust | P&L availability | Share of portfolios with complete transaction history |
| Quality | Sync success rate | Successful syncs ÷ attempted syncs |
| Quality | Support contact rate | Tickets ÷ weekly active users |
| Quality | Error rate | Failed API requests ÷ total requests |

## Event taxonomy

Events are named `object_action`, carry a consent state, and never contain PII. Example events:

| Event | Properties (non-PII) |
|-------|----------------------|
| `user_signed_up` | locale, base_currency, auth_method |
| `onboarding_path_selected` | path, step |
| `account_created` | type, chain_count, exchange_code |
| `account_sync_completed` | duration_bucket, asset_count, error_code (if any) |
| `portfolio_created` | source, target_template_used |
| `valuation_viewed` | portfolio_age_bucket, asset_count, unpriced_count |
| `transaction_imported` | source, row_count_bucket, error_count |
| `pnl_viewed` | coverage_bucket |
| `alert_delivered` | type, channel |
| `alert_clicked` | type, channel |
| `brief_opened` | period |
| `checkout_started` | plan, billing_period |
| `subscription_activated` | plan, billing_period, currency |
| `support_ticket_created` | category |
| `export_requested` | format, scope |

## Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-ANL-01 | Maintain a versioned event taxonomy with owners and descriptions. | Must |
| FR-ANL-02 | Collect analytics only with the consent required in the user's jurisdiction. | Must |
| FR-ANL-03 | Never send PII, addresses, emails, credentials, or tokens to analytics. | Must |
| FR-ANL-04 | Provide activation, engagement, retention, conversion, trust, and quality dashboards. | Must |
| FR-ANL-05 | Track time to first value by onboarding path. | Must |
| FR-ANL-06 | Track funnel drop-off per onboarding and import step. | Must |
| FR-ANL-07 | Track data coverage and P&L availability as product-health metrics. | Must |
| FR-ANL-08 | Track support contact rate and top ticket categories. | Should |
| FR-ANL-09 | Alert when a guardrail metric breaches its threshold. | Should |
| FR-ANL-10 | Support feature flags with owner, expiry, and rollout percentage. | Must |
| FR-ANL-11 | Support experiments with a pre-registered hypothesis, sample size, and guardrails. | Should |
| FR-ANL-12 | Stop an experiment automatically when a guardrail is breached. | Should |
| FR-ANL-13 | Record experiment exposure without PII. | Must |
| FR-ANL-14 | Provide a data-quality check for missing or duplicated events. | Should |
| FR-ANL-15 | Document retention and deletion of analytics data (BRD-017). | Must |

## Business rules

| ID | Rule |
|----|------|
| BR-ANL-01 | No PII or secrets in analytics events. |
| BR-ANL-02 | Consent is checked before any non-essential analytics call. |
| BR-ANL-03 | Withdrawing consent stops collection immediately. |
| BR-ANL-04 | Metrics are defined once and used consistently across dashboards. |
| BR-ANL-05 | Experiments never degrade security, privacy, or accessibility. |
| BR-ANL-06 | An experiment has an owner, a hypothesis, a duration, and a guardrail. |
| BR-ANL-07 | Feature flags expire; a flag without an owner is removed. |
| BR-ANL-08 | Analytics never becomes a shadow user database. |
| BR-ANL-09 | Metrics that nobody acts on are reviewed and removed. |
| BR-ANL-10 | Product decisions record the metric they intend to move. |

## Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-ANL-01 | A user without analytics consent produces no non-essential events. |
| AC-ANL-02 | A `valuation_viewed` event contains no email, address, or portfolio name. |
| AC-ANL-03 | Time-to-first-value is visible by onboarding path. |
| AC-ANL-04 | The activation funnel identifies the step with the largest drop-off. |
| AC-ANL-05 | A guardrail breach pauses an active experiment. |
| AC-ANL-06 | An expired feature flag is removed or renewed with an owner. |
| AC-ANL-07 | Data-coverage and P&L-availability dashboards are available to product and operations. |
| AC-ANL-08 | Analytics data is deleted according to the retention schedule. |
| AC-ANL-09 | Every experiment has a documented hypothesis and guardrail. |
| AC-ANL-10 | No analytics dashboard exposes individual user PII. |

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Which analytics platform is consent-compliant in launch markets? | Legal/Product |
| OQ-002 | Which metrics are the first-quarter success criteria? | Product |
| OQ-003 | What sample-size and guardrail policy applies to experiments? | Data |
| OQ-004 | Which feature-flag system is used across services? | Engineering |
| OQ-005 | What is the analytics retention period? | Legal |
