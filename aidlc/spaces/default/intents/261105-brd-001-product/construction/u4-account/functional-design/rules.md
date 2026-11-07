# Business Rules — u4-account

The YAML block below is the source of truth for the business rules. The table after it is a short
human-readable summary. Rules are grouped: BR1 account lifecycle, BR2 credential verification,
BR3 wallets and WalletConnect, BR4 sync, BR5 CSV import, BR6 holdings, BR7 notifications and audit.

```yaml
rules:
  - id: BR1.1
    statement: Every account belongs to one user; a read needs the owner or an approved support grant.
    category: authorization
    applies_to: Account
    trigger: An account, sync, import, or holdings read is requested.
    logic: IF the caller is the owning user THEN allow; IF the caller is a support user with an active, reason-recorded grant verified through Identity's C3 grant check THEN allow the metadata read only; ELSE refuse.
    violation: The response is not-found or forbidden and no data is returned. No blanket admin exception exists.
    source: FR1.6, FR11.2

  - id: BR1.2
    statement: Account labels are unique per user (case-insensitive).
    category: validation
    applies_to: Account
    trigger: An account is created or relabelled.
    logic: IF another account of the user has the same label ignoring case THEN refuse with a field problem.
    violation: The account is not created or relabelled.
    source: FR2.1

  - id: BR1.3
    statement: No private key or seed phrase is accepted anywhere.
    category: constraint
    applies_to: Account, WalletConnection
    trigger: Any onboarding or edit flow.
    logic: IF an input looks like a private key or seed phrase THEN refuse it and never store it.
    violation: Nothing is stored and the request is refused.
    source: FR2.6

  - id: BR1.4
    statement: An exchange account row exists only after its key verifies.
    category: policy
    applies_to: Account, Credential
    trigger: An exchange key is submitted.
    logic: IF the key verifies read-only THEN create the Account and Credential; ELSE store neither, destroy the candidate Vault secret, and return the reason.
    violation: A rejected key leaves no account, no credential, and no secret; the SPA keeps the label and pre-fills it on retry.
    source: US1.3, AC1.3.2

  - id: BR2.1
    statement: An exchange key is verified read-only before use and periodically.
    category: authorization
    applies_to: Credential
    trigger: A key is added, or the periodic verification runs.
    logic: IF a read-only balances call succeeds AND the permissions check shows no withdrawal or trading right THEN mark the credential verified and readOnly; ELSE reject it.
    violation: A rejected key is not used for a sync; the reason is shown without echoing the key.
    source: FR2.1, NFR9

  - id: BR2.2
    statement: A replacement key is verified before the old credential is swapped out.
    category: policy
    applies_to: Credential
    trigger: The user submits a new key for an existing account.
    logic: IF the new key verifies THEN swap it in and destroy the old Vault secret; ELSE keep the old credential active, record the failure, and keep the label for the retry.
    violation: A failed replacement leaves the old credential usable; the old secret is never destroyed before the new key verifies.
    source: FR2.1

  - id: BR2.3
    statement: A key that fails verification is retryable without re-entering the label.
    category: policy
    applies_to: Credential
    trigger: Verification fails.
    logic: IF the user retries THEN the SPA keeps the label and asks only for the key.
    violation: The account is not created until a key verifies.
    source: US1.3

  - id: BR2.4
    statement: Account maintains the has-exchange-credentials claim on the Auth user.
    category: policy
    applies_to: Credential
    trigger: A credential is stored, rejected, replaced, or removed.
    logic: IF the principal's validated JWT aal is not aal2 THEN refuse to store the credential; IF a credential is stored THEN set app_metadata.has_exchange_credentials true through the Supabase Auth admin API (C15); IF a credential is rejected or removed and no other verified credential remains THEN clear the claim.
    violation: A stale claim leaves MFA gating wrong and is recorded as an error; the next credential event corrects it.
    source: FR1.2, NFR10

  - id: BR2.5
    statement: A key that now carries trading or withdrawal permission is revoked.
    category: authorization
    applies_to: Credential
    trigger: A periodic verification finds trading or withdrawal permission.
    logic: IF the permissions check finds either permission THEN mark the credential rejected, destroy its secret, disconnect the account from sync, clear the claim, and notify the user.
    violation: The account stops syncing; no data is taken with the wider key.
    source: FR2.1, US7.1

  - id: BR3.1
    statement: Wallet chains come from the Asset Registry's supported-chains read, plus Solana.
    category: policy
    applies_to: Account
    trigger: A wallet address is added.
    logic: IF the declared chain resolves through the C4 supported-chains read or is solana THEN accept it; ELSE reject with the supported list.
    violation: The address is not added. A newly registered chain becomes selectable on the next read, with no Account redeploy.
    source: FR2.2

  - id: BR3.2
    statement: A malformed or unsupported address is refused and kept in the form.
    category: validation
    applies_to: WalletConnection
    trigger: An address is submitted.
    logic: IF the address does not match the chain's format THEN refuse with the supported list and keep the input in the form.
    violation: No account is created.
    source: US2.1

  - id: BR3.3
    statement: A WalletConnect connection is read-only and holds no signing capability.
    category: constraint
    applies_to: WalletConnection
    trigger: A WalletConnect session is approved or used.
    logic: IF the session is approved THEN store the verified address and an opaque session reference; never store a signing key or offer signing or transfer.
    violation: A rejected or expired session adds no account.
    source: FR2.2, US2.2

  - id: BR4.1
    statement: One sync runs at a time per account while its run lease is live.
    category: constraint
    applies_to: SyncRun
    trigger: A scheduled or manual sync starts.
    logic: IF a run is running with a live lease THEN the scheduled path returns 409 and the manual path returns 202 with the active run id; ELSE claim the account and start a run.
    violation: A second concurrent run is not created.
    source: FR2.5, FR10.1

  - id: BR4.2
    statement: A run whose lease expires is marked failed and releases the guard.
    category: policy
    applies_to: SyncRun
    trigger: A sweep or the next sync finds a running run with an expired lease.
    logic: IF a running run's leaseUntil is in the past THEN mark it failed with errorCategory data and release the single-flight guard; ELSE leave it running.
    violation: A killed process never blocks later syncs.
    source: FR2.5, NFR1

  - id: BR4.3
    statement: Sync status and last-synced time are always shown.
    category: policy
    applies_to: Account, SyncRun
    trigger: An account or the portfolio is viewed.
    logic: IF a run is running THEN show in-progress; when it finishes show the new last-synced time or the failure reason.
    violation: The status is never blank; a failure keeps the previous balances and labels them.
    source: FR2.5, US2.3

  - id: BR4.4
    statement: A failed sync keeps the previous holdings and labels them.
    category: policy
    applies_to: SyncRun
    trigger: A sync fails.
    logic: IF the provider or network fails THEN record the non-secret reason, keep the last successful holdings, and label them with the last-synced time.
    violation: No fabricated or zeroed holdings are shown.
    source: US2.3

  - id: BR4.5
    statement: An account leaves disconnected when a new credential verifies.
    category: policy
    applies_to: Account
    trigger: A replacement key verifies for a disconnected account.
    logic: IF the new credential becomes verified THEN move the account from disconnected to active and queue the first sync.
    violation: A disconnected account stays out of the portfolio until its credential verifies.
    source: FR2.1, FR2.5

  - id: BR5.1
    statement: Account owns CSV import for Binance, OKX, and a generic format.
    category: policy
    applies_to: ImportBatch
    trigger: A CSV is uploaded.
    logic: IF the file matches one of the three formats THEN import it into ImportedTransaction rows; ELSE refuse with the supported formats.
    violation: Nothing is imported from an unknown format.
    source: FR2.3, US1.4

  - id: BR5.2
    statement: Duplicate rows are skipped and counted.
    category: policy
    applies_to: ImportedTransaction
    trigger: A row is imported.
    logic: IF an ImportedTransaction with the same dedupeKey exists for the account THEN skip and count the row as skipped.
    violation: A duplicate is never stored twice.
    source: US1.4

  - id: BR5.3
    statement: Every bad row is reported with its line number and later rows still import.
    category: policy
    applies_to: ImportRowError
    trigger: A row fails validation.
    logic: IF a row is malformed THEN record its line number and reason, increment failedCount, and continue with the next row.
    violation: One bad row never stops the batch.
    source: US1.4

  - id: BR5.4
    statement: The import summary states imported, skipped, and failed counts.
    category: policy
    applies_to: ImportBatch
    trigger: An import finishes.
    logic: IF the batch finishes THEN return the three counts and the row errors.
    violation: The counts always reconcile to the data rows.
    source: US1.4

  - id: BR6.1
    statement: Holdings are a per-sync snapshot plus a computed current view.
    category: policy
    applies_to: HoldingSnapshot
    trigger: Holdings are read.
    logic: IF a sync has completed THEN serve the snapshot; the current view recomputes quantity from the latest observation and carries the last-synced time.
    violation: A missing snapshot leaves the read empty, never fabricated.
    source: FR4.1, US1.2

  - id: BR6.2
    statement: Quantities are decimals with the observed token identity and a canonical asset id where one resolves.
    category: calculation
    applies_to: HoldingSnapshot
    trigger: A holding is recorded.
    logic: IF the observed token resolves through the Asset Registry THEN record its canonical asset id and decimal quantity; ELSE record the observed token, leave assetId empty, and label it spam.
    violation: No binary floating point is stored or served.
    source: FR3.1, NFR7

  - id: BR6.3
    statement: Spam holdings are labelled and kept; pricing and dust belong to Portfolio.
    category: policy
    applies_to: HoldingSnapshot
    trigger: A holding is recorded or read.
    logic: IF the asset is absent from the canonical registry THEN spam is true and the holding stays in the snapshot with its observed token; Account records no price and no dust flag.
    violation: No holding is silently deleted; Portfolio computes priced and dust when it values the holdings.
    source: FR3.5

  - id: BR7.1
    statement: A sync failure and a key revocation notify the user.
    category: policy
    applies_to: Account
    trigger: A sync fails or a periodic verification revokes a key.
    logic: IF the event occurs THEN send C9 type sync_failed or key_revoked with an idempotency key of (accountId, event, runId or credentialId) and a bounded retry.
    violation: A failed notification is logged and retried within the bound; it never blocks the sync record.
    source: FR7.2, US2.3

  - id: BR7.2
    statement: Credential, account, sync, and import events are audited with C10's required fields.
    category: policy
    applies_to: Account
    trigger: Any of these events occurs.
    logic: IF the event occurs THEN write a C10 event with actorId (or system), action, target, occurredAt, context (the account id and label only; never a wallet address, key, or token), and outcome for: credential added, replaced, verified, rejected, revoked; account added, disconnected; sync failed; import completed.
    violation: A failed audit write is logged; a security-sensitive event is retried and surfaced, never silently dropped.
    source: NFR19

  - id: BR8.1
    statement: A privacy export returns the account data for the owning user.
    category: policy
    applies_to: Account
    trigger: Identity calls the C5 privacy export for a user.
    logic: IF the caller is the Identity service token THEN return the account rows, labels, wallet addresses, holdings, and imported transactions for that user; ELSE refuse.
    violation: The caller receives a problem document and no data.
    source: FR8.1, NFR13

  - id: BR8.2
    statement: Privacy anonymise removes personal account fields and keeps labelled retention rows.
    category: policy
    applies_to: Account
    trigger: Identity calls the C5 privacy anonymise for a user.
    logic: IF the caller is the Identity service token THEN remove the labels and wallet addresses, anonymise the personal fields, keep the labelled financial-retention rows, and return the retained categories; ELSE refuse.
    violation: A failed step leaves the deletion failed and retryable; nothing is silently dropped.
    source: FR8.2, NFR13
```

