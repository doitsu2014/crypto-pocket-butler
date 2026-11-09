# Functional Design Questions — u4-account

These questions settle the choices the requirements, stories, and contracts leave open for the
Account service. Answer each with a letter (A–D or X).

## Q1. [Architect] How is an exchange API key verified?

The component requires key permissions to be verified before use and periodically, and rejects
withdrawal or trading permission.

- A. Call a read-only balances endpoint on add, and a permissions endpoint periodically.
- B. Call a permissions endpoint only.
- C. Call a balances endpoint only.
- D. Manual verification by an admin.
- X. Other (please specify)

[Answer]: A — Call a read-only balances endpoint on add, and a permissions endpoint periodically.

## Q2. [Architect] How are exchange credentials stored and replaced?

- A. One Vault secret per credential; re-entering a key replaces it, resets verification, and destroys the old secret.
- B. One Vault secret per account; re-entering updates it.
- C. Credentials encrypted in the service database by pgsodium.
- D. Manual admin provisioning.
- X. Other (please specify)

[Answer]: A — One Vault secret per credential; re-entering a key replaces it, resets verification, and destroys the old secret.

## Q3. [Architect] How does Account know the supported EVM chains?

The registry settles the exact chain list (OQ-1), and the requirements leave it to the registry work.

- A. From the Asset Registry's `evm_chains` list (dynamic; no chain list hardcoded in Account).
- B. A hardcoded list in Account.
- C. A config file per deployment.
- D. Only Ethereum mainnet at launch.
- X. Other (please specify)

[Answer]: A — From the Asset Registry's evm_chains list (dynamic; no chain list hardcoded in Account).

## Q4. [Security engineer] Where does a WalletConnect session live?

US2.2 requires a read-only connection where the wallet's balances appear and no signing or transfer
action is offered.

- A. The SPA holds the WalletConnect session; Account receives the verified address and stores only the connection reference.
- B. Account holds the session server-side.
- C. A short-lived signed address attestation from the relay.
- D. WalletConnect is out of scope for this unit.
- X. Other (please specify)

[Answer]: A — The SPA holds the WalletConnect session; Account receives the verified address and stores only the connection reference.

## Q5. [Architect] Which CSV formats are imported?

US1.4 requires CSV import with duplicate skipping and per-line error reporting.

- A. Binance and OKX exports plus a generic CSV with a documented column set.
- B. Binance and OKX exports only.
- C. A generic CSV only.
- D. Any CSV with a user mapping step.
- X. Other (please specify)

[Answer]: A — Binance and OKX exports plus a generic CSV with a documented column set.

## Q6. [Architect] What is the duplicate key for imported transactions?

- A. `(accountId, externalId)` when the row carries one, otherwise `(accountId, kind, asset, quantity, executedAt)`.
- B. `(accountId, executedAt, asset, quantity)`.
- C. A content hash over all fields.
- D. No dedupe in Account; the Ledger handles it.
- X. Other (please specify)

[Answer]: A — (accountId, externalId) when present, otherwise (accountId, kind, asset, quantity, executedAt).

## Q7. [Architect] Are holdings stored or computed?

- A. Computed on read from the latest venue/RPC observation, with the last-synced time.
- B. Stored as a holdings snapshot per sync.
- C. Both — a snapshot per sync plus a computed current view.
- D. Delegated to the Portfolio unit.
- X. Other (please specify)

[Answer]: C — Both a snapshot per sync and a computed current view.

## Q8. [Quality engineer] What is the spam and dust rule?

- A. Registry-driven: tokens absent from the canonical registry are labelled spam; dust is valued under $1; both are kept but excluded from default totals.
- B. Spam by allowlist only; no dust threshold.
- C. Hide both from every view.
- D. No filtering; show everything.
- X. Other (please specify)

[Answer]: A — Registry-driven spam labelling plus a $1 dust threshold; both kept but excluded from default totals.
