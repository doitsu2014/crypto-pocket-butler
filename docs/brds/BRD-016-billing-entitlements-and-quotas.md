# BRD-016 — Billing, Entitlements & Quotas

Draft v1.0 · Product/Finance · 2026-11-04. A global consumer product needs a lawful, localised way to charge for value and a server-side way to enforce what each plan includes. Pricing remains a hypothesis until validated; the mechanics must not be.

**In scope:** plan catalogue · entitlements and quotas · subscription lifecycle (trial, active, past due, cancelled, expired) · monthly and annual billing · local currencies and purchasing-power pricing · taxes (VAT/GST/sales tax) · invoices and receipts · refunds, dunning, and chargebacks · coupons and referrals · entitlement enforcement · abuse limits · app-store billing compatibility.
**Out of scope:** crypto payments at launch (Could) · enterprise contracts and procurement · reseller/white-label billing · tax advice.

## Plans and entitlements

The commercial hypothesis from BRD-001 is refined here; prices are not committed until OQ-001 closes. **Free and Pro launch first; Team and Enterprise are post-MVP and depend on shared portfolios/multi-user (BRD-002).**

| Entitlement | Free | Pro | Team | Enterprise |
|-------------|------|-----|------|------------|
| Accounts / wallets | 3 | 50 | 250 | Custom |
| Portfolios | 1 | 10 | 50 | Custom |
| Members | 1 | 1 | 10 | Custom |
| Price refresh | 15 min | 1 min (top assets) | 1 min | Custom |
| Snapshot history | 90 days | 5 years | 5 years | Custom |
| Transaction history | 1 year | Full | Full | Custom |
| Notifications | In-app + email | + push, digests | + team routing | Custom |
| Exports | CSV, monthly | CSV/PDF, on demand | + scheduled | Custom |
| API access | No | Read-only | Read-only | Custom |
| Support | Help centre | Priority email | Named contact | SLA |

## Subscription model

| State | Meaning | Access |
|-------|---------|--------|
| `trialing` | Time-limited trial | Plan entitlements; card optional where the provider supports it |
| `active` | Paid and current | Plan entitlements |
| `past_due` | Renewal failed | Grace period, then downgrade to Free |
| `cancelled` | User cancelled | Entitlements until period end |
| `expired` | Period ended | Free entitlements; data retained per BRD-017 |

Downgrades never delete data. Exceeding an entitlement makes the excess read-only, not destroyed; the user chooses what to keep active. History limits govern what a plan can **view and export**; they do not shorten the underlying retention in BRD-017.

## Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-BIL-01 | Show plan catalogue, prices, billing period, and tax-inclusive or tax-exclusive status before checkout. | Must |
| FR-BIL-02 | Support monthly and annual billing with a clear renewal date. | Must |
| FR-BIL-03 | Use a merchant of record or equivalent to calculate and remit VAT/GST/sales tax. | Must |
| FR-BIL-04 | Display prices in the user's local currency where supported, with purchasing-power pricing where approved. | Should |
| FR-BIL-05 | Provide a trial with explicit duration and conversion terms. | Should |
| FR-BIL-06 | Enforce entitlements and quotas server-side on every gated action. | Must |
| FR-BIL-07 | Show usage against quota (accounts, portfolios, exports, API calls). | Should |
| FR-BIL-08 | Support upgrade, downgrade, cancel, and reactivate. | Must |
| FR-BIL-09 | Send renewal, payment-failure, and cancellation notices; honour auto-renewal disclosure rules. | Must |
| FR-BIL-10 | Provide invoices and receipts in the user's locale and currency. | Must |
| FR-BIL-11 | Handle dunning with a defined grace period; never delete data on first failure. | Must |
| FR-BIL-12 | Support refunds and chargebacks with an auditable record. | Must |
| FR-BIL-13 | Support coupons, referrals, and promotional entitlements with expiry. | Could |
| FR-BIL-14 | Record plan changes with actor, time, and reason. | Must |
| FR-BIL-15 | Prevent one user from bypassing quotas through multiple accounts or API keys. | Must |
| FR-BIL-16 | Respect app-store billing rules if a native app is shipped. | Should |
| FR-BIL-17 | Expose billing state to support with read-only access and audit. | Should |
| FR-BIL-18 | Provide a tax-ready export of subscription revenue per jurisdiction. | Should |
| FR-BIL-19 | Handle provider outages without blocking core read-only portfolio access. | Must |
| FR-BIL-20 | Let users delete payment methods and close billing without deleting portfolio data. | Must |

## Business rules

| ID | Rule |
|----|------|
| BR-BIL-01 | Entitlements are enforced on the server, never only in the client. |
| BR-BIL-02 | Prices show whether tax is included; the final charge is shown before confirmation. |
| BR-BIL-03 | Auto-renewal is disclosed before purchase and in the receipt; cancellation is as easy as sign-up. |
| BR-BIL-04 | Downgrade or expiry preserves data and makes excess read-only. |
| BR-BIL-05 | A payment failure does not delete portfolios, accounts, snapshots, or transactions. |
| BR-BIL-06 | Billing records are retained per BRD-017 and applicable tax law. |
| BR-BIL-07 | Quotas are measured consistently and shown to the user before they are hit. |
| BR-BIL-08 | Support cannot change a plan or price without an audited admin action. |
| BR-BIL-09 | App-store billing rules override web checkout where required. |
| BR-BIL-10 | No dark patterns in cancellation, trial conversion, or renewal. |

## Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-BIL-01 | A Free user adding a fourth account is blocked with an upgrade path and no data loss. |
| AC-BIL-02 | A Pro user downgrades to Free; excess accounts become read-only and remain visible. |
| AC-BIL-03 | A failed renewal enters `past_due`, sends a notice, and preserves data through the grace period. |
| AC-BIL-04 | Checkout shows tax status, final charge, and renewal date before confirmation. |
| AC-BIL-05 | Cancellation takes effect at period end and is confirmed by email. |
| AC-BIL-06 | A quota is enforced server-side even when the client bypasses the UI. |
| AC-BIL-07 | Every plan change appears in the audit log. |
| AC-BIL-08 | A refund produces an auditable record and updates entitlements correctly. |
| AC-BIL-09 | An expired plan leaves portfolio data intact and readable. |
| AC-BIL-10 | No billing screen uses pre-checked consent or hidden cancellation. |

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | What prices and plan boundaries are committed for launch? | Product/Leadership |
| OQ-002 | Merchant of record: Paddle, Lemon Squeezy, Stripe Tax, or another? | Finance/Engineering |
| OQ-003 | Which local currencies and purchasing-power tiers launch first? | Product/Finance |
| OQ-004 | Is a free tier required in every launch market? | Product |
| OQ-005 | What grace period applies to `past_due`? | Finance |
| OQ-006 | Will a native app use IAP, web checkout, or both? | Product/Legal |
