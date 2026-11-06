# User Stories — Story Plan Questions

These questions settle how the stories are shaped before they are written. Answer each with a
letter after `[Answer]:` (for example `[Answer]: A`), or pick `X` and say what you want instead.

## Q1. [Product manager] Which personas should the stories cover?

BRD-001 names mainstream retail as primary, crypto-native active investors as secondary, and
family office / advisor as post-MVP.

- A. The two MVP personas only — mainstream retail and crypto-native active investor
- B. All three, with family office marked post-MVP and no stories written for it yet
- C. Add an admin/support operator persona alongside the two MVP personas
- X. Other (please specify)

[Answer]: A — Two MVP personas

## Q2. [Product manager] How small should each story be?

- A. One story per user-visible capability, split by workflow step — sized for one iteration and test-first delivery
- B. One story per requirement in requirements.md, even where a requirement spans several steps
- C. Epic-level stories with sub-stories under each
- X. Other (please specify)

[Answer]: A — One per capability

## Q3. [Product manager] How should the stories be grouped?

- A. By user journey — onboarding, connect accounts, see the portfolio, act on drift, report, settings
- B. By feature area — the FR groups in requirements.md
- C. By persona
- X. Other (please specify)

[Answer]: A — By user journey

## Q4. [Product manager] How should priority be assigned?

The committed Must set is the baseline for this release; the Should fast-follows are excluded.

- A. Mark committed-scope stories Must Have; mark fast-follow items Should Have and exclude them from this release
- B. Mark every story Must Have
- C. Use the full MoSCoW scale, including Could Have and Won't Have
- X. Other (please specify)

[Answer]: A — Must for committed scope

## Q5. [Quality engineer] How many acceptance criteria per story?

The team's practices require the happy path plus at least two error or edge cases per test file.

- A. 3–6 Given/When/Then criteria per story, covering the happy path and at least one error or edge case
- B. A minimum of 2 criteria per story
- C. Comprehensive — 5 or more criteria per story including boundary and failure cases
- X. Other (please specify)

[Answer]: A — 3–6 per story
