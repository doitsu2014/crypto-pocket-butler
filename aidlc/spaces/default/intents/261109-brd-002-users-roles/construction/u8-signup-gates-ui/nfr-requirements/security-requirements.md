# Security Requirements — u8-signup-gates-ui

**Upstream:** `functional-spec.md` (revision 2 — §2.1 routing, §2.4 submit order, §5 security
posture, §6 open items), `requirements.md` (FR15.9, FR19.2–FR19.5, NFR21–NFR26),
`contract-summary.md` (C1/C2, C3, C6, C8; timeout/retry/audit rules), codekb
`technology-stack.md`.
**Prefix note:** `NFR9` (transport/secrets), `NFR10` (token/session), and `NFR11` (abuse)
rows inherit the BRD-001 intent's prefixes and refine them for this UI unit; `NFR22`/`NFR24`
rows refine this intent's inception NFRs. Sub-numbers are unit-local.

## Transport and session security

| ID | Requirement | Source |
|---|---|---|
| NFR9.1 | Every call from the gate flow goes over TLS to the API Gateway only; the sole other browser-external origin is Supabase Auth. No third-party origin (CDN, dataset, analytics) is called at runtime — the country data module is bundled, not fetched | BRD-012 NFR-SEC-01; project mandate (SPA → gateway only) |
| NFR10.1 | Browser tokens remain in memory only: nothing in the gate flow writes a token, a gate state, or an acceptance to localStorage, sessionStorage, IndexedDB, or a cookie | BRD-012 NFR-SEC-10 |
| NFR10.2 | The SPA never validates or decodes the JWT for a decision: role, status, consent sufficiency, and jurisdiction are read only from G1/G2 answers; Supabase Auth remains the only credential source | FR15.9; spec §5 |
| NFR11.1 | G3 submit is subject to the gateway's stricter auth-trigger rate limits; the SPA never auto-retries a submit and reuses one idempotency key per **pending submission** — the key lives in an in-memory holder above the screen (tech-stack T4), so an in-app remount during a pending submission reuses it, and it is discarded only on success or sign-out | BRD-012 NFR-SEC-12; spec §2.4/§2.6 |
| NFR11.2 | A full page reload loses the in-memory key, so the end-to-end idempotence guarantee must not rest on it alone: G3 (and the C6 grant behind it) shall treat already-recorded consent kinds as a no-op rather than appending duplicates. This is recorded as a **contract dependency on G3/C6 idempotency** to be confirmed before Code Generation; if C6 cannot guarantee it, the key holder must survive reloads by a non-persisted mechanism the server accepts | spec §2.6 recovery path; C6 `POST /internal/users/{id}/consent` states no idempotency semantics |

## Data protection and sensitive identifiers

| ID | Requirement | Source |
|---|---|---|
| NFR22.1 | The gate flow never renders or logs the provider subject, a subject hash, an internal user id, or an email: error surfaces carry the correlation id only, and ProblemDetails `detail` text is never rendered | NFR22; spec §5 |
| NFR22.2 | Pre-gate data minimisation: the only personal datum the screen collects is the self-declared country code (plus three booleans) and it is sent only to G3; client telemetry/analytics receives nothing from the gate flow — no country, no acceptance, no error payload beyond a counted event | FR19.4; `components.md` (never requests personal data before the gates pass) |
| NFR22.3 | The principal and gate-state reads this unit depends on shall carry **no raw provider subject or other sensitive identifier in the URL path or query** — the edge logs URLs by default (`web/nginx.conf`), so a subject in the path is a subject in the logs, which NFR22 forbids. The route shape shall be self-scoped (`/v1/me/...`, as the repository already uses for the caller's own resources — e.g. the existing `GET /v1/me/consent/state`), identifying the caller from the validated token only. The functional spec's G1 (`GET /v1/principals/{authUserId}`) does not satisfy this: the required gateway/U1 contract change to a read-only self-scoped principal route is recorded in `tech-stack-decisions.md` §3 and must land before Code Generation; if the exposure is instead accepted, that acceptance must be recorded explicitly as a deviation from NFR22 with its reason | NFR22; spec §6 G1; `web/nginx.conf` |
| NFR24.1 | Gate state is server-fresh: G1/G2 are re-fetched on every authenticated entry (first load and hard navigation), after every retryable submit failure, after every G3 422, and invalidated on submit success (`staleTime: 0`, no persisted cache). A cached "passed" state never unlocks a route | NFR24; spec §2.1/§2.3/§2.6 |
| NFR24.2 | The route guard and every client-side check are presentational only (FR15.9): a visitor who bypasses them is still refused server-side by U4's consent gate and the gateway route table; no security decision in this unit derives from client state | FR15.9; spec §2.1 |

## Threat considerations (unit-specific)

| Threat | Mitigation in this design |
|---|---|
| IP spoofing to defeat the jurisdiction gate | The client IP for C8 is derived by the gateway from the trusted proxy chain; the SPA sends no IP field or header, and G3's body is exactly `{countryCode, terms, risk, age}` (spec §1, open item 4) |
| Country shopping against a restriction | The Blocked-Jurisdiction Notice is terminal for the session: Sign out only, no in-screen re-selection (spec §2.5) |
| Existence disclosure through the gate flow | The blocked notice is identical whichever signal restricted (`restrictedBy` never surfaced); error surfaces show correlation ids only, indistinguishable across denial reasons |
| Partial-application confusion after a mid-sequence failure | Retry re-fetches G2 and renders only what is still missing; "nothing was saved" wording is used only where nothing can have committed (spec §2.6) |
| Pre-gate data collection | No field beyond country + three acceptances exists anywhere on the screen (spec §3) |

## Compliance — accessibility (NFR25)

| ID | Requirement |
|---|---|
| NFR25.1 | The gate screen, blocked notice, and every error state meet WCAG 2.2 AA: full keyboard operation (combobox per WAI-ARIA APG), screen-reader announcement of the validation summary (`role="alert"` + focus move) and of the blocked notice (page-level status), contrast via Parchment Ledger tokens, and no motion-dependent interaction |
| NFR25.2 | Colour never carries meaning alone: every error/blocked state pairs colour with an icon and text; the validation summary names each missing item in words |
| NFR25.3 | Tab order equals DOM order — summary links (when present) → country → terms → risk → age → continue; the summary sits above the fields in DOM order (spec §4) |
| NFR25.4 | Every rendered state passes jest-axe in the unit's test suite (accessibility is a build gate, not a review checklist item) |

## Compliance — localisation (NFR26)

| ID | Requirement |
|---|---|
| NFR26.1 | Every user-facing string — field labels, validation summary, blocked notice, retryable-error wording, and the full country list — is available in English and Vietnamese through the existing `web/src/i18n` module and the generated country data module |
| NFR26.2 | Locale resolution on the gate screen uses the SPA's existing `resolveLocale` order, unchanged: `?lang=` URL parameter → the signed-in user's stored server preference → browser locale → English. A first-time visitor has no stored preference, so the browser locale decides; a returning signed-in visitor sees their stored preference on this screen too, and `?lang=` applies here exactly as on every other screen. No locale data is fetched at runtime and nothing locale-related leaves the browser |
| NFR26.3 | Any date/time the flow displays (none are required on the gate screen; the blocked notice carries none) follows the user's locale format when present |

## Out of scope for this unit

Audit writes (NFR21 retention, C9), rate-limit configuration (gateway), JWKS validation
(Identity), and the restricted-jurisdiction list itself (U6, compliance-owned) are server-side;
this unit's obligation is never to undermine them (NFR22.1, NFR11.1, NFR24.2).
