# Entity Model — u4-account

The YAML block below is the source of truth for the entity model. The prose after it is a short
human-readable summary. Technology-agnostic: no SQL, no framework, no language types.

Inputs: the Account unit in `inception/units-generation/unit-of-work.md`, the requirements in
`inception/requirements-analysis/requirements.md`, and the Account component in
`inception/domain-design/components.md`, which supplies the entity set.

```yaml
entities:
  - name: Account
    description: One read-only source of holdings for a user.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: userId
        type: uuid
        required: true
        references:
          entity: User
          relationship: many Accounts belong to one user (Identity owns User)
      - name: kind
        type: enum
        required: true
        allowed_values: [exchange, wallet, walletconnect, csv]
      - name: venue
        type: string
        required: true
        description: binance, okx, evm:<chainId> (from the registry's supported-chains read), solana, or csv:<label>.
      - name: label
        type: string
        required: true
        min_length: 1
      - name: status
        type: enum
        required: true
        allowed_values: [active, syncing, error, disconnected]
        default: active
      - name: lastSyncedAt
        type: timestamp
        required: false
      - name: createdAt
        type: timestamp
        required: true
      - name: updatedAt
        type: timestamp
        required: true
    constraints:
      - venue for a wallet is a chain returned by the Asset Registry's supported-chains read, or solana.
      - label is unique per user (case-insensitive).
      - A csv account is created by an import and has no credential.
      - The account row is created only after an exchange key verifies; a rejected key leaves no account.

  - name: Credential
    description: One read-only exchange credential, stored as a Vault secret reference.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: accountId
        type: uuid
        required: true
        references:
          entity: Account
          relationship: one Account has at most one active Credential
      - name: vaultSecretRef
        type: string
        required: true
        description: Vault secret reference; key material never enters the service.
      - name: readOnly
        type: boolean
        required: true
      - name: verificationStatus
        type: enum
        required: true
        allowed_values: [pending, verified, rejected, stale]
        default: pending
      - name: permissionsVerifiedAt
        type: timestamp
        required: false
      - name: createdAt
        type: timestamp
        required: true
    constraints:
      - readOnly is true only after a permissions check that found no withdrawal or trading permission.
      - A rejected credential is never used for a sync; its Vault secret is destroyed.
      - On replacement the new key is verified first; only then is the old secret destroyed.

  - name: WalletConnection
    description: A wallet address or WalletConnect read-only connection attached to an account.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: accountId
        type: uuid
        required: true
        references:
          entity: Account
          relationship: one Account has one connection
      - name: address
        type: string
        required: true
      - name: observedChain
        type: string
        required: true
        description: The chain id the address was validated against.
      - name: sessionRef
        type: string
        required: false
        description: Opaque WalletConnect connection reference when the SPA holds the session.
      - name: connectedAt
        type: timestamp
        required: true
    constraints:
      - No private key or seed phrase is ever accepted or stored.
      - A WalletConnect connection is read-only; no signing or transfer action exists.

  - name: SyncRun
    description: One sync attempt for one account.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: accountId
        type: uuid
        required: true
        references:
          entity: Account
          relationship: many runs belong to one Account
      - name: startedAt
        type: timestamp
        required: true
      - name: leaseUntil
        type: timestamp
        required: true
        description: A run whose lease expires without finishing is marked failed and releases the single-flight guard.
      - name: finishedAt
        type: timestamp
        required: false
      - name: status
        type: enum
        required: true
        allowed_values: [running, succeeded, failed]
        default: running
      - name: errorCategory
        type: enum
        required: false
        allowed_values: [auth, rate_limit, provider, network, data]
      - name: errorMessage
        type: text
        required: false
        description: A non-secret summary; never a key or token.
      - name: retries
        type: integer
        required: true
        min: 0
        default: 0
    constraints:
      - At most one run in running per account while its lease is live.
      - A failed or lease-expired run keeps the previous holdings and labels them.

  - name: HoldingSnapshot
    description: The observed holdings for one account at one sync.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: syncRunId
        type: uuid
        required: true
        references:
          entity: SyncRun
          relationship: many holdings belong to one run
      - name: accountId
        type: uuid
        required: true
      - name: observedToken
        type: string
        required: true
        description: The observed token identity as seen on the venue or chain (chain plus contract address, or venue symbol).
      - name: assetId
        type: string
        required: false
        description: The canonical asset id from the Asset Registry; absent for a spam holding.
      - name: quantity
        type: decimal
        required: true
        description: Decimal string; no binary floating point.
      - name: observedAt
        type: timestamp
        required: true
      - name: spam
        type: boolean
        required: true
        default: false
    constraints:
      - quantity is a decimal; never a float.
      - spam is true when the asset is absent from the canonical registry; a spam holding has observedToken and no assetId.
      - Pricing and dust are Portfolio's job: Account records no price and no dust flag.

  - name: ImportBatch
    description: One CSV import into a csv account (Account owns the import surface).
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: accountId
        type: uuid
        required: true
      - name: source
        type: enum
        required: true
        allowed_values: [binance, okx, generic]
      - name: fileName
        type: string
        required: true
      - name: importedCount
        type: integer
        required: true
        min: 0
      - name: skippedCount
        type: integer
        required: true
        min: 0
      - name: failedCount
        type: integer
        required: true
        min: 0
      - name: createdAt
        type: timestamp
        required: true
    constraints:
      - Imported plus skipped plus failed equals the number of data rows.

  - name: ImportedTransaction
    description: One transaction row parsed by a CSV import, before Ledger consumes it.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: batchId
        type: uuid
        required: true
        references:
          entity: ImportBatch
          relationship: many rows belong to one batch
      - name: accountId
        type: uuid
        required: true
      - name: externalId
        type: string
        required: false
        description: The venue's own row id when the CSV carries one.
      - name: dedupeKey
        type: string
        required: true
        description: (accountId, externalId) when present, otherwise (accountId, kind, asset, quantity, executedAt).
      - name: assetId
        type: string
        required: true
      - name: kind
        type: enum
        required: true
        allowed_values: [buy, sell, transfer_in, transfer_out, fee]
      - name: quantity
        type: decimal
        required: true
      - name: price
        type: decimal
        required: false
      - name: executedAt
        type: timestamp
        required: true
    constraints:
      - dedupeKey is unique per account.
      - A row that cannot be parsed is an ImportRowError, not an ImportedTransaction.

  - name: ImportRowError
    description: One bad CSV row, with its line number and reason.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: batchId
        type: uuid
        required: true
        references:
          entity: ImportBatch
          relationship: many errors belong to one batch
      - name: lineNumber
        type: integer
        required: true
        min: 1
      - name: reason
        type: string
        required: true

relationships:
  - from: Account
    to: Credential
    cardinality: one-to-one-active
    direction: Account owns Credential
  - from: Account
    to: WalletConnection
    cardinality: one-to-one
    direction: Account owns WalletConnection
  - from: Account
    to: SyncRun
    cardinality: one-to-many
    direction: Account owns SyncRun
  - from: SyncRun
    to: HoldingSnapshot
    cardinality: one-to-many
    direction: SyncRun owns HoldingSnapshot
  - from: Account
    to: ImportBatch
    cardinality: one-to-many
    direction: Account owns ImportBatch
  - from: ImportBatch
    to: ImportedTransaction
    cardinality: one-to-many
    direction: ImportBatch owns ImportedTransaction
  - from: ImportBatch
    to: ImportRowError
    cardinality: one-to-many
    direction: ImportBatch owns ImportRowError
```

## Summary

Account owns the read-only sources of holdings — exchange credentials, public wallet addresses,
WalletConnect references, and CSV imports — plus the sync runs, the per-sync holding snapshots, and
the import batches with their imported rows and per-line errors. Credentials are Vault references
only; no key, token, private key, or seed phrase is stored or logged. Holdings are decimal
quantities with the observed token identity and, when the registry resolves it, a canonical asset
id; a spam holding keeps only the observed token. Pricing and dust belong to Portfolio, which
already values holdings.
