# Cross-Unit Traceability — Build and Test

Run: 2026-11-12T05:54:34Z, sensor re-run on the loop-back replay 2026-11-12T07:48Z · Intent
`261109-brd-002-users-roles` · Stage Build and Test. The replay returns the same seventeen
uncovered ids with the same causes; the repair closed F2a and lifted one crate's coverage but did
not touch any evidence file.

This file carries the **id-level** view of upstream coverage across the ten Units: which
requirement ids this intent set out to serve are actually reached by generated code, which Unit
owns each, and which are uncovered. The per-target verdicts (build gates, suites, coverage
floors) are in `build-and-test-summary.md` §4; the raw evidence is in `test-results.md`.

The check was run with the stage's traceability sensor:

```
$ aidlc engine sensor-traceability --stage build-and-test
```

**Verdict: fail.** Seventeen ids are uncovered — eight of them owned by a Unit in this plan but
not implemented or not traced, one claimed by no Unit at all, and the remainder either deferred by
design (and correctly still `Deferred`) or malformed in a Unit's traceability file. Every other id
in the intent's requirement set resolves to at least one generated file and is marked
`covered: true`.

## 1. Coverage summary

| Group | Count | Covered | Uncovered |
|---|---|---|---|
| Functional requirements (`FR*`) | 24 | 21 | 3 (FR17.4, FR18.1–18.3, FR19.4 — five ids) |
| Non-functional requirements (`NFR*`) | 27 | 24 | 3 (NFR23, NFR26 plus a malformed-target defect) |
| Acceptance criteria (`AC*`) | 61 | 52 | 9 (AC4.4.2, AC5.1.1–AC5.3.2, AC6.4.1, AC6.4.2, AC8.1.1) |
| Business rules (`BR*`) | 31 | 31 | 0 |

The uncovered ids fall into four distinct causes, each handled separately in §2–§5.

## 2. Covered — no action

Every id from AC1.x, AC2.x, AC3.x, AC6.1–AC6.3 and AC6.5, AC7.x, BR1.x–BR5.x, FR16.x, and
NFR1–NFR22 (except those named in §3) resolves to generated source and is exercised by a test that
passed in this run. Their producing files are listed per Unit in
`construction/<unit>/code-generation/traceability.json`; the suites that run them are the ones in
`test-results.md` §2.

## 3. Claimed but not traced — no file carries the id

| Id | Owning Unit | Requirement | Where it should live | Status |
|---|---|---|---|---|
| AC6.4.1 | `u6-jurisdiction-gate` | a restricted sign-up creates no user record, profile row or shredding key | `services/identity/application/src/principal.rs` — the gate clears before any personal data is written (line 176–178) | behaviour exists, id untraced |
| AC6.4.2 | `u6-jurisdiction-gate` | the same refusal is recorded without personal data | `services/identity/application/src/principal.rs` plus its application suite | behaviour exists, id untraced |

`u6-jurisdiction-gate` lists AC6.4.1 and AC6.4.2 in `upstream_ids` — the ids it claims to serve —
with no `coverage` entry pointing at a file, so the sensor cannot see them. The code that
satisfies both is present and already tested; what is missing is the evidence row. This is an
evidence defect, not unbuilt work.

**Correcting an earlier reading of this run.** The first pass through this file attributed
AC5.1.1–AC5.3.2 and FR18.1–18.3 to `u2-authorisation` as "claimed but never implemented". That was
wrong: `u2-authorisation/code-generation/traceability.json` marks all six AC5 ids `Deferred` with a
reason each — the portfolio-link storage they describe belongs to the Account and Portfolio
services, which are not part of this intent — and U2 ships only the ownership decision those
services call. Those six ids, with FR18.1–18.3, are recorded in §5 instead.

## 4. Unclaimed — no Unit traces this id

| Id | Requirement | Where it appears | Status |
|---|---|---|---|
| AC8.1.1 | an acceptance criterion of BRD-002 §8 | `inception/user-stories/stories.md` only | never picked up by any Unit |

`grep -l "AC8.1.1" aidlc/spaces/default/intents/261109-brd-002-users-roles/construction/*/code-generation/traceability.json`
returns nothing. AC8.1.1 exists in the user stories and was never carried into Units Generation, so
no Unit owns it and no file implements it. This is a requirement that fell out of the plan.

