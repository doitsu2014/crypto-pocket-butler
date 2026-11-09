# Cross-Unit Traceability — Build and Test

Source: `aidlc engine sensor-traceability --stage build-and-test` (run 2026-11-09).

## Verdict: FAIL

27 of the traced upstream IDs are uncovered, and most per-unit `NFRx.y` rows in the
`traceability.json` files carry a multi-file or prose `target` that the sensor cannot resolve to
one existing file.

## Uncovered upstream IDs

- **Functional requirements:** FR1.5, FR3.1, FR11.3, FR12.3, FR13.1, FR13.2.
  These are architectural or operator-facing requirements that no unit's traceability claims. FR13
  is the gateway API surface/rate-limit requirement, designed in Domain Design and delivered in
  the gateway; FR3.1 (asset registry), FR11.3 (registry audit), FR12.3 (feature flags) and FR1.5
  (admin role authorisation) likewise have no owning row.
- **Top-level NFRs:** NFR1, NFR2, NFR3, NFR5, NFR6, NFR7, NFR9, NFR10, NFR11, NFR12, NFR13,
  NFR14, NFR15, NFR16, NFR17, NFR18, NFR19, NFR20. The units trace the detailed `NFRx.y` rows but
  not the top-level ID itself.
- **Acceptance criteria:** AC9.2.1, AC9.2.2, AC9.2.3. Their targets are semicolon-separated file
  lists, which the sensor reads as one non-existent path.

## Invalid target shapes

The sensor reports 80 per-unit rows whose `target` is not one existing workspace-relative file.
They fall into three classes:

1. **Semicolon-separated file lists** — for example
   `services/asset-registry/domain/src/contract.rs; services/asset-registry/domain/src/asset.rs; migrations/0001_init.up.sql`
   (u1 AC9.2.1).
2. **Prose plus a path** — for example `services/audit/application/src/append.rs with
   services/audit/domain/src/event.rs: actor, target, occurredAt, outcome, sensitivity, and the
   derived class are validated and stored` (u12 AC9.2.2, and most u12 `NFR1.x` rows).
3. **Multiple artefacts for one target** — for example
   `services/audit/http/src/server.rs /healthz and /readyz; infra/swarm-stack.yml health check and
   restart policy` (u12 NFR1.1).

Every one of these needs a single existing implementation or test file as its target, with any
explanation moved out of the `target` field. The pattern repeats across u1, u12, u3, u4, u5, u6,
u7, u8, u9, u10, u11 and u13.

## Per-unit status

| Unit | Status |
|---|---|
| u1-walking-skeleton | Invalid targets on AC9.2.1–AC9.2.3 |
| u2-web-app | All 121 assigned IDs covered (2 deliberate GAP rows: NFR2.6, NFR6.1) |
| u3-identity … u13-support-console | Detailed NFR rows carry multi-file/prose targets; top-level IDs uncovered |

## What this means

The per-unit coverage tables are honest about what each unit built, but their `target` values are
written for a human reader rather than the sensor's one-file contract, and no unit claims the
top-level FR/NFR umbrella IDs. The IDs above are build-and-test findings for the person at the
approval gate.