## Rules summary

| Rule | Category | Statement |
|---|---|---|
| BR1.1 | authorization | Owner or an approved support grant only |
| BR1.2 | validation | Labels unique per user |
| BR1.3 | constraint | No private key or seed phrase accepted |
| BR1.4 | policy | Account exists only after its key verifies |
| BR2.1 | authorization | Key verified read-only before use and periodically |
| BR2.2 | policy | Replacement verifies first, then swaps |
| BR2.3 | policy | Failed verification retryable without re-entering the label |
| BR2.4 | policy | Maintain/clear the has-exchange-credentials claim |
| BR2.5 | authorization | Revoke a key that gains trading or withdrawal permission |
| BR3.1 | policy | Chains from the C4 supported-chains read, plus Solana |
| BR3.2 | validation | Malformed or unsupported address refused, input kept |
| BR3.3 | constraint | WalletConnect read-only, no signing |
| BR4.1 | constraint | One live sync per account; 409 scheduled, 202 manual |
| BR4.2 | policy | Expired lease marks the run failed and releases the guard |
| BR4.3 | policy | Sync status and last-synced time always shown |
| BR4.4 | policy | Failed sync keeps and labels previous holdings |
| BR4.5 | policy | Disconnected leaves for active on a verified new credential |
| BR5.1 | policy | Account owns CSV import (Binance, OKX, generic) |
| BR5.2 | policy | Duplicates skipped by dedupeKey |
| BR5.3 | policy | Every bad row reported with its line number |
| BR5.4 | policy | Summary states imported, skipped, failed |
| BR6.1 | policy | Snapshot per sync plus computed current view |
| BR6.2 | calculation | Decimal quantities, observed token, canonical id where resolved |
| BR6.3 | policy | Spam labelled; pricing and dust are Portfolio's |
| BR7.1 | policy | Sync failure and key revocation notify (C9 types) |
| BR7.2 | policy | Credential, account, sync, import events audited (no full address) |
| BR8.1 | policy | Privacy export for the owning user (C5) |
| BR8.2 | policy | Privacy anonymise keeps labelled retention rows (C5) |