## 5. Deferred by design — correctly still `Deferred`

| Id | Last owning stage | Why deferred | Verdict |
|---|---|---|---|
| AC4.4.2 | legal hold (a later compliance Unit not in this plan) | legal hold needs the BRD-017 register and the statutory retention schedule; neither is in this intent | Unverified — not a failure of this stage |
| FR17.4 | the same legal-hold Unit | as above | Unverified |
| FR19.4 | a later Unit not in this plan | the erasure/retention follow-up named in U5's summary | Unverified |
| NFR26.2 | a console-wide localisation Unit (does not exist) | the Vietnamese half of the support console; U9 and U10 both deferred it and no Unit adopted it | Unverified — the deferral has no home |
| AC5.1.1 | none in this plan | portfolio valuation inclusion across two linked source accounts; the link storage belongs to the Account and Portfolio services | Unverified — outside this intent |
| AC5.1.2 | none in this plan | BRD-006 membership decides inclusion, no double counting | Unverified — outside this intent |
| AC5.2.1 | none in this plan | a source account may never link to another user's portfolio; U2 ships the ownership decision, the link storage is Account's | Unverified — outside this intent |
| AC5.2.2 | none in this plan | the refused cross-owner attempt is audited | Unverified — outside this intent |
| AC5.3.1 | none in this plan | deleting a connection removes the connection and its links (Account storage) | Unverified — outside this intent |
| AC5.3.2 | none in this plan | other accounts in the portfolio remain unchanged | Unverified — outside this intent |
| FR18.1 | none in this plan | one source account may feed several portfolios under BRD-006 | Unverified — outside this intent |
| FR18.2 | none in this plan | a source account may never link to another user's portfolio | Unverified — outside this intent |
| FR18.3 | none in this plan | deleting a source connection removes only the connection and its links | Unverified — outside this intent |

The rule this stage applies: a deferral is only legitimately `Deferred` when a later stage in the
plan owns it. None of these four has a scheduled owner. They are recorded `Unverified`, never as
"successfully deferred".

## 6. Malformed targets — `u6-jurisdiction-gate`

The sensor reported three defects in `u6-jurisdiction-gate`'s traceability file itself:

```
u6-jurisdiction-gate: NFR23.1: target must be a workspace-relative file path
u6-jurisdiction-gate: NFR23.2: target must be a workspace-relative file path
u6-jurisdiction-gate: NFR23.3: target must be a workspace-relative file path
```

The replay's sensor output additionally lists `u6-jurisdiction-gate: NFR23.4` among NFR23's
`not_built` entries while naming only 23.1–23.3 in the reason, so NFR23.4 needs the same check:
either its target is malformed too, or it is genuinely unbuilt.

The three NFR23 entries point at something that is not a file path under the workspace (the
Unit's `traceability.json` carries prose or an absolute path where a relative path is required),
so the sensor cannot resolve them and counts NFR23 as uncovered even though U6's own
implementation of the jurisdiction gate exists. The defect is in the **evidence file**, not
necessarily in the code — but until the targets name real files, the claim is unproven.

`u6-jurisdiction-gate` also lists AC6.4.1 and AC6.4.2 in `upstream_ids` without a coverage entry
(§3), so U6 carries both an unbuilt acceptance criterion and a malformed evidence file.

## 7. What would turn this green

| Cause | Fix | Who can do it | Effort |
|---|---|---|---|
| §3 untraced (2 ids) | add the two coverage rows in `u6-jurisdiction-gate`'s traceability file, pointing at the code and tests that already satisfy them | a loop-back into U6's evidence file | minutes |
| §4 unclaimed (AC8.1.1) | decide whether AC8.1.1 belongs to an existing Unit or to a new one, then build it | a plan change, not a build fix | needs a decision first |
| §5 deferred (13 ids) | schedule the legal-hold, localisation and portfolio-link work, or record them as out of scope for this intent | a plan change | decision only |
| §6 malformed (NFR23.1–23.3) | rewrite the three targets as workspace-relative paths to the files that implement them | a loop-back into U6's evidence file | minutes, once the target files are chosen |

None of these is a relaxation of a target: the sensor's rule is unchanged, and the ids stay
uncovered until the work exists.