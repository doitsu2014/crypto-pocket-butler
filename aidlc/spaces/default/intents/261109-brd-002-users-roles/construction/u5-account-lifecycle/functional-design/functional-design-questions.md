# Functional Design Questions — u5-account-lifecycle

Unit: `u5-account-lifecycle` (U5) · Stage: construction/functional-design
Answers go after `[Answer]:`. Answering in chat is fine too — I write them back here.

## Q1. [Product] What starts and measures the 30-day deletion window

- A. Exactly 720 hours from the moment the request commits
- B. Midnight UTC on the 30th calendar day after the request

[Answer]: B. Midnight UTC on the 30th calendar day after the request

## Q2. [Security] Legal holds before BRD-017's register exists

- A. Unconditional erasure; the register adds its check when it lands
- B. A `legal_hold` flag an admin can set; the erasure job skips a held account and records why

[Answer]: A. Unconditional erasure; the register adds its check when it lands

## Q3. [Architect] How erasure runs

- A. A scheduled job in `identity-worker` picks up accounts whose window has ended and erases them, idempotently
- B. Synchronous, inline with the mutation that ends the window

[Answer]: A. A scheduled job in `identity-worker` picks up accounts whose window has ended and erases them, idempotently

## Q4. [Product] What a pending-deletion account may do during the window

- A. Read-only: sign in, restore, and read its data; no new sources and no writes
- B. Full use until the window closes

[Answer]: A. Read-only: sign in, restore, and read its data; no new sources and no writes
