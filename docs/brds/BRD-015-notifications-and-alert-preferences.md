# BRD-015 — Notifications & Alert Preferences

Draft v1.0 · Product/Operations · 2026-11-04. Drift, stale data, a revoked exchange key, or a completed brief is only useful if it reaches the user. The MVP's differentiators are passive without a delivery layer; this BRD makes them active.

**In scope:** in-app, email, and web-push notifications · per-type and per-channel preferences · quiet hours and timezone · digest and instant modes · security alerts · delivery log and retries · unsubscribe · localization · provider failure handling.
**Out of scope:** SMS at launch (Could) · mobile native push (post-MVP, BRD-001/012) · marketing campaigns (consent-governed, BRD-014/021) · trade execution or advice.

## Notification catalogue

| Type | Default channel | Mandatory | Trigger |
|------|-----------------|-----------|---------|
| Drift beyond guardrail | In-app + email | No | Valuation crosses the portfolio guardrail (BRD-006/007) |
| Price alert | In-app + push | No | User-defined price threshold (BRD-005) |
| Sync failure | In-app + email | No | An account sync fails repeatedly (BRD-004) |
| Exchange key revoked/expired | In-app + email | **Yes** | Permission verification fails (BRD-004) |
| Brief ready | In-app + email | No | Daily/weekly brief generated (BRD-009) |
| Security: new sign-in | Email | **Yes** | New device or unusual location (BRD-003) |
| Billing: payment failed | In-app + email | **Yes** | Renewal failure (BRD-016) |
| Product update | In-app | No | Release note published |
| Tax-season reminder | Email | No | Configured season (BRD-013) |

Mandatory alerts are security, billing, and credential-integrity events. They cannot be disabled entirely; the user may choose the channel.

## Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-NOT-01 | Deliver in-app notifications with read/unread state and history. | Must |
| FR-NOT-02 | Deliver email notifications through a transactional provider. | Must |
| FR-NOT-03 | Deliver web-push notifications to installed PWAs. | Should |
| FR-NOT-04 | Let users choose channel per notification type. | Must |
| FR-NOT-05 | Let users set quiet hours and timezone; non-urgent alerts wait. | Should |
| FR-NOT-06 | Support instant and daily/weekly digest modes per type. | Should |
| FR-NOT-07 | Mandatory security and billing alerts cannot be fully disabled. | Must |
| FR-NOT-08 | Provide one-click unsubscribe for non-mandatory email; honour it immediately. | Must |
| FR-NOT-09 | Deduplicate alerts per user, type, and subject within a configurable window. | Must |
| FR-NOT-10 | Rate-limit alerts per user and channel to prevent storms. | Must |
| FR-NOT-11 | Record delivery attempts, provider response, and final status without message secrets. | Must |
| FR-NOT-12 | Retry bounded transient failures with backoff; never duplicate a delivered alert. | Must |
| FR-NOT-13 | Degrade gracefully when a provider is down; keep the in-app record. | Must |
| FR-NOT-14 | Localize notification content for the user's locale. | Must |
| FR-NOT-15 | Include a deep link to the relevant portfolio, account, or brief. | Should |
| FR-NOT-16 | Include data freshness and the data timestamp in valuation alerts. | Must |
| FR-NOT-17 | Never include credentials, tokens, private keys, or full addresses in email/push. | Must |
| FR-NOT-18 | Let users pause all non-mandatory notifications. | Should |
| FR-NOT-19 | Expose notification preferences in settings and during onboarding. | Must |
| FR-NOT-20 | Support admin-triggered operational notices to affected users. | Could |

## Business rules

| ID | Rule |
|----|------|
| BR-NOT-01 | Notification consent is explicit; non-mandatory channels are opt-in. |
| BR-NOT-02 | Mandatory security and billing alerts cannot be fully disabled. |
| BR-NOT-03 | Every alert is tied to one owner and never crosses users. |
| BR-NOT-04 | Notification content is factual and never investment advice. |
| BR-NOT-05 | No credentials or secrets appear in any channel or delivery log. |
| BR-NOT-06 | Quiet hours are respected for non-urgent alerts; mandatory alerts bypass them. |
| BR-NOT-07 | An alert is delivered at most once per dedupe window. |
| BR-NOT-08 | Provider failure never loses the in-app record. |
| BR-NOT-09 | Unsubscribe is immediate and auditable. |
| BR-NOT-10 | Alerts carry the data timestamp when they reference valuation or price. |

## Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-NOT-01 | A drift breach creates one in-app alert and one email when email is enabled. |
| AC-NOT-02 | A user disables drift email; no email is sent, but the in-app alert remains. |
| AC-NOT-03 | A revoked exchange key produces a mandatory alert on all enabled channels. |
| AC-NOT-04 | Quiet hours delay a non-urgent alert until the window ends. |
| AC-NOT-05 | Re-running the same trigger within the dedupe window produces no duplicate. |
| AC-NOT-06 | A provider outage leaves the in-app notification and records the failure. |
| AC-NOT-07 | One-click unsubscribe stops further non-mandatory email. |
| AC-NOT-08 | No notification or delivery log contains a credential or token. |
| AC-NOT-09 | A localized user receives the notification in their locale. |
| AC-NOT-10 | A mandatory billing alert cannot be disabled from settings. |

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Resolved by D-008: in-app and email at launch; web push is a Should. | Product |
| OQ-002 | Which transactional email provider and data region? | Engineering/Legal |
| OQ-003 | Are SMS or messaging-app alerts needed for launch markets? | Product |
| OQ-004 | What are the dedupe windows per alert type? | Product |
| OQ-005 | Which alerts are mandatory in each jurisdiction? | Compliance |
