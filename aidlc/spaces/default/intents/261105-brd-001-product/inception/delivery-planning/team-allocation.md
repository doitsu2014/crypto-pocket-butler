# Team Allocation — Crypto Pocket Butler

A **Bolt** is one build pass over one Unit of Work, ending in something that runs. A **mob** is
the group that builds it. Team Formation (stage 1.5) does not run in the classic scope, so every
Bolt is built by the AI developer agent (`aidlc-developer-agent`), with the human reviewing each
change before merge.

| Bolt | Unit | Owner | Human checkpoint |
|---|---|---|---|
| B1 | U1 `u1-walking-skeleton` | `aidlc-developer-agent` (AI) | Approve the skeleton end to end before later Bolts |
| B2 | U12 `u12-audit` | `aidlc-developer-agent` (AI) | Review the append-only guarantee |
| B3 | U3 `u3-identity` | `aidlc-developer-agent` (AI) | Review roles, MFA, and privacy paths |
| B4 | U13 `u13-support-console` | `aidlc-developer-agent` (AI) | Review least-privilege access |
| B5 | U5 `u5-market-data` | `aidlc-developer-agent` (AI) | Review the unpriced path |
| B6 | U4 `u4-account` | `aidlc-developer-agent` (AI) | Review credential handling |
| B7 | U10 `u10-analytics` | `aidlc-developer-agent` (AI) | Review the consent gate |
| B8 | U9 `u9-notification` | `aidlc-developer-agent` (AI) | Review dedupe and mandatory email |
| B9 | U7 `u7-ledger` | `aidlc-developer-agent` (AI) | Review cost basis and P&L coverage |
| B10 | U6 `u6-portfolio` | `aidlc-developer-agent` (AI) | Review valuation, drift, and order intents |
| B11 | U8 `u8-reporting` | `aidlc-developer-agent` (AI) | Review brief failure handling |
| B12 | U11 `u11-job-orchestration` | `aidlc-developer-agent` (AI) | Review missed-run detection |
| B13 | U2 `u2-web-app` | `aidlc-developer-agent` (AI) | Review accessibility and both languages |

## Parallelism

After B1 ships, the graph allows parallel Bolts wherever two Units share no dependency:

- B3, B5, and B6 can run together.
- B4 can run alongside B5, B6, and B7.
- B7 and B9 can run together once U3 and U4 exist.
- B8 can run alongside B9.
- B10 waits for its inputs (U4, U5, U7, U9); B11 waits for B10; B12 waits for B7–B11;
  B13 waits for the services it calls.

The recorded construction choice is `Construction Iteration: unit-major`, `Construction
Checkpoints: enabled`, and `Construction Execution: serial`, so the engine builds one Unit at a
time with a verified checkpoint after each, even where the plan marks Bolts as parallel-capable.
Running several at once would require an explicit stage-major plus swarm choice from the human.

## Human involvement

- Every change is reviewed by a human before merge (`memory/org.md`).
- The walking skeleton is demonstrated end to end and approved before feature breadth.
- Each Unit checkpoint is approved by the human against the recorded verification command.
- Reviews focus on correctness and security before style (`memory/team.md`).
