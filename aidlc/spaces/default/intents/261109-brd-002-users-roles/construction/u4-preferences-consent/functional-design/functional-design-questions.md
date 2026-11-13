# Functional Design Questions — u4-preferences-consent

Unit: `u4-preferences-consent` (U4) · Stage: construction/functional-design
Answers go after `[Answer]:`. Answering in chat is fine too — I write them back here.

## Q1. [Architect] How consent records are stored

The ConsentRecord entity carries `grantedAt` and `withdrawnAt`, which can be read two ways, and the choice decides whether history is recoverable.

- A. Append-only events — one row per grant or withdrawal, never updated; current state is the newest row per kind
- B. Current row plus history — a mutable row per kind holding the state, and a history table written on each change

[Answer]: A. Append-only events — one row per grant or withdrawal, never updated; current state is the newest row per kind

## Q2. [Architect] Where preferences live

The component catalogue gives UserPreferences its own entity, keyed by userId.

- A. A `user_preferences` row per user, beside the user record
- B. Locale, base currency, timezone and notification columns on the existing `users` row

[Answer]: A. A `user_preferences` row per user, beside the user record

## Q3. [Product] Launch locale and base-currency set

NFR26 requires English and Vietnamese. The currency set was never pinned.

- A. USD, EUR, GBP, VND
- B. USD, EUR only

[Answer]: A. USD, EUR, GBP, VND

## Q4. [Security] What a withdrawal does

FR19.2 requires terms, risk and age acceptance before the first source connection.

- A. A withdrawn terms or risk consent immediately blocks connecting a new source and names the missing acceptance
- B. The withdrawal is recorded and surfaced, but blocks nothing in this release

[Answer]: A. A withdrawn terms or risk consent immediately blocks connecting a new source and names the missing acceptance
