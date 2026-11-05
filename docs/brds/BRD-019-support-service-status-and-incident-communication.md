# BRD-019 — Support, Service Status & Incident Communication

Draft v1.0 · Operations/Product · 2026-11-04. A global paid product is judged as much by what happens when something breaks as by the happy path. Support must resolve problems without seeing secrets, and incidents must be communicated before users discover them.

**In scope:** help centre · in-app help and contact · support tickets · support SLA by plan · read-only, time-bound support access with audit · status page · incident and maintenance communication · severity model and postmortems · feedback and bug reporting · knowledge base · localized support handoff.
**Out of scope:** telephone support at launch · 24/7 coverage at launch · in-person support · legal advice · financial advice.

## Support tiers

| Plan | Channel | First response | Resolution target |
|------|---------|----------------|-------------------|
| Free | Help centre + email | 2 business days | Best effort |
| Pro | Email + in-app | 1 business day | 3 business days |
| Team | Email + in-app + named contact | 4 business hours | 2 business days |
| Enterprise | Contractual | Contractual | Contractual |

Support hours are published per region. Security and billing incidents are never queued behind general support.

## Severity model

| Severity | Definition | Communication |
|----------|------------|---------------|
| SEV-1 | Data loss, credential exposure, or all users unable to access portfolios | Status page within 15 minutes; updates every 30 minutes |
| SEV-2 | Major feature unavailable for many users | Status page within 30 minutes; updates hourly |
| SEV-3 | Degraded experience or a subset of users affected | Status page when confirmed; daily update |
| SEV-4 | Cosmetic or single-user issue | Ticket response |

## Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-SUP-01 | Provide a searchable help centre covering onboarding, accounts, valuation, P&L, billing, and privacy. | Must |
| FR-SUP-02 | Provide in-app contact and bug report with context that excludes secrets. | Must |
| FR-SUP-03 | Provide a ticket system with status, history, and email notifications. | Must |
| FR-SUP-04 | Localize the help centre and support handoff for launch locales. | Should |
| FR-SUP-05 | Publish support hours, response targets, and escalation paths. | Must |
| FR-SUP-06 | Give support read-only, time-bound access to a user's account only with a recorded reason. | Must |
| FR-SUP-07 | Audit every support access and every data view. | Must |
| FR-SUP-08 | Never display credentials, tokens, or full seed phrases to support. | Must |
| FR-SUP-09 | Publish a status page with component health and incident history. | Must |
| FR-SUP-10 | Notify subscribers of incidents and maintenance by email and in-app. | Should |
| FR-SUP-11 | Publish a postmortem for every SEV-1 and SEV-2 within five business days. | Should |
| FR-SUP-12 | Provide an in-app feedback channel with triage and closure. | Should |
| FR-SUP-13 | Link errors to actionable help articles. | Should |
| FR-SUP-14 | Track first response, resolution time, reopen rate, and satisfaction. | Must |
| FR-SUP-15 | Escalate security and billing incidents immediately. | Must |
| FR-SUP-16 | Provide a public changelog for user-visible changes. | Should |
| FR-SUP-17 | Support account recovery without exposing secrets. | Must |
| FR-SUP-18 | Let users reopen a ticket and view prior conversations. | Must |
| FR-SUP-19 | Provide accessibility for support channels. | Must |
| FR-SUP-20 | Maintain runbooks for common support scenarios. | Must |

## Business rules

| ID | Rule |
|----|------|
| BR-SUP-01 | Support access is least-privilege, time-bound, reason-recorded, and audited. |
| BR-SUP-02 | Support never asks for a password, private key, seed phrase, or full API secret. |
| BR-SUP-03 | A support action never changes portfolio data without the user's explicit request. |
| BR-SUP-04 | Security and billing incidents bypass general queues. |
| BR-SUP-05 | Status page updates are factual and give the next update time. |
| BR-SUP-06 | No incident is closed without a user-visible resolution or workaround. |
| BR-SUP-07 | Support metrics are reported monthly; alerts that nobody acts on are removed. |
| BR-SUP-08 | User data seen in support is retained only per BRD-017. |
| BR-SUP-09 | Ticket content is not used for marketing without consent. |
| BR-SUP-10 | A user can request deletion of support attachments. |

## Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-SUP-01 | A user submits a ticket and receives an acknowledgement with a ticket ID and target response. |
| AC-SUP-02 | Support opens a user account with a recorded reason; the access appears in the audit log. |
| AC-SUP-03 | No support screen displays an exchange secret or token. |
| AC-SUP-04 | A SEV-1 incident appears on the status page within 15 minutes. |
| AC-SUP-05 | A status-page subscriber receives incident and resolution notices. |
| AC-SUP-06 | A SEV-1 postmortem is published within five business days. |
| AC-SUP-07 | Support access expires automatically after the configured window. |
| AC-SUP-08 | A reopened ticket retains its history. |
| AC-SUP-09 | Support response and resolution times are reported monthly. |
| AC-SUP-10 | The help centre is reachable and readable at mobile width. |

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Which support channels and hours launch first? | Operations |
| OQ-002 | Which help-desk platform and data region? | Operations/Legal |
| OQ-003 | What are the contractual SLAs for Team and Enterprise? | Leadership |
| OQ-004 | Is live chat required at launch? | Product |
| OQ-005 | Who owns the status page and incident communications rota? | Operations |
