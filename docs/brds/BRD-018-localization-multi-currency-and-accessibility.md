# BRD-018 — Localization, Multi-Currency & Accessibility

Draft v1.0 · Product/Design · 2026-11-04. Global retail is not a translated English product. Users think in their own currency, read their own language, and use assistive technology. This BRD makes locale, currency, timezone, and accessibility first-class requirements.

**In scope:** locale model · language and translation · base currency and FX display · number, date, and currency formatting · timezone and EOD cutoffs · translated legal documents for launch markets · right-to-left support where required · accessible components and flows · localized notifications and emails · localized support handoff.
**Out of scope:** translating every marketing page at launch · all world languages · jurisdiction-specific tax calculation · culturally specific advice.

## Locale model

| Dimension | Examples | Default |
|-----------|----------|---------|
| Language | `en`, `vi`, `es`, `pt-BR`, `fr`, `de`, `ja`, `ko` | Browser, then English |
| Base currency | `USD`, `EUR`, `GBP`, `JPY`, `VND`, `BRL` | User choice; USD fallback |
| Timezone | IANA zone | Browser |
| Number format | `1,234.56` / `1.234,56` | Locale |
| Date format | `MM/DD/YYYY` / `DD/MM/YYYY` / `YYYY-MM-DD` | Locale |
| Text direction | LTR / RTL | Language |

Locale changes never change stored business values. Amounts are stored in the transaction currency and converted at display; the FX timestamp is shown.

## Launch locales

The launch set is a decision, not an assumption. The default recommendation is English plus the top three languages by validated demand; legal documents are translated for every launch market where consumer law requires it. Additional languages follow usage data, not ambition.

## Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-L10N-01 | Store a locale, base currency, and timezone per user. | Must |
| FR-L10N-02 | Format numbers, currency, percentages, and dates according to locale. | Must |
| FR-L10N-03 | Display each amount in the user's base currency with the FX rate and timestamp. | Must |
| FR-L10N-04 | Keep the original transaction currency and value alongside the converted amount. | Must |
| FR-L10N-05 | Use a sourced FX series with historical rates for transactions and snapshots. | Must |
| FR-L10N-06 | Apply the user's timezone to EOD snapshots, briefs, quiet hours, and date grouping. | Must |
| FR-L10N-07 | Provide translation for all UI, errors, notifications, and onboarding in launch locales. | Must |
| FR-L10N-08 | Support pluralisation, gender, and locale-specific currency symbols. | Must |
| FR-L10N-09 | Translate legal documents (terms, privacy, risk disclosure) for launch markets where required. | Must |
| FR-L10N-10 | Support RTL layout when an RTL locale is launched. | Could |
| FR-L10N-11 | Meet WCAG 2.2 AA for core flows: keyboard, focus, contrast, labels, errors, motion. | Must |
| FR-L10N-12 | Provide accessible charts with text alternatives and data tables. | Must |
| FR-L10N-13 | Provide screen-reader announcements for async sync, errors, and alerts. | Must |
| FR-L10N-14 | Let users change locale and base currency without losing data. | Must |
| FR-L10N-15 | Localize email, push, and in-app notifications. | Must |
| FR-L10N-16 | Localize support handoff and help-centre links. | Should |
| FR-L10N-17 | Fall back to English for missing strings without breaking layout. | Must |
| FR-L10N-18 | Support locale-aware sorting and search. | Should |
| FR-L10N-19 | Show prices in the product's billing currency with tax status. | Must |
| FR-L10N-20 | Test every locale at mobile width and with long translated strings. | Must |

## Business rules

| ID | Rule |
|----|------|
| BR-L10N-01 | Stored values never depend on display locale. |
| BR-L10N-02 | FX conversion is display/conversion only; the original value is retained. |
| BR-L10N-03 | The FX source and timestamp are shown wherever converted amounts appear. |
| BR-L10N-04 | Missing translations fall back to English, never to a blank or raw key. |
| BR-L10N-05 | Legal text is not machine-translated without legal review. |
| BR-L10N-06 | Accessibility is a release gate for core flows, not a post-launch fix. |
| BR-L10N-07 | Timezone changes do not rewrite historical snapshot dates. |
| BR-L10N-08 | Currency symbols and separators follow the locale, not a hard-coded format. |
| BR-L10N-09 | RTL support is required before launching any RTL locale. |
| BR-L10N-10 | Localized content must preserve disclaimers and risk language. |

## Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-L10N-01 | A user in Vietnam sees VND amounts with Vietnamese number formatting and the FX timestamp. |
| AC-L10N-02 | A user in Germany sees EUR with `1.234,56` formatting and DD.MM.YYYY dates. |
| AC-L10N-03 | Changing base currency updates displays without altering stored values. |
| AC-L10N-04 | EOD snapshots follow the user's timezone, including daylight-saving transitions. |
| AC-L10N-05 | Every core flow passes WCAG 2.2 AA checks at mobile width. |
| AC-L10N-06 | A chart has a text alternative and an accessible data table. |
| AC-L10N-07 | A missing translation falls back to English without layout breakage. |
| AC-L10N-08 | Notifications arrive in the user's locale. |
| AC-L10N-09 | Legal documents exist in the required launch-market languages. |
| AC-L10N-10 | Long translated strings do not overflow buttons or cards. |

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Which languages and markets launch first? | Product |
| OQ-002 | Which FX provider and licensing model? | Data/Legal |
| OQ-003 | Which legal documents require local-language versions per market? | Legal |
| OQ-004 | Is RTL in the first year? | Product |
| OQ-005 | Resolved by D-016: WCAG 2.2 AA for core flows. | Product/Design |
