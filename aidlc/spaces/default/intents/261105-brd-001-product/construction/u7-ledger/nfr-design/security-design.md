# Security Design — u7-ledger

Inputs: `nfr-requirements/security-requirements.md`, `functional-design/rules.md`.

- **Auth**: the user JWT is validated against Supabase JWKS; owner-only reads and corrections.
  Internal callers (Portfolio, Reporting) use per-caller service tokens with an operation allow-list.
- **Append-only enforcement**: `ledger_svc` has SELECT/INSERT on `transactions` and `corrections`
  only; UPDATE and DELETE are not granted. A correction is a new row.
- **Mutable tables**: `ledger_svc` has SELECT/INSERT/UPDATE on `ledger_subjects`, `cost_basis_lots`,
  `pnl_snapshots`, and `transfer_pairs` (no DELETE except prune batches on lots).
- **Anonymisation versus append-only**: transactions reference a mutable `LedgerSubject` row
  (`subjectId`), not the user id directly. Anonymise (C7) clears `ledger_subject.user_id` and sets a
  random `anonymised_ref` on that row; the transaction rows stay untouched. A separate
  `ledger_anonymiser` role holds UPDATE on `ledger_subjects` only and is used by the one-shot
  anonymise job; the runtime role never has it. The anonymise action is audited (actor, subject
  ref, outcome) and is idempotent.
- **Audit**: corrections and recorded trades write C10 with actor, transaction, field, old/new
  value, and a reason hash; the free-text reason stays on the correction row. A failed audit write
  is retried and paged; the correction is not rolled back once committed.
- **Privacy**: C7 exposes export (transactions + corrections) and anonymise (user id replaced with a
  non-reversible reference while the financial rows remain) for Identity's orchestration.
- **Validation**: quantity > 0, price ≥ 0, kind enum, executedAt within a 300 s future skew (later
  values clamped), assetId resolved via C4; RFC 7807 field-level problems.
- **Transport**: TLS on the internal overlay and every external call; Postgres `sslmode=require`.
- **Least privilege**: schema `ledger`, role `ledger_svc`; no cross-schema grant, no RLS.

NFR references: NFR9.1–NFR9.6, NFR13.1, NFR12.1.
